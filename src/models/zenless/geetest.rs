use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeetestResponse {
    pub code: String,
    pub risk_code: i64,
    pub gt: String,
    pub challenge: String,
    pub success: i64,
    pub is_risk: bool,
}
