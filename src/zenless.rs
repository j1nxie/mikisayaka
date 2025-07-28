use reqwest::header::{
    HeaderMap, HeaderValue, ACCEPT, ACCEPT_ENCODING, CONNECTION, COOKIE, ORIGIN, REFERER,
    USER_AGENT,
};

use crate::{
    constants::zenless::{HOYOLAB_API_BASE, USER_AGENT_STR, ZZZ_ACT_ID},
    models::zenless::ZenlessResponse,
};

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

    pub async fn daily_sign_in(&self, cookie: String) -> anyhow::Result<ZenlessResponse> {
        let resp = self
            .client
            .post(HOYOLAB_API_BASE.to_owned() + "/event/luna/zzz/os/sign")
            .header(COOKIE, HeaderValue::from_str(&cookie)?)
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
