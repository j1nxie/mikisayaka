use poise::serenity_prelude::*;
use thousands::Separable;
use time::format_description::well_known;

use crate::{
    constants::gas_prices::{GAS_PRICES_ENDPOINT, RELEVANT_GAS_IDS},
    models::gas_prices::{GasPrice, GasResponse},
    Data, Error,
};

#[tracing::instrument(skip_all)]
pub async fn gas_prices(http: &Http, data: &Data) -> Result<(), Error> {
    tracing::info!("started checking for new gas prices!");

    let current_data = sqlx::query_as!(
        GasPrice,
        r#"
            SELECT
                id AS "id!: String",
                gas_name,
                zone1_price,
                zone2_price,
                last_modified
            FROM gas_prices;
        "#
    )
    .fetch_all(&data.db)
    .await
    .inspect_err(
        |e| tracing::error!(err = ?e, "an error occurred when fetching gas data from db"),
    )?;

    let resp = data
        .reqwest_client
        .get(GAS_PRICES_ENDPOINT)
        .send()
        .await
        .inspect_err(
            |e| tracing::error!(err = ?e, "an error occurred when fetching gas prices from API"),
        )?;

    let resp: GasResponse = resp.json().await.unwrap();

    let changes: Vec<_> = resp
        .objects
        .into_iter()
        .filter(|gas| RELEVANT_GAS_IDS.contains(&gas.id.as_str()))
        .map(|gas| {
            let current = current_data.iter().find(|current| current.id == gas.id);

            (gas, current)
        })
        .collect();

    let mut any_updates = false;

    let mut gas_embed = CreateEmbed::default().title("Cập nhật giá xăng");

    for (new_gas, current_gas) in changes {
        match current_gas {
            Some(current_gas) => {
                if new_gas.last_modified <= current_gas.last_modified {
                    continue;
                }

                tracing::info!(id = %new_gas.id, gas_name = %new_gas.gas_name, "got new price update for gas");

                any_updates = true;

                sqlx::query!(
                    r#"
                        INSERT INTO
                            gas_prices (id, gas_name, zone1_price, zone2_price, last_modified)
                        VALUES
                            ($1, $2, $3, $4, $5)
                        ON CONFLICT (id)
                        DO UPDATE SET
                            zone1_price = excluded.zone1_price,
                            zone2_price = excluded.zone2_price,
                            last_modified = excluded.last_modified;
                    "#,
                    new_gas.id,
                    new_gas.gas_name,
                    new_gas.zone1_price,
                    new_gas.zone2_price,
                    new_gas.last_modified,
                )
                .execute(&data.db)
                .await
                .inspect_err(|e| {
                    tracing::error!(err = ?e, "an error occurred when inserting new gas data into db")
                })?;

                let zone1_diff = new_gas.zone1_price - current_gas.zone1_price;
                let zone2_diff = new_gas.zone2_price - current_gas.zone2_price;

                let gas_price_string = format!(
                    "- Vùng 1: {}đ/lít ({}đ/lít)\n- Vùng 2: {}đ/lít ({}đ/lít)",
                    new_gas.zone1_price.separate_with_dots(),
                    if zone1_diff > 0 {
                        format!("+{}", zone1_diff.separate_with_dots())
                    } else {
                        zone1_diff.separate_with_dots()
                    },
                    new_gas.zone2_price.separate_with_dots(),
                    if zone2_diff > 0 {
                        format!("+{}", zone2_diff.separate_with_dots())
                    } else {
                        zone2_diff.separate_with_dots()
                    },
                );

                gas_embed = gas_embed
                    .field(
                        if zone1_diff < 0 {
                            format!(
                                "<a:ARROW_IS_DOWN_ANIM:1360156568137502771> {}",
                                new_gas.gas_name
                            )
                        } else {
                            format!(
                                "<a:ARROW_IS_UP_ANIM:1360156587611783219> {}",
                                new_gas.gas_name
                            )
                        },
                        gas_price_string,
                        false,
                    )
                    .timestamp(
                        new_gas
                            .last_modified
                            .format(&well_known::Rfc3339)
                            .unwrap()
                            .parse::<Timestamp>()
                            .unwrap(),
                    );
            }
            None => {
                tracing::info!(id = %new_gas.id, gas_name = %new_gas.gas_name, "got new price update for gas");

                sqlx::query!(
                    r#"
                        INSERT INTO
                            gas_prices (id, gas_name, zone1_price, zone2_price, last_modified)
                        VALUES
                            ($1, $2, $3, $4, $5)
                        ON CONFLICT (id)
                        DO UPDATE SET
                            zone1_price = excluded.zone1_price,
                            zone2_price = excluded.zone2_price,
                            last_modified = excluded.last_modified;
                    "#,
                    new_gas.id,
                    new_gas.gas_name,
                    new_gas.zone1_price,
                    new_gas.zone2_price,
                    new_gas.last_modified,
                )
                .execute(&data.db)
                .await
                .inspect_err(|e| {
                    tracing::error!(err = ?e, "an error occurred when inserting gas data into db")
                })?;

                let gas_price_string = format!(
                    "- Vùng 1: {}đ/lít\n- Vùng 2: {}đ/lít",
                    new_gas.zone1_price.separate_with_dots(),
                    new_gas.zone2_price.separate_with_dots(),
                );

                gas_embed = gas_embed
                    .field(new_gas.gas_name, gas_price_string, false)
                    .timestamp(
                        new_gas
                            .last_modified
                            .format(&well_known::Rfc3339)
                            .unwrap()
                            .parse::<Timestamp>()
                            .unwrap(),
                    );
            }
        }
    }

    if any_updates && data.gas_prices_channel_id.is_some() {
        data.gas_prices_channel_id
            .unwrap()
            .send_message(&http, CreateMessage::new().add_embed(gas_embed))
            .await
            .inspect_err(|e| tracing::error!(err = ?e, "an error occurred when sending message"))?;
    }

    tracing::info!("finished checking for new gas prices!");

    Ok(())
}
