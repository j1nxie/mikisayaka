use crate::{Context, Error};

#[poise::command(prefix_command, slash_command)]
pub async fn squartatrice(ctx: Context<'_>) -> Result<(), Error> {
    let random_number = rand::random::<u8>();

    let content = if random_number > 127 {
        "https://www.youtube.com/watch?v=mdWEHMxQqn8"
    } else {
        "https://www.youtube.com/watch?v=a2qUNdQySgw"
    };

    ctx.send(poise::CreateReply::default().reply(true).content(content))
        .await?;

    Ok(())
}
