use poise::serenity_prelude as serenity;

use crate::constants::embeds::{FIXUPX_URL_REGEX, TWITTER_URL_REGEX};
use crate::{Context, Error};

#[tracing::instrument(skip_all)]
#[poise::command(context_menu_command = "Translate tweet to English")]
pub async fn translate(ctx: Context<'_>, mut message: serenity::Message) -> Result<(), Error> {
    let content = message.content.clone();

    let captures = TWITTER_URL_REGEX
        .captures(&content)
        .ok()
        .flatten()
        .or_else(|| FIXUPX_URL_REGEX.captures(&content).ok().flatten());

    match captures {
        Some(captures) => {
            let user = &captures[2];
            let tweet_id = &captures[3];

            let replacement_url = format!("https://fixupx.com/{}/status/{}/en", &user, &tweet_id);

            if message.author.id == ctx.framework().bot_id {
                ctx.defer_ephemeral().await?;

                message
                    .edit(
                        ctx.http(),
                        serenity::EditMessage::new()
                            .content(replacement_url)
                            .allowed_mentions(
                                serenity::CreateAllowedMentions::new().replied_user(false),
                            ),
                    )
                    .await?;

                ctx.send(
                    poise::CreateReply::default()
                        .content("all done!")
                        .ephemeral(true),
                )
                .await?;
            } else {
                ctx.defer().await?;

                message
                    .edit(
                        ctx.http(),
                        serenity::EditMessage::new().suppress_embeds(true),
                    )
                    .await?;

                ctx.reply(&replacement_url).await?;
            }
        }
        _ => {
            ctx.send(
                poise::CreateReply::default()
                    .content("didn't match a (untranslated) twitter URL in your message!")
                    .ephemeral(true),
            )
            .await
            .inspect_err(|e| tracing::error!(err = ?e, "an error occurred when sending reply"))?;
        }
    }

    Ok(())
}
