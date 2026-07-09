use time::OffsetDateTime;

use crate::commands::conversion::upsert_currency_rates;
use crate::constants::currency::{ALL_RATES_TODAY_ENDPOINT, CURRENCY_TARGETS};
use crate::models::currency::{AllRatesTodayResponse, CurrencyFromJPY};
use crate::{Data, Error};

#[tracing::instrument(skip_all)]
pub async fn currency_rates(data: &Data) -> Result<(), Error> {
    tracing::info!("started fetching new currency rates!");

    let api_key = std::env::var("CONVERSION_API_TOKEN").expect("missing CONVERSION_API_TOKEN");
    let now = OffsetDateTime::now_utc();

    let mut rates = CurrencyFromJPY {
        id: now.date().to_string(),
        date: now,
        vnd: 0.0,
        usd: 0.0,
        eur: 0.0,
        gbp: 0.0,
    };

    for target in CURRENCY_TARGETS {
        let resp: AllRatesTodayResponse = data
            .reqwest_client
            .get(ALL_RATES_TODAY_ENDPOINT)
            .bearer_auth(&api_key)
            .query(&[("source", "JPY"), ("target", target)])
            .send()
            .await
            .inspect_err(
                |e| tracing::error!(err = ?e, target, "an error occurred when fetching currency rate"),
            )?
            .json()
            .await
            .inspect_err(
                |e| tracing::error!(err = ?e, target, "an error occurred when decoding currency rate response"),
            )?;

        match target {
            "VND" => rates.vnd = resp.rate as f32,
            "USD" => rates.usd = resp.rate as f32,
            "EUR" => rates.eur = resp.rate as f32,
            "GBP" => rates.gbp = resp.rate as f32,
            _ => {}
        }
    }

    upsert_currency_rates(&data.db, &rates).await.inspect_err(
        |e| tracing::error!(err = ?e, "an error occurred when upserting currency rates"),
    )?;

    tracing::info!("finished fetching new currency rates!");

    Ok(())
}
