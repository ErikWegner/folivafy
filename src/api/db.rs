use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use entity::collection::Model;
pub(crate) use entity::{DELETED_AT_FIELD, DELETED_BY_FIELD};
use sea_orm::{
    ActiveModelTrait, ActiveValue::NotSet, ColumnTrait, ConnectionTrait, DatabaseConnection,
    DatabaseTransaction, EntityTrait, FromQueryResult, JsonValue, PaginatorTrait, QueryFilter, Set,
    Statement,
};
use sea_query::{
    Alias, Condition, Expr, JoinType, Order, Query, SelectStatement, SimpleExpr, Value,
};
use std::ops::Sub;
use tracing::{debug, error, info};
use uuid::Uuid;

use crate::api::auth::User;
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

use super::dto::Grant;
use super::hooks::{
    StoreDocument, StoreNewDocument, StoreNewDocumentCollection, StoreNewDocumentOwner,
};

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

impl CollectionDocumentVisibility {
    pub(crate) fn get_userid_for_sql_clause(&self) -> Option<Uuid> {
        match self {
            CollectionDocumentVisibility::PrivateAndUserCanAccessAllDocuments => None,
            CollectionDocumentVisibility::PrivateAndUserIs(userid) => Some(*userid),
            CollectionDocumentVisibility::PublicAndUserIsReader => None,
        }
    }
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
pub(crate) struct ListDocumentParams {
    pub(crate) collection: Uuid,
    pub(crate) exact_title: Option<String>,
    pub(crate) user_grants: Vec<Grant>,
    pub(crate) extra_fields: Vec<String>,
    pub(crate) sort_fields: Option<String>,
    pub(crate) filters: Vec<FieldFilter>,
    pub(crate) pagination: Pagination,
}

pub(crate) async fn list_documents(
    db: &DatabaseConnection,
    params: ListDocumentParams,
) -> Result<(u32, Vec<JsonValue>), ApiErrors> {
    let mut basefind = Documents::find()
        .filter(entity::collection_document::Column::CollectionId.eq(params.collection))
        .filter(Expr::cust(format!(
            r#""collection_document"."f"{} is null"#,
            field_path_json(DELETED_AT_FIELD),
        )));

    for filter in &params.filters {
        match filter {
            FieldFilter::ExactFieldMatch { field_name, value } => {
                basefind = basefind.filter(Expr::cust_with_values(
                    format!(
                        r#""collection_document"."f"{}=$1"#,
                        field_path_json(field_name),
                    ),
                    vec![value],
                ));
            }
            FieldFilter::FieldContains { field_name, value } => {
                basefind = basefind.filter(Expr::cust_with_values(
                    format!(
                        r#"lower("collection_document"."f"{}) like $1"#,
                        field_path_json(field_name),
                    ),
                    vec![format!("%{}%", value.to_lowercase())],
                ))
            }
            FieldFilter::FieldStartsWith { field_name, value } => {
                basefind = basefind.filter(Expr::cust_with_values(
                    format!(
                        r#"lower("collection_document"."f"{}) like $1"#,
                        field_path_json(field_name),
                    ),
                    vec![format!("{}%", value.to_lowercase())],
                ))
            }
            FieldFilter::FieldValueInMatch {
                field_name: _,
                values: _,
            } => {}
            FieldFilter::DateFieldLessThan { field_name, value } => {
                basefind = basefind.filter(Expr::cust_with_values(
                    format!(
                        r#"("collection_document"."f"{})::timestamp < $1"#,
                        field_path_json(field_name),
                    ),
                    vec![Value::ChronoDateTimeUtc(Some(Box::new(*value)))],
                ))
            }
            FieldFilter::FieldIsNull { field_name } => {
                basefind = basefind.filter(Expr::cust(format!(
                    r#""collection_document"."f"{} is null"#,
                    field_path_json(field_name),
                )))
            }
        }
    }

    if let Some(ref title) = params.exact_title {
        basefind = basefind.filter(sea_query::Expr::cust_with_values(
            r#""f"->>'title' = $1"#,
            [title],
        ));
    }

    // TODO: restore code
    // match params.oao_access {
    //     CollectionDocumentVisibility::PrivateAndUserCanAccessAllDocuments => {}
    //     CollectionDocumentVisibility::PrivateAndUserIs(uuid) => {
    //         basefind = basefind.filter(entity::collection_document::Column::Owner.eq(uuid));
    //     }
    //     CollectionDocumentVisibility::PublicAndUserIsReader => {}
    // }

    basefind = basefind.filter(Expr::cust(format!(
        r#"lower("collection_document"."f"{}) is null"#,
        field_path_json(DELETED_AT_FIELD),
    )));

    let total = basefind
        .clone()
        .count(db)
        .await
        .map_err(ApiErrors::from)
        .map(|t| u32::try_from(t).unwrap_or_default())?;

    let sql = select_documents_sql(
        &params.collection,
        params.extra_fields,
        &params.exact_title,
        params.sort_fields,
        params.filters,
    )
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

fn select_documents_sql(
    collection: &Uuid,
    extra_fields: Vec<String>,
    exact_title: &Option<String>,
    sort_fields: Option<String>,
    filters: Vec<FieldFilter>,
) -> SelectStatement {
    let j: SelectStatement = Query::select()
        .expr(Expr::cust_with_expr(
            r#"jsonb_object_agg("key", "value") as "new_f" from jsonb_each("f") as x("key", "value") WHERE "key" in $1"#,
            SimpleExpr::Tuple(extra_fields.into_iter().map(|s| s.into()).collect()),
        ))
        .to_owned();
    let mut b = Query::select();
    let mut q = b
        .from_as(Documents, Alias::new("d"))
        .column(DocumentsColumns::Id)
        // .order_by_expr(Expr::cust(r#""d"."f"->>'created'"#), Order::Asc)
        .expr_as(Expr::cust(r#""t"."new_f""#), Alias::new("f"))
        .join_lateral(
            JoinType::InnerJoin,
            j,
            sea_orm::IntoIdentity::into_identity("t"),
            Condition::all(),
        )
        .and_where(Expr::col(DocumentsColumns::CollectionId).eq(*collection));

    // TODO: restore code
    // if let Some(user_id) = oao_access.get_userid_for_sql_clause() {
    //     q = q.and_where(Expr::col(DocumentsColumns::Owner).eq(user_id));
    // }

    for filter in filters {
        match filter {
            FieldFilter::ExactFieldMatch { field_name, value } => {
                q = q.and_where(Expr::cust_with_values(
                    format!(r#""d"."f"{}=$1"#, field_path_json(&field_name),),
                    vec![value],
                ));
            }
            FieldFilter::FieldContains { field_name, value } => {
                q = q.and_where(Expr::cust_with_values(
                    format!(r#"lower("d"."f"{}) like $1"#, field_path_json(&field_name),),
                    vec![format!("%{}%", value.to_lowercase())],
                ))
            }
            FieldFilter::FieldStartsWith { field_name, value } => {
                q = q.and_where(Expr::cust_with_values(
                    format!(r#"lower("d"."f"{}) like $1"#, field_path_json(&field_name),),
                    vec![format!("{}%", value.to_lowercase())],
                ))
            }
            FieldFilter::FieldValueInMatch { field_name, values } => {
                q = q.and_where(
                    Expr::expr(Expr::cust(format!(
                        r#""d"."f"{}"#,
                        field_path_json(&field_name),
                    )))
                    .is_in(values),
                );
            }
            FieldFilter::DateFieldLessThan { field_name, value } => {
                q = q.and_where(Expr::cust_with_values(
                    format!(r#""d"."f"{} < $1"#, field_path_json(&field_name),),
                    vec![format!("{}%", value)],
                ))
            }
            FieldFilter::FieldIsNull { field_name } => {
                q = q.and_where(
                    Expr::expr(Expr::cust(format!(
                        r#""d"."f"{}"#,
                        field_path_json(&field_name),
                    )))
                    .is_null(),
                );
            }
        }
    }

    if let Some(title) = exact_title {
        q = q.and_where(Expr::cust_with_values(r#""f"->>'title' = $1"#, [title]));
    }

    let sort_fields = sort_fields_parser(sort_fields);
    for sort_field in sort_fields {
        q = q.order_by_expr(Expr::cust(sort_field.0), sort_field.1);
    }
    q.to_owned()
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

pub(crate) async fn save_document_events_mails(
    txn: &DatabaseTransaction,
    user: &dto::User,
    document: Option<dto::CollectionDocument>,
    insert: Option<InsertDocumentData>,
    events: Vec<Event>,
    grants: Vec<Grant>,
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
    save_documents_events_mails(txn, user, documents, events, mails).await
}

pub(crate) async fn save_documents_events_mails(
    txn: &DatabaseTransaction,
    user: &dto::User,
    documents: Vec<StoreDocument>,
    events: Vec<Event>,
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

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use sea_query::PostgresQueryBuilder;
    use validator::Validate;

    use crate::api::list_documents::ListDocumentParams;

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
    fn test_select_documents_sql_basic_query() {
        // Arrange
        let collection = Uuid::new_v4();
        let userid = Uuid::new_v4();
        let sort_fields = "created+".to_string();

        // Act
        let sql = select_documents_sql(
            &collection,
            vec!["title".to_string()],
            &None,
            Some(sort_fields),
            vec![],
        )
        .to_string(PostgresQueryBuilder);

        // Assert
        assert_eq!(
            sql,
            format!(
                r#"SELECT "id", "t"."new_f" AS "f" FROM "collection_document" AS "d" INNER JOIN LATERAL (SELECT jsonb_object_agg("key", "value") as "new_f" from jsonb_each("f") as x("key", "value") WHERE "key" in ('title')) AS "t" ON TRUE WHERE "collection_id" = '{collection}' AND "owner" = '{userid}' ORDER BY "d"."f"->>'created' ASC"#
            )
        );
    }

    #[test]
    fn test_select_documents_sql_query2() {
        // Arrange
        let collection = Uuid::new_v4();
        let _userid = Uuid::new_v4();
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

        // Act
        let sql = select_documents_sql(
            &collection,
            vec!["title".to_string()],
            &None,
            Some(sort_fields),
            filters,
        )
        .to_string(PostgresQueryBuilder);

        // Assert
        assert_eq!(
            sql,
            format!(
                r#"SELECT "id", "t"."new_f" AS "f" FROM "collection_document" AS "d" INNER JOIN LATERAL (SELECT jsonb_object_agg("key", "value") as "new_f" from jsonb_each("f") as x("key", "value") WHERE "key" in ('title')) AS "t" ON TRUE WHERE "collection_id" = '{collection}' AND "d"."f"->'orgaddr'->>'zip'='11101' AND ("d"."f"->'wf1'->>'seq') IN ('1', '2') ORDER BY "d"."f"->>'created' ASC"#
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

        // Act
        let sql = select_documents_sql(
            &collection,
            vec!["title".to_string()],
            &None,
            Some(sort_fields),
            filters,
        )
        .to_string(PostgresQueryBuilder);

        // Assert
        assert_eq!(
            sql,
            format!(
                r#"SELECT "id", "t"."new_f" AS "f" FROM "collection_document" AS "d" INNER JOIN LATERAL (SELECT jsonb_object_agg("key", "value") as "new_f" from jsonb_each("f") as x("key", "value") WHERE "key" in ('title')) AS "t" ON TRUE WHERE "collection_id" = '{collection}' AND "owner" = '{userid}' AND "d"."f"->'orgaddr'->>'zip'='11101' ORDER BY "d"."f"->>'created' ASC"#
            )
        );
    }
}

pub(crate) async fn get_accessible_document(
    ctx: &ApiContext,
    _user: &User,
    uuid: Uuid,
    collection: &Model,
) -> result::Result<Option<entity::collection_document::Model>, ApiErrors> {
    Ok(Documents::find_by_id(uuid)
        .one(&ctx.db)
        .await?
        .and_then(|doc| (doc.collection_id == collection.id).then_some(doc))
        // TODO: restore code
        // .and_then(|doc| {
        //     if collection.oao && doc.owner != user.subuuid() {
        //         None
        //     } else {
        //         Some(doc)
        //     }
        // })
        .and_then(|doc| {
            let f = doc.f.get(DELETED_AT_FIELD);
            if let Some(v) = f {
                if !v.is_null() {
                    if let Some(s) = v.as_str() {
                        if !s.is_empty() {
                            return None;
                        }
                    }
                }
            }
            Some(doc)
        }))
}
