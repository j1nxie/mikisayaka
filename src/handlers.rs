use crate::models::songlink::SonglinkResponse;
use crate::Data;
use fancy_regex::Captures;
use poise::serenity_prelude::{
    self as serenity, CreateActionRow, CreateAllowedMentions, CreateButton, CreateEmbed,
    CreateMessage, EditMessage, EmojiId, Message, MessageReference,
};

pub async fn youtube_handler(
    ctx: &serenity::Context,
    data: &Data,
    new_message: &Message,
    captures: Captures<'_>,
) -> anyhow::Result<()> {
    let mut msg = new_message
        .channel_id
        .send_message(
            ctx,
            CreateMessage::default()
                .reference_message(MessageReference::from(new_message))
                .allowed_mentions(CreateAllowedMentions::new().replied_user(false))
                .content("got a youtube link! attempting to match it with songlink..."),
        )
        .await
        .inspect_err(|e| tracing::error!(err = ?e, "an error occurred when sending reply"))?;

    let url_encoded = urlencoding::encode(&captures[0]);

    let res = data
        .reqwest_client
        .get(format!(
            "https://api.song.link/v1-alpha.1/links?url={}&userCountry=JP",
            url_encoded
        ))
        .send()
        .await
        .inspect_err(
            |e| tracing::error!(err = ?e, "an error occurred when fetching song from songlink"),
        )?;

    let res: SonglinkResponse = res.json().await.inspect_err(
        |e| tracing::error!(err = ?e, "an error occurred when decoding songlink response"),
    )?;

    match res.links_by_platform.spotify {
        Some(spotify) => {
            msg.edit(
                ctx,
                EditMessage::default()
                    .allowed_mentions(CreateAllowedMentions::new().replied_user(false))
                    .content(format!("here's your spotify link: {}", spotify.url)),
            )
            .await
            .inspect_err(|e| tracing::error!(err = ?e, "an error occurred when editing message"))?;
        }
        None => {
            msg.edit(
                ctx,
                EditMessage::default()
                    .allowed_mentions(CreateAllowedMentions::new().replied_user(false))
                    .content("i didn't match anything for your link..."),
            )
            .await
            .inspect_err(|e| tracing::error!(err = ?e, "an error occurred when editing message"))?;

            tokio::time::sleep(std::time::Duration::from_secs(5)).await;

            msg.delete(ctx).await?;
        }
    }

    Ok(())
}

pub async fn spotify_handler(
    ctx: &serenity::Context,
    data: &Data,
    new_message: &Message,
    captures: Captures<'_>,
) -> anyhow::Result<()> {
    let mut msg = new_message
        .channel_id
        .send_message(
            ctx,
            CreateMessage::default()
                .reference_message(MessageReference::from(new_message))
                .allowed_mentions(CreateAllowedMentions::new().replied_user(false))
                .content("got a spotify link! attempting to match it with songlink..."),
        )
        .await
        .inspect_err(|e| tracing::error!(err = ?e, "an error occurred when sending reply"))?;

    let url_encoded = urlencoding::encode(&captures[0]);

    let res = data
        .reqwest_client
        .get(format!(
            "https://api.song.link/v1-alpha.1/links?url={}&userCountry=JP",
            url_encoded
        ))
        .send()
        .await
        .inspect_err(
            |e| tracing::error!(err = ?e, "an error occurred when fetching song from songlink"),
        )?;

    let res: SonglinkResponse = res.json().await.inspect_err(
        |e| tracing::error!(err = ?e, "an error occurred when decoding songlink response"),
    )?;

    match res.links_by_platform.youtube_music {
        Some(youtube_music) => {
            msg.edit(
                ctx,
                EditMessage::default()
                    .allowed_mentions(CreateAllowedMentions::new().replied_user(false))
                    .content(format!("here's your youtube link: {}", youtube_music.url)),
            )
            .await
            .inspect_err(|e| tracing::error!(err = ?e, "an error occurred when editing message"))?;
        }
        None => {
            msg.edit(
                ctx,
                EditMessage::default()
                    .allowed_mentions(CreateAllowedMentions::new().replied_user(false))
                    .content("i didn't match anything for your link..."),
            )
            .await
            .inspect_err(|e| tracing::error!(err = ?e, "an error occurred when editing message"))?;

            tokio::time::sleep(std::time::Duration::from_secs(5)).await;

            msg.delete(ctx).await?;
        }
    }

    Ok(())
}

pub async fn md_handler(
    ctx: &serenity::Context,
    data: &Data,
    new_message: &Message,
    captures: Captures<'_>,
) -> anyhow::Result<()> {
    let uuid = uuid::Uuid::try_parse(&captures[1]);

    match uuid {
        Ok(uuid) => {
            let mut msg = new_message
                .channel_id
                .send_message(
                    ctx,
                    CreateMessage::default()
                        .reference_message(MessageReference::from(new_message))
                        .allowed_mentions(CreateAllowedMentions::new().replied_user(false))
                        .content("got a mangadex link! fetching data..."),
                )
                .await
                .inspect_err(
                    |e| tracing::error!(err = ?e, "an error occurred when sending reply"),
                )?;

            // FIXME: better error handling here
            // this currently silently errors and hangs instead of returning - the message will just hang at "fetching data...".
            let manga = data
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
            let manga = manga.data.attributes;

            let en_title = manga.title.get(&mangadex_api_types_rust::Language::English);

            let title = match en_title {
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

            let tags = match manga.content_rating {
                Some(content_rating) => format!("**{}**, {}", content_rating, tags),
                None => tags,
            };

            let statistics = data
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

            let buttons = match manga.links {
                Some(links) => {
                    let mut result = vec![];

                    if let Some(anilist) = links.anilist {
                        result.push(
                            CreateButton::new_link(format!("https://anilist.co/manga/{anilist}"))
                                .label("AniList")
                                .emoji(EmojiId::new(1349211782287331398)),
                        );
                    }

                    if let Some(mal) = links.my_anime_list {
                        result.push(
                            CreateButton::new_link(mal.to_string())
                                .label("MyAnimeList")
                                .emoji(EmojiId::new(1349211802537562253)),
                        );
                    }

                    result
                }
                None => vec![],
            };

            new_message
                .channel_id
                .edit_message(
                    &ctx.http,
                    new_message.id,
                    EditMessage::new().suppress_embeds(true),
                )
                .await?;

            msg.edit(
                ctx,
                EditMessage::default()
                    .allowed_mentions(CreateAllowedMentions::new().replied_user(false))
                    .content("here's your manga!")
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
                            .field(
                                "publication",
                                match manga.year {
                                    Some(year) => {
                                        format!("{}, {}", year, manga.status)
                                    }
                                    None => manga.status.to_string(),
                                },
                                true,
                            )
                            .field(
                                "statistics",
                                match statistics.rating.bayesian {
                                    Some(avg) => {
                                        format!("{} follows, {:.02} ☆", statistics.follows, avg)
                                    }
                                    None => statistics.follows.to_string(),
                                },
                                true,
                            )
                            .field("tags", tags, false),
                    )
                    .components(vec![CreateActionRow::Buttons(buttons)]),
            )
            .await
            .inspect_err(|e| tracing::error!(err = ?e, "an error occurred when editing message"))?;
        }

        _ => {
            return Ok(());
        }
    }

    Ok(())
}
