use std::collections::HashMap;

use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use sea_query::RcOrArc;
use uuid::Uuid;

use crate::api::{
    db::{get_collection_by_name, FieldFilter},
    dto::{self, CollectionDocument},
    ApiErrors,
};
use entity::collection_document::{Column as DocumentsColumns, Entity as Documents};
use tracing::debug;

struct CachedCollection {
    id: Uuid,
    dto: dto::Collection,
}

pub(crate) struct DocumentService {
    collection_id_cache: HashMap<String, std::sync::Arc<CachedCollection>>,
}

impl DocumentService {
    pub(crate) fn new() -> Self {
        Self {
            collection_id_cache: HashMap::new(),
        }
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

    async fn lookup_get_collection_by_name(
        &self,
        db: &DatabaseConnection,
        collection_name: &str,
    ) -> Option<CachedCollection> {
        if self.collection_id_cache.contains_key(collection_name) {
            debug!("Found cached collection {}", collection_name);
            return Some(self.collection_id_cache[collection_name].clone());
        }

        let dbmodel = crate::api::db::get_collection_by_name(db, collection_name).await?;

        let cc = std::sync::Arc::new(CachedCollection {
            id: dbmodel.id,
            dto: dbmodel.into(),
        });
        debug!("Adding cached collection {}", collection_name);
        self.collection_id_cache
            .insert(collection_name.to_string(), cc.clone());

        Some(cc)
    }

    pub(crate) async fn get_collection_by_name(
        &self,
        db: &DatabaseConnection,
        collection_name: &str,
    ) -> Option<dto::Collection> {
        let collection = self
            .lookup_get_collection_by_name(db, collection_name)
            .await;
        collection.as_ref()?;
        Some((&collection.unwrap()).into())
    }

    pub(crate) async fn get_collection_documents(
        &self,
        db: &DatabaseConnection,
        collection_name: &str,
    ) -> anyhow::Result<Vec<dto::CollectionDocument>> {
        let collection = self
            .lookup_get_collection_by_name(db, collection_name)
            .await;

        let items = Documents::find()
            .filter(DocumentsColumns::CollectionId.eq(collection.id))
            .all(db)
            .await?;
        debug!("Found {} documents", items.len());
        Ok(items.into_iter().map(|item| (&item).into()).collect())
    }
}

#[cfg(test)]
mod tests {
    use sea_orm::{DbBackend, EntityTrait, QueryFilter, QueryTrait};
    use sea_query::Expr;
    use uuid::Uuid;

    #[test]
    fn it_works() {
        // Arrange
        let uid = Uuid::new_v4();
        let stmt = entity::collection_document::Entity::find()
            .filter(Expr::cust(r#""f"->'user'->>'id'"#).eq(uid))
            .build(DbBackend::Postgres);

        // Act
        let sql = stmt.to_string();

        // Assert
        assert_eq!(
            sql,
            format!(
                r#"SELECT "collection_document"."id", "collection_document"."collection_id", "collection_document"."owner", "collection_document"."f" FROM "collection_document" WHERE ("f"->'user'->>'id') = '{uid}'"#
            )
        );
    }
}
