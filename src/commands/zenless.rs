use fancy_regex::Regex;
use time::{OffsetDateTime, UtcOffset};

use crate::models::zenless::{HoyolabAccount, ZenlessReturnCode};
use crate::{Context, Error};

fn get_cookie_value(cookie: &str, cookie_name: &str) -> Option<String> {
    let regex = Regex::new(&format!(r"(^| ){cookie_name}=([^;]+)")).unwrap();

    let captures = regex.captures(cookie).unwrap();

    captures.map(|captures| captures[2].to_owned())
}

#[tracing::instrument(skip_all)]
#[poise::command(
    prefix_command,
    subcommand_required,
    aliases("zzz"),
    subcommands("add", "daily")
)]
pub async fn zenless(_: Context<'_>) -> Result<(), Error> {
    Ok(())
}

#[tracing::instrument(skip_all)]
#[poise::command(prefix_command, dm_only)]
pub async fn add(ctx: Context<'_>, #[rest] cookie: String) -> Result<(), Error> {
    let token = get_cookie_value(&cookie, "ltoken_v2");
    let uid = get_cookie_value(&cookie, "ltuid_v2");
    let discord_id = &ctx.author().id.to_string();

    if let (Some(token), Some(uid)) = (token, uid) {
        let cookie_normalized = format!("ltoken_v2={token}; ltuid_v2={uid}");
        sqlx::query!(
            r#"
                INSERT INTO
                    hoyolab_accounts (user_id, hoyolab_token)
                VALUES
                    ($1, $2)
                ON CONFLICT (user_id)
                DO UPDATE SET
                    hoyolab_token = excluded.hoyolab_token;
            "#,
            discord_id,
            cookie_normalized
        )
        .execute(&ctx.data().db)
        .await
        .inspect_err(
            |e| tracing::error!(err = ?e, "an error occurred when inserting token to database"),
        )?;

        ctx.reply("added your token to the database!")
            .await
            .inspect_err(|e| tracing::error!(err = ?e, "an error occurred when sending reply"))?;
    }

    Ok(())
}

#[tracing::instrument(skip_all)]
#[poise::command(prefix_command, broadcast_typing)]
pub async fn daily(ctx: Context<'_>) -> Result<(), Error> {
    let id = &ctx.author().id.to_string();
    let account = sqlx::query_as!(
        HoyolabAccount,
        r#"
            SELECT
                id as "id!", user_id, hoyolab_token
            FROM
                hoyolab_accounts
            WHERE
                user_id = $1;
        "#,
        id
    )
    .fetch_optional(&ctx.data().db)
    .await
    .inspect_err(
        |e| tracing::error!(err = ?e, "an error occurred when fetching cookie from database"),
    )?;

    match account {
        Some(account) => {
            let daily_resp = ctx
                .data()
                .zenless_client
                .claim_daily_reward(&account.hoyolab_token)
                .await?;

            let status_resp = ctx
                .data()
                .zenless_client
                .get_daily_reward_status(&account.hoyolab_token)
                .await?;

            let offset = UtcOffset::from_hms(8, 0, 0).unwrap();
            let now = OffsetDateTime::now_utc().to_offset(offset);
            let month = now.month();

            let days_checked_in_str = format!(
                "you have checked in for **{}** days during {}!",
                status_resp.data().unwrap().total_days_signed_in,
                month,
            );

            match daily_resp.is_success() {
                true => {
                    let resp_str = if status_resp.is_success() {
                        String::from("successfully checked in for today!~ ") + &days_checked_in_str
                    } else {
                        String::from("successfully checked in for today!~")
                    };

                    ctx.reply(resp_str).await.inspect_err(
                        |e| tracing::error!(err = ?e, "an error occurred when sending reply"),
                    )?;
                }
                false => {
                    if daily_resp.retcode == ZenlessReturnCode::AlreadyClaimed {
                        let resp_str = if status_resp.is_success() {
                            String::from("you have already checked in for today! ")
                                + &days_checked_in_str
                        } else {
                            String::from("you have already checked in for today!")
                        };

                        ctx.reply(resp_str).await.inspect_err(
                            |e| tracing::error!(err = ?e, "an error occurred when sending reply"),
                        )?;
                    } else {
                        ctx.reply("something wrong happened while checking in.")
                        .await
                        .inspect_err(
                            |e| tracing::error!(err = ?e, "an error occurred when sending reply"),
                        )?;
                    }
                }
            }
        }
        None => {
            ctx.reply("you don't have a HoYoLab cookie registered.")
                .await
                .inspect_err(
                    |e| tracing::error!(err = ?e, "an error occurred when sending reply"),
                )?;
        }
    }

    Ok(())
}
