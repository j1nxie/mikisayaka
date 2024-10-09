use async_trait::async_trait;
use sea_orm_migration::prelude::*;

mod m_20240810_000001_initial_setup;
mod m_20241009_000002_mangadex;

pub struct Migrator;

#[async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn sea_orm_migration::prelude::MigrationTrait>> {
        vec![
            Box::new(m_20240810_000001_initial_setup::Migration),
            Box::new(m_20241009_000002_mangadex::Migration),
        ]
    }
}

#[derive(Iden)]
pub enum Roles {
    Table,
    Id,
    Name,
    RoleId,
}

#[derive(Iden)]
#[allow(clippy::enum_variant_names)]
pub enum Manga {
    Table,
    Id,
    MangaDexId,
    LastUpdated,
}
