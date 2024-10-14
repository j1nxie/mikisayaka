use std::cmp::Ordering;

use crate::{
    constants::{MD_BLOCKED_LIST, MD_URL_REGEX},
    models::manga,
    Context, Error,
};
use mangadex_api_types_rust::MangaFeedSortOrder;
use poise::serenity_prelude::{self, CreateAllowedMentions, CreateEmbed};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, PaginatorTrait, QueryFilter, Set,
};

struct InternalManga {
    title: String,
    id: uuid::Uuid,
    last_updated: Option<time::PrimitiveDateTime>,
}

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

    let now = time::OffsetDateTime::now_utc();

    let model = manga::ActiveModel {
        manga_dex_id: Set(uuid),
        last_updated: Set(time::PrimitiveDateTime::new(now.date(), now.time())),
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
            .content(resp_string + &format!("added title [**{}**](https://mangadex.org/title/{}) to the tracking list! you will be notified when a new chapter is uploaded.", title, uuid)),
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

    let msg = ctx
        .send(
            poise::CreateReply::default()
                .reply(true)
                .allowed_mentions(CreateAllowedMentions::new().replied_user(false))
                .content("loading... please watch warmly..."),
        )
        .await?;

    let manga_list = manga::Entity::find()
        .all(&ctx.data().db)
        .await
        .map_err(|e| {
            tracing::error!("there was an error fetching from database: {}", e);
            e
        })?;

    if manga_list.is_empty() {
        msg.edit(
            ctx,
            poise::CreateReply::default()
                .reply(true)
                .allowed_mentions(CreateAllowedMentions::new().replied_user(false))
                .content("there are no manga in the tracking list."),
        )
        .await?;

        return Ok(());
    }

    let mut result_list: Vec<InternalManga> = vec![];

    for db_manga in manga_list {
        let manga_id = db_manga.manga_dex_id;

        let manga = ctx
            .data()
            .md
            .as_ref()
            .unwrap()
            .manga()
            .id(db_manga.manga_dex_id)
            .get()
            .send()
            .await?;

        let manga = manga.data.attributes;

        let title =
            if let Some(en_title) = manga.title.get(&mangadex_api_types_rust::Language::English) {
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

        result_list.push(InternalManga {
            title: title.to_string(),
            id: manga_id,
            last_updated: db_manga.last_chapter_date,
        });
    }

    result_list.sort_by(|a, b| {
        if a.last_updated.is_none() {
            return Ordering::Greater;
        }

        if b.last_updated.is_none() {
            return Ordering::Less;
        }

        b.last_updated.unwrap().cmp(&a.last_updated.unwrap())
    });

    let mut pages: Vec<String> = vec![];
    let mut current_page: usize = 0;

    for (page, chunk) in result_list.chunks(10).enumerate() {
        let mut manga_list_str = String::new();
        for (idx, manga) in chunk.iter().enumerate() {
            let entry_str = if let Some(timestamp) = manga.last_updated {
                format!(
                    "{}. [{}](https://mangadex.org/title/{}) (last updated: <t:{}:R>)\n",
                    idx + 1 + page * 10,
                    manga.title,
                    manga.id,
                    timestamp.assume_utc().unix_timestamp(),
                )
            } else {
                format!(
                    "{}. [{}](https://mangadex.org/title/{})\n",
                    idx + 1 + page * 10,
                    manga.title,
                    manga.id,
                )
            };

            manga_list_str = manga_list_str + &entry_str;
        }

        pages.push(manga_list_str);
    }

    let ctx_id = ctx.id();
    let author_id = ctx.author().id;
    let prev_id = format!("{}prev", ctx_id);
    let next_id = format!("{}next", ctx_id);

    msg.edit(
        ctx,
        poise::CreateReply::default()
            .reply(true)
            .allowed_mentions(CreateAllowedMentions::new().replied_user(false))
            .content("here's your manga list!")
            .embed(
                CreateEmbed::default()
                    .title("list of tracked manga titles")
                    .url(format!(
                        "https://mangadex.org/list/{}",
                        ctx.data().mdlist_id.unwrap()
                    ))
                    .description(pages[0].clone()),
            )
            .components(vec![serenity_prelude::CreateActionRow::Buttons(vec![
                serenity_prelude::CreateButton::new(&prev_id)
                    .emoji('◀')
                    .disabled(true),
                serenity_prelude::CreateButton::new(&next_id).emoji('▶'),
            ])]),
    )
    .await?;

    while let Some(press) = serenity_prelude::collector::ComponentInteractionCollector::new(ctx)
        .filter(move |press| {
            press.data.custom_id.starts_with(&ctx_id.to_string()) && press.user.id == author_id
        })
        .timeout(std::time::Duration::from_secs(60))
        .await
    {
        if press.data.custom_id == prev_id {
            current_page = current_page.saturating_sub(1);
        } else if press.data.custom_id == next_id {
            current_page += 1;
        } else {
            continue;
        }

        press
            .create_response(
                ctx,
                serenity_prelude::CreateInteractionResponse::UpdateMessage(
                    serenity_prelude::CreateInteractionResponseMessage::new()
                        .embed(
                            CreateEmbed::default()
                                .title("list of tracked manga titles")
                                .url(format!(
                                    "https://mangadex.org/list/{}",
                                    ctx.data().mdlist_id.unwrap()
                                ))
                                .description(pages[current_page].clone()),
                        )
                        .components(vec![serenity_prelude::CreateActionRow::Buttons(vec![
                            serenity_prelude::CreateButton::new(&prev_id)
                                .emoji('◀')
                                .disabled(current_page == 0),
                            serenity_prelude::CreateButton::new(&next_id)
                                .emoji('▶')
                                .disabled(current_page == pages.len() - 1),
                        ])]),
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

    ctx.data()
        .md
        .as_ref()
        .unwrap()
        .oauth()
        .refresh()
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
