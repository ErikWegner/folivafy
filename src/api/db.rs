use anyhow::{Context, Result};
use entity::collection::Model;
use sea_orm::{
    ActiveModelTrait, ActiveValue::NotSet, ColumnTrait, DatabaseConnection, DatabaseTransaction,
    EntityTrait, FromQueryResult, JsonValue, PaginatorTrait, QueryFilter, Set,
};
use tracing::{debug, error, info};
use uuid::Uuid;

use super::{
    auth::User,
    dto::{self, Event},
    hooks::CronDocumentSelector,
    types::Pagination,
    ApiErrors,
};
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
    extra_fields: String,
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

    let sort_fields = sort_fields_sql(sort_fields);

    let items: Vec<JsonValue> =
        JsonValue::find_by_statement(sea_orm::Statement::from_sql_and_values(
            sea_orm::DbBackend::Postgres,
            select_documents_sql(&extra_fields, &oao_access, &exact_title, &sort_fields).as_str(),
            [
                collection.into(),
                pagination.limit().into(),
                pagination.offset().into(),
                oao_access.get_userid().unwrap_or_default().into(),
                exact_title.unwrap_or_default().into(),
            ],
        ))
        .all(db)
        .await
        .map_err(ApiErrors::from)?;

    Ok((total, items))
}

fn select_documents_sql(
    extra_fields: &String,
    oao_access: &CollectionDocumentVisibility,
    exact_title: &Option<String>,
    sort_fields: &String,
) -> String {
    format!(
        "{}{extra_fields}{}{}{} ORDER BY {sort_fields} {}",
        r#"SELECT "id", "t"."new_f" as "f"
                FROM "collection_document"
                cross join lateral (
                 select jsonb_object_agg("key", "value") as "new_f"
                 from jsonb_each("f") as x("key", "value")
                 WHERE
                    "key" in ("#,
        r#")
                  ) as "t"
                WHERE "collection_id" = $1 "#,
        match oao_access.get_userid() {
            Some(_) => r#"AND "owner" = $4 "#,
            None => "",
        },
        if exact_title.is_some() {
            r#"AND "f"->>'title' = $5 "#
        } else {
            ""
        },
        r#"LIMIT $2
                OFFSET $3"#
    )
}

fn sort_fields_sql(fields: Option<String>) -> String {
    fields
        .unwrap_or_else(|| "created+".to_string())
        .split(',')
        .map(|s| {
            let mut char_vec_from_s = s.chars().collect::<Vec<char>>();
            let last_character = char_vec_from_s.pop().unwrap();
            let field_name = char_vec_from_s.into_iter().collect::<String>();

            let sort_direction = match last_character == '+' {
                true => "ASC",
                false => "DESC",
            };

            if !field_name.contains('.') {
                return format!(r#""f"->>'{field_name}' {sort_direction}"#);
            }
            // split field_name on dots
            let mut field_struct = field_name
                .split('.')
                .map(|s| format!("'{s}'"))
                .collect::<Vec<String>>();
            let field_name = field_struct.pop().unwrap();
            let field_path = field_struct
                // .into_iter()
                // .map(|f| format!("'{f}'"))
                .join("->");
            format!(r#""f"->{field_path}->>{field_name} {sort_direction}"#)
        })
        .collect::<Vec<_>>()
        .join(",")
}

pub(crate) struct InsertDocumentData {
    pub(crate) owner: Uuid,
    pub(crate) collection_id: Uuid,
}

pub(crate) async fn save_document_and_events(
    txn: &DatabaseTransaction,
    user: &User,
    document: Option<dto::CollectionDocument>,
    insert: Option<InsertDocumentData>,
    events: Vec<Event>,
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
            user: Set(user.subuuid()),
            payload: Set(event.payload().clone()),
        };
        let res = dbevent.save(txn).await.context("Saving event")?;

        debug!("Event {:?} saved", res.id);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
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
        let sql = sort_fields_sql(Some(sort_fields));

        // Assert
        assert_eq!(
            sql,
            "\"f\"->>'title' ASC,\"f\"->>'price' DESC,\"f\"->>'length' DESC"
        );
    }

    #[test]
    fn sort_fields_sql_test_subfield() {
        // Arrange
        let sort_fields = "title+,company.title-,supplier.city+".to_string();

        // Act
        let sql = sort_fields_sql(Some(sort_fields));

        // Assert
        assert_eq!(
            sql,
            "\"f\"->>'title' ASC,\"f\"->'company'->>'title' DESC,\"f\"->'supplier'->>'city' ASC"
        );
    }

    #[test]
    fn sort_fields_sql_test_subsubfield() {
        // Arrange
        let sort_fields = "title+,company.hq.addr.city-,supplier.city+".to_string();

        // Act
        let sql = sort_fields_sql(Some(sort_fields));

        // Assert
        assert_eq!(
            sql,
            "\"f\"->>'title' ASC,\"f\"->'company'->'hq'->'addr'->>'city' DESC,\"f\"->'supplier'->>'city' ASC"
        );
    }
}
