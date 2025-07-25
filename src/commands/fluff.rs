use poise::serenity_prelude::*;

use crate::{Context, Error};

/// あたしって、ほんとばか。
///
/// you want to listen to squartatrice? here you go.
#[tracing::instrument(skip_all)]
#[poise::command(prefix_command)]
pub async fn quartatrice(ctx: Context<'_>) -> Result<(), Error> {
    let random_number = rand::random::<u8>();

    let content = if random_number > 127 {
        "https://www.youtube.com/watch?v=mdWEHMxQqn8"
    } else {
        "https://www.youtube.com/watch?v=a2qUNdQySgw"
    };

    ctx.send(
        poise::CreateReply::default()
            .reply(true)
            .allowed_mentions(CreateAllowedMentions::new().replied_user(false))
            .content(content),
    )
    .await
    .inspect_err(|e| tracing::error!(err = ?e, "an error occurred when sending reply"))?;

    Ok(())
}

/// do not play this with 0.7 slide delay.
#[tracing::instrument(skip_all)]
#[poise::command(prefix_command)]
pub async fn itl(ctx: Context<'_>) -> Result<(), Error> {
    let random_number = rand::random::<u8>();

    let content = if random_number > 127 {
        "https://www.youtube.com/watch?v=MKuicDvnaFc"
    } else {
        "https://www.youtube.com/watch?v=zqH9qgVNzHI"
    };

    ctx.send(
        poise::CreateReply::default()
            .reply(true)
            .allowed_mentions(CreateAllowedMentions::new().replied_user(false))
            .content(content),
    )
    .await
    .inspect_err(|e| tracing::error!(err = ?e, "an error occurred when sending reply"))?;

    Ok(())
}
