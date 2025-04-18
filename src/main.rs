use std::str::FromStr;

use constants::{
    manga::MD_URL_REGEX,
    music::{SPOTIFY_URL_REGEX, YOUTUBE_URL_REGEX},
    STARTUP_TIME,
};
use futures::StreamExt;
use mangadex_api::{v5::schema::oauth::ClientInfo, MangaDexClient};
use mangadex_api_types_rust::{Password, Username};
use models::songlink::SonglinkResponse;
use poise::serenity_prelude::{
    self as serenity, ChannelId, CreateActionRow, CreateAllowedMentions, CreateButton, CreateEmbed,
    CreateMessage, EditMessage, EmojiId, MessageReference,
};
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    Pool, Sqlite,
};
use tracing::{level_filters::LevelFilter, Instrument};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

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
mod models;

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

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("initializing... please wait warmly.");
    let token = std::env::var("DISCORD_TOKEN").expect("missing DISCORD_TOKEN");
    let db_url = std::env::var("DATABASE_URL").expect("missing DATABASE_URL");
    let intents =
        serenity::GatewayIntents::non_privileged() | serenity::GatewayIntents::MESSAGE_CONTENT;

    tracing::info!("initializing database connection...");
    let opts = SqliteConnectOptions::from_str(&db_url)
        .expect("invalid DATABASE_URL")
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);
    let db = SqlitePoolOptions::new()
        .max_connections(20)
        .connect_with(opts)
        .await?;

    tracing::info!("running migrations...");
    sqlx::migrate!("./migrations").run(&db).await?;
    tracing::info!("finished running migrations!");

    tracing::info!("initializing mangadex client...");

    let mdlist_id = std::env::var("MANGADEX_MDLIST_ID")
        .ok()
        .and_then(|id| uuid::Uuid::try_parse(&id).ok());

    if mdlist_id.is_none() {
        tracing::warn!("no manga update channel id found. mangadex links will not be watched.");
    }

    let md = match (
        std::env::var("MANGADEX_CLIENT_ID"),
        std::env::var("MANGADEX_CLIENT_SECRET"),
        std::env::var("MANGADEX_USERNAME"),
        std::env::var("MANGADEX_PASSWORD"),
    ) {
        (Ok(client_id), Ok(client_secret), Ok(username), Ok(password)) => {
            let md_client = MangaDexClient::default();

            md_client
                .set_client_info(&ClientInfo {
                    client_id,
                    client_secret,
                })
                .await?;

            tracing::info!("logging in to mangadex...");
            md_client
                .oauth()
                .login()
                .username(Username::parse(username)?)
                .password(Password::parse(password)?)
                .send()
                .await
                .inspect_err(
                    |e| tracing::warn!(err = ?e, "an error occurred when logging into mangadex"),
                )?;

            Some(md_client)
        }
        _ => {
            tracing::warn!("missing mangadex credentials - manga features will be disabled");
            None
        }
    };

    let manga_update_channel_id = std::env::var("MANGA_UPDATE_CHANNEL_ID")
        .ok()
        .and_then(|id| id.parse::<u64>().ok())
        .map(|id| {
            tracing::info!("watching channel with id {} for mangadex links.", id);
            ChannelId::new(id)
        });

    if manga_update_channel_id.is_none() {
        tracing::warn!("no manga update channel id found. mangadex links will not be watched.");
    }

    let music_channel_id = std::env::var("MUSIC_CHANNEL_ID")
        .ok()
        .and_then(|id| id.parse::<u64>().ok())
        .map(|id| {
            tracing::info!(
                "watching channel with id {} for youtube / spotify links.",
                id
            );
            ChannelId::new(id)
        });

    let reqwest_client = reqwest::Client::new();

    if music_channel_id.is_none() {
        tracing::warn!("no music channel id found. youtube / spotify links will not be watched.");
    }

    let gas_prices_channel_id = std::env::var("GAS_PRICES_CHANNEL_ID")
        .ok()
        .and_then(|id| id.parse::<u64>().ok())
        .map(|id| {
            tracing::info!("sending new gas prices updates to channel with id {}.", id);
            ChannelId::new(id)
        });

    if gas_prices_channel_id.is_none() {
        tracing::warn!("no channel id found for gas prices updates. they will not be sent.");
    }

    let data = Data {
        gas_prices_channel_id,
        manga_update_channel_id,
        music_channel_id,
        reqwest_client,
        db,
        md,
        mdlist_id,
    };

    let md_data = data.clone();
    let gas_data = data.clone();

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![
                commands::gas_prices::gas_prices(),
                commands::help::help(),
                commands::status::status(),
                commands::role::role(),
                commands::fluff::quartatrice(),
                commands::fluff::itl(),
                commands::manga::manga(),
            ],
            prefix_options: poise::PrefixFrameworkOptions {
                prefix: Some("s>".into()),
                ..Default::default()
            },
            event_handler: |ctx, event, framework, data| {
                Box::pin(event_handler(ctx, event, framework, data))
            },
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands)
                    .await
                    .inspect_err(|e| tracing::error!(err = ?e, "an error occurred when registering commands"))?;

                Ok(data)
            }.in_current_span())
        })
        .build();

    let mut client = serenity::ClientBuilder::new(&token, intents)
        .framework(framework)
        .activity(serenity::ActivityData {
            name: "squartatrice - 美樹 さやか vs. 美樹 さやか (fw. 美樹 さやか)".into(),
            kind: serenity::ActivityType::Listening,
            state: None,
            url: None,
        })
        .await
        .unwrap();

    let md_http = client.http.clone();
    let gas_http = client.http.clone();

    tracing::info!("finished initializing!");
    let bot_handle = tokio::spawn(async move { client.start().await.unwrap() });

    if md_data.md.is_some() {
        tracing::info!("initialized chapter tracker!");

        tokio::spawn(
            async move {
                let interval = tokio::time::interval(std::time::Duration::from_secs(900));
                let task = futures::stream::unfold(interval, |mut interval| async {
                    interval.tick().await;
                    let _ = chapter_tracker::chapter_tracker(&md_http, &md_data).await;

                    Some(((), interval))
                });

                task.for_each(|_| async {}).await;
            }
            .in_current_span(),
        );
    }

    tracing::info!("initialized gas prices tracker!");

    tokio::spawn(
        async move {
            let interval = tokio::time::interval(std::time::Duration::from_secs(900));
            let task = futures::stream::unfold(interval, |mut interval| async {
                interval.tick().await;

                let _ = gas_prices::gas_prices(&gas_http, &gas_data).await;
                Some(((), interval))
            });

            task.for_each(|_| async {}).await;
        }
        .in_current_span(),
    );

    bot_handle.await?;

    Ok(())
}
