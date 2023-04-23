use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use axum_macros::debug_handler;
use entity::collection_document::{self, Entity as Documents};
use garde::Validate;
use jwt_authorizer::JwtClaims;
use openapi::models::CollectionItem;
use sea_orm::{error::DbErr, prelude::Uuid, ActiveModelTrait, EntityTrait, RuntimeErr, Set};
use tracing::{debug, error, warn};

use crate::api::auth::User;

use super::{db::get_collection_by_name, ApiContext, ApiErrors};

#[debug_handler]
pub(crate) async fn api_update_document(
    State(ctx): State<ApiContext>,
    Path(collection_name): Path<String>,
    JwtClaims(user): JwtClaims<User>,
    Json(payload): Json<CollectionItem>,
) -> Result<(StatusCode, String), ApiErrors> {
    // Validate the payload
    payload.validate(&()).map_err(ApiErrors::from)?;

    let document_id = payload.id.to_string();
    let uuid = Uuid::parse_str(&document_id)
        .map_err(|_| ApiErrors::BadRequest("Invalid uuid".to_string()))?;

    let collection = get_collection_by_name(&ctx.db, &collection_name).await;
    if collection.is_none() {
        return Err(ApiErrors::NotFound(collection_name));
    }

    if !user.is_collection_editor(&collection_name) {
        warn!("User {} is not a collection editor", user.name_and_sub());
        return Err(ApiErrors::PermissionDenied);
    }

    let collection = collection.unwrap();
    let document = Documents::find_by_id(uuid)
        .one(&ctx.db)
        .await?
        .and_then(|doc| {
            if collection.oao && doc.owner != user.subuuid() {
                None
            } else {
                Some(doc)
            }
        });

    if document.is_none() {
        return Err(ApiErrors::NotFound(format!(
            "Document {document_id} not found"
        )));
    }
    let mut document: collection_document::ActiveModel = document.unwrap().into();
    document.f = Set(payload.f);
    let _ = document.update(&ctx.db).await.map_err(|err| match err {
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
        "Document {:?} updated in collection {}",
        document_id, collection_name
    );
    Ok((StatusCode::CREATED, "Document updated".to_string()))
}
