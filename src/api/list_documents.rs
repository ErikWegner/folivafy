use std::str::FromStr;

use axum::{
    extract::{Path, State},
    Json,
};
use entity::collection_document::Entity as Documents;
use jwt_authorizer::JwtClaims;
use lazy_static::lazy_static;
use openapi::models::{CollectionItem, CollectionItemsList};
use regex::Regex;
use sea_orm::{
    prelude::Uuid, ColumnTrait, DbBackend, EntityTrait, FromQueryResult, JsonValue, PaginatorTrait,
    QueryFilter, Statement,
};
use sea_query::Expr;
use serde::Deserialize;
use tracing::warn;
use validator::Validate;

use crate::{api::auth::User, axumext::extractors::ValidatedQueryParams};

use super::{db::get_collection_by_name, types::Pagination, ApiContext, ApiErrors};

lazy_static! {
    static ref RE_EXTRA_FIELDS: Regex = Regex::new(r"^[a-zA-Z0-9]+(,[a-zA-Z0-9]+)*$").unwrap();
    static ref RE_SORT_FIELDS: Regex =
        Regex::new(r"^[a-zA-Z0-9]+(\.[a-zA-Z0-9]+)*[\+-](,[a-zA-Z0-9]+(\.[a-zA-Z0-9]+)*[\+-])*$")
            .unwrap();
}

#[derive(Debug, Default, Deserialize, Validate)]
#[serde(default)]
pub(crate) struct ListDocumentParams {
    #[serde(rename = "exactTitle")]
    exact_title: Option<String>,

    #[validate(regex = "RE_EXTRA_FIELDS")]
    #[serde(rename = "extraFields")]
    extra_fields: Option<String>,

    #[validate(regex = "RE_SORT_FIELDS")]
    #[serde(rename = "sort")]
    sort_fields: Option<String>,
}

pub(crate) async fn api_list_document(
    State(ctx): State<ApiContext>,
    ValidatedQueryParams(pagination): ValidatedQueryParams<Pagination>,
    ValidatedQueryParams(list_params): ValidatedQueryParams<ListDocumentParams>,
    Path(collection_name): Path<String>,
    JwtClaims(user): JwtClaims<User>,
) -> Result<Json<CollectionItemsList>, ApiErrors> {
    let extra_fields = list_params.extra_fields.unwrap_or("title".to_string());
    let collection = get_collection_by_name(&ctx.db, &collection_name).await;
    if collection.is_none() {
        return Err(ApiErrors::NotFound(collection_name));
    }
    let collection = collection.unwrap();

    if !user.is_collection_reader(&collection_name) {
        warn!("User {} is not a collection reader", user.name_and_sub());
        return Err(ApiErrors::PermissionDenied);
    }

    let mut basefind = Documents::find()
        .filter(entity::collection_document::Column::CollectionId.eq(collection.id));

    if let Some(ref title) = list_params.exact_title {
        basefind = basefind.filter(Expr::cust_with_values(r#""f"->>'title' = $1"#, [title]));
    }

    if collection.oao {
        basefind = basefind.filter(entity::collection_document::Column::Owner.eq(user.subuuid()));
    }

    let total = basefind
        .clone()
        .count(&ctx.db)
        .await
        .map_err(ApiErrors::from)
        .map(|t| u32::try_from(t).unwrap_or_default())?;

    let mut extra_fields: Vec<String> = extra_fields.split(',').map(|s| s.to_string()).collect();
    let title = "title".to_string();
    if !extra_fields.contains(&title) {
        extra_fields.push(title);
    }

    let sort_fields = sort_fields_sql(list_params.sort_fields);

    let extra_fields = extra_fields
        .into_iter()
        .map(|f| format!("'{f}'"))
        .collect::<Vec<_>>()
        .join(",");

    let items: Vec<JsonValue> = JsonValue::find_by_statement(Statement::from_sql_and_values(
        DbBackend::Postgres,
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
            if collection.oao {
                r#"AND "owner" = $4 "#
            } else {
                ""
            },
            if list_params.exact_title.is_some() {
                r#"AND "f"->>'title' = $5 "#
            } else {
                ""
            },
            r#"LIMIT $2
                OFFSET $3"#
        )
        .as_str(),
        [
            collection.id.into(),
            pagination.limit().into(),
            pagination.offset().into(),
            user.subuuid().into(),
            list_params.exact_title.unwrap_or_default().into(),
        ],
    ))
    .all(&ctx.db)
    .await
    .map_err(ApiErrors::from)?;

    let items = items
        .into_iter()
        .map(|i| CollectionItem {
            id: Uuid::from_str(i["id"].as_str().unwrap()).unwrap(),
            f: i["f"].clone(),
        })
        .collect();

    Ok(Json(CollectionItemsList {
        limit: pagination.limit(),
        offset: pagination.offset(),
        total,
        items,
    }))
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

#[cfg(test)]
mod tests {
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
