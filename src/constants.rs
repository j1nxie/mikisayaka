use std::sync::LazyLock;

pub mod embeds;
pub mod gas_prices;
pub mod manga;
pub mod music;
pub mod version;
pub mod zenless;

pub static POISE_VERSION: &str = "0.6.1";
pub static STARTUP_TIME: LazyLock<std::time::SystemTime> =
    LazyLock::new(std::time::SystemTime::now);
