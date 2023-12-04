pub use sea_orm_migration::prelude::*;

mod m20220101_000001_basic;
mod m20230623_190444_events;
mod m20231203_180149_grants;

pub struct Migrator;
pub use m20220101_000001_basic::CollectionDocument;
pub use m20231203_180149_grants::Grant;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_basic::Migration),
            Box::new(m20230623_190444_events::Migration),
            Box::new(m20231203_180149_grants::Migration),
        ]
    }
}
