use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "manga")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub manga_dex_id: Uuid,
    pub last_updated: TimeDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
