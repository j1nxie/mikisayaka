use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct GasPrice {
    #[serde(rename = "ID")]
    pub id: String,

    #[serde(rename = "Title")]
    pub gas_name: String,

    pub zone1_price: i64,
    pub zone2_price: i64,
    #[serde(with = "time::serde::rfc3339")]
    pub last_modified: OffsetDateTime,
}

#[derive(Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct GasResponse {
    pub objects: Vec<GasPrice>,
}
