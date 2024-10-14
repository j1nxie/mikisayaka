use async_trait::async_trait;
use sea_orm_migration::prelude::*;

use super::Manga;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m_20241014_000003_manga_last_updated"
    }
}

#[async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let update = Table::alter()
            .table(Manga::Table)
            .add_column_if_not_exists(ColumnDef::new(Manga::LastChapterDate).date_time())
            .to_owned();

        manager.alter_table(update).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let update = Table::alter()
            .table(Manga::Table)
            .drop_column(Manga::LastChapterDate)
            .to_owned();

        manager.alter_table(update).await?;

        Ok(())
    }
}
