use std::cmp::Ordering;

use crate::{
    constants::{MD_BLOCKED_LIST, MD_URL_REGEX},
    models::manga::Manga,
    Context, Error,
};
use mangadex_api_types_rust::MangaFeedSortOrder;
use poise::serenity_prelude::{
    self, CreateAllowedMentions, CreateEmbed, CreateEmbedFooter, EditMessage,
};

struct InternalManga {
    title: String,
    id: uuid::Uuid,
    last_updated: Option<time::OffsetDateTime>,
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
        .await
        .inspect_err(|e| tracing::error!(err = ?e, "an error occurred when sending reply"))?;

        return Err("mangadex client is not initialized.".into());
    }

    if ctx.data().mdlist_id.is_none() {
        ctx.send(
            poise::CreateReply::default()
                .reply(true)
                .allowed_mentions(CreateAllowedMentions::new().replied_user(false))
                .content("mdlist uuid is not set. this command will not work."),
        )
        .await
        .inspect_err(|e| tracing::error!(err = ?e, "an error occurred when sending reply"))?;

        return Err("mdlist uuid is not set.".into());
    }

    Ok(())
}

/// commands related to manga tracking.
#[poise::command(
    prefix_command,
    subcommand_required,
    guild_only,
    subcommands("add", "list", "sync")
)]
#[tracing::instrument(skip_all)]
pub async fn manga(_: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// add a manga to the tracking list.
#[poise::command(prefix_command)]
#[tracing::instrument(skip_all, fields(input = %input))]
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
        .await
        .inspect_err(|e| tracing::error!(err = ?e, "an error occurred when refreshing token"))?;

    let uuid = match MD_URL_REGEX.captures(&input) {
        Ok(Some(captures)) => match uuid::Uuid::try_parse(&captures[1]) {
            Ok(u) => {
                tracing::info!(uuid = %u, "got uuid from link");
                u
            }
            _ => {
                ctx.send(
                    poise::CreateReply::default()
                        .reply(true)
                        .allowed_mentions(CreateAllowedMentions::new().replied_user(false))
                        .content("invalid uuid supplied."),
                )
                .await
                .inspect_err(
                    |e| tracing::error!(err = ?e, "an error occurred when sending reply"),
                )?;

                return Ok(());
            }
        },
        Ok(None) => match uuid::Uuid::try_parse(&input) {
            Ok(u) => {
                tracing::info!(uuid = %u, "got uuid from input string");
                u
            }
            _ => {
                ctx.send(
                    poise::CreateReply::default()
                        .reply(true)
                        .allowed_mentions(CreateAllowedMentions::new().replied_user(false))
                        .content("invalid link supplied."),
                )
                .await
                .inspect_err(
                    |e| tracing::error!(err = ?e, "an error occurred when sending reply"),
                )?;

                return Ok(());
            }
        },
        Err(e) => {
            tracing::error!(err = ?e, "an error occurred when parsing input");
            ctx.send(
                poise::CreateReply::default()
                    .reply(true)
                    .allowed_mentions(CreateAllowedMentions::new().replied_user(false))
                    .content("an error occurred when parsing input."),
            )
            .await
            .inspect_err(|e| tracing::error!(err = ?e, "an error occurred when sending reply"))?;

            return Ok(());
        }
    };

    let manga_list = sqlx::query_as!(
        Manga,
        r#"
            SELECT
                id,
                manga_dex_id AS "manga_dex_id: uuid::fmt::Hyphenated",
                last_updated,
                last_chapter_date
            FROM manga;
        "#
    )
    .fetch_all(&ctx.data().db)
    .await
    .inspect_err(
        |e| tracing::error!(err = ?e, "an error occurred when fetching manga from database"),
    )?;

    let mdlist = ctx
        .data()
        .md
        .as_ref()
        .unwrap()
        .custom_list()
        .id(ctx.data().mdlist_id.unwrap())
        .get()
        .send()
        .await
        .inspect_err(|e| tracing::error!(err = ?e, "an error occurred when fetching mdlist"))?;

    let manga = ctx
        .data()
        .md
        .as_ref()
        .unwrap()
        .manga()
        .id(uuid)
        .get()
        .send()
        .await
        .inspect_err(
            |e| tracing::error!(err = ?e, uuid = %uuid, "an error occurred when fetching manga"),
        )?;
    let manga_id = manga.data.id;

    let chapter_feed = ctx
        .data()
        .md
        .as_ref()
        .unwrap()
        .manga()
        .id(uuid)
        .feed()
        .get()
        .add_translated_language(&mangadex_api_types_rust::Language::English)
        .order(MangaFeedSortOrder::Chapter(
            mangadex_api_types_rust::OrderDirection::Descending,
        ))
        .excluded_groups(MD_BLOCKED_LIST.clone())
        .limit(1u32)
        .send()
        .await
        .inspect_err(
            |e| tracing::error!(err = ?e, uuid = %uuid, "an error occurred when fetching chapter feed"),
        )?;

    let manga = manga.data.attributes;

    let title = match manga.title.get(&mangadex_api_types_rust::Language::English) {
        Some(en_title) => en_title,
        None => {
            match manga
                .title
                .get(&mangadex_api_types_rust::Language::JapaneseRomanized)
            {
                Some(jp_ro) => jp_ro,
                None => {
                    // FIXME: don't unwrap here - this will literally kill the main thread
                    manga
                        .title
                        .get(&mangadex_api_types_rust::Language::Japanese)
                        .unwrap()
                }
            }
        }
    };

    let existing_title =  sqlx::query_as!(Manga,
        r#"
        SELECT
            id AS "id!: i64",
            manga_dex_id AS "manga_dex_id: uuid::fmt::Hyphenated",
            last_updated,
            last_chapter_date
        FROM manga
        WHERE manga_dex_id = $1;
        "#,
        uuid
    ).fetch_optional(&ctx.data().db)
    .await
    .inspect_err(|e| tracing::error!(err = ?e, uuid = %uuid, "an error occurred when fetching manga from database"))?;

    if existing_title.is_some() {
        ctx.send(
            poise::CreateReply::default()
                .reply(true)
                .allowed_mentions(CreateAllowedMentions::new().replied_user(false))
                .content(format!("**{}** is already in the tracking list.", title)),
        )
        .await
        .inspect_err(|e| tracing::error!(err = ?e, "an error occurred when sending reply"))?;

        return Ok(());
    }

    let tags = manga
        .tags
        .iter()
        .map(|tag| {
            tag.attributes
                .name
                .get(&mangadex_api_types_rust::Language::English)
                .unwrap()
                .to_string()
        })
        .collect::<Vec<String>>()
        .join(", ");

    let statistics = ctx
        .data()
        .md
        .as_ref()
        .unwrap()
        .statistics()
        .manga()
        .id(uuid)
        .get()
        .send()
        .await
        .inspect_err(
            |e| tracing::error!(err = ?e, uuid = %uuid, "an error occurred when fetching manga stats"),
        )?;

    let statistics = statistics.statistics.get(&uuid).unwrap();

    let latest_chapter_date = if chapter_feed.result == mangadex_api_types_rust::ResultType::Ok
        && !chapter_feed.data.is_empty()
    {
        let chapter = chapter_feed.data.first().unwrap();

        let chapter_data = &chapter.attributes;

        chapter_data.publish_at.map(|timestamp| {
            time::OffsetDateTime::new_utc(timestamp.as_ref().date(), timestamp.as_ref().time())
        })
    } else {
        None
    };

    let now = time::OffsetDateTime::now_utc();
    let last_updated = time::OffsetDateTime::new_utc(now.date(), now.time());

    sqlx::query!(
        r#"
        INSERT INTO
        manga (manga_dex_id, last_chapter_date, last_updated)
        VALUES
        ($1, $2, $3);
        "#,
        uuid,
        latest_chapter_date,
        last_updated,
    )
    .execute(&ctx.data().db)
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
        builder.add_manga_id(manga.manga_dex_id.into_uuid());
    }

    let mut resp_string = String::new();

    ctx.channel_id()
        .edit_message(
            &ctx.http(),
            ctx.id(),
            EditMessage::new().suppress_embeds(true),
        )
        .await?;

    let _ = builder
        .add_manga_id(uuid)
        .version(mdlist.data.attributes.version)
        .build()
        .unwrap()
        .send()
        .await
        .map_err(|e| {
            tracing::warn!(err = ?e, "an error occurred when updating the mdlist");
            resp_string = "*failed to update the mdlist. it will (hopefully) be updated the next time you add a manga. you can also try running `s>manga sync` to sync the mdlist.*\n\n".to_string()
        });

    ctx.send(
        poise::CreateReply::default()
            .reply(true)
            .allowed_mentions(CreateAllowedMentions::new().replied_user(false))
            .content(resp_string + &format!("added [**{}**](https://mangadex.org/title/{}) to the tracking list! you will be notified when a new chapter is uploaded.", title, uuid))
            .embed(
                CreateEmbed::default()
                    .title(title)
                    .url(format!("https://mangadex.org/title/{}", manga_id))
                    .description(
                        match manga
                            .description
                            .get(&mangadex_api_types_rust::Language::English)
                        {
                            Some(d) => d,
                            None => "",
                        },
                    )
                    .image(format!(
                        "https://og.mangadex.org/og-image/manga/{}",
                        manga_id
                    ))
                    .field("status", manga.status.to_string(), true)
                    .field(
                        "year",
                        match manga.year {
                            Some(year) => year.to_string(),
                            None => "unknown".to_string(),
                        },
                        true,
                    )
                    .field(
                        "demographic",
                        match manga.publication_demographic {
                            Some(demographic) => demographic.to_string(),
                            None => "unknown".to_string(),
                        },
                        true,
                    )
                    .field(
                        "rating",
                        match statistics.rating.bayesian {
                            Some(avg) => avg.to_string(),
                            None => "unknown".to_string(),
                        },
                        true,
                    )
                    .field("follows", statistics.follows.to_string(), true)
                    .field(
                        "content rating",
                        match manga.content_rating {
                            Some(content_rating) => content_rating.to_string(),
                            None => "unknown".to_string(),
                        },
                        true,
                    )
                    .field("tags", tags, false),
            ),
    )
    .await
    .inspect_err(|e| tracing::error!(err = ?e, "an error occurred when sending reply"))?;

    Ok(())
}

/// print the currently tracked list.
#[poise::command(prefix_command)]
#[tracing::instrument(skip_all)]
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
        .await
        .inspect_err(|e| tracing::error!(err = ?e, "an error occurred when sending reply"))?;

    let manga_list = sqlx::query_as!(
        Manga,
        r#"
            SELECT
                id,
                manga_dex_id AS "manga_dex_id: uuid::fmt::Hyphenated",
                last_updated,
                last_chapter_date
            FROM manga;
        "#
    )
    .fetch_all(&ctx.data().db)
    .await
    .inspect_err(
        |e| tracing::error!(err = ?e, "an error occurred when fetching manga from database"),
    )?;

    if manga_list.is_empty() {
        msg.edit(
            ctx,
            poise::CreateReply::default()
                .reply(true)
                .allowed_mentions(CreateAllowedMentions::new().replied_user(false))
                .content("there are no manga in the tracking list."),
        )
        .await
        .inspect_err(|e| tracing::error!(err = ?e, "an error occurred when sending reply"))?;

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
            .id(db_manga.manga_dex_id.into())
            .get()
            .send()
            .await
            .inspect_err(
                |e| tracing::error!(err = ?e, uuid = %db_manga.manga_dex_id, "an error occurred when fetching manga"),
            )?;

        let manga = manga.data.attributes;

        let title = match manga.title.get(&mangadex_api_types_rust::Language::English) {
            Some(en_title) => en_title,
            None => {
                match manga
                    .title
                    .get(&mangadex_api_types_rust::Language::JapaneseRomanized)
                {
                    Some(jp_ro) => jp_ro,
                    None => manga
                        .title
                        .get(&mangadex_api_types_rust::Language::Japanese)
                        .unwrap(),
                }
            }
        };

        result_list.push(InternalManga {
            title: title.to_string(),
            id: manga_id.into(),
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
            let entry_str = match manga.last_updated {
                Some(timestamp) => {
                    format!(
                        "{}. [{}](https://mangadex.org/title/{}) (last updated: <t:{}:R>)\n",
                        idx + 1 + page * 10,
                        manga.title,
                        manga.id,
                        timestamp.unix_timestamp(),
                    )
                }
                _ => {
                    format!(
                        "{}. [{}](https://mangadex.org/title/{})\n",
                        idx + 1 + page * 10,
                        manga.title,
                        manga.id,
                    )
                }
            };

            manga_list_str = manga_list_str + &entry_str;
        }

        pages.push(manga_list_str);
    }

    let ctx_id = ctx.id();
    let author_id = ctx.author().id;
    let first_id = format!("{}first", ctx_id);
    let last_id = format!("{}last", ctx_id);
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
                    .description(pages[0].clone())
                    .footer(CreateEmbedFooter::new(format!(
                        "page {}/{}",
                        current_page + 1,
                        pages.len(),
                    ))),
            )
            .components(vec![serenity_prelude::CreateActionRow::Buttons(vec![
                serenity_prelude::CreateButton::new(&first_id)
                    .emoji('⏮')
                    .disabled(true),
                serenity_prelude::CreateButton::new(&prev_id)
                    .emoji('◀')
                    .disabled(true),
                serenity_prelude::CreateButton::new(&next_id)
                    .emoji('▶')
                    .disabled(current_page == pages.len() - 1),
                serenity_prelude::CreateButton::new(&last_id)
                    .emoji('⏭')
                    .disabled(current_page == pages.len() - 1),
            ])]),
    )
    .await
    .inspect_err(|e| tracing::error!(err = ?e, "an error occurred when editing message"))?;

    while let Some(press) = serenity_prelude::collector::ComponentInteractionCollector::new(ctx)
        .filter(move |press| press.data.custom_id.starts_with(&ctx_id.to_string()))
        .timeout(std::time::Duration::from_secs(60))
        .await
    {
        if press.user.id != author_id {
            press
                .create_response(
                    ctx,
                    serenity_prelude::CreateInteractionResponse::Message(
                        serenity_prelude::CreateInteractionResponseMessage::new()
                            .content("you cannot interact with another user's invoked command!")
                            .ephemeral(true),
                    ),
                )
                .await
                .inspect_err(
                    |e| tracing::error!(err = ?e, "an error occurred when creating response"),
                )?;

            continue;
        }

        if press.data.custom_id == prev_id {
            current_page = current_page.saturating_sub(1);
        } else if press.data.custom_id == next_id {
            current_page += 1;
        } else if press.data.custom_id == first_id {
            current_page = 0;
        } else if press.data.custom_id == last_id {
            current_page = pages.len() - 1;
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
                                .description(pages[current_page].clone())
                                .footer(CreateEmbedFooter::new(format!(
                                    "page {}/{}",
                                    current_page + 1,
                                    pages.len(),
                                ))),
                        )
                        .components(vec![serenity_prelude::CreateActionRow::Buttons(vec![
                            serenity_prelude::CreateButton::new(&first_id)
                                .emoji('⏮')
                                .disabled(current_page == 0),
                            serenity_prelude::CreateButton::new(&prev_id)
                                .emoji('◀')
                                .disabled(current_page == 0),
                            serenity_prelude::CreateButton::new(&next_id)
                                .emoji('▶')
                                .disabled(current_page == pages.len() - 1),
                            serenity_prelude::CreateButton::new(&last_id)
                                .emoji('⏭')
                                .disabled(current_page == pages.len() - 1),
                        ])]),
                ),
            )
            .await
            .inspect_err(
                |e| tracing::error!(err = ?e, "an error occurred when creating response"),
            )?;
    }

    msg.into_message()
        .await?
        .edit(ctx, EditMessage::default().components(vec![]))
        .await
        .inspect_err(|e| tracing::error!(err = ?e, "an error occurred when editing message"))?;

    Ok(())
}

/// sync the local database to the mdlist.
#[poise::command(prefix_command)]
#[tracing::instrument(skip_all)]
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
        .await
        .inspect_err(
            |e| tracing::error!(err = ?e, "an error occurred when refreshing mangadex token"),
        )?;

    let msg = ctx
        .send(
            poise::CreateReply::default()
                .reply(true)
                .allowed_mentions(CreateAllowedMentions::new().replied_user(false))
                .content("fetching the manga list from the database..."),
        )
        .await
        .inspect_err(|e| tracing::error!(err = ?e, "an error occurred when sending reply"))?;

    let manga_list = sqlx::query_as!(
        Manga,
        r#"
            SELECT
                id,
                manga_dex_id AS "manga_dex_id: uuid::fmt::Hyphenated",
                last_updated,
                last_chapter_date
            FROM manga;
        "#
    )
    .fetch_all(&ctx.data().db)
    .await
    .inspect_err(
        |e| tracing::error!(err = ?e, "an error occurred when fetching manga from database"),
    )?;

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
        builder.add_manga_id(manga.manga_dex_id.into_uuid());
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
            .await
            .inspect_err(|e| tracing::error!(err = ?e, "an error occurred when editing message"))?;
        }

        Err(e) => {
            tracing::warn!(err = ?e, "an error occurred when updating mdlist");
            msg.edit(
                ctx,
                poise::CreateReply::default()
                    .reply(true)
                    .content("failed to update the mdlist. check back later!"),
            )
            .await
            .inspect_err(|e| tracing::error!(err = ?e, "an error occurred when editing message"))?;
        }
    }

    Ok(())
}
