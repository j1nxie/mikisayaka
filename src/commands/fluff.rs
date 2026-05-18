use poise::serenity_prelude::*;
use rand::prelude::*;

use crate::{Context, Error};

/// あたしって、ほんとばか。
///
/// you want to listen to squartatrice? here you go.
#[tracing::instrument(skip_all)]
#[poise::command(prefix_command)]
pub async fn quartatrice(ctx: Context<'_>) -> Result<(), Error> {
    let random_number = random::<u8>();

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
    let random_number = random::<u8>();

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

/// pick an option between user-provided options
///
/// example: s>pick option1 / option2 / option3
#[tracing::instrument(skip_all)]
#[poise::command(prefix_command)]
pub async fn pick(ctx: Context<'_>, #[rest] input: String) -> Result<(), Error> {
    let options = input.split('/').map(|f| f.trim()).collect::<Vec<_>>();

    if options.is_empty() {
        ctx.send(poise::CreateReply::default().content("no options provided."))
            .await
            .inspect_err(|e| tracing::error!(err = ?e, "an error occurred when sending reply"))?;
    }

    let choice = {
        let mut rng = thread_rng();
        *options.choose(&mut rng).unwrap()
    };

    ctx.send(poise::CreateReply::default().content(choice))
        .await
        .inspect_err(|e| tracing::error!(err = ?e, "an error occurred when sending reply"))?;

    Ok(())
}
