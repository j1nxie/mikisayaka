use std::sync::LazyLock;

pub mod version;

pub static POISE_VERSION: &str = "0.6.1";
pub static STARTUP_TIME: LazyLock<std::time::SystemTime> =
    LazyLock::new(std::time::SystemTime::now);
pub static MD_URL_REGEX: LazyLock<fancy_regex::Regex> = LazyLock::new(|| {
    fancy_regex::Regex::new(r"(?<!<)https://mangadex\.org/title/([a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12})(?!>)").unwrap()
});
pub static YOUTUBE_URL_REGEX: LazyLock<fancy_regex::Regex> = LazyLock::new(|| {
    fancy_regex::Regex::new(r"(?:https?://)?(?:(?:www\.)?youtube\.com/watch\?v=|(?:www\.)?youtu\.be/|(?:music\.youtube\.com)/watch\?v=)([a-zA-Z0-9_-]{11})").unwrap()
});
pub static SPOTIFY_URL_REGEX: LazyLock<fancy_regex::Regex> = LazyLock::new(|| {
    fancy_regex::Regex::new(
        r"(?:https?://)?(?:open\.)?spotify\.com/(?:track|album)/([a-zA-Z0-9]{22})(?:\?.*)?",
    )
    .unwrap()
});
pub static AZUKI_MANGA: LazyLock<uuid::Uuid> =
    LazyLock::new(|| uuid::Uuid::try_parse("5fed0576-8b94-4f9a-b6a7-08eecd69800d").unwrap());
pub static BILIBILI_COMICS: LazyLock<uuid::Uuid> =
    LazyLock::new(|| uuid::Uuid::try_parse("06a9fecb-b608-4f19-b93c-7caab06b7f44").unwrap());
pub static COMIKEY: LazyLock<uuid::Uuid> =
    LazyLock::new(|| uuid::Uuid::try_parse("8d8ecf83-8d42-4f8c-add8-60963f9f28d9").unwrap());
pub static INKR: LazyLock<uuid::Uuid> =
    LazyLock::new(|| uuid::Uuid::try_parse("caa63201-4a17-4b7f-95ff-ed884a2b7e60").unwrap());
pub static MANGAHOT: LazyLock<uuid::Uuid> =
    LazyLock::new(|| uuid::Uuid::try_parse("319c1b10-cbd0-4f55-a46e-c4ee17e65139").unwrap());
pub static MANGAPLUS: LazyLock<uuid::Uuid> =
    LazyLock::new(|| uuid::Uuid::try_parse("4f1de6a2-f0c5-4ac5-bce5-02c7dbb67deb").unwrap());
pub static MD_BLOCKED_LIST: LazyLock<Vec<uuid::Uuid>> = LazyLock::new(|| {
    vec![
        *AZUKI_MANGA,
        *BILIBILI_COMICS,
        *COMIKEY,
        *INKR,
        *MANGAHOT,
        *MANGAPLUS,
    ]
});
