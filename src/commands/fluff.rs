use poise::serenity_prelude::CreateAllowedMentions;

use crate::{Context, Error};

/// あたしって、ほんとばか。
///
/// you want to listen to squartatrice? here you go.
#[poise::command(prefix_command, slash_command)]
pub async fn squartatrice(ctx: Context<'_>) -> Result<(), Error> {
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
