//! `SeaORM` Entity, @generated by sea-orm-codegen 1.1.3

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "grant")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub document_id: Uuid,
    pub realm: String,
    pub grant: Uuid,
    pub view: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::collection_document::Entity",
        from = "Column::DocumentId",
        to = "super::collection_document::Column::Id",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    CollectionDocument,
}

impl Related<super::collection_document::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::CollectionDocument.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
