use async_trait::async_trait;
use sea_orm_migration::prelude::*;

use super::Roles;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m_20240810_000001_initial_setup"
    }
}

#[async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let roles = Table::create()
            .table(Roles::Table)
            .if_not_exists()
            .col(
                ColumnDef::new(Roles::Id)
                    .integer()
                    .not_null()
                    .primary_key()
                    .auto_increment(),
            )
            .col(ColumnDef::new(Roles::Name).string().not_null())
            .col(ColumnDef::new(Roles::RoleId).string().not_null())
            .to_owned();

        manager.create_table(roles).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let roles = Table::drop().table(Roles::Table).to_owned();

        manager.drop_table(roles).await?;

        Ok(())
    }
}
