use poise::serenity_prelude::*;
use reqwest::header::{
    ACCEPT, ACCEPT_ENCODING, CONNECTION, COOKIE, HeaderMap, HeaderValue, ORIGIN, REFERER,
    USER_AGENT,
};

use crate::constants::zenless::{HOYOLAB_API_BASE, USER_AGENT_STR, ZZZ_ACT_ID};
use crate::models::zenless::daily::{DailyReward, DailyRewardStatus};
use crate::models::zenless::geetest::GeetestResponse;
use crate::models::zenless::{HoyolabAccount, HoyolabResponse, ZenlessReturnCode};
use crate::{Data, Error};

#[derive(Clone)]
pub struct ZenlessClient {
    pub client: reqwest::Client,
}

impl ZenlessClient {
    pub fn new() -> Self {
        let mut headers = HeaderMap::new();
        headers.insert(
            ACCEPT,
            HeaderValue::from_str("application/json, text/plain, */*").unwrap(),
        );
        headers.insert(
            ACCEPT_ENCODING,
            HeaderValue::from_str("gzip, deflate, br").unwrap(),
        );
        headers.insert(CONNECTION, HeaderValue::from_str("keep-alive").unwrap());
        headers.insert(
            "x-rpc-app_version",
            HeaderValue::from_str("2.34.1").unwrap(),
        );
        headers.insert(USER_AGENT, HeaderValue::from_str(USER_AGENT_STR).unwrap());
        headers.insert("x-rpc-client_type", HeaderValue::from_str("4").unwrap());
        headers.insert("x-rpc-signgame", HeaderValue::from_str("zzz").unwrap());
        headers.insert(
            REFERER,
            HeaderValue::from_str("https://act.hoyolab.com/").unwrap(),
        );
        headers.insert(
            ORIGIN,
            HeaderValue::from_str("https://act.hoyolab.com").unwrap(),
        );

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .unwrap();

        ZenlessClient { client }
    }

    pub async fn get_monthly_rewards(
        &self,
        cookie: &str,
    ) -> anyhow::Result<HoyolabResponse<Vec<DailyReward>>> {
        let resp = self
            .client
            .get(HOYOLAB_API_BASE.to_owned() + "/event/luna/zzz/os/home")
            .header(COOKIE, HeaderValue::from_str(cookie)?)
            .query(&[("lang", "en-us"), ("act_id", ZZZ_ACT_ID)])
            .send()
            .await
            .inspect_err(
                |e| tracing::error!(err = ?e, "an error occurred when sending daily request"),
            )?;

        let text = resp.text().await.inspect_err(
            |e| tracing::error!(err = ?e, "an error occurred when receiving response text"),
        )?;

        let body = serde_json::from_str(&text).inspect_err(
            |e| tracing::error!(err = ?e, text = %text, "an error occurred when parsing response body"),
        )?;

        Ok(body)
    }

    pub async fn get_daily_reward_status(
        &self,
        cookie: &str,
    ) -> anyhow::Result<HoyolabResponse<DailyRewardStatus>> {
        let resp = self
            .client
            .get(HOYOLAB_API_BASE.to_owned() + "/event/luna/zzz/os/info")
            .header(COOKIE, HeaderValue::from_str(cookie)?)
            .query(&[("lang", "en-us"), ("act_id", ZZZ_ACT_ID)])
            .send()
            .await
            .inspect_err(
                |e| tracing::error!(err = ?e, "an error occurred when sending daily request"),
            )?;

        let text = resp.text().await.inspect_err(
            |e| tracing::error!(err = ?e, "an error occurred when receiving response text"),
        )?;

        let body = serde_json::from_str(&text).inspect_err(
            |e| tracing::error!(err = ?e, text = %text, "an error occurred when parsing response body"),
        )?;

        Ok(body)
    }

    pub async fn claim_daily_reward(
        &self,
        cookie: &str,
    ) -> anyhow::Result<HoyolabResponse<GeetestResponse>> {
        let resp = self
            .client
            .post(HOYOLAB_API_BASE.to_owned() + "/event/luna/zzz/os/sign")
            .header(COOKIE, HeaderValue::from_str(cookie)?)
            .query(&[("lang", "en-us"), ("act_id", ZZZ_ACT_ID)])
            .send()
            .await
            .inspect_err(
                |e| tracing::error!(err = ?e, "an error occurred when sending daily request"),
            )?;

        let text = resp.text().await.inspect_err(
            |e| tracing::error!(err = ?e, "an error occurred when receiving response text"),
        )?;

        let body = serde_json::from_str(&text).inspect_err(
            |e| tracing::error!(err = ?e, text = %text, "an error occurred when parsing response body"),
        )?;

        Ok(body)
    }
}

#[tracing::instrument(skip_all)]
pub async fn scheduled_claim_daily_reward(http: &Http, data: &Data) -> Result<(), Error> {
    tracing::info!("started claiming ZZZ daily reward!");

    let accounts = sqlx::query_as!(
        HoyolabAccount,
        r#"
            SELECT
                id as "id!", user_id, hoyolab_token
            FROM
                hoyolab_accounts;
        "#,
    )
    .fetch_all(&data.db)
    .await
    .inspect_err(
        |e| tracing::error!(err = ?e, "an error occurred when fetching cookie from database"),
    )?;

    if accounts.is_empty() {
        tracing::info!("no HoyoLab accounts were configured, exiting");

        return Ok(());
    }

    let tasks: Vec<_> = accounts
        .iter()
        .map(|account| {
            data.zenless_client
                .claim_daily_reward(&account.hoyolab_token)
        })
        .collect();

    let results = futures::future::join_all(tasks).await;

    let mut resp_str = String::from("today's daily claim status:\n");

    for (idx, (account, result)) in accounts.iter().zip(results.iter()).enumerate() {
        match result {
            // got response, not sure whether success or failure on the API side
            Ok(resp) => {
                match resp.is_success() {
                    // API response is good
                    true => {
                        resp_str += &format!(
                            "{}. <@{}>: daily reward claimed successfully.",
                            idx + 1,
                            account.user_id
                        );
                        tracing::info!(user = %account.user_id, "automatically claimed daily reward");
                    }
                    // some non-zero return code happened
                    false => {
                        if resp.retcode == ZenlessReturnCode::AlreadyClaimed {
                            resp_str += &format!(
                                "{}. <@{}>: you've already claimed your daily reward for today.",
                                idx + 1,
                                account.user_id
                            );
                            tracing::warn!(user = %account.user_id, "user has already claimed daily reward");
                        } else {
                            resp_str += &format!(
                                "{}. <@{}>: an error occurred while claiming your daily reward. \
                                 please try claiming manually using `s>zzz daily`.",
                                idx + 1,
                                account.user_id
                            );
                            tracing::error!(err = ?resp.retcode, user = %account.user_id, "an error occurred when claiming daily reward");
                        }
                    }
                }
            }
            Err(e) => {
                tracing::error!(err = ?e, user = %account.user_id, "an error occurred when sending daily claim request");
            }
        }
    }

    if let Some(channel_id) = data.zzz_daily_result_channel_id {
        channel_id
            .send_message(&http, CreateMessage::default().content(resp_str))
            .await
            .inspect_err(|e| tracing::error!(err = ?e, "an error occurred when sending reply"))?;
    }

    tracing::info!("finished claiming ZZZ daily reward!");

    Ok(())
}
