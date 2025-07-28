use std::str::FromStr;

use futures::StreamExt;
use mangadex_api::{v5::schema::oauth::ClientInfo, MangaDexClient};
use mangadex_api_types_rust::{Password, Username};
use poise::serenity_prelude::{self as serenity, *};
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    Pool, Sqlite,
};
use time::OffsetDateTime;
use tracing::Instrument;

use crate::{
    chapter_tracker, commands, event_handler, gas_prices, telemetry, zenless::ZenlessClient, Data,
};

async fn init_database() -> anyhow::Result<Pool<Sqlite>> {
    let db_url = std::env::var("DATABASE_URL").expect("missing DATABASE_URL");

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

    Ok(db)
}

async fn init_md() -> anyhow::Result<Option<MangaDexClient>> {
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

    Ok(md)
}

fn init_channel_ids() -> (Option<ChannelId>, Option<ChannelId>, Option<ChannelId>) {
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

    (
        manga_update_channel_id,
        music_channel_id,
        gas_prices_channel_id,
    )
}

fn init_mdlist_id() -> Option<uuid::Uuid> {
    let mdlist_id = std::env::var("MANGADEX_MDLIST_ID")
        .ok()
        .and_then(|id| uuid::Uuid::try_parse(&id).ok());

    if mdlist_id.is_none() {
        tracing::warn!("no manga update channel id found. mangadex links will not be watched.");
    }

    mdlist_id
}

async fn init_discord_client(token: &str, data: Data) -> anyhow::Result<Client> {
    let intents =
        serenity::GatewayIntents::non_privileged() | serenity::GatewayIntents::MESSAGE_CONTENT;

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![
                commands::zenless::zenless(),
                commands::gas_prices::gas_prices(),
                commands::help::help(),
                commands::status::status(),
                commands::role::role(),
                commands::fluff::quartatrice(),
                commands::fluff::itl(),
                commands::manga::manga(),
                commands::quote::quote(),
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

    let client = ClientBuilder::new(token, intents)
        .framework(framework)
        .activity(serenity::ActivityData {
            name: "squartatrice - 美樹 さやか vs. 美樹 さやか (fw. 美樹 さやか)".into(),
            kind: serenity::ActivityType::Listening,
            state: None,
            url: None,
        })
        .await?;

    Ok(client)
}

fn spawn_background_tasks(client: &Client, data: &Data) {
    let md_data = data.clone();
    let gas_data = data.clone();
    let md_http = client.http.clone();
    let gas_http = client.http.clone();

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
}

pub async fn init() -> anyhow::Result<Client> {
    tracing::info!("initializing... please wait warmly.");

    let token = std::env::var("DISCORD_TOKEN").expect("missing DISCORD_TOKEN");

    telemetry::init_telemetry().expect("Failed to initialize OpenTelemetry");

    let db = init_database().await?;
    let md = init_md().await?;
    let mdlist_id = init_mdlist_id();
    let (manga_update_channel_id, music_channel_id, gas_prices_channel_id) = init_channel_ids();
    let reqwest_client = reqwest::Client::new();
    let zenless_client = ZenlessClient::new();

    let data = Data {
        gas_prices_channel_id,
        manga_update_channel_id,
        music_channel_id,
        reqwest_client,
        zenless_client,
        db,
        md,
        mdlist_id,
    };

    let client = init_discord_client(&token, data.clone()).await?;
    spawn_background_tasks(&client, &data);

    tracing::info!("finished initializing!");
    Ok(client)
}
