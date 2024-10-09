use std::sync::LazyLock;

pub mod version;

pub static POISE_VERSION: &str = "0.6.1";
pub static STARTUP_TIME: LazyLock<std::time::SystemTime> =
    LazyLock::new(std::time::SystemTime::now);
pub static MD_URL_REGEX: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"https://mangadex\.org/title/([a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12})").unwrap()
});
