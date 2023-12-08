use std::str::FromStr;

use sea_orm::{ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, FromQueryResult, QueryFilter};
use sea_query::{Alias, Expr, Query};
use serde_json::Value as JsonValue;
use uuid::Uuid;

use crate::api::{
    db::get_collection_by_name,
    dto::{self, CollectionDocument},
    ApiErrors,
};
use entity::collection_document::{Column as DocumentsColumns, Entity as Documents};
use tracing::debug;

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
        let collection = get_collection_by_name(db, collection_name).await;
        collection.as_ref()?;

        let collection = collection.unwrap();

        let document = Documents::find_by_id(uuid)
            .one(db)
            .await
            .ok()?
            .and_then(|doc| (doc.collection_id == collection.id).then_some(doc));

        document.as_ref()?;
        Some((&document.unwrap()).into())
    }

    pub(crate) async fn get_collection_by_name(
        &self,
        db: &DatabaseConnection,
        collection_name: &str,
    ) -> Option<dto::Collection> {
        crate::api::db::get_collection_by_name(db, collection_name)
            .await
            .map(|m| (&m).into())
    }

    pub(crate) async fn get_collection_documents(
        &self,
        db: &DatabaseConnection,
        collection_name: &str,
    ) -> anyhow::Result<Vec<dto::CollectionDocument>> {
        let collection = crate::api::db::get_collection_by_name(db, collection_name).await;

        let collection = collection.unwrap();

        let documents_alias = Alias::new("d");
        let mut b = Query::select();
        let q = b
            .from_as(Documents, documents_alias.clone())
            .and_where(Expr::col(DocumentsColumns::CollectionId).eq(collection.id));
        let sql = q.to_owned();
        let builder = db.get_database_backend();
        let stmt = builder.build(&sql);

        let items = Documents::find()
            .filter(DocumentsColumns::CollectionId.eq(collection.id))
            .all(db)
        .await?;
        debug!("Found {} documents", items.len());
        Ok(items.into_iter().map(|item| (&item).into()).collect())
    }
}
