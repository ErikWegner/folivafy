use axum::{extract::State, http::StatusCode, Json};
use axum_macros::debug_handler;
use entity::collection;
use jwt_authorizer::JwtClaims;
use sea_orm::{DbErr, EntityTrait, RuntimeErr, Set};
use tracing::{error, info, warn};
use validator::Validate;

use crate::api::{auth::User, ApiContext, ApiErrors};
use crate::models::CreateCollectionRequest;

/// Create a collection
///
/// Create a new collection on this server
#[debug_handler]
#[utoipa::path(
    post,
    path="/collections",
    operation_id = "createCollection",
    responses(
        (status = CREATED, description = "Collection created successfully" ),
        (status = UNAUTHORIZED, description = "User is not a collections admin" ),
        (status = BAD_REQUEST, description = "Invalid request" ),
        (status = INTERNAL_SERVER_ERROR, description = "Internal server error"),
    ),
    request_body(content = CreateCollectionRequest, description = "Create a new collection", content_type = "application/json"),
    tag = super::TAG_ADMINISTRATION,
)]
pub(crate) async fn api_create_collection(
    State(ctx): State<ApiContext>,
    JwtClaims(user): JwtClaims<User>,
    Json(payload): Json<CreateCollectionRequest>,
) -> Result<(StatusCode, String), ApiErrors> {
    if !user.is_collections_administrator() {
        warn!("User {} is not a collections admin", user.name_and_sub());
        return Err(ApiErrors::PermissionDenied);
    }
    payload.validate().map_err(ApiErrors::from)?;
    let mut collection = collection::ActiveModel {
        ..Default::default()
    };
    collection.name = Set(payload.name.clone());
    collection.title = Set(payload.title.clone());
    collection.oao = Set(payload.oao);

    let res = entity::collection::Entity::insert(collection)
        .exec(&ctx.db)
        .await
        .map_err(|err| match err {
            DbErr::Exec(RuntimeErr::SqlxError(error)) => match error {
                sqlx::error::Error::Database(e) => {
                    let code: String = e.code().unwrap_or_default().to_string();
                    // We check the error code thrown by the database (PostgreSQL in this case),
                    // `23505` means `value violates unique constraint`: we have a duplicate key in the table.
                    if code == "23505" {
                        ApiErrors::BadRequestJsonSimpleMsg("Duplicate collection name".to_string())
                    } else {
                        error!("Database runtime error: {}", e);
                        ApiErrors::BadRequestJsonSimpleMsg(format!(
                            "Cannot create collection (code {code})",
                        ))
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

    info!(
        "Created new collection: {} {}",
        payload.name, res.last_insert_id
    );
    Ok((
        StatusCode::CREATED,
        format!("Collection {} created", payload.name),
    ))
}
