use async_trait::async_trait;
use sea_orm_migration::prelude::*;

use super::Manga;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m_20241009_000002_mangadex"
    }
}

#[async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let roles = Table::create()
            .table(Manga::Table)
            .if_not_exists()
            .col(
                ColumnDef::new(Manga::Id)
                    .integer()
                    .not_null()
                    .primary_key()
                    .auto_increment(),
            )
            .col(ColumnDef::new(Manga::MangaDexId).uuid().not_null())
            .col(ColumnDef::new(Manga::LastUpdated).date_time().not_null())
            .to_owned();

        manager.create_table(roles).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let roles = Table::drop().table(Manga::Table).to_owned();

        manager.drop_table(roles).await?;

        Ok(())
    }
}
