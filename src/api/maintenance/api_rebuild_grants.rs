use crate::api::auth::User;
use crate::api::db::{self, get_collection_by_name, get_document_by_id_in_trx, list_document_ids};
use crate::api::dto::GrantForDocument;
use crate::api::grants::hook_or_default_document_grants;
use crate::api::{ApiContext, ApiErrors};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum_macros::debug_handler;
use jwt_authorizer::JwtClaims;
use sea_orm::{TransactionError, TransactionTrait};
use tracing::{debug, error};

/// Rebuild grants for a collection
///
/// Iterate over all documents and refresh grants.
#[debug_handler]
#[utoipa::path(
    post,
    path = "/api/maintenance/{collection}/rebuild-grants",
    operation_id = "rebuildGrants",
    params(
        ("collection_name" = String, Path, description = "Name of the collection", pattern = r"^[a-z][-a-z0-9]*$" ),
    ),
    responses(
        (status = CREATED, description = "Grants rebuilt successfully" ),
        (status = UNAUTHORIZED, description = "User is not a collection admin" ),
        (status = NOT_FOUND, description = "Collection not found" ),
        (status = INTERNAL_SERVER_ERROR, description = "Internal server error"),
    ),
    tag = crate::api::TAG_MAINTENANCE,
)]
pub(crate) async fn api_rebuild_grants(
    State(ctx): State<ApiContext>,
    JwtClaims(user): JwtClaims<User>,
    Path(collection_name): Path<String>,
) -> Result<(StatusCode, String), ApiErrors> {
    let collection = get_collection_by_name(&ctx.db, &collection_name).await;
    if collection.is_none() {
        debug!("Collection {} not found", collection_name);
        return Err(ApiErrors::NotFound(format!(
            "Collection {} not found",
            collection_name
        )));
    }
    let collection = collection.unwrap();

    if !user.is_collections_administrator() {
        debug!("User {} is not a collection admin", user.name_and_sub());
        return Err(ApiErrors::PermissionDenied);
    }

    ctx.db
        .transaction::<_, (StatusCode, String), ApiErrors>(|txn| {
            Box::pin(async move {
                let ids = list_document_ids(txn, collection.id).await?;
                for id in ids {
                    debug!("Rebuilding grants for document {id} in collection {collection_name}");
                    let document = get_document_by_id_in_trx(id, txn).await?;
                    if document.is_none() {
                        continue;
                    }
                    let document = document.unwrap();
                    let author_id = document.owner;

                    let grants = hook_or_default_document_grants(
                        &ctx.hooks,
                        (&collection).into(),
                        (&document).into(),
                        ctx.data_service.clone(),
                        author_id,
                    )
                    .await?;
                    db::replace_grants(
                        txn,
                        grants
                            .into_iter()
                            .map(|grant| GrantForDocument::new(grant, id))
                            .collect(),
                    )
                    .await
                    .map_err(|e| {
                        error!("Failed to replace grants: {:?}", e);
                        ApiErrors::InternalServerError
                    })?;
                }
                Ok((StatusCode::CREATED, "OK".to_string()))
            })
        })
        .await
        .map_err(|err| match err {
            TransactionError::Connection(c) => Into::<ApiErrors>::into(c),
            TransactionError::Transaction(t) => t,
        })
}
