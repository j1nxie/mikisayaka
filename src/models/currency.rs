use std::f32;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct CurrencyFromJPY {
    #[serde(rename = "ID")]
    pub id: String,

    #[serde(with = "time::serde::rfc3339")]
    pub date: OffsetDateTime,

    pub vnd: f32,
    pub usd: f32,
    pub eur: f32,
    pub gbp: f32,
}

#[derive(Clone, PartialEq, Deserialize, Serialize)]
pub struct AllRatesTodayResponse {
    pub rate: f32,
    pub source: String,
}
