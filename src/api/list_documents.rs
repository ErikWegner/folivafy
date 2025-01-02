use std::str::FromStr;

use axum::{
    extract::{Path, State},
    Json,
};

use entity::DELETED_AT_FIELD;
use jwt_authorizer::JwtClaims;
use lazy_static::lazy_static;
use regex::Regex;
use sea_orm::prelude::Uuid;
use sea_orm::DatabaseConnection;

use serde::Deserialize;
use tracing::warn;
use typed_builder::TypedBuilder;
use validator::Validate;

use crate::api::grants::{hook_or_default_user_grants, GrantCollection};
use crate::models::{CollectionItem, CollectionItemsList};
use crate::{
    api::{
        auth::User,
        db::{list_documents, FieldFilter},
        types::Pagination,
        ApiContext, ApiErrors,
    },
    axumext::extractors::ValidatedQueryParams,
};

use super::{
    db::{get_unlocked_collection_by_name, DbListDocumentParams, ListDocumentGrants},
    search_documents::{SearchFilter, SearchFilterFieldOp},
};

lazy_static! {
    pub(crate) static ref RE_EXTRA_FIELDS: Regex =
        Regex::new(r"^[a-zA-Z0-9_]+(,[a-zA-Z0-9_]+)*$").unwrap();
    pub(crate) static ref RE_SORT_FIELDS: Regex = Regex::new(
        r"^[a-zA-Z0-9_]+(\.[a-zA-Z0-9_]+)*[\+\-fb](,[a-zA-Z0-9_]+(\.[a-zA-Z0-9_]+)*[\+\-fb])*$"
    )
    .unwrap();
}

pub(crate) enum DeletedDocuments {
    LimitToDeletedDocuments,
    Exclude,
}

#[derive(Debug, Default, Deserialize, Validate, utoipa::IntoParams)]
#[serde(default)]
#[into_params(parameter_in = Query)]
pub(crate) struct ListDocumentParams {
    /// Search for documents with this exact title (upper and lower case are respected)
    #[serde(rename = "exactTitle")]
    pub(crate) exact_title: Option<String>,

    /// A comma separated list of document fields that should be contained in the response
    #[validate(regex(path= *RE_EXTRA_FIELDS))]
    #[serde(rename = "extraFields")]
    #[param(
        example = "price,length",
        pattern = r#"^[a-zA-Z0-9_]+(,[a-zA-Z0-9_]+)*$"#
    )]
    pub(crate) extra_fields: Option<String>,

    /// A comma separated list of document fields that should be used to sort the collection.
    ///
    /// The sort order is determined by the last character in the field name:
    ///  - `+` for ascending order, the field is a direct child of the document's field `f`
    ///  - `-` for descending order, the field is a direct child of the document's field `f`
    ///  - `f` for ascending order, the field is in dotted notation, e. g. `my_workflow.signature.date`
    ///  - `b` for descending order, the field is in dotted notation, e. g. `my_workflow.signature.date`
    ///
    /// Example: `price+,length-,my_workflow.signature.dateb` will order documents by price (ascending), length (descending) and signature date (descending).
    #[validate(regex(path= *RE_SORT_FIELDS))]
    #[serde(rename = "sort")]
    #[param(
        default = "created+",
        example = "email+,created-",
        pattern = r#"^[a-zA-Z0-9_]+(\.[a-zA-Z0-9_]+)*[\+\-fb](,[a-zA-Z0-9_]+(\.[a-zA-Z0-9_]+)*[\+\-fb])*$"#
    )]
    pub(crate) sort_fields: Option<String>,

    /// A pound sign (`&`) separated list of filter conditions.
    ///
    /// Each filter condition consists of a field name and a value. Supported operators are: equality, value in array, starts with, contains.
    ///
    /// Examples:
    ///  - `f1='v12'` matches documents where field `f1` equals `"v12"`
    ///  - `a='k'&f1=4` matches documents where field `a` equals `"k"` and field `f1` equals `4`
    ///  - `a='k'&f3=['p1','p4','p9']` matches documents where field `a` equals `"k"` and field `f3` is one of the values `"p1"`, `"p4"`, or `"p9"`
    ///  - `az=@'kl'` matches documents where field `az` starts with `"kl"`
    ///  - `pt=~'imi'` matches documents where field `pt` contains the substring `"imi"`
    #[serde(rename = "pfilter")]
    pub(crate) pfilter: Option<String>,
}

/// List collection items (documents).
///
/// Get a list of items within the collection, i. e. list all documents.
#[utoipa::path(
    get,
    path = "/collections/{collection_name}",
    operation_id = "listCollectionItems",
    params(
        Pagination,
        ListDocumentParams,
        (
            "collection_name" = String,
            Path,
            description = "Name of the collection",
            min_length = 1,
            max_length = 32,
            pattern = r"^[a-z][-a-z0-9]*$",
        ),
    ),
    responses(
        (status = OK, description = "List of documents", body = CollectionItemsList ),
        (status = UNAUTHORIZED, description = "User is not a collection reader" ),
        (status = NOT_FOUND, description = "Collection not found" ),
        (status = BAD_REQUEST, description = "Invalid request" ),
        (status = INTERNAL_SERVER_ERROR, description = "Internal server error"),
    ),
    tag = super::TAG_COLLECTION,
)]
pub(crate) async fn api_list_documents(
    State(ctx): State<ApiContext>,
    ValidatedQueryParams(pagination): ValidatedQueryParams<Pagination>,
    ValidatedQueryParams(list_params): ValidatedQueryParams<ListDocumentParams>,
    Path(collection_name): Path<String>,
    JwtClaims(user): JwtClaims<User>,
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
    let mut request_filters = parse_pfilter(list_params.pfilter);
    if let Some(title) = list_params.exact_title {
        request_filters.push(FieldFilter::ExactFieldMatch {
            field_name: "title".to_string(),
            value: title,
        });
    }

    generic_list_documents(
        &ctx.db,
        collection.id,
        DeletedDocuments::Exclude,
        GenericListDocumentsParams::builder()
            .sort_fields(list_params.sort_fields.clone())
            .extra_fields(list_params.extra_fields.clone())
            .filter(if request_filters.is_empty() {
                None
            } else {
                Some(request_filters.into())
            })
            .build(),
        grants,
        pagination,
    )
    .await
}

pub(crate) fn parse_pfilter(s: Option<String>) -> Vec<FieldFilter> {
    // Split s by ampersand
    s.map(|s| s.split('&').filter_map(FieldFilter::from_str).collect())
        .unwrap_or_default()
}

#[derive(Debug, TypedBuilder)]
pub(crate) struct GenericListDocumentsParams {
    extra_fields: Option<String>,
    sort_fields: Option<String>,
    filter: Option<SearchFilter>,
}

pub(crate) async fn generic_list_documents(
    db: &DatabaseConnection,
    collection_id: Uuid,
    deleted_documents: DeletedDocuments,
    list_params: GenericListDocumentsParams,
    grants: ListDocumentGrants,
    pagination: Pagination,
) -> Result<Json<CollectionItemsList>, ApiErrors> {
    let extra_fields = list_params.extra_fields.unwrap_or("title".to_string());
    let mut extra_fields: Vec<String> = extra_fields.split(',').map(|s| s.to_string()).collect();
    let extra_field_author = "author_id".to_string();

    let include_author = extra_fields.contains(&extra_field_author);
    if include_author {
        extra_fields.retain(|f| f != &extra_field_author);
    }

    let title = "title".to_string();
    if !extra_fields.contains(&title) {
        extra_fields.push(title);
    }

    let deleted_documents_condition = SearchFilter::FieldOp(
        SearchFilterFieldOp::builder()
            .field(DELETED_AT_FIELD.to_string())
            .operation(match deleted_documents {
                DeletedDocuments::LimitToDeletedDocuments => {
                    super::search_documents::Operation::NotNull
                }
                DeletedDocuments::Exclude => super::search_documents::Operation::Null,
            })
            .build(),
    );

    let filters = match list_params.filter {
        Some(filters) => SearchFilter::Group(super::search_documents::SearchGroup::AndGroup(vec![
            deleted_documents_condition,
            filters,
        ])),
        None => deleted_documents_condition,
    };

    let db_params = DbListDocumentParams::builder()
        .collection(collection_id)
        .grants(grants)
        .extra_fields(extra_fields)
        .sort_fields(list_params.sort_fields)
        .filters(filters)
        .pagination(pagination.clone())
        .include_author_id(include_author)
        .build();

    let (total, items) = list_documents(db, &db_params)
        .await
        .map_err(ApiErrors::from)?;

    let items = items
        .into_iter()
        .map(|i| {
            let mut f = i["f"].clone();
            if include_author {
                f["author_id"] = i["author_id"].clone();
            }
            CollectionItem {
                id: Uuid::from_str(i["id"].as_str().unwrap()).unwrap(),
                f,
            }
        })
        .collect();

    Ok(Json(CollectionItemsList {
        limit: pagination.limit(),
        offset: pagination.offset(),
        total,
        items,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn test_empty_pfilter() {
        assert_eq!(parse_pfilter(Some("".to_string())).len(), 0, "Empty string");
        assert_eq!(parse_pfilter(None).len(), 0, "None value");
    }

    #[test]
    pub fn test_simple() {
        // Arrange
        let s = "f1='v12'";

        // Act
        let r = parse_pfilter(Some(s.to_string()));

        // Assert
        assert_eq!(r.len(), 1);
        match r.first().unwrap() {
            FieldFilter::ExactFieldMatch { field_name, value } => {
                assert_eq!(field_name, "f1");
                assert_eq!(value, "v12");
            }
            _ => panic!("Unexpected value"),
        }
    }

    #[test]
    pub fn test_multiple() {
        // Arrange
        let s = "a='k'&f1=4";

        // Act
        let r = parse_pfilter(Some(s.to_string()));

        // Assert
        assert_eq!(r.len(), 2);
        match r.first().unwrap() {
            FieldFilter::ExactFieldMatch { field_name, value } => {
                assert_eq!(field_name, "a");
                assert_eq!(value, "k");
            }
            _ => panic!("Unexpected value"),
        }
        match r.get(1).unwrap() {
            FieldFilter::ExactFieldMatch { field_name, value } => {
                assert_eq!(field_name, "f1");
                assert_eq!(value, "4");
            }
            _ => panic!("Unexpected value"),
        }
    }

    #[test]
    pub fn test_list() {
        // Arrange
        let s = "a='k'&f3=['p1','p4','p9']";
        let expected_values = ["p1", "p4", "p9"]
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>();

        // Act
        let r = parse_pfilter(Some(s.to_string()));

        // Assert
        assert_eq!(r.len(), 2);
        match r.first().unwrap() {
            FieldFilter::ExactFieldMatch { field_name, value } => {
                assert_eq!(field_name, "a");
                assert_eq!(value, "k");
            }
            _ => panic!("Unexpected value"),
        }
        match r.get(1).unwrap() {
            FieldFilter::FieldValueInMatch { field_name, values } => {
                assert_eq!(field_name, "f3");
                assert_eq!(values, &expected_values);
            }
            _ => panic!("Unexpected value"),
        }
    }

    #[test]
    pub fn test_starts_with() {
        // Arrange
        let s = "az=@'kl'";

        // Act
        let r = parse_pfilter(Some(s.to_string()));

        // Assert
        assert_eq!(r.len(), 1);
        match r.first().unwrap() {
            FieldFilter::FieldStartsWith { field_name, value } => {
                assert_eq!(field_name, "az");
                assert_eq!(value, "kl");
            }
            _ => panic!("Unexpected value"),
        }
    }

    #[test]
    pub fn test_contains() {
        // Arrange
        let s = "pt=~'imi'";

        // Act
        let r = parse_pfilter(Some(s.to_string()));

        // Assert
        assert_eq!(r.len(), 1);
        match r.first().unwrap() {
            FieldFilter::FieldContains { field_name, value } => {
                assert_eq!(field_name, "pt");
                assert_eq!(value, "imi");
            }
            _ => panic!("Unexpected value"),
        }
    }
}
