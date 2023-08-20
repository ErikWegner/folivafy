use anyhow::{Context, Result};
use entity::collection::Model;
use sea_orm::{
    ActiveModelTrait, ActiveValue::NotSet, ColumnTrait, ConnectionTrait, DatabaseConnection,
    DatabaseTransaction, EntityTrait, FromQueryResult, JsonValue, PaginatorTrait, QueryFilter, Set,
    Statement,
};
use sea_query::{
    Alias, Condition, Expr, JoinType, Order, PostgresQueryBuilder, Query, SelectStatement,
    SimpleExpr,
};
use tracing::{debug, error, info};
use uuid::Uuid;

use super::{
    dto::{self, Event, MailMessage},
    hooks::CronDocumentSelector,
    types::Pagination,
    ApiErrors,
};
use entity::collection_document::Column as DocumentsColumns;
use entity::collection_document::Entity as Documents;

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

#[derive(Debug, PartialEq)]
pub(crate) enum CollectionDocumentVisibility {
    PublicAndUserIsReader,
    PrivateAndUserIs(Uuid),
    PrivateAndUserCanAccessAllDocuments,
}

impl CollectionDocumentVisibility {
    pub(crate) fn get_userid(&self) -> Option<Uuid> {
        match self {
            CollectionDocumentVisibility::PublicAndUserIsReader => None,
            CollectionDocumentVisibility::PrivateAndUserIs(userid) => Some(*userid),
            CollectionDocumentVisibility::PrivateAndUserCanAccessAllDocuments => None,
        }
    }
}

pub(crate) enum FieldFilter {
    ExactFieldMatch { field_name: String, value: String },
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
        }
    }
}

pub(crate) async fn list_documents(
    db: &DatabaseConnection,
    collection: Uuid,
    exact_title: Option<String>,
    oao_access: CollectionDocumentVisibility,
    extra_fields: Vec<String>,
    sort_fields: Option<String>,
    filters: Vec<FieldFilter>,
    pagination: &Pagination,
) -> Result<(u32, Vec<JsonValue>), ApiErrors> {
    let mut basefind =
        Documents::find().filter(entity::collection_document::Column::CollectionId.eq(collection));

    if let Some(ref title) = exact_title {
        basefind = basefind.filter(sea_query::Expr::cust_with_values(
            r#""f"->>'title' = $1"#,
            [title],
        ));
    }

    match oao_access {
        CollectionDocumentVisibility::PublicAndUserIsReader => {}
        CollectionDocumentVisibility::PrivateAndUserIs(uuid) => {
            basefind = basefind.filter(entity::collection_document::Column::Owner.eq(uuid));
        }
        CollectionDocumentVisibility::PrivateAndUserCanAccessAllDocuments => {}
    }

    let total = basefind
        .clone()
        .count(db)
        .await
        .map_err(ApiErrors::from)
        .map(|t| u32::try_from(t).unwrap_or_default())?;

    let sql = select_documents_sql(
        &collection,
        extra_fields,
        &oao_access,
        &exact_title,
        sort_fields,
        filters,
    )
    .limit(pagination.limit().into())
    .offset(pagination.offset().into())
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
    oao_access: &CollectionDocumentVisibility,
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
        .and_where(Expr::col(DocumentsColumns::CollectionId).eq(collection.to_string()));

    if let Some(user_id) = oao_access.get_userid() {
        q = q.and_where(Expr::col(DocumentsColumns::Owner).eq(user_id));
    }

    for filter in filters {
        match filter {
            FieldFilter::ExactFieldMatch { field_name, value } => {
                q = q.and_where(Expr::cust_with_values(
                    format!(r#""d"."f"{}=$1"#, field_path_json(&field_name),),
                    vec![value],
                ));
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

            let sort_direction = match last_character == '+' {
                true => Order::Asc,
                false => Order::Desc,
            };
            (
                format!(r#""d"."f"{}"#, field_path_json(&field_name)),
                sort_direction,
            )
        })
        .collect()
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
    pub(crate) owner: Uuid,
    pub(crate) collection_id: Uuid,
}

pub(crate) async fn save_document_events_mails(
    txn: &DatabaseTransaction,
    user_id: &Uuid,
    document: Option<dto::CollectionDocument>,
    insert: Option<InsertDocumentData>,
    events: Vec<Event>,
    mails: Vec<MailMessage>,
) -> anyhow::Result<()> {
    if let Some(document) = document {
        debug!("Saving document");
        match insert {
            Some(insert_data) => {
                entity::collection_document::ActiveModel {
                    id: Set(*document.id()),
                    owner: Set(insert_data.owner),
                    collection_id: Set(insert_data.collection_id),
                    f: Set(document.fields().clone()),
                }
                .insert(txn)
                .await
                .context("Saving new document")?;
            }
            None => {
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

    debug!("Try to create {} event(s)", events.len());
    for event in events {
        // Create the event in the database
        let dbevent = entity::event::ActiveModel {
            id: NotSet,
            category_id: Set(event.category()),
            timestamp: NotSet,
            document_id: Set(event.document_id()),
            user: Set(user_id.clone()),
            payload: Set(event.payload().clone()),
        };
        let res = dbevent.save(txn).await.context("Saving event")?;

        debug!("Event {:?} saved", res.id);
    }

    debug!("Trying to store {} mail(s) in queue", mails.len());
    for mailmessage in mails {
        let document_fields =
            serde_json::to_value(mailmessage).expect("Failed to serialize mail message");
        entity::collection_document::ActiveModel {
            id: NotSet,
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
    use pretty_assertions::{assert_eq, assert_ne};
    use validator::Validate;

    use crate::api::list_documents::ListDocumentParams;

    use super::*;

    #[test]
    fn param_validation_test() {
        let all_fields_empty = ListDocumentParams {
            exact_title: None,
            extra_fields: None,
            sort_fields: None,
        };

        assert!(all_fields_empty.validate().is_ok());

        let valid_sort_fields = ListDocumentParams {
            exact_title: None,
            extra_fields: None,
            sort_fields: Some("title+,price-,length-".to_string()),
        };
        assert!(valid_sort_fields.validate().is_ok());

        let invalid_sort_fields = ListDocumentParams {
            exact_title: None,
            extra_fields: None,
            sort_fields: Some("title,price-".to_string()),
        };
        assert!(invalid_sort_fields.validate().is_err());

        let invalid_extra_fields = ListDocumentParams {
            exact_title: None,
            extra_fields: Some("title📣".to_string()),
            sort_fields: None,
        };
        assert!(invalid_extra_fields.validate().is_err());
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
                ("\"d\".\"f\"->>'length'".to_string(), Order::Desc)
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
                ("\"d\".\"f\"->'supplier'->>'city'".to_string(), Order::Asc)
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
            &CollectionDocumentVisibility::PrivateAndUserIs(userid),
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
    fn test_select_documents_sql_query1() {
        // Arrange
        let collection = Uuid::new_v4();
        let userid = Uuid::new_v4();
        let sort_fields = "created+".to_string();
        let extra = String::new();
        let filters = vec![CronDocumentSelector::ByFieldEqualsValue {
            field: "orgaddr.zip".to_string(),
            value: "11101".to_string(),
        }
        .into()];

        // Act
        let sql = select_documents_sql(
            &collection,
            vec!["title".to_string()],
            &CollectionDocumentVisibility::PrivateAndUserIs(userid),
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
