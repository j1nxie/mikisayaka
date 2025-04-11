use std::sync::LazyLock;

pub static YOUTUBE_URL_REGEX: LazyLock<fancy_regex::Regex> = LazyLock::new(|| {
    fancy_regex::Regex::new(r"(?:https?://)?(?:(?:www\.)?youtube\.com/watch\?v=|(?:www\.)?youtu\.be/|(?:music\.youtube\.com)/watch\?v=)([a-zA-Z0-9_-]{11})").unwrap()
});
pub static SPOTIFY_URL_REGEX: LazyLock<fancy_regex::Regex> = LazyLock::new(|| {
    fancy_regex::Regex::new(
        r"(?:https?://)?(?:open\.)?spotify\.com/(?:track|album)/([a-zA-Z0-9]{22})(?:\?.*)?",
    )
    .unwrap()
});
