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
                    .table(Grant::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Grant::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Grant::DocumentId).uuid().not_null())
                    .col(ColumnDef::new(Grant::Realm).string_len(150).not_null())
                    .col(ColumnDef::new(Grant::Grant).uuid().not_null())
                    .col(
                        ColumnDef::new(Grant::View)
                            .boolean()
                            .default(true)
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-event-document_id")
                            .from(Grant::Table, Grant::DocumentId)
                            .to(CollectionDocument::Table, CollectionDocument::Id),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Grant::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Grant {
    Table,
    Id,
    DocumentId,
    Realm,
    Grant,
    View,
}
