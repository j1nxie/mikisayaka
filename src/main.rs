use constants::{
    manga::MD_URL_REGEX,
    music::{SPOTIFY_URL_REGEX, YOUTUBE_URL_REGEX},
    STARTUP_TIME,
};
use mangadex_api::MangaDexClient;
use models::songlink::SonglinkResponse;
use poise::serenity_prelude::{
    self as serenity, ChannelId, CreateActionRow, CreateAllowedMentions, CreateButton, CreateEmbed,
    CreateMessage, EditMessage, EmojiId, MessageReference,
};
use sqlx::{Pool, Sqlite};

#[derive(Clone)]
struct Data {
    gas_prices_channel_id: Option<ChannelId>,
    manga_update_channel_id: Option<ChannelId>,
    music_channel_id: Option<ChannelId>,
    reqwest_client: reqwest::Client,
    db: Pool<Sqlite>,
    md: Option<MangaDexClient>,
    mdlist_id: Option<uuid::Uuid>,
}

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

mod chapter_tracker;
mod commands;
mod constants;
mod gas_prices;
mod init;
mod models;
mod telemetry;

#[tracing::instrument(skip_all)]
async fn event_handler(
    ctx: &serenity::Context,
    event: &serenity::FullEvent,
    _framework: poise::FrameworkContext<'_, Data, Error>,
    data: &Data,
) -> Result<(), Error> {
    if let serenity::FullEvent::Message { new_message } = event {
        if new_message.author.bot
            || data.md.is_none()
            || data.manga_update_channel_id.is_none()
            || new_message.content.starts_with("s>")
        {
            return Ok(());
        }

        if new_message.channel_id == data.music_channel_id.unwrap() {
            if let Ok(Some(captures)) = YOUTUBE_URL_REGEX.captures(&new_message.content) {
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
                    .inspect_err(
                        |e| tracing::error!(err = ?e, "an error occurred when sending reply"),
                    )?;

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
                    |e| tracing::error!(err = ?e, "an error occurred when fetching song from songlink")
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
                        .inspect_err(
                            |e| tracing::error!(err = ?e, "an error occurred when editing message"),
                        )?;
                    }
                    None => {
                        msg.edit(
                            ctx,
                            EditMessage::default()
                                .allowed_mentions(CreateAllowedMentions::new().replied_user(false))
                                .content("i didn't match anything for your link..."),
                        )
                        .await
                        .inspect_err(
                            |e| tracing::error!(err = ?e, "an error occurred when editing message"),
                        )?;

                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;

                        msg.delete(ctx).await?;
                    }
                }
            }

            if let Ok(Some(captures)) = SPOTIFY_URL_REGEX.captures(&new_message.content) {
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
                    .inspect_err(
                        |e| tracing::error!(err = ?e, "an error occurred when sending reply"),
                    )?;

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
                            |e| tracing::error!(err = ?e, "an error occurred when fetching song from songlink")
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
                                .content(format!(
                                    "here's your youtube link: {}",
                                    youtube_music.url
                                )),
                        )
                        .await
                        .inspect_err(
                            |e| tracing::error!(err = ?e, "an error occurred when editing message"),
                        )?;
                    }
                    None => {
                        msg.edit(
                            ctx,
                            EditMessage::default()
                                .allowed_mentions(CreateAllowedMentions::new().replied_user(false))
                                .content("i didn't match anything for your link..."),
                        )
                        .await
                        .inspect_err(
                            |e| tracing::error!(err = ?e, "an error occurred when editing message"),
                        )?;

                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;

                        msg.delete(ctx).await?;
                    }
                }
            }
        }

        if new_message.channel_id == data.manga_update_channel_id.unwrap() {
            if let Ok(Some(captures)) = MD_URL_REGEX.captures(&new_message.content) {
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
                                        CreateButton::new_link(format!(
                                            "https://anilist.co/manga/{anilist}"
                                        ))
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
                                                Some(avg) => format!(
                                                    "{} follows, {:.02} ☆",
                                                    statistics.follows, avg
                                                ),
                                                None => statistics.follows.to_string(),
                                            },
                                            true,
                                        )
                                        .field("tags", tags, false),
                                )
                                .components(vec![CreateActionRow::Buttons(buttons)]),
                        )
                        .await
                        .inspect_err(
                            |e| tracing::error!(err = ?e, "an error occurred when editing message"),
                        )?;
                    }

                    _ => {
                        return Ok(());
                    }
                }
            }
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();
    let _ = &*STARTUP_TIME;

    let mut client = init::init().await?;
    client.start().await?;

    Ok(())
}
