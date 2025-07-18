use std::sync::LazyLock;

use fancy_regex::Regex;

pub static TWITTER_URL_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"https?://(?:www\.)?(twitter\.com|x\.com)/([\w.-]+)/status/(\d+)").unwrap()
});

pub static TIKTOK_URL_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"https?://vt\.tiktok\.com/(\w+)").unwrap());

pub static PIXIV_ARTWORK_URL_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"https?://(?:www\.)pixiv\.net/(?:(?P<lang>[a-z]{2})/)?artworks/(?P<id>\d+)(?:/(?P<idx>\d+))?",
    )
    .unwrap()
});

pub static PIXIV_SHORT_URL_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"https?://(?:www\.)pixiv\.net/i/(\d+)").unwrap());

pub static PIXIV_LEGACY_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"https?://(?:www\.)?pixiv\.net/member_illust\.php\?illust_id=(\d+)").unwrap()
});
