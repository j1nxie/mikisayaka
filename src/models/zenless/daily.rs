use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyReward {
    pub name: String,
    #[serde(rename = "cnt")]
    pub amount: i32,
    pub icon: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyRewardStatus {
    #[serde(rename = "total_sign_day")]
    pub total_days_signed_in: i32,
    pub today: String,
    #[serde(rename = "is_sign")]
    pub is_signed_in: bool,
    pub is_sub: bool,
    pub region: String,
    pub sign_cnt_missed: i32,
    pub short_sign_day: i32,
    pub send_first: bool,
}
