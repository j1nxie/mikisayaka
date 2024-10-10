use crate::{constants::MD_URL_REGEX, models::manga, Context, Error};
use poise::serenity_prelude::{CreateAllowedMentions, CreateEmbed};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, Set};

/// check mangadex client's availability.
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

    if ctx.data().mdlist_id.is_none() {
        ctx.send(
            poise::CreateReply::default()
                .reply(true)
                .allowed_mentions(CreateAllowedMentions::new().replied_user(false))
                .content("mdlist uuid is not set. this command will not work."),
        )
        .await?;

        return Err("mdlist uuid is not set.".into());
    }

    Ok(())
}

/// commands related to manga tracking.
#[poise::command(
    slash_command,
    prefix_command,
    subcommand_required,
    guild_only,
    subcommands("add", "list", "sync")
)]
pub async fn manga(_: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// add a manga to the tracking list.
#[poise::command(prefix_command, slash_command)]
pub async fn add(
    ctx: Context<'_>,
    #[description = "mangadex uuid or link of the manga you want to add."] input: String,
) -> Result<(), Error> {
    if check_md_client(ctx).await.is_err() {
        return Ok(());
    }

    ctx.data()
        .md
        .as_ref()
        .unwrap()
        .oauth()
        .refresh()
        .send()
        .await?;

    let uuid = if let Some(captures) = MD_URL_REGEX.captures(&input) {
        if let Ok(u) = uuid::Uuid::try_parse(&captures[1]) {
            tracing::info!("got uuid from link: {}", u);
            u
        } else {
            ctx.send(
                poise::CreateReply::default()
                    .reply(true)
                    .allowed_mentions(CreateAllowedMentions::new().replied_user(false))
                    .content("invalid uuid supplied."),
            )
            .await?;

            return Ok(());
        }
    } else if let Ok(u) = uuid::Uuid::try_parse(&input) {
        tracing::info!("got uuid from input string: {}", u);
        u
    } else {
        ctx.send(
            poise::CreateReply::default()
                .reply(true)
                .allowed_mentions(CreateAllowedMentions::new().replied_user(false))
                .content("invalid link supplied."),
        )
        .await?;

        return Ok(());
    };

    let manga_list = manga::Entity::find().all(&ctx.data().db).await?;

    let mdlist = ctx
        .data()
        .md
        .as_ref()
        .unwrap()
        .custom_list()
        .id(ctx.data().mdlist_id.unwrap())
        .get()
        .send()
        .await?;

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

    let manga = manga.data.attributes;

    let title = if let Some(en_title) = manga.title.get(&mangadex_api_types_rust::Language::English)
    {
        en_title
    } else if let Some(jp_ro) = manga
        .title
        .get(&mangadex_api_types_rust::Language::JapaneseRomanized)
    {
        jp_ro
    } else {
        manga
            .title
            .get(&mangadex_api_types_rust::Language::Japanese)
            .unwrap()
    };

    if manga::Entity::find()
        .filter(manga::Column::MangaDexId.eq(uuid))
        .one(&ctx.data().db)
        .await?
        .is_some()
    {
        ctx.send(
            poise::CreateReply::default()
                .reply(true)
                .allowed_mentions(CreateAllowedMentions::new().replied_user(false))
                .content(format!(
                    "title **{}** is already in the tracking list.",
                    title
                )),
        )
        .await?;

        return Ok(());
    }

    let model = manga::ActiveModel {
        manga_dex_id: Set(uuid),
        last_updated: Set(chrono::Utc::now().naive_utc()),
        ..Default::default()
    };

    model.insert(&ctx.data().db).await?;

    let mut builder = ctx
        .data()
        .md
        .as_ref()
        .unwrap()
        .custom_list()
        .id(ctx.data().mdlist_id.unwrap())
        .put();

    for manga in manga_list {
        builder.add_manga_id(manga.manga_dex_id);
    }

    let mut resp_string = String::new();

    let _ = builder
        .add_manga_id(uuid)
        .version(mdlist.data.attributes.version)
        .build()
        .unwrap()
        .send()
        .await
        .map_err(|e| {
            tracing::warn!("an error happened while updating the mdlist: {}", e);
            resp_string = "*failed to update the mdlist. it will (hopefully) be updated the next time you add a manga. you can also try running `s>manga sync` to sync the mdlist.*\n\n".to_string()
        });

    ctx.send(
        poise::CreateReply::default()
            .reply(true)
            .allowed_mentions(CreateAllowedMentions::new().replied_user(false))
            .content(resp_string + &format!("added title **{}** to the tracking list! you will be notified when a new chapter is uploaded.", title)),
    )
    .await?;

    Ok(())
}

/// print the currently tracked list.
#[poise::command(prefix_command, slash_command)]
pub async fn list(ctx: Context<'_>) -> Result<(), Error> {
    if check_md_client(ctx).await.is_err() {
        return Ok(());
    }

    let list = manga::Entity::find()
        .paginate(&ctx.data().db, 10)
        .fetch_and_next()
        .await
        .map_err(|e| {
            tracing::error!("there was an error fetching from database: {}", e);
            e
        })?;

    if let Some(manga_list) = list {
        if manga_list.is_empty() {
            ctx.send(
                poise::CreateReply::default()
                    .reply(true)
                    .allowed_mentions(CreateAllowedMentions::new().replied_user(false))
                    .content("there are no manga in the tracking list."),
            )
            .await?;

            return Ok(());
        }

        let mut manga_list_str = String::new();

        for (idx, manga) in manga_list.iter().enumerate() {
            let manga_id = manga.manga_dex_id;

            let manga = ctx
                .data()
                .md
                .as_ref()
                .unwrap()
                .manga()
                .id(manga.manga_dex_id)
                .get()
                .send()
                .await?;

            let manga = manga.data.attributes;

            let title = if let Some(en_title) =
                manga.title.get(&mangadex_api_types_rust::Language::English)
            {
                en_title
            } else if let Some(jp_ro) = manga
                .title
                .get(&mangadex_api_types_rust::Language::JapaneseRomanized)
            {
                jp_ro
            } else {
                manga
                    .title
                    .get(&mangadex_api_types_rust::Language::Japanese)
                    .unwrap()
            };

            manga_list_str = manga_list_str
                + &format!(
                    "{}. [{}](https://mangadex.org/title/{})\n",
                    idx + 1,
                    title,
                    manga_id,
                );
        }

        ctx.send(
            poise::CreateReply::default()
                .reply(true)
                .allowed_mentions(CreateAllowedMentions::new().replied_user(false))
                .embed(
                    CreateEmbed::default()
                        .title("list of tracked manga titles")
                        .url(format!(
                            "https://mangadex.org/list/{}",
                            ctx.data().mdlist_id.unwrap()
                        ))
                        .description(manga_list_str),
                ),
        )
        .await?;
    }

    Ok(())
}

/// sync the local database to the mdlist.
#[poise::command(prefix_command, slash_command)]
pub async fn sync(ctx: Context<'_>) -> Result<(), Error> {
    if check_md_client(ctx).await.is_err() {
        return Ok(());
    }

    let manga_list = manga::Entity::find().all(&ctx.data().db).await?;

    let mdlist = ctx
        .data()
        .md
        .as_ref()
        .unwrap()
        .custom_list()
        .id(ctx.data().mdlist_id.unwrap())
        .get()
        .send()
        .await?;

    let msg = ctx
        .send(
            poise::CreateReply::default()
                .reply(true)
                .allowed_mentions(CreateAllowedMentions::new().replied_user(false))
                .content("fetching the manga list from the database..."),
        )
        .await?;

    let mut builder = ctx
        .data()
        .md
        .as_ref()
        .unwrap()
        .custom_list()
        .id(ctx.data().mdlist_id.unwrap())
        .put();

    for manga in manga_list {
        builder.add_manga_id(manga.manga_dex_id);
    }

    match builder
        .version(mdlist.data.attributes.version)
        .build()
        .unwrap()
        .send()
        .await
    {
        Ok(_) => {
            msg.edit(
                ctx,
                poise::CreateReply::default()
                    .reply(true)
                    .allowed_mentions(CreateAllowedMentions::new().replied_user(false))
                    .content("successfully updated the mdlist!"),
            )
            .await?;
        }

        Err(e) => {
            tracing::warn!("failed to update the mdlist: {}", e);
            msg.edit(
                ctx,
                poise::CreateReply::default()
                    .reply(true)
                    .content("failed to update the mdlist. check back later!"),
            )
            .await?;
        }
    }

    Ok(())
}
