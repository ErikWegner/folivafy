use axum::{
    extract::{Path, State},
    Json,
};
use jwt_authorizer::JwtClaims;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::warn;
use typed_builder::TypedBuilder;
use validator::Validate;

use crate::{axumext::extractors::ValidatedQueryParams, models::CollectionItemsList};

use super::{
    auth::User,
    db::{get_unlocked_collection_by_name, FieldFilter, ListDocumentGrants},
    grants::{hook_or_default_user_grants, GrantCollection},
    list_documents::{
        generic_list_documents, DeletedDocuments, GenericListDocumentsParams, RE_EXTRA_FIELDS,
        RE_SORT_FIELDS,
    },
    types::Pagination,
    ApiContext, ApiErrors,
};

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Copy, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
pub(crate) enum OperationWithValue {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    StartsWith,
    ContainsText,
    In,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, TypedBuilder, utoipa::ToSchema)]
pub(crate) struct SearchFilterFieldOpValue {
    /// The name of the field to filter. Can contain dots to access nested fields.
    #[serde(rename = "f")]
    #[schema(examples("name", "price.currency"))]
    field: String,

    /// Operator
    #[serde(rename = "o")]
    operation: OperationWithValue,

    /// The value to compare with the field. Can be string, boolean or number
    #[serde(rename = "v")]
    value: Value,
}

impl SearchFilterFieldOpValue {
    pub(crate) fn field(&self) -> &str {
        self.field.as_ref()
    }

    pub(crate) fn operation(&self) -> OperationWithValue {
        self.operation
    }

    pub(crate) fn value(&self) -> &Value {
        &self.value
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Copy, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Operation {
    Null,
    NotNull,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, TypedBuilder, utoipa::ToSchema)]
pub(crate) struct SearchFilterFieldOp {
    /// Field name
    #[serde(rename = "f")]
    field: String,

    /// Operator
    #[serde(rename = "o")]
    operation: Operation,
}

impl SearchFilterFieldOp {
    pub(crate) fn field(&self) -> &str {
        self.field.as_ref()
    }

    pub(crate) fn operation(&self) -> Operation {
        self.operation
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, utoipa::ToSchema)]
pub(crate) enum SearchGroup {
    /// Join filters using AND operation
    #[serde(rename = "and")]
    #[schema(no_recursion)]
    AndGroup(Vec<SearchFilter>),

    /// Join filters using OR operation
    #[serde(rename = "or")]
    #[schema(no_recursion)]
    OrGroup(Vec<SearchFilter>),
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, utoipa::ToSchema)]
#[serde(untagged)]
#[schema(description = "A search filter")]
pub(crate) enum SearchFilter {
    FieldOpValue(SearchFilterFieldOpValue),
    FieldOp(SearchFilterFieldOp),
    Group(SearchGroup),
}

impl From<&FieldFilter> for SearchFilter {
    fn from(value: &FieldFilter) -> Self {
        match value {
            FieldFilter::ExactFieldMatch { field_name, value } => {
                SearchFilter::FieldOpValue(SearchFilterFieldOpValue {
                    field: field_name.clone(),
                    operation: OperationWithValue::Eq,
                    value: Value::String(value.clone()),
                })
            }
            FieldFilter::FieldStartsWith { field_name, value } => {
                SearchFilter::FieldOpValue(SearchFilterFieldOpValue {
                    field: field_name.clone(),
                    operation: OperationWithValue::StartsWith,
                    value: Value::String(value.clone()),
                })
            }
            FieldFilter::FieldContains { field_name, value } => {
                SearchFilter::FieldOpValue(SearchFilterFieldOpValue {
                    field: field_name.clone(),
                    operation: OperationWithValue::ContainsText,
                    value: Value::String(value.clone()),
                })
            }
            FieldFilter::FieldValueInMatch { field_name, values } => {
                SearchFilter::FieldOpValue(SearchFilterFieldOpValue {
                    field: field_name.clone(),
                    operation: OperationWithValue::In,
                    value: Value::Array(values.iter().cloned().map(Value::String).collect()),
                })
            }
            FieldFilter::FieldIsNull { field_name } => SearchFilter::FieldOp(SearchFilterFieldOp {
                field: field_name.clone(),
                operation: Operation::Null,
            }),
            FieldFilter::FieldIsNotNull { field_name } => {
                SearchFilter::FieldOp(SearchFilterFieldOp {
                    field: field_name.clone(),
                    operation: Operation::NotNull,
                })
            }
            FieldFilter::DateFieldLessThan { field_name, value } => {
                SearchFilter::FieldOpValue(SearchFilterFieldOpValue {
                    field: field_name.clone(),
                    operation: OperationWithValue::Lt,
                    value: Value::String(value.format("%Y-%m-%d").to_string()),
                })
            }
        }
    }
}

impl From<Vec<FieldFilter>> for SearchFilter {
    fn from(value: Vec<FieldFilter>) -> Self {
        SearchFilter::Group(SearchGroup::AndGroup(
            value.into_iter().map(|v| (&v).into()).collect(),
        ))
    }
}

#[derive(Debug, Default, Deserialize, Validate, utoipa::IntoParams)]
#[serde(default)]
#[into_params(parameter_in = Query)]
pub(crate) struct SearchDocumentParams {
    /// A comma separated list of document fields that should be contained in the response
    #[validate(regex(path= *RE_EXTRA_FIELDS))]
    #[serde(rename = "extraFields")]
    #[param(
        example = "price,length",
        pattern = r#"^[a-zA-Z0-9_]+(,[a-zA-Z0-9_]+)*$"#
    )]
    pub(crate) extra_fields: Option<String>,

    /// A comma separated list of document fields that should be contained in the response
    #[validate(regex(path= *RE_SORT_FIELDS))]
    #[serde(rename = "sort")]
    #[param(
        example = "price,length",
        pattern = r#"^[a-zA-Z0-9_]+(,[a-zA-Z0-9_]+)*$"#
    )]
    pub(crate) sort_fields: Option<String>,
}

#[derive(Debug, Default, Deserialize, Validate, utoipa::ToSchema)]
#[schema(description = "Search filters")]
pub(crate) struct SearchDocumentsBody {
    filter: Option<SearchFilter>,
}

/// Search items
///
/// Search a list of items within the collection
#[utoipa::path(
    post,
    path = "/collections/{collection_name}/search",
    operation_id = "searchCollection",
    params(
        Pagination,
        SearchDocumentParams,
        ("collection_name" = String, Path, description = "Name of the collection", pattern = r"^[a-z][-a-z0-9]*$" ),
    ),
    responses(

    ),
    request_body(content = SearchDocumentsBody, description = "Create a new document", content_type = "application/json"),
    tag = super::TAG_COLLECTION,
)
]
pub(crate) async fn api_search_documents(
    State(ctx): State<ApiContext>,
    ValidatedQueryParams(pagination): ValidatedQueryParams<Pagination>,
    ValidatedQueryParams(search_params): ValidatedQueryParams<SearchDocumentParams>,
    Path(collection_name): Path<String>,
    JwtClaims(user): JwtClaims<User>,
    Json(payload): Json<SearchDocumentsBody>,
) -> Result<Json<CollectionItemsList>, ApiErrors> {
    let collection = get_unlocked_collection_by_name(&ctx.db, &collection_name)
        .await
        .ok_or_else(|| ApiErrors::NotFound(collection_name.clone()))?;

    let user_is_permitted = user.is_collection_admin(&collection_name)
        || user.can_access_all_documents(&collection_name)
        || user.is_collection_reader(&collection_name);
    if !user_is_permitted {
        warn!("User {} is not a collection reader", user.name_and_sub());
        return Err(ApiErrors::PermissionDenied);
    }

    let dto_collection: GrantCollection = (&collection).into();
    let user_grants =
        hook_or_default_user_grants(&ctx.hooks, &dto_collection, &user, ctx.data_service.clone())
            .await?;

    let grants = ListDocumentGrants::Restricted(user_grants);

    generic_list_documents(
        &ctx.db,
        collection.id,
        DeletedDocuments::Exclude,
        GenericListDocumentsParams::builder()
            .sort_fields(search_params.sort_fields.clone())
            .extra_fields(search_params.extra_fields.clone())
            .filter(payload.filter)
            .build(),
        grants,
        pagination,
    )
    .await
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn it_works_for_fieldop() {
        // Arrange
        let p = SearchFilterFieldOp {
            field: "my_name".to_string(),
            operation: Operation::NotNull,
        };

        // Act
        let s = serde_json::to_string(&p).unwrap();

        // Assert
        assert_eq!(s, r#"{"f":"my_name","o":"notnull"}"#);
    }

    #[test]
    fn it_works_for_fieldopvalue() {
        // Arrange
        let p = SearchFilterFieldOpValue {
            field: "my_name".to_string(),
            operation: OperationWithValue::Ne,
            value: Value::String("my_value".to_string()),
        };

        // Act
        let s = serde_json::to_string(&p).unwrap();

        // Assert
        assert_eq!(s, r#"{"f":"my_name","o":"ne","v":"my_value"}"#);
    }

    #[test]
    fn it_works_for_searchgroup() {
        // Arrange
        let p = SearchGroup::OrGroup(vec![
            SearchFilter::FieldOpValue(SearchFilterFieldOpValue {
                field: "my_name".to_string(),
                operation: OperationWithValue::Eq,
                value: Value::String("my_value".to_string()),
            }),
            SearchFilter::FieldOp(SearchFilterFieldOp {
                field: "other".to_string(),
                operation: Operation::NotNull,
            }),
            SearchFilter::Group(SearchGroup::AndGroup(vec![
                SearchFilter::FieldOpValue(SearchFilterFieldOpValue {
                    field: "my_name3".to_string(),
                    operation: OperationWithValue::Eq,
                    value: Value::String("my_value3".to_string()),
                }),
                SearchFilter::FieldOp(SearchFilterFieldOp {
                    field: "other4".to_string(),
                    operation: Operation::Null,
                }),
            ])),
            SearchFilter::Group(SearchGroup::OrGroup(vec![
                SearchFilter::FieldOpValue(SearchFilterFieldOpValue {
                    field: "my_name5".to_string(),
                    operation: OperationWithValue::Eq,
                    value: Value::String("my_value5".to_string()),
                }),
                SearchFilter::FieldOp(SearchFilterFieldOp {
                    field: "other6".to_string(),
                    operation: Operation::Null,
                }),
            ])),
        ]);

        // Act
        let s = serde_json::to_string(&p).unwrap();

        // Assert
        assert_eq!(
            s,
            r#"{"or":[{"f":"my_name","o":"eq","v":"my_value"},{"f":"other","o":"notnull"},{"and":[{"f":"my_name3","o":"eq","v":"my_value3"},{"f":"other4","o":"null"}]},{"or":[{"f":"my_name5","o":"eq","v":"my_value5"},{"f":"other6","o":"null"}]}]}"#
        );
    }

    #[test]
    fn it_can_deserialize_searchgroup() {
        // Arrange
        let s = r#"{"or":[{"f":"my_name","o":"eq","v":"my_value"},{"f":"other","o":"notnull"},{"and":[{"f":"my_name3","o":"eq","v":"my_value3"},{"f":"other4","o":"null"}]},{"or":[{"f":"my_name5","o":"eq","v":"my_value5"},{"f":"other6","o":"null"}]}]}"#;
        // Act
        let p: SearchGroup = serde_json::from_str(s).unwrap();

        // Assert
        assert_eq!(
            p,
            SearchGroup::OrGroup(vec![
                SearchFilter::FieldOpValue(SearchFilterFieldOpValue {
                    field: "my_name".to_string(),
                    operation: OperationWithValue::Eq,
                    value: Value::String("my_value".to_string()),
                }),
                SearchFilter::FieldOp(SearchFilterFieldOp {
                    field: "other".to_string(),
                    operation: Operation::NotNull,
                }),
                SearchFilter::Group(SearchGroup::AndGroup(vec![
                    SearchFilter::FieldOpValue(SearchFilterFieldOpValue {
                        field: "my_name3".to_string(),
                        operation: OperationWithValue::Eq,
                        value: Value::String("my_value3".to_string()),
                    }),
                    SearchFilter::FieldOp(SearchFilterFieldOp {
                        field: "other4".to_string(),
                        operation: Operation::Null,
                    }),
                ])),
                SearchFilter::Group(SearchGroup::OrGroup(vec![
                    SearchFilter::FieldOpValue(SearchFilterFieldOpValue {
                        field: "my_name5".to_string(),
                        operation: OperationWithValue::Eq,
                        value: Value::String("my_value5".to_string()),
                    }),
                    SearchFilter::FieldOp(SearchFilterFieldOp {
                        field: "other6".to_string(),
                        operation: Operation::Null,
                    }),
                ])),
            ])
        );
    }

    #[test]
    fn it_convers_in_clause() {
        // Arrange
        let i = vec![FieldFilter::FieldValueInMatch {
            field_name: "f4".to_string(),
            values: vec!["191".to_string(), "291".to_string()],
        }];

        // Act
        let r: SearchFilter = i.into();

        // Assert
        assert_eq!(
            r,
            SearchFilter::Group(SearchGroup::AndGroup(vec![SearchFilter::FieldOpValue(
                SearchFilterFieldOpValue {
                    field: "f4".to_string(),
                    operation: OperationWithValue::In,
                    value: serde_json::json!(vec!["191", "291"])
                }
            )]))
        )
    }
}
