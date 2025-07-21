use mangadex_api_types_rust::MangaFeedSortOrder;
use poise::serenity_prelude::*;

use crate::{constants::manga::MD_BLOCKED_LIST, models::manga::Manga, Data, Error};

#[tracing::instrument(skip_all)]
pub async fn chapter_tracker(http: &Http, data: &Data) -> Result<(), Error> {
    tracing::info!("started checking for new chapters!");

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
    .fetch_all(&data.db)
    .await?;

    let mut chapter_list: Vec<CreateEmbed> = vec![];

    for db_manga in manga_list {
        let uuid = db_manga.manga_dex_id;

        let manga = match data
            .md
            .as_ref()
            .unwrap()
            .manga()
            .id(uuid.into())
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
            .id(db_manga.manga_dex_id.into())
            .feed()
            .get()
            .add_translated_language(&mangadex_api_types_rust::Language::English)
            .publish_at_since(mangadex_api_types_rust::MangaDexDateTime::new(
                &db_manga.last_updated,
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

        let mut db_manga_insert = db_manga;
        let now = time::OffsetDateTime::now_utc();

        if chapter_feed.result == mangadex_api_types_rust::ResultType::Ok
            && !chapter_feed.data.is_empty()
        {
            let chapter = chapter_feed.data.first().unwrap();

            let chapter_data = &chapter.attributes;

            match &chapter_data.chapter {
                Some(chap) => {
                    tracing::info!(uuid = %uuid, "got chapter for manga");

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
                            "https://og.mangadex.org/og-image/chapter/{}",
                            chapter.id
                        ));

                    if let Some(timestamp) = chapter_data.publish_at {
                        db_manga_insert.last_chapter_date = Some(time::OffsetDateTime::new_utc(
                            timestamp.as_ref().date(),
                            timestamp.as_ref().time(),
                        ))
                    }

                    chapter_list.push(embed);
                }

                None => {
                    continue;
                }
            };
        }

        db_manga_insert.last_updated = time::OffsetDateTime::new_utc(now.date(), now.time());

        sqlx::query!(
            r#"
                INSERT INTO
                    manga (id, manga_dex_id, last_updated, last_chapter_date)
                VALUES
                    ($1, $2, $3, $4)
                ON CONFLICT (manga_dex_id)
                DO UPDATE SET
                    last_updated = excluded.last_updated,
                    last_chapter_date = excluded.last_chapter_date;
            "#,
            db_manga_insert.id,
            db_manga_insert.manga_dex_id,
            db_manga_insert.last_updated,
            db_manga_insert.last_chapter_date,
        )
        .execute(&data.db)
        .await?;
    }

    if chapter_list.is_empty() {
        return Ok(());
    }

    let chunks = chapter_list.chunks(10);

    for chunk in chunks {
        data.manga_update_channel_id
            .unwrap()
            .send_message(
                &http,
                CreateMessage::default()
                    .content(if chunk.len() > 1 {
                        "New chapters are out!"
                    } else {
                        "A new chapter is out!"
                    })
                    .embeds(chunk.to_vec()),
            )
            .await
            .inspect_err(|e| tracing::error!(err = ?e, "an error occurred when sending reply"))?;
    }

    tracing::info!("finished checking for new chapters!");

    Ok(())
}
