//! `SeaORM` Entity. Generated by sea-orm-codegen 0.11.3

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "collection_document")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub collection_id: Uuid,
    pub owner: Uuid,
    #[sea_orm(column_type = "JsonBinary")]
    pub f: Json,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::collection::Entity",
        from = "Column::CollectionId",
        to = "super::collection::Column::Id",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    Collection,
    #[sea_orm(has_many = "super::event::Entity")]
    Event,
}

impl Related<super::collection::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Collection.def()
    }
}

impl Related<super::event::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Event.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
