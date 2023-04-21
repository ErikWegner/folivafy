use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Collection::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Collection::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Collection::Name)
                            .string_len(32)
                            .unique_key()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Collection::Title).string_len(150).not_null())
                    .col(ColumnDef::new(Collection::Oao).boolean().not_null())
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx-col_name")
                    .table(Collection::Table)
                    .col(Collection::Name)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(Index::drop().name("idx-col_name").to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Collection::Table).to_owned())
            .await?;
        Ok(())
    }
}

/// Learn more at https://docs.rs/sea-query#iden
#[derive(Iden)]
enum Collection {
    Table,
    Id,
    Name,
    Title,
    Oao,
}
