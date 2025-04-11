use crate::{models::gas_prices::GasPrice, Context, Error};
use poise::serenity_prelude::{CreateEmbed, Timestamp};
use thousands::Separable;
use time::format_description::well_known;

/// get the current gas prices.
#[poise::command(prefix_command, rename = "gasprices", aliases("gas"))]
#[tracing::instrument(skip_all)]
pub async fn gas_prices(ctx: Context<'_>) -> Result<(), Error> {
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
    .fetch_all(&ctx.data().db)
    .await
    .inspect_err(
        |e| tracing::error!(err = ?e, "an error occurred when fetching gas data from db"),
    )?;

    let mut gas_embed = CreateEmbed::default().title("Giá xăng hiện tại");

    for gas in current_data {
        let gas_price_string = format!(
            "- Vùng 1: {}đ/lít\n- Vùng 2: {}đ/lít",
            gas.zone1_price.separate_with_dots(),
            gas.zone2_price.separate_with_dots(),
        );

        gas_embed = gas_embed
            .field(gas.gas_name, gas_price_string, false)
            .timestamp(
                gas.last_modified
                    .format(&well_known::Rfc3339)
                    .unwrap()
                    .parse::<Timestamp>()
                    .unwrap(),
            );
    }

    ctx.send(poise::CreateReply::default().embed(gas_embed))
        .await
        .inspect_err(|e| tracing::error!(err = ?e, "an error occurred when sending reply"))?;

    Ok(())
}
