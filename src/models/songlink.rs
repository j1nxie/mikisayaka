use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SonglinkResponse {
    pub links_by_platform: LinkByPlatform,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkByPlatform {
    pub spotify: Option<LinkByPlatformInner>,
    pub youtube_music: Option<LinkByPlatformInner>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkByPlatformInner {
    pub url: String,
    pub entity_unique_id: String,
}
