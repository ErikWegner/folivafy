use entity::event::{self, Entity as Events};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use uuid::Uuid;

use crate::api::dto;

pub(crate) struct DocumentEventService {}

impl DocumentEventService {
    pub(crate) fn new() -> Self {
        Self {}
    }

    pub(crate) async fn get_document_events_newest_first(
        &self,
        db: &sea_orm::DatabaseConnection,
        document_id: Uuid,
    ) -> Result<Vec<crate::api::dto::ExistingEvent>, anyhow::Error> {
        Ok(Events::find()
            .filter(event::Column::DocumentId.eq(document_id))
            .order_by_desc(event::Column::Id)
            .all(db)
            .await?
            .into_iter()
            .map(|event| dto::ExistingEvent::from(&event))
            .collect())
    }
}
