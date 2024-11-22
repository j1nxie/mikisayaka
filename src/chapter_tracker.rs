use mangadex_api_types_rust::MangaFeedSortOrder;
use poise::serenity_prelude::{CreateEmbed, ExecuteWebhook, Http, Webhook};
use sea_orm::{ActiveModelTrait, EntityTrait, IntoActiveModel, Set};

use crate::{constants::MD_BLOCKED_LIST, models::manga, Data, Error};

#[tracing::instrument(skip_all)]
pub async fn chapter_tracker(http: &Http, webhook: &Webhook, data: &Data) -> Result<(), Error> {
    let manga_list = manga::Entity::find().all(&data.db).await?;

    let mut chapter_list: Vec<CreateEmbed> = vec![];

    for db_manga in manga_list {
        let uuid = db_manga.manga_dex_id;

        let manga = match data
            .md
            .as_ref()
            .unwrap()
            .manga()
            .id(uuid)
            .get()
            .send()
            .await
        {
            Ok(manga) => manga,
            Err(e) => {
                tracing::error!(err = ?e, uuid = %uuid, "an error occurred when fetching manga");
                continue;
            }
        };

        let manga_id = manga.data.id;
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

        let chapter_feed = match data
            .md
            .as_ref()
            .unwrap()
            .manga()
            .id(db_manga.manga_dex_id)
            .feed()
            .get()
            .add_translated_language(&mangadex_api_types_rust::Language::English)
            .publish_at_since(mangadex_api_types_rust::MangaDexDateTime::new(
                &db_manga.last_updated.assume_utc(),
            ))
            .order(MangaFeedSortOrder::Chapter(
                mangadex_api_types_rust::OrderDirection::Descending,
            ))
            .excluded_groups(MD_BLOCKED_LIST.clone())
            .limit(1u32)
            .send()
            .await
        {
            Ok(feed) => feed,
            Err(e) => {
                tracing::error!(err = ?e, uuid = %uuid, "an error occurred when fetching chapter feed");
                continue;
            }
        };

        let mut db_manga_insert = db_manga.into_active_model();
        let now = time::OffsetDateTime::now_utc();

        if chapter_feed.result == mangadex_api_types_rust::ResultType::Ok
            && !chapter_feed.data.is_empty()
        {
            let chapter = chapter_feed.data.first().unwrap();

            let chapter_data = &chapter.attributes;

            match &chapter_data.chapter {
                Some(chap) => {
                    let mut vol_chap_str = match &chapter_data.volume {
                        Some(vol) => format!("Vol. {}, Ch. {}", vol, chap),
                        None => format!("Ch. {}", chap),
                    };

                    if let Some(chapter_title) = &chapter_data.title {
                        vol_chap_str = vol_chap_str + &format!(" - {}", chapter_title);
                    }

                    let embed = CreateEmbed::default()
                        .title(title)
                        .url(format!("https://mangadex.org/chapter/{}", chapter.id))
                        .description(vol_chap_str)
                        .image(format!(
                            "https://og.mangadex.org/og-image/manga/{}",
                            manga_id
                        ));

                    if let Some(timestamp) = chapter_data.publish_at {
                        db_manga_insert.last_chapter_date =
                            Set(Some(time::PrimitiveDateTime::new(
                                timestamp.as_ref().date(),
                                timestamp.as_ref().time(),
                            )));
                    }

                    chapter_list.push(embed);
                }

                None => {
                    continue;
                }
            };
        }

        db_manga_insert.last_updated = Set(time::PrimitiveDateTime::new(now.date(), now.time()));

        if let Err(e) = db_manga_insert.update(&data.db).await {
            tracing::error!(err = ?e, uuid = %uuid, "an error occurred when updating manga in database");
            continue;
        };
    }

    if chapter_list.is_empty() {
        return Ok(());
    }

    if chapter_list.len() > 10 {
        let chunks = chapter_list.chunks(10);

        for chunk in chunks {
            let builder = ExecuteWebhook::new()
                .content("New chapters are out!")
                .embeds(chunk.to_vec());

            webhook.execute(http, false, builder).await?;
        }

        return Ok(());
    }

    let builder = ExecuteWebhook::new()
        .content("New chapters are out!")
        .embeds(chapter_list)
        .avatar_url(
            http.get_current_user()
                .await?
                .avatar_url()
                .unwrap_or_default(),
        );

    webhook.execute(http, false, builder).await?;

    Ok(())
}
