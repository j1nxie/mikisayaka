use crate::{
    models::zenless::{HoyolabAccount, ZenlessResponse, ZenlessReturnCode},
    Context, Error,
};
use fancy_regex::Regex;
use poise::serenity_prelude as serenity;

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
#[poise::command(prefix_command)]
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
            let resp = ctx
                .data()
                .zenless_client
                .daily_sign_in(account.hoyolab_token)
                .await?;

            match resp {
                ZenlessResponse::Success(_t) => {
                    ctx.reply("successfully checked in for today!~")
                        .await
                        .inspect_err(
                            |e| tracing::error!(err = ?e, "an error occurred when sending reply"),
                        )?;
                }
                ZenlessResponse::Failure(e) => {
                    if e.retcode == ZenlessReturnCode::AlreadyClaimed {
                        ctx.reply("you are already checked in.").await.inspect_err(
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
