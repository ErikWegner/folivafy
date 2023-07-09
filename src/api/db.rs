use anyhow::Context;
use entity::collection::Model;
use sea_orm::{
    ActiveModelTrait, ActiveValue::NotSet, ColumnTrait, DatabaseConnection, DatabaseTransaction,
    EntityTrait, QueryFilter, Set,
};
use tracing::{debug, error, info};
use uuid::Uuid;

use super::{
    auth::User,
    dto::{self, Event},
};

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
