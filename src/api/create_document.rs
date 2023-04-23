use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use axum_macros::debug_handler;
use entity::collection_document;
use garde::Validate;
use jwt_authorizer::JwtClaims;
use migration::DbErr;
use openapi::models::CollectionItem;
use sea_orm::{EntityTrait, RuntimeErr, Set};
use tracing::{debug, error, warn};

use crate::api::{auth::User, ApiErrors};

use super::{db::get_collection_by_name, ApiContext};

#[debug_handler]
pub(crate) async fn api_create_document(
    State(ctx): State<ApiContext>,
    JwtClaims(user): JwtClaims<User>,
    Path(collection_name): Path<String>,
    Json(payload): Json<CollectionItem>,
) -> Result<(StatusCode, String), ApiErrors> {
    // Check if user is allowed to create a document within the collection
    if !user.is_collection_editor(&collection_name) {
        warn!("User {} is not a collections admin", user.name_and_sub());
        return Err(ApiErrors::PermissionDenied);
    }

    // Validate the payload
    payload.validate(&()).map_err(ApiErrors::from)?;

    let collection = get_collection_by_name(&ctx.db, &collection_name).await;

    if collection.is_none() {
        return Err(ApiErrors::NotFound(collection_name));
    }
    let collection = collection.unwrap();

    // Check if collection is locked
    if collection.locked {
        warn!(
            "User {} tried to add document to locked collection {}",
            user.name_and_sub(),
            collection_name
        );
        return Err(ApiErrors::BadRequest("Read only collection".into()));
    }

    let document = collection_document::ActiveModel {
        id: Set(payload.id),
        f: Set(payload.f.clone()),
        collection_id: Set(collection.id),
        owner: Set(user.subuuid()),
    };

    let res = entity::collection_document::Entity::insert(document)
        .exec(&ctx.db)
        .await
        .map_err(|err| match err {
            DbErr::Exec(RuntimeErr::SqlxError(error)) => match error {
                sqlx::error::Error::Database(e) => {
                    let code: String = e.code().unwrap_or_default().to_string();
                    // We check the error code thrown by the database (PostgreSQL in this case),
                    // `23505` means `value violates unique constraint`: we have a duplicate key in the table.
                    if code == "23505" {
                        ApiErrors::BadRequest("Duplicate document".to_string())
                    } else {
                        error!("Database runtime error: {}", e);
                        ApiErrors::BadRequest(format!("Cannot create document (code {})", code))
                    }
                }
                _ => {
                    error!("Database runtime error: {}", error);
                    ApiErrors::InternalServerError
                }
            },
            _ => {
                println!("{:?}", err);
                error!("Database error: {}", err);
                ApiErrors::InternalServerError
            }
        })?;

    debug!(
        "Document {:?} saved to {}",
        res.last_insert_id, collection_name
    );
    Ok((StatusCode::CREATED, "Document saved".to_string()))
}
