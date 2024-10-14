use poise::serenity_prelude::{CreateEmbed, ExecuteWebhook, Http, Webhook};
use sea_orm::{ActiveModelTrait, EntityTrait, IntoActiveModel, Set};

use crate::{constants::MD_BLOCKED_LIST, models::manga, Data, Error};

pub async fn chapter_tracker(http: &Http, webhook: &Webhook, data: &Data) -> Result<(), Error> {
    let manga_list = manga::Entity::find().all(&data.db).await?;

    let mut chapter_list: Vec<CreateEmbed> = vec![];

    for db_manga in manga_list {
        let manga = data
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

        let chapter_feed = data
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
            .excluded_groups(MD_BLOCKED_LIST.clone())
            .limit(1u32)
            .send()
            .await?;

        if chapter_feed.result == mangadex_api_types_rust::ResultType::Ok
            && !chapter_feed.data.is_empty()
        {
            let chapter = chapter_feed.data.first().unwrap();

            let chapter_data = &chapter.attributes;

            let mut vol_chap_str = if let Some(vol) = &chapter_data.volume {
                format!(
                    "Vol. {}, Ch. {}",
                    vol,
                    chapter_data.chapter.as_ref().unwrap()
                )
            } else {
                format!("Ch. {}", chapter_data.chapter.as_ref().unwrap())
            };

            if let Some(chapter_title) = &chapter_data.title {
                vol_chap_str = vol_chap_str + &format!(" - {}", chapter_title);
            }

            let embed = CreateEmbed::default()
                .title(title)
                .url(format!("https://mangadex.org/chapter/{}", chapter.id))
                .description(vol_chap_str)
                .thumbnail(format!(
                    "https://og.mangadex.org/og-image/manga/{}",
                    db_manga.manga_dex_id,
                ));

            chapter_list.push(embed);
        }

        let mut db_manga = db_manga.into_active_model();
        let now = time::OffsetDateTime::now_utc();
        db_manga.last_updated = Set(time::PrimitiveDateTime::new(now.date(), now.time()));

        db_manga.update(&data.db).await?;
    }

    if chapter_list.is_empty() {
        return Ok(());
    }

    let builder = ExecuteWebhook::new()
        .content("New chapters are out!")
        .embeds(chapter_list);

    webhook.execute(http, false, builder).await?;

    Ok(())
}
