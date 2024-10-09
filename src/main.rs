use constants::MD_URL_REGEX;
use dotenvy::dotenv;
use mangadex_api::{v5::schema::oauth::ClientInfo, MangaDexClient};
use migrator::Migrator;
use poise::serenity_prelude::{
    self as serenity, ChannelId, CreateAllowedMentions, CreateEmbed, CreateMessage, EditMessage,
    MessageReference,
};
use sea_orm::{Database, DatabaseConnection};
use sea_orm_migration::MigratorTrait;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;

struct Data {
    manga_update_channel_id: Option<ChannelId>,
    db: DatabaseConnection,
    md: Option<MangaDexClient>,
}

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

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
                .await?;

            let manga = data
                .md
                .as_ref()
                .unwrap()
                .manga()
                .id(uuid)
                .get()
                .send()
                .await?;

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
            .await?;
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    dotenv().expect(".env file not found.");

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .init();

    tracing::info!("initializing... please wait warmly.");
    let token = std::env::var("DISCORD_TOKEN").expect("missing DISCORD_TOKEN");
    let db_url = std::env::var("DATABASE_URL").expect("missing DATABASE_URL");
    let intents =
        serenity::GatewayIntents::non_privileged() | serenity::GatewayIntents::MESSAGE_CONTENT;

    tracing::info!("initializing database connection...");
    let db = Database::connect(db_url).await?;

    tracing::info!("initializing mangadex client...");
    let md_client = MangaDexClient::default();
    let md_client_id = std::env::var("MANGADEX_CLIENT_ID").map_err(|_| {
        tracing::warn!("missing mangadex client id. manga commands will not be initialized.");
    });
    let md_client_secret = std::env::var("MANGADEX_CLIENT_SECRET").map_err(|_| {
        tracing::warn!("missing mangadex client secret. manga commands will not be initialized.");
    });

    let mut md: Option<MangaDexClient> = None;

    if let (Ok(client_id), Ok(client_secret)) = (md_client_id, md_client_secret) {
        md_client
            .set_client_info(&ClientInfo {
                client_id,
                client_secret,
            })
            .await?;

        md = Some(md_client);
    }

    Migrator::up(&db, None).await?;

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
            let manga_update_channel_id = if let Ok(id) = std::env::var("MANGA_UPDATE_CHANNEL_ID") {
                if let Ok(id) = id.parse::<u64>() {
                    Some(ChannelId::new(id))
                } else {
                    tracing::warn!("invalid channel id found. mangadex links will not be watched.");
                    None
                }
            } else {
                tracing::warn!(
                    "no manga update channel id found. mangadex links will not be watched."
                );
                None
            };
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data {
                    manga_update_channel_id,
                    db,
                    md,
                })
            })
        })
        .build();

    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .activity(serenity::ActivityData {
            name: "squartatrice - 美樹 さやか vs. 美樹 さやか (fw. 美樹 さやか)".into(),
            kind: serenity::ActivityType::Listening,
            state: None,
            url: None,
        })
        .await;

    tracing::info!("finished initializing!");
    client.unwrap().start().await.unwrap();

    Ok(())
}
