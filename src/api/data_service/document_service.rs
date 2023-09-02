use sea_orm::{DatabaseConnection, EntityTrait};
use uuid::Uuid;

use crate::api::{db::get_collection_by_name, dto};
use entity::{collection_document::Entity as Documents, event};

pub(crate) struct DocumentService {}

impl DocumentService {
    pub(crate) fn new() -> Self {
        Self {}
    }

    pub(crate) async fn get_document(
        &self,
        db: &DatabaseConnection,
        collection_name: &str,
        uuid: Uuid,
    ) -> Option<dto::CollectionDocument> {
        let collection = get_collection_by_name(db, &collection_name).await;
        if collection.is_none() {
            return None;
        }

        let collection = collection.unwrap();

        let document = Documents::find_by_id(uuid)
            .one(db)
            .await
            .ok()?
            .and_then(|doc| (doc.collection_id == collection.id).then_some(doc));

        if document.is_none() {
            return None;
        }
        Some((&document.unwrap()).into())
    }
}
