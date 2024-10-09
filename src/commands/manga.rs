use crate::{Context, Error};
use poise::serenity_prelude::{self as serenity, CreateAllowedMentions};

/// check mangadex client's availability
async fn check_md_client(ctx: Context<'_>) -> Result<(), Error> {
    if ctx.data().md.is_none() {
        ctx.send(
            poise::CreateReply::default()
                .reply(true)
                .allowed_mentions(CreateAllowedMentions::new().replied_user(false))
                .content("mangadex client is not initialized. this command will not work."),
        )
        .await?;

        return Err("mangadex client is not initialized.".into());
    }

    Ok(())
}

/// commands related to self-assignable roles.
#[poise::command(
    slash_command,
    prefix_command,
    subcommand_required,
    guild_only,
    subcommands("add")
)]
pub async fn manga(_: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// add a manga to the tracking list
#[poise::command(prefix_command, slash_command)]
pub async fn add(
    ctx: Context<'_>,
    #[description = "mangadex uuid of the manga you want to add."] uuid: String,
) -> Result<(), Error> {
    if check_md_client(ctx).await.is_err() {
        return Ok(());
    }

    let uuid = uuid::Uuid::try_parse(&uuid);

    if uuid.is_err() {
        ctx.send(
            poise::CreateReply::default()
                .reply(true)
                .allowed_mentions(CreateAllowedMentions::new().replied_user(false))
                .content("invalid uuid supplied."),
        )
        .await?;

        return Ok(());
    }

    let uuid = uuid.unwrap();

    let manga = ctx
        .data()
        .md
        .as_ref()
        .unwrap()
        .manga()
        .id(uuid)
        .get()
        .send()
        .await?;

    let chapter_id = manga.data.attributes.latest_uploaded_chapter;

    if chapter_id.is_none() {
        ctx.send(
            poise::CreateReply::default()
                .reply(true)
                .allowed_mentions(CreateAllowedMentions::new().replied_user(false))
                .content("manga has no chapters."),
        )
        .await?;

        return Ok(());
    }

    let chapter_id = chapter_id.unwrap();

    let chapter = ctx
        .data()
        .md
        .as_ref()
        .unwrap()
        .chapter()
        .id(chapter_id)
        .get()
        .send()
        .await?;

    ctx.send(
        poise::CreateReply::default()
            .reply(true)
            .allowed_mentions(CreateAllowedMentions::new().replied_user(false))
            .content("this feature is not yet implemented"),
    )
    .await?;

    // ctx.send(
    //     poise::CreateReply::default()
    //         .reply(true)
    //         .allowed_mentions(CreateAllowedMentions::new().replied_user(false))
    //         .content(format!(
    //             "manga result: {:?}\nlast chapter date: {:?}",
    //             manga.data.attributes.latest_uploaded_chapter, chapter.data.attributes.created_at,
    //         )),
    // )
    // .await?;

    Ok(())
}
