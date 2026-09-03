use poise::serenity_prelude::{CreateEmbed, CreateEmbedFooter};
use sqlx::{Pool, Sqlite};
use thousands::Separable;

use crate::models::currency::CurrencyFromJPY;
use crate::{Context, Error};

pub async fn upsert_currency_rates(db: &Pool<Sqlite>, rates: &CurrencyFromJPY) -> sqlx::Result<()> {
    sqlx::query!(
        r#"
            INSERT INTO
                currency_rates (id, date, vnd, usd, eur, gbp)
            VALUES
                ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (id)
            DO UPDATE SET
                date = excluded.date,
                vnd = excluded.vnd,
                usd = excluded.usd,
                eur = excluded.eur,
                gbp = excluded.gbp;
        "#,
        rates.id,
        rates.date,
        rates.vnd,
        rates.usd,
        rates.eur,
        rates.gbp,
    )
    .execute(db)
    .await?;

    Ok(())
}

fn format_amount(value: f64) -> String {
    format!("{value:.2}").separate_with_commas()
}

fn rate_for(rates: &CurrencyFromJPY, code: &str) -> Option<f32> {
    match code {
        "jpy" => Some(1.0),
        "vnd" => Some(rates.vnd),
        "usd" => Some(rates.usd),
        "eur" => Some(rates.eur),
        "gbp" => Some(rates.gbp),
        _ => None,
    }
}

#[tracing::instrument(skip(ctx))]
#[poise::command(prefix_command, slash_command, rename = "conversion", aliases("conv"))]
pub async fn conversion(
    ctx: Context<'_>,
    #[description = "amount to convert"] amount: f64,
    #[description = "source currency code, e.g. jpy"] source: String,
    #[description = "target currency code, e.g. usd"] target: String,
) -> Result<(), Error> {
    let source = source.to_lowercase();
    let target = target.to_lowercase();

    let latest = sqlx::query_as!(
        CurrencyFromJPY,
        r#"
            SELECT
                id AS "id!: String",
                date AS "date!: time::OffsetDateTime",
                vnd AS "vnd!: f32",
                usd AS "usd!: f32",
                eur AS "eur!: f32",
                gbp AS "gbp!: f32"
            FROM currency_rates
            ORDER BY date DESC
            LIMIT 1;
        "#
    )
    .fetch_optional(&ctx.data().db)
    .await
    .inspect_err(
        |e| tracing::error!(err = ?e, "an error occurred when fetching currency rates from db"),
    )?;

    let Some(latest) = latest else {
        ctx.say("no currency rates have been recorded yet, please try again later.")
            .await
            .inspect_err(|e| tracing::error!(err = ?e, "an error occurred when sending reply"))?;

        return Ok(());
    };

    let (Some(source_rate), Some(target_rate)) =
        (rate_for(&latest, &source), rate_for(&latest, &target))
    else {
        ctx.say("unsupported currency code, supported codes are: jpy, vnd, usd, eur, gbp.")
            .await
            .inspect_err(|e| tracing::error!(err = ?e, "an error occurred when sending reply"))?;

        return Ok(());
    };

    let rate = (target_rate / source_rate) as f64;
    let converted = amount * rate;

    let source = source.to_uppercase();
    let target = target.to_uppercase();

    let embed = CreateEmbed::default()
        .title("conversion")
        .description(format!(
            "{} {source} = {} {target}",
            format_amount(amount),
            format_amount(converted),
        ))
        .field("rate", format!("1 {source} = {rate:.4} {target}"), false)
        .footer(CreateEmbedFooter::new(format!(
            "updated {}",
            latest.date.date()
        )));

    ctx.send(poise::CreateReply::default().embed(embed))
        .await
        .inspect_err(|e| tracing::error!(err = ?e, "an error occurred when sending reply"))?;

    Ok(())
}
