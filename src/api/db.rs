use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use entity::collection::Model;
pub(crate) use entity::{DELETED_AT_FIELD, DELETED_BY_FIELD};
use migration::CollectionDocument;
use migration::Grant;
use sea_orm::QueryResult;
use sea_orm::{
    ActiveModelTrait, ActiveValue::NotSet, ColumnTrait, ConnectionTrait, DatabaseConnection,
    DatabaseTransaction, EntityTrait, FromQueryResult, JsonValue, QueryFilter, Set, Statement,
};
use sea_orm::{DbErr, ModelTrait, QuerySelect};
use sea_query::Cond;
use sea_query::Func;
use sea_query::Iden;
use sea_query::{Alias, Condition, Expr, JoinType, Order, Query, SelectStatement, SimpleExpr};
use serde::Deserialize;
use std::ops::Sub;
use tracing::{debug, error, info};
use typed_builder::TypedBuilder;
use uuid::Uuid;

use crate::api::{
    create_document::create_document_event,
    dto::{self, Event, MailMessage},
    hooks::CronDocumentSelector,
    types::Pagination,
    ApiContext, ApiErrors,
};
use entity::collection_document::Column as DocumentsColumns;
use entity::collection_document::Entity as Documents;
use std::result;

use super::hooks::GrantSettingsOnEvents;
use super::hooks::{
    StoreDocument, StoreNewDocument, StoreNewDocumentCollection, StoreNewDocumentOwner,
};

pub(crate) async fn get_unlocked_collection_by_name(
    db: &DatabaseConnection,
    collection_name: &str,
) -> Option<Model> {
    get_collection_by_name(db, collection_name)
        .await
        .and_then(|c| if c.locked { None } else { Some(c) })
}

pub(crate) async fn get_collection_by_name(
    db: &DatabaseConnection,
    collection_name: &str,
) -> Option<Model> {
    let query_result = entity::collection::Entity::find()
        .filter(entity::collection::Column::Name.eq(collection_name))
        .one(db)
        .await;

    match query_result {
        Ok(Some(col)) => {
            debug!("Collection with name {} has id {}", collection_name, col.id);
            Some(col)
        }
        Ok(None) => {
            info!("Collection not found: {}", collection_name);
            None
        }
        Err(dberr) => {
            error!(
                "Failed to check if collection {} is locked: {}",
                collection_name, dberr
            );
            None
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum CollectionDocumentVisibility {
    PrivateAndUserCanAccessAllDocuments,
    PrivateAndUserIs(Uuid),
    PublicAndUserIsReader,
}

#[derive(Debug, Clone)]
pub(crate) enum FieldFilter {
    ExactFieldMatch {
        field_name: String,
        value: String,
    },
    FieldContains {
        field_name: String,
        value: String,
    },
    FieldStartsWith {
        field_name: String,
        value: String,
    },
    FieldValueInMatch {
        field_name: String,
        values: Vec<String>,
    },
    FieldIsNull {
        field_name: String,
    },
    FieldIsNotNull {
        field_name: String,
    },
    DateFieldLessThan {
        field_name: String,
        value: DateTime<Utc>,
    },
}

impl FieldFilter {
    pub(crate) fn from_str(s: &str) -> Option<FieldFilter> {
        if s.is_empty() {
            return None;
        }

        // Remove quotes
        let value_trimmer =
            |value: &str| -> String { value.trim_matches('"').trim_matches('\'').to_string() };

        // Split at first equal sign
        let (field_name, value) = s.split_once('=')?;

        if value.starts_with('~') {
            let value = value.trim_start_matches('~');
            return Some(FieldFilter::FieldContains {
                field_name: field_name.to_string(),
                value: value_trimmer(value),
            });
        }

        if value.starts_with('@') {
            let value = value.trim_start_matches('@');
            return Some(FieldFilter::FieldStartsWith {
                field_name: field_name.to_string(),
                value: value_trimmer(value),
            });
        }

        // If value is inside square brackets, then it's a list of values
        if value.starts_with('[') && value.ends_with(']') {
            let values: Vec<String> = value[1..value.len() - 1]
                .split(',')
                .map(|v| value_trimmer(v.trim()))
                .collect();

            return if values.is_empty() {
                None
            } else {
                Some(FieldFilter::FieldValueInMatch {
                    field_name: field_name.to_string(),
                    values,
                })
            };
        }

        Some(FieldFilter::ExactFieldMatch {
            field_name: field_name.to_string(),
            value: value_trimmer(value),
        })
    }
}

impl From<CronDocumentSelector> for FieldFilter {
    fn from(cds: CronDocumentSelector) -> Self {
        match cds {
            CronDocumentSelector::ByFieldEqualsValue { field, value } => {
                FieldFilter::ExactFieldMatch {
                    field_name: field,
                    value,
                }
            }
            CronDocumentSelector::ByDateFieldOlderThan { field, value } => {
                FieldFilter::DateFieldLessThan {
                    field_name: field,
                    value: chrono::Utc::now().sub(value),
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) enum ListDocumentGrants {
    IgnoredForCron,
    IgnoredForAdmin,
    Restricted(Vec<dto::Grant>),
}

#[derive(Debug, Clone, TypedBuilder)]
pub(crate) struct DbListDocumentParams {
    pub(crate) collection: Uuid,
    pub(crate) exact_title: Option<String>,
    pub(crate) grants: ListDocumentGrants,
    pub(crate) extra_fields: Vec<String>,
    pub(crate) sort_fields: Option<String>,
    pub(crate) filters: Vec<FieldFilter>,
    #[builder(default)]
    pub(crate) pagination: Pagination,
}

pub(crate) async fn list_documents(
    db: &DatabaseConnection,
    params: &DbListDocumentParams,
) -> Result<(u32, Vec<JsonValue>), ApiErrors> {
    let count_sql = count_documents_sql(params);
    let count_stmt = db.get_database_backend().build(&count_sql);
    let query_res: Option<QueryResult> = db.query_one(count_stmt).await?;
    let query_res = query_res.unwrap();
    let total = query_res
        .try_get_by(0)
        .map(|count: i64| u32::try_from(count).unwrap_or(u32::MAX))?;

    let sql = select_documents_sql(params)
        .limit(params.pagination.limit().into())
        .offset(params.pagination.offset().into())
        .to_owned();
    let builder = db.get_database_backend();
    let stmt: Statement = builder.build(&sql);

    let items: Vec<JsonValue> = JsonValue::find_by_statement(stmt)
        .all(db)
        .await
        .map_err(ApiErrors::from)?;

    Ok((total, items))
}

#[derive(FromQueryResult, Debug, Deserialize)]
struct IdOnly {
    pub(crate) id: Uuid,
}

pub(crate) async fn list_document_ids(
    db: &DatabaseTransaction,
    collection_id: Uuid,
) -> Result<Vec<Uuid>, ApiErrors> {
    let items = Documents::find()
        .select_only()
        .column(DocumentsColumns::Id)
        .filter(DocumentsColumns::CollectionId.eq(collection_id))
        .into_model::<IdOnly>()
        .all(db)
        .await?;
    debug!("Found {} documents", items.len());
    Ok(items.into_iter().map(|item| (item.id)).collect())
}

fn grants_conditions(user_grants: &Vec<dto::Grant>) -> Condition {
    let mut grant_conditions = Cond::any();
    for user_grant in user_grants {
        grant_conditions = grant_conditions.add(
            Cond::all()
                .add(Expr::col((Grant::Table, Grant::Realm)).eq(user_grant.realm()))
                .add(Expr::col((Grant::Table, Grant::Grant)).eq(user_grant.grant_id())),
        );
    }
    grant_conditions
}

struct SortField(String);
impl Iden for SortField {
    fn prepare(&self, s: &mut dyn std::fmt::Write, _q: sea_query::Quote) {
        self.unquoted(s);
    }

    fn unquoted(&self, s: &mut dyn std::fmt::Write) {
        write!(s, "{}", self.0).unwrap();
    }
}

fn base_documents_sql(params: &DbListDocumentParams) -> (SelectStatement, Alias) {
    let documents_alias = Alias::new("d");
    let mut b = Query::select();
    let mut q = b.from_as(Documents, documents_alias.clone());
    match params.grants {
        ListDocumentGrants::IgnoredForCron => {
            debug!("No grant restrictions for cron access");
        }
        ListDocumentGrants::IgnoredForAdmin => {
            info!("No grant restrictions for user with admin role");
        }
        ListDocumentGrants::Restricted(ref user_grants) => {
            q.join(
                JoinType::Join,
                Grant::Table,
                Expr::col((documents_alias.clone(), CollectionDocument::Id))
                    .equals((Grant::Table, Grant::DocumentId)),
            )
            .and_where(Expr::col(DocumentsColumns::CollectionId).eq(params.collection));
            q = q.cond_where(grants_conditions(user_grants));
        }
    }

    for filter in &params.filters {
        match filter {
            FieldFilter::ExactFieldMatch { field_name, value } => {
                q = q.and_where(Expr::cust_with_values(
                    format!(r#""d"."f"{}=$1"#, field_path_json(field_name),),
                    vec![value],
                ));
            }
            FieldFilter::FieldContains { field_name, value } => {
                q = q.and_where(Expr::cust_with_values(
                    format!(r#"lower("d"."f"{}) like $1"#, field_path_json(field_name),),
                    vec![format!("%{}%", value.to_lowercase())],
                ))
            }
            FieldFilter::FieldStartsWith { field_name, value } => {
                q = q.and_where(Expr::cust_with_values(
                    format!(r#"lower("d"."f"{}) like $1"#, field_path_json(field_name),),
                    vec![format!("{}%", value.to_lowercase())],
                ))
            }
            FieldFilter::FieldValueInMatch { field_name, values } => {
                q = q.and_where(
                    Expr::expr(Expr::cust(format!(
                        r#""d"."f"{}"#,
                        field_path_json(field_name),
                    )))
                    .is_in(values),
                );
            }
            FieldFilter::DateFieldLessThan { field_name, value } => {
                q = q.and_where(Expr::cust_with_values(
                    format!(r#""d"."f"{} < $1"#, field_path_json(field_name),),
                    vec![format!("{}%", value)],
                ))
            }
            FieldFilter::FieldIsNull { field_name } => {
                q = q.and_where(
                    Expr::expr(Expr::cust(format!(
                        r#""d"."f"{}"#,
                        field_path_json(field_name),
                    )))
                    .is_null(),
                );
            }
            FieldFilter::FieldIsNotNull { field_name } => {
                q = q.and_where(
                    Expr::expr(Expr::cust(format!(
                        r#""d"."f"{}"#,
                        field_path_json(field_name),
                    )))
                    .is_not_null(),
                );
            }
        }
    }

    if let Some(title) = &params.exact_title {
        q = q.and_where(Expr::cust_with_values(r#""f"->>'title' = $1"#, [title]));
    }

    (q.to_owned(), documents_alias)
}

fn count_documents_sql(params: &DbListDocumentParams) -> SelectStatement {
    let (mut q, alias) = base_documents_sql(params);
    q.expr(Func::count(Expr::cust_with_expr(
        "DISTINCT $1",
        Expr::col((alias, CollectionDocument::Id)),
    )))
    .to_owned()
}

fn select_documents_sql(params: &DbListDocumentParams) -> SelectStatement {
    let j: SelectStatement = Query::select()
        .expr(Expr::cust_with_expr(
            r#"jsonb_object_agg("key", "value") as "new_f" from jsonb_each("f") as x("key", "value") WHERE "key" in $1"#,
            SimpleExpr::Tuple(params.extra_fields.iter().cloned().map(|s| s.into()).collect()),
        ))
        .to_owned();
    let (mut id_select, documents_alias) = base_documents_sql(params);
    id_select
        .distinct()
        .column((documents_alias, DocumentsColumns::Id));

    let documents_alias = Alias::new("d");
    let mut document_select = Query::select();
    document_select
        .column((documents_alias.clone(), CollectionDocument::Id))
        .from_as(CollectionDocument::Table, documents_alias.clone())
        .expr_as(Expr::cust(r#""t"."new_f""#), Alias::new("f"))
        .join_lateral(
            JoinType::InnerJoin,
            j,
            sea_orm::IntoIdentity::into_identity("t"),
            Condition::all(),
        )
        .and_where(
            Expr::col((documents_alias.clone(), CollectionDocument::Id)).in_subquery(id_select),
        );

    let sort_fields = sort_fields_parser(params.sort_fields.as_ref().cloned());
    for sort_field in sort_fields {
        document_select.order_by_expr(Expr::cust(sort_field.0), sort_field.1);
    }

    document_select.to_owned()
}

fn sort_fields_parser(fields: Option<String>) -> Vec<(String, Order)> {
    fields
        .unwrap_or_else(|| "created+".to_string())
        .split(',')
        .map(|s| {
            let mut char_vec_from_s = s.chars().collect::<Vec<char>>();
            let last_character = char_vec_from_s.pop().unwrap();
            let field_name = char_vec_from_s.into_iter().collect::<String>();

            match last_character {
                '+' => (
                    format!(r#""d"."f"{}"#, field_path_json(&field_name)),
                    Order::Asc,
                ),
                '-' => (
                    format!(r#""d"."f"{}"#, field_path_json(&field_name)),
                    Order::Desc,
                ),
                'f' => (
                    format!(r#""d"."f"{}"#, field_path_json_native(&field_name)),
                    Order::Asc,
                ),
                'b' => (
                    format!(r#""d"."f"{}"#, field_path_json_native(&field_name)),
                    Order::Desc,
                ),
                _ => unreachable!(),
            }
        })
        .collect()
}

fn field_path_json_native(field_name: &str) -> String {
    // split field_name on dots
    let field_struct = field_name
        .split('.')
        .map(|s| format!("'{s}'"))
        .collect::<Vec<String>>();
    let field_path = field_struct.join("->");
    format!(r#"->{field_path}"#)
}

fn field_path_json(field_name: &str) -> String {
    if !field_name.contains('.') {
        return format!(r#"->>'{field_name}'"#);
    }
    // split field_name on dots
    let mut field_struct = field_name
        .split('.')
        .map(|s| format!("'{s}'"))
        .collect::<Vec<String>>();
    let field_name = field_struct.pop().unwrap();
    let field_path = field_struct.join("->");
    format!(r#"->{field_path}->>{field_name}"#)
}

pub(crate) struct InsertDocumentData {
    pub(crate) collection_id: Uuid,
}

pub(crate) enum DbGrantUpdate {
    Keep,
    Replace(Vec<dto::GrantForDocument>),
}

impl From<GrantSettingsOnEvents> for DbGrantUpdate {
    fn from(value: GrantSettingsOnEvents) -> Self {
        match value {
            GrantSettingsOnEvents::NoChange => Self::Keep,
            GrantSettingsOnEvents::Replace(grants) => Self::Replace(grants),
        }
    }
}

pub(crate) async fn save_document_events_mails(
    txn: &DatabaseTransaction,
    user: &dto::User,
    document: Option<dto::CollectionDocument>,
    insert: Option<InsertDocumentData>,
    events: Vec<Event>,
    grants: DbGrantUpdate,
    mails: Vec<MailMessage>,
) -> anyhow::Result<()> {
    let mut documents = Vec::with_capacity(1);
    if let Some(document) = document {
        debug!("Mapping document");
        documents.push(match insert {
            Some(insert_data) => StoreDocument::as_new(StoreNewDocument {
                owner: StoreNewDocumentOwner::User(user.clone()),
                collection: StoreNewDocumentCollection::Id(insert_data.collection_id),
                document,
            }),
            None => StoreDocument::Update { document },
        });
    };
    save_documents_events_mails(txn, user, documents, events, grants, mails).await
}

pub(crate) async fn save_documents_events_mails(
    txn: &DatabaseTransaction,
    user: &dto::User,
    documents: Vec<StoreDocument>,
    events: Vec<Event>,
    grants: DbGrantUpdate,
    mails: Vec<MailMessage>,
) -> anyhow::Result<()> {
    let mut document_created_events = Vec::with_capacity(documents.len());
    for document in documents {
        debug!("Saving document");
        match document {
            StoreDocument::New(n) => {
                let collection_id = match n.collection {
                    crate::api::hooks::StoreNewDocumentCollection::Name(ref collection_name) => {
                        entity::collection::Entity::find()
                            .filter(entity::collection::Column::Name.eq(collection_name))
                            .one(txn)
                            .await?
                            .ok_or_else(|| {
                                error!("Could not find collection {collection_name}");
                                ApiErrors::InternalServerError
                            })?
                            .id
                    }
                    crate::api::hooks::StoreNewDocumentCollection::Id(id) => id,
                };
                let owner = match n.owner {
                    StoreNewDocumentOwner::User(ref u) => u,
                    StoreNewDocumentOwner::Callee => user,
                };

                let document_created_event = create_document_event(*(n.document.id()), owner);
                document_created_events.push(document_created_event);

                entity::collection_document::ActiveModel {
                    id: Set(*n.document.id()),
                    owner: Set(owner.id()),
                    collection_id: Set(collection_id),
                    f: Set(n.document.fields().clone()),
                }
                .insert(txn)
                .await
                .context("Saving new document")?;
            }
            StoreDocument::Update { document } => {
                entity::collection_document::ActiveModel {
                    id: Set(*document.id()),
                    owner: NotSet,
                    collection_id: NotSet,
                    f: Set(document.fields().clone()),
                }
                .save(txn)
                .await
                .context("Updating document")?;
            }
        };
    }

    let all_events: Vec<dto::Event> = document_created_events.into_iter().chain(events).collect();

    match grants {
        DbGrantUpdate::Keep => debug!("No grants changed"),
        DbGrantUpdate::Replace(grants) => {
            debug!("Try to update {} grant(s)", grants.len());
            let mut related_grants = Vec::new();
            grants.iter().for_each(|g| {
                let document_id = g.document_id();
                if !related_grants.contains(&document_id) {
                    related_grants.push(document_id);
                }
            });
            debug!("Removing grants for documents {:?}", related_grants);
            entity::grant::Entity::delete_many()
                .filter(entity::grant::Column::DocumentId.is_in(related_grants))
                .exec(txn)
                .await?;
            for grant_for_document in grants {
                let document_id = grant_for_document.document_id();
                let grant = grant_for_document.grant();
                let dbgrant = entity::grant::ActiveModel {
                    id: NotSet,
                    document_id: Set(document_id),
                    realm: Set(grant.realm().into()),
                    grant: Set(grant.grant_id()),
                    view: Set(grant.view()),
                };
                let res = dbgrant
                    .save(txn)
                    .await
                    .with_context(|| format!("Saving grant {:?}", grant_for_document))?;
                debug!("Grant {:?} saved ({})", grant_for_document, res.id.unwrap());
            }
        }
    }

    debug!("Try to create {} event(s)", all_events.len());
    for event in all_events {
        // Create the event in the database
        let dbevent = entity::event::ActiveModel {
            id: NotSet,
            category_id: Set(event.category()),
            timestamp: NotSet,
            document_id: Set(event.document_id()),
            user: Set(user.id()),
            payload: Set(event.payload().clone()),
        };
        let res = dbevent.save(txn).await.context("Saving event")?;

        debug!("Event {} saved", res.id.unwrap());
    }

    debug!("Trying to store {} mail(s) in queue", mails.len());
    for mailmessage in mails {
        let document_fields =
            serde_json::to_value(mailmessage).expect("Failed to serialize mail message");
        entity::collection_document::ActiveModel {
            id: Set(Uuid::new_v4()),
            owner: Set(*crate::cron::CRON_USER_ID),
            collection_id: Set(*crate::mail::FOLIVAFY_MAIL_COLLECTION_ID),
            f: Set(document_fields),
        }
        .insert(txn)
        .await
        .context("Saving new document")?;
    }
    Ok(())
}

pub(crate) async fn replace_grants(
    txn: &DatabaseTransaction,
    grants: Vec<dto::GrantForDocument>,
) -> Result<()> {
    debug!("Try to update {} grant(s)", grants.len());
    let mut related_grants = Vec::new();
    grants.iter().for_each(|g| {
        let document_id = g.document_id();
        if !related_grants.contains(&document_id) {
            related_grants.push(document_id);
        }
    });
    debug!("Removing grants for documents {:?}", related_grants);
    entity::grant::Entity::delete_many()
        .filter(entity::grant::Column::DocumentId.is_in(related_grants))
        .exec(txn)
        .await?;
    for grant_for_document in grants {
        let document_id = grant_for_document.document_id();
        let grant = grant_for_document.grant();
        let dbgrant = entity::grant::ActiveModel {
            id: NotSet,
            document_id: Set(document_id),
            realm: Set(grant.realm().into()),
            grant: Set(grant.grant_id()),
            view: Set(grant.view()),
        };
        let res = dbgrant
            .save(txn)
            .await
            .with_context(|| format!("Saving grant {:?}", grant_for_document))?;
        debug!("Grant {:?} saved ({})", grant_for_document, res.id.unwrap());
    }

    Ok(())
}

pub(crate) async fn get_document_by_id(
    document_uuid: Uuid,
    db: &DatabaseConnection,
) -> core::result::Result<Option<entity::collection_document::Model>, DbErr> {
    Documents::find_by_id(document_uuid).one(db).await
}
pub(crate) async fn get_document_by_id_in_trx(
    document_uuid: Uuid,
    db: &DatabaseTransaction,
) -> core::result::Result<Option<entity::collection_document::Model>, DbErr> {
    Documents::find_by_id(document_uuid).one(db).await
}

pub(crate) async fn get_accessible_document(
    ctx: &ApiContext,
    user_grants: &[dto::Grant],
    user_id: Uuid,
    collection: &Model,
    document_uuid: Uuid,
) -> result::Result<Option<entity::collection_document::Model>, ApiErrors> {
    let doc = get_document_by_id(document_uuid, &ctx.db)
        .await?
        .and_then(|doc| (doc.collection_id == collection.id).then_some(doc));
    if doc.is_none() {
        debug!("Document ({document_uuid}) not found",);
        return Ok(None);
    }
    let doc = doc.unwrap();

    // Load referenced document grants:
    let document_grants = doc
        .find_related(entity::grant::Entity)
        .all(&ctx.db)
        .await
        .map_err(|e| {
            error!("Error loading document ({document_uuid}) grants: {}", e);
            ApiErrors::InternalServerError
        })?;

    // Compare user grants with document grants
    let intersection = user_grants.iter().any(|user_grant| {
        document_grants
            .iter()
            .any(|document_grant| user_grant == document_grant)
    });
    if !intersection {
        info!("User {user_id} does not have access to document ({document_uuid})",);
        return Ok(None);
    }

    // Do not provide document if it has been deleted
    if doc.is_deleted() {
        debug!("Document ({document_uuid}) is deleted",);
        return Ok(None);
    }

    Ok(Some(doc))
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use sea_query::PostgresQueryBuilder;
    use validator::Validate;

    use crate::api::db::ListDocumentGrants::Restricted;
    use crate::api::{
        grants::{default_user_grants, DefaultUserGrantsParameters},
        list_documents::ListDocumentParams,
    };

    use super::*;

    #[test]
    fn param_validation_test() {
        let all_fields_empty = ListDocumentParams {
            exact_title: None,
            extra_fields: None,
            sort_fields: None,
            pfilter: None,
        };

        assert!(all_fields_empty.validate().is_ok());

        let valid_sort_fields = ListDocumentParams {
            exact_title: None,
            extra_fields: None,
            sort_fields: Some("title+,price-,length-".to_string()),
            pfilter: None,
        };
        assert!(valid_sort_fields.validate().is_ok());

        let invalid_sort_fields = ListDocumentParams {
            exact_title: None,
            extra_fields: None,
            sort_fields: Some("title,price-".to_string()),
            pfilter: None,
        };
        assert!(invalid_sort_fields.validate().is_err());

        let invalid_extra_fields = ListDocumentParams {
            exact_title: None,
            extra_fields: Some("title📣".to_string()),
            sort_fields: None,
            pfilter: None,
        };
        assert!(invalid_extra_fields.validate().is_err());
    }

    #[test]
    fn sort_fields_sql_test_simple_native() {
        // Arrange
        let sort_fields = "title+,priceb,lengthf".to_string();

        // Act
        let sql = sort_fields_parser(Some(sort_fields));

        // Assert
        assert_eq!(
            sql,
            vec![
                ("\"d\".\"f\"->>'title'".to_string(), Order::Asc),
                ("\"d\".\"f\"->'price'".to_string(), Order::Desc),
                ("\"d\".\"f\"->'length'".to_string(), Order::Asc),
            ]
        );
    }

    #[test]
    fn sort_fields_sql_test_simple() {
        // Arrange
        let sort_fields = "title+,price-,length-".to_string();

        // Act
        let sql = sort_fields_parser(Some(sort_fields));

        // Assert
        assert_eq!(
            sql,
            vec![
                ("\"d\".\"f\"->>'title'".to_string(), Order::Asc),
                ("\"d\".\"f\"->>'price'".to_string(), Order::Desc),
                ("\"d\".\"f\"->>'length'".to_string(), Order::Desc),
            ]
        );
    }

    #[test]
    fn sort_fields_sql_test_subfield() {
        // Arrange
        let sort_fields = "title+,company.title-,supplier.city+".to_string();

        // Act
        let sql = sort_fields_parser(Some(sort_fields));

        // Assert
        assert_eq!(
            sql,
            vec![
                ("\"d\".\"f\"->>'title'".to_string(), Order::Asc),
                ("\"d\".\"f\"->'company'->>'title'".to_string(), Order::Desc),
                ("\"d\".\"f\"->'supplier'->>'city'".to_string(), Order::Asc),
            ]
        );
    }

    #[test]
    fn sort_fields_sql_test_subfield_native() {
        // Arrange
        let sort_fields = "title+,item.priceb,m.lengthf".to_string();

        // Act
        let sql = sort_fields_parser(Some(sort_fields));

        // Assert
        assert_eq!(
            sql,
            vec![
                ("\"d\".\"f\"->>'title'".to_string(), Order::Asc),
                ("\"d\".\"f\"->'item'->'price'".to_string(), Order::Desc),
                ("\"d\".\"f\"->'m'->'length'".to_string(), Order::Asc),
            ]
        );
    }

    #[test]
    fn test_count_documents_query1() {
        // Arrange
        let collection = Uuid::new_v4();
        let userid = Uuid::new_v4();
        let sort_fields = "created+".to_string();
        let grants = default_user_grants(
            DefaultUserGrantsParameters::builder()
                .visibility(CollectionDocumentVisibility::PrivateAndUserIs(userid))
                .collection_uuid(collection)
                .build(),
        );
        let params = DbListDocumentParams::builder()
            .collection(collection)
            .extra_fields(vec!["title".to_string()])
            .exact_title(None)
            .sort_fields(Some(sort_fields))
            .filters(vec![])
            .grants(Restricted(grants))
            .build();

        // Act
        let sql = count_documents_sql(&params).to_string(PostgresQueryBuilder);

        // Assert
        assert_eq!(
            sql,
            format!(
                r#"SELECT COUNT(DISTINCT "d"."id") FROM "collection_document" AS "d" JOIN "grant" ON "d"."id" = "grant"."document_id" WHERE "collection_id" = '{collection}' AND ("grant"."realm" = 'author' AND "grant"."grant" = '{userid}')"#
            )
        );
    }

    #[test]
    fn test_count_documents_query2() {
        // Arrange
        let collection = Uuid::new_v4();
        let sort_fields = "created+".to_string();
        let grants = default_user_grants(
            DefaultUserGrantsParameters::builder()
                .visibility(CollectionDocumentVisibility::PublicAndUserIsReader)
                .collection_uuid(collection)
                .build(),
        );
        let params = DbListDocumentParams::builder()
            .collection(collection)
            .extra_fields(vec!["title".to_string()])
            .exact_title(None)
            .sort_fields(Some(sort_fields))
            .filters(vec![])
            .grants(Restricted(grants))
            .build();

        // Act
        let sql = count_documents_sql(&params).to_string(PostgresQueryBuilder);

        // Assert
        assert_eq!(
            sql,
            format!(
                r#"SELECT COUNT(DISTINCT "d"."id") FROM "collection_document" AS "d" JOIN "grant" ON "d"."id" = "grant"."document_id" WHERE "collection_id" = '{collection}' AND ("grant"."realm" = 'read-collection' AND "grant"."grant" = '{collection}')"#
            )
        );
    }

    #[test]
    fn test_count_documents_query3() {
        // Arrange
        let collection = Uuid::new_v4();
        let sort_fields = "created+".to_string();
        let grants = default_user_grants(
            DefaultUserGrantsParameters::builder()
                .visibility(CollectionDocumentVisibility::PrivateAndUserCanAccessAllDocuments)
                .collection_uuid(collection)
                .build(),
        );
        let params = DbListDocumentParams::builder()
            .collection(collection)
            .extra_fields(vec!["title".to_string()])
            .exact_title(None)
            .sort_fields(Some(sort_fields))
            .filters(vec![])
            .grants(Restricted(grants))
            .build();

        // Act
        let sql = count_documents_sql(&params).to_string(PostgresQueryBuilder);
        // Assert
        assert_eq!(
            sql,
            format!(
                r#"SELECT COUNT(DISTINCT "d"."id") FROM "collection_document" AS "d" JOIN "grant" ON "d"."id" = "grant"."document_id" WHERE "collection_id" = '{collection}' AND ("grant"."realm" = 'read-all-collection' AND "grant"."grant" = '{collection}')"#
            )
        );
    }

    #[test]
    fn test_select_documents_sql_basic_query() {
        // Arrange
        let collection = Uuid::new_v4();
        let userid = Uuid::new_v4();
        let sort_fields = "created+".to_string();
        let grants = default_user_grants(
            DefaultUserGrantsParameters::builder()
                .visibility(CollectionDocumentVisibility::PrivateAndUserIs(userid))
                .collection_uuid(collection)
                .build(),
        );
        let params = DbListDocumentParams::builder()
            .collection(collection)
            .extra_fields(vec!["title".to_string()])
            .exact_title(None)
            .sort_fields(Some(sort_fields))
            .filters(vec![])
            .grants(Restricted(grants))
            .build();

        // Act
        let sql = select_documents_sql(&params).to_string(PostgresQueryBuilder);

        // Assert
        assert_eq!(
            sql,
            format!(
                r#"SELECT "d"."id", "t"."new_f" AS "f" FROM "collection_document" AS "d" INNER JOIN LATERAL (SELECT jsonb_object_agg("key", "value") as "new_f" from jsonb_each("f") as x("key", "value") WHERE "key" in ('title')) AS "t" ON TRUE WHERE "d"."id" IN (SELECT DISTINCT "d"."id" FROM "collection_document" AS "d" JOIN "grant" ON "d"."id" = "grant"."document_id" WHERE "collection_id" = '{collection}' AND ("grant"."realm" = 'author' AND "grant"."grant" = '{userid}')) ORDER BY "d"."f"->>'created' ASC"#
            )
        );
    }

    #[test]
    fn test_select_documents_sql_query2() {
        // Arrange
        let collection = Uuid::new_v4();
        let sort_fields = "created+".to_string();
        let filters = vec![
            FieldFilter::ExactFieldMatch {
                field_name: "orgaddr.zip".to_string(),
                value: "11101".to_string(),
            },
            FieldFilter::FieldValueInMatch {
                field_name: "wf1.seq".to_string(),
                values: vec!["1".to_string(), "2".to_string()],
            },
        ];
        let grants = default_user_grants(
            DefaultUserGrantsParameters::builder()
                .visibility(CollectionDocumentVisibility::PublicAndUserIsReader)
                .collection_uuid(collection)
                .build(),
        );
        let params = DbListDocumentParams::builder()
            .collection(collection)
            .extra_fields(vec!["title".to_string()])
            .exact_title(None)
            .sort_fields(Some(sort_fields))
            .filters(filters)
            .grants(Restricted(grants))
            .build();

        // Act
        let sql = select_documents_sql(&params).to_string(PostgresQueryBuilder);

        // Assert
        assert_eq!(
            sql,
            format!(
                r#"SELECT "d"."id", "t"."new_f" AS "f" FROM "collection_document" AS "d" INNER JOIN LATERAL (SELECT jsonb_object_agg("key", "value") as "new_f" from jsonb_each("f") as x("key", "value") WHERE "key" in ('title')) AS "t" ON TRUE WHERE "d"."id" IN (SELECT DISTINCT "d"."id" FROM "collection_document" AS "d" JOIN "grant" ON "d"."id" = "grant"."document_id" WHERE "collection_id" = '{collection}' AND ("grant"."realm" = 'read-collection' AND "grant"."grant" = '{collection}') AND ("d"."f"->'orgaddr'->>'zip'='11101') AND ("d"."f"->'wf1'->>'seq') IN ('1', '2')) ORDER BY "d"."f"->>'created' ASC"#
            )
        );
    }

    #[test]
    fn test_select_documents_sql_query1() {
        // Arrange
        let collection = Uuid::new_v4();
        let userid = Uuid::new_v4();
        let sort_fields = "created+".to_string();
        let filters = vec![CronDocumentSelector::ByFieldEqualsValue {
            field: "orgaddr.zip".to_string(),
            value: "11101".to_string(),
        }
        .into()];
        let grants = default_user_grants(
            DefaultUserGrantsParameters::builder()
                .visibility(CollectionDocumentVisibility::PrivateAndUserIs(userid))
                .collection_uuid(collection)
                .build(),
        );
        let params = DbListDocumentParams::builder()
            .collection(collection)
            .extra_fields(vec!["title".to_string()])
            .exact_title(None)
            .sort_fields(Some(sort_fields))
            .filters(filters)
            .grants(Restricted(grants))
            .build();

        // Act
        let sql = select_documents_sql(&params).to_string(PostgresQueryBuilder);

        // Assert
        assert_eq!(
            sql,
            format!(
                r#"SELECT "d"."id", "t"."new_f" AS "f" FROM "collection_document" AS "d" INNER JOIN LATERAL (SELECT jsonb_object_agg("key", "value") as "new_f" from jsonb_each("f") as x("key", "value") WHERE "key" in ('title')) AS "t" ON TRUE WHERE "d"."id" IN (SELECT DISTINCT "d"."id" FROM "collection_document" AS "d" JOIN "grant" ON "d"."id" = "grant"."document_id" WHERE "collection_id" = '{collection}' AND ("grant"."realm" = 'author' AND "grant"."grant" = '{userid}') AND ("d"."f"->'orgaddr'->>'zip'='11101')) ORDER BY "d"."f"->>'created' ASC"#
            )
        );
    }
}
