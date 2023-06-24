use sea_orm_migration::prelude::*;

use crate::CollectionDocument;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Event::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Event::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Event::Timestamp)
                            .timestamp()
                            .default(SimpleExpr::Keyword(Keyword::CurrentTimestamp)),
                    )
                    .col(ColumnDef::new(Event::DocumentId).uuid().not_null())
                    .col(ColumnDef::new(Event::User).uuid().not_null())
                    .col(ColumnDef::new(Event::CategoryId).integer().not_null())
                    .col(ColumnDef::new(Event::Payload).json_binary().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-event-document_id")
                            .from(Event::Table, Event::DocumentId)
                            .to(CollectionDocument::Table, CollectionDocument::Id),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Event::Table).to_owned())
            .await
    }
}

/// Learn more at https://docs.rs/sea-query#iden
#[derive(Iden)]
enum Event {
    Table,
    Id,
    Timestamp,
    User,
    DocumentId,
    CategoryId,
    Payload,
}
