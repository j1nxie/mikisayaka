use async_trait::async_trait;
use sea_orm_migration::prelude::*;

mod m_20240810_000001_initial_setup;

pub struct Migrator;

#[async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn sea_orm_migration::prelude::MigrationTrait>> {
        vec![Box::new(m_20240810_000001_initial_setup::Migration)]
    }
}

#[derive(Iden)]
pub enum Roles {
    Table,
    Id,
    Name,
    RoleId,
}
