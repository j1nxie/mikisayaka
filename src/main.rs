use constants::{MD_URL_REGEX, STARTUP_TIME};
use futures::StreamExt;
use mangadex_api::{v5::schema::oauth::ClientInfo, MangaDexClient};
use mangadex_api_types_rust::{Password, Username};
use migrator::Migrator;
use poise::serenity_prelude::{
    self as serenity, ChannelId, CreateAllowedMentions, CreateEmbed, CreateMessage, EditMessage,
    MessageReference,
};
use sea_orm::{Database, DatabaseConnection};
use sea_orm_migration::MigratorTrait;
use tracing::{level_filters::LevelFilter, Instrument};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[derive(Clone)]
struct Data {
    manga_update_channel_id: Option<ChannelId>,
    db: DatabaseConnection,
    md: Option<MangaDexClient>,
    mdlist_id: Option<uuid::Uuid>,
}

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

mod chapter_tracker;
mod commands;
mod constants;
mod migrator;
mod models;

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
            || new_message.channel_id != data.manga_update_channel_id.unwrap()
            || new_message.content.starts_with("s>")
        {
            return Ok(());
        }

        if let Some(captures) = MD_URL_REGEX.captures(&new_message.content) {
            let uuid = uuid::Uuid::try_parse(&captures[1]);

            if uuid.is_err() {
                return Ok(());
            }

            let uuid = uuid.unwrap();

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

            let title = if let Some(en_title) = en_title {
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

            let statistics = data
                .md
                .as_ref()
                .unwrap()
                .statistics()
                .manga()
                .id(uuid)
                .get()
                .send()
                .await?;

            let statistics = statistics.statistics.get(&uuid).unwrap();

            msg.edit(
                ctx,
                EditMessage::default()
                    .allowed_mentions(CreateAllowedMentions::new().replied_user(false))
                    .content("here's your manga!")
                    .embed(
                        CreateEmbed::default()
                            .title(title)
                            .url(format!("https://mangadex.org/title/{}", manga_id))
                            // .description(
                            //     manga
                            //         .description
                            //         .get(&mangadex_api_types_rust::Language::English)
                            //         .unwrap(),
                            // )
                            .field("status", manga.status.to_string(), true)
                            .field(
                                "year",
                                if let Some(year) = manga.year {
                                    year.to_string()
                                } else {
                                    "unknown".to_string()
                                },
                                true,
                            )
                            .field(
                                "demographic",
                                if let Some(demographic) = manga.publication_demographic {
                                    demographic.to_string()
                                } else {
                                    "unknown".to_string()
                                },
                                true,
                            )
                            .field(
                                "rating",
                                if let Some(avg) = statistics.rating.average {
                                    avg.to_string()
                                } else {
                                    "unknown".to_string()
                                },
                                true,
                            )
                            .field("follows", statistics.follows.to_string(), true)
                            .field(
                                "content rating",
                                if let Some(content_rating) = manga.content_rating {
                                    content_rating.to_string()
                                } else {
                                    "unknown".to_string()
                                },
                                true,
                            )
                            .field("tags", tags, false),
                    ),
            )
            .await
            .inspect_err(|e| tracing::error!(err = ?e, "an error occurred when editing message"))?;
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
    let db = Database::connect(db_url).await?;
    Migrator::up(&db, None).await?;

    tracing::info!("initializing mangadex client...");
    let md_client_id = std::env::var("MANGADEX_CLIENT_ID").inspect_err(|_| {
        tracing::warn!("missing mangadex client id. manga commands will not be initialized.");
    });
    let md_client_secret = std::env::var("MANGADEX_CLIENT_SECRET").inspect_err(|_| {
        tracing::warn!("missing mangadex client secret. manga commands will not be initialized.");
    });
    let md_mdlist_id = std::env::var("MANGADEX_MDLIST_ID").inspect_err(|_| {
        tracing::warn!("missing mangadex mdlist id. manga commands will not be initialized.");
    });
    let md_username = std::env::var("MANGADEX_USERNAME").inspect_err(|_| {
        tracing::warn!("missing mangadex username. mdlist commands will not be initialized.");
    });
    let md_password = std::env::var("MANGADEX_PASSWORD").inspect_err(|_| {
        tracing::warn!("missing mangadex password. mdlist commands will not be initialized.");
    });

    let md = if let (Ok(client_id), Ok(client_secret)) = (md_client_id, md_client_secret) {
        let md_client = MangaDexClient::default();

        md_client
            .set_client_info(&ClientInfo {
                client_id,
                client_secret,
            })
            .await?;

        if let (Ok(username), Ok(password)) = (md_username, md_password) {
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
        }

        Some(md_client)
    } else {
        None
    };

    let manga_update_channel_id = if let Ok(id) = std::env::var("MANGA_UPDATE_CHANNEL_ID") {
        if let Ok(id) = id.parse::<u64>() {
            tracing::info!("watching channel with id {} for mangadex links.", id);
            Some(ChannelId::new(id))
        } else {
            tracing::warn!("invalid channel id found. mangadex links will not be watched.");
            None
        }
    } else {
        tracing::warn!("no manga update channel id found. mangadex links will not be watched.");
        None
    };

    let mdlist_id = if let Ok(mdlist_id) = md_mdlist_id {
        if let Ok(id) = uuid::Uuid::try_parse(&mdlist_id) {
            Some(id)
        } else {
            None
        }
    } else {
        None
    };

    let data = Data {
        manga_update_channel_id,
        db,
        md,
        mdlist_id,
    };

    let data_clone = data.clone();

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![
                commands::help::help(),
                commands::status::status(),
                commands::role::role(),
                commands::fluff::squartatrice(),
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
            })
        })
        .build();

    let webhook_url = std::env::var("DISCORD_WEBHOOK_URL").map_err(|_| {
        tracing::warn!("missing discord webhook url. tracker will not be initialized.");
    });

    let client = serenity::ClientBuilder::new(&token, intents)
        .framework(framework)
        .activity(serenity::ActivityData {
            name: "squartatrice - 美樹 さやか vs. 美樹 さやか (fw. 美樹 さやか)".into(),
            kind: serenity::ActivityType::Listening,
            state: None,
            url: None,
        })
        .await;

    tracing::info!("finished initializing!");
    let bot_handle = tokio::spawn(async move { client.unwrap().start().await.unwrap() });

    if data_clone.md.is_some() && webhook_url.is_ok() {
        tracing::info!("initialized chapter tracker!");
        let http = serenity::Http::new(&token);
        let webhook = serenity::Webhook::from_url(&http, &webhook_url.unwrap())
            .await
            .inspect_err(
                |e| tracing::error!(err = ?e, "an error occurred when creating webhook"),
            )?;

        tokio::spawn(
            async move {
                let interval = tokio::time::interval(std::time::Duration::from_secs(7200));
                let task = futures::stream::unfold(interval, |mut interval| async {
                    interval.tick().await;
                    let _ = chapter_tracker::chapter_tracker(&http, &webhook, &data_clone).await;

                    Some(((), interval))
                });

                task.for_each(|_| async {}).await;
            }
            .in_current_span(),
        );
    }

    bot_handle.await?;

    Ok(())
}
