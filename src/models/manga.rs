use time::OffsetDateTime;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Manga {
    pub id: i64,
    pub manga_dex_id: uuid::fmt::Hyphenated,
    pub last_updated: OffsetDateTime,
    pub last_chapter_date: Option<OffsetDateTime>,
}
