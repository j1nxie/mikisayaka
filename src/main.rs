use constants::STARTUP_TIME;
use constants::manga::MD_URL_REGEX;
use constants::music::{SPOTIFY_URL_REGEX, YOUTUBE_URL_REGEX};
use mangadex_api::MangaDexClient;
use poise::serenity_prelude::{self as serenity, *};
use sqlx::{Pool, Sqlite};

use crate::constants::embeds::{TIKTOK_URL_REGEX, TWITTER_URL_REGEX};
use crate::handlers::{
    md_handler, pixiv_handler, quote_handler, spotify_handler, tiktok_handler, twitter_handler,
    youtube_handler,
};
use crate::zenless::ZenlessClient;

#[derive(Clone)]
struct Data {
    gas_prices_channel_id: Option<ChannelId>,
    manga_update_channel_id: Option<ChannelId>,
    music_channel_id: Option<ChannelId>,
    reqwest_client: reqwest::Client,
    zenless_client: ZenlessClient,
    db: Pool<Sqlite>,
    md: Option<MangaDexClient>,
    mdlist_id: Option<uuid::Uuid>,
}

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

mod chapter_tracker;
mod commands;
mod constants;
mod gas_prices;
mod handlers;
mod init;
mod models;
mod telemetry;
mod zenless;

#[tracing::instrument(skip(ctx, _framework, data))]
async fn event_handler(
    ctx: &serenity::Context,
    event: &serenity::FullEvent,
    _framework: poise::FrameworkContext<'_, Data, Error>,
    data: &Data,
) -> Result<(), Error> {
    if let serenity::FullEvent::Message { new_message } = event {
        if new_message.author.bot
            || data.md.is_none()
            || data.manga_update_channel_id.is_none()
            || new_message.content.starts_with("s>")
        {
            return Ok(());
        }

        if new_message.channel_id == data.music_channel_id.unwrap() {
            if let Ok(Some(captures)) = YOUTUBE_URL_REGEX.captures(&new_message.content) {
                youtube_handler(ctx, data, new_message, captures).await?;
            }

            if let Ok(Some(captures)) = SPOTIFY_URL_REGEX.captures(&new_message.content) {
                spotify_handler(ctx, data, new_message, captures).await?;
            }
        }

        if new_message.channel_id == data.manga_update_channel_id.unwrap() {
            if let Ok(Some(captures)) = MD_URL_REGEX.captures(&new_message.content) {
                md_handler(ctx, data, new_message, captures).await?;
            }
        }

        if let Ok(Some(captures)) = TWITTER_URL_REGEX.captures(&new_message.content) {
            twitter_handler(ctx, new_message, captures).await?;
        }

        if let Ok(Some(captures)) = TIKTOK_URL_REGEX.captures(&new_message.content) {
            tiktok_handler(ctx, new_message, captures).await?;
        }

        if new_message.content.starts_with("... ") {
            quote_handler(ctx, data, new_message).await?;
        }

        pixiv_handler(ctx, new_message).await?;
    }

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();
    let _ = &*STARTUP_TIME;

    let mut client = init::init().await?;
    client.start().await?;

    Ok(())
}
