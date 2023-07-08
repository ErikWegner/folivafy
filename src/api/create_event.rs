use axum::{extract::State, http::StatusCode, Json};
use axum_macros::debug_handler;
use entity::collection_document::Entity as Documents;
use jwt_authorizer::JwtClaims;
use openapi::models::CreateEventBody;
use sea_orm::{
    ActiveModelTrait, ActiveValue::NotSet, DatabaseTransaction, DbErr, EntityTrait, Set,
    TransactionError, TransactionTrait,
};
use tokio::sync::oneshot;
use tracing::debug;
use validator::Validate;

use crate::api::{
    db::get_collection_by_name,
    hooks::{HookContext, HookContextData, ItemActionStage, ItemActionType, RequestContext},
};

use super::{auth::User, dto::Event, hooks::HookSuccessResult, ApiContext, ApiErrors};

#[debug_handler]
pub(crate) async fn api_create_event(
    State(ctx): State<ApiContext>,
    JwtClaims(user): JwtClaims<User>,
    Json(payload): Json<CreateEventBody>,
) -> Result<(StatusCode, String), ApiErrors> {
    // Validate the payload
    payload.validate().map_err(ApiErrors::from)?;
    let unchecked_collection_name = payload.collection;
    let unchecked_document_id = payload.document;

    let collection = get_collection_by_name(&ctx.db, &unchecked_collection_name).await;
    if collection.is_none() {
        debug!("Collection {} not found", unchecked_collection_name);
        return Err(ApiErrors::PermissionDenied);
    }
    let collection_name = unchecked_collection_name;

    if !user.is_collection_reader(&collection_name) {
        debug!("User {} is not a collection reader", user.name_and_sub());
        return Err(ApiErrors::PermissionDenied);
    }

    let collection = collection.unwrap();
    let hook_transmitter = ctx.hooks.get_registered_hook(
        collection_name.as_ref(),
        ItemActionType::AppendEvent,
        ItemActionStage::Before,
    );

    ctx.db
        .transaction::<_, (StatusCode, String), ApiErrors>(|txn| {
            Box::pin(async move {
                let document = select_document_for_update(unchecked_document_id, txn).await?;
                if document.is_none() {
                    debug!("Document {} not found", unchecked_document_id);
                    return Err(ApiErrors::PermissionDenied);
                }
                let document = document.unwrap();

                if hook_transmitter.is_none() {
                    debug!("No hook was executed");
                    return Err(ApiErrors::BadRequest("Event not accepted".to_string()));
                }
                let hook_transmitter = hook_transmitter.unwrap();

                let (tx, rx) = oneshot::channel::<Result<HookSuccessResult, ApiErrors>>();
                let cdctx = HookContext::new(
                    HookContextData::EventAdding {
                        document: (&document).into(),
                        collection: (&collection).into(),
                        event: Event::new(payload.category, payload.e.clone()),
                    },
                    RequestContext::new(collection),
                    tx,
                );

                hook_transmitter
                    .send(cdctx)
                    .await
                    .map_err(|_e| ApiErrors::InternalServerError)?;

                let events = rx
                    .await
                    .map_err(|_e| ApiErrors::InternalServerError)??
                    .events;
                if events.is_empty() {
                    debug!("No events were permitted");
                    return Err(ApiErrors::PermissionDenied);
                }

                debug!("Try to create {} event(s)", events.len());
                for event in events {
                    // Create the event in the database
                    let dbevent = entity::event::ActiveModel {
                        id: NotSet,
                        category_id: Set(event.category()),
                        timestamp: NotSet,
                        document_id: Set(document.id),
                        user: Set(user.subuuid()),
                        payload: Set(payload.e.clone()),
                    };
                    let res = dbevent.save(txn).await?;

                    debug!("Event {:?} saved", res.id);
                }

                Ok((StatusCode::CREATED, "Done".to_string()))
            })
        })
        .await
        .map_err(|err| match err {
            TransactionError::Connection(c) => Into::<ApiErrors>::into(c),
            TransactionError::Transaction(t) => t,
        })
}

async fn select_document_for_update(
    unchecked_document_id: uuid::Uuid,
    txn: &DatabaseTransaction,
) -> Result<Option<entity::collection_document::Model>, DbErr> {
    
    Documents::find()
        .from_raw_sql(sea_orm::Statement::from_sql_and_values(
            sea_orm::DbBackend::Postgres,
            r#"SELECT * FROM "collection_document" WHERE "id" = $1 FOR UPDATE"#,
            [unchecked_document_id.into()],
        ))
        .one(txn)
        .await
}
