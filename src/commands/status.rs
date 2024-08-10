use std::time::UNIX_EPOCH;

use crate::{
    commands::get_bot_avatar,
    constants::{version::get_version, POISE_VERSION, STARTUP_TIME},
    Context, Error,
};
use poise::serenity_prelude as serenity;

/// get the bot's status
#[poise::command(prefix_command, slash_command)]
pub async fn status(ctx: Context<'_>) -> Result<(), Error> {
    ctx.send(poise::CreateReply::default().embed(
        serenity::CreateEmbed::new()
        .field(
            "about the bot",
            "[Miki Sayaka](https://github.com/j1nxie/mikisayaka) is a housekeeper Discord bot for Rylie's Nazunacord, written by [j1nxie](https://github.com/j1nxie), using the [poise](https://github.com/serenity-rs/poise) framework.".to_string(),
            false
        )
        .field("version", get_version(), false)
        .field("rust", format!("[{0}](https://releases.rs/docs/{0})", rustc_version_runtime::version().to_string()), true)
        .field("poise", format!("[{0}](https://docs.rs/crate/poise/{0})", POISE_VERSION), true)
        .field("uptime", format!("<t:{}:R>", STARTUP_TIME.duration_since(UNIX_EPOCH).unwrap().as_secs()), true)
        .thumbnail(get_bot_avatar(ctx))
    )).await?;

    Ok(())
}