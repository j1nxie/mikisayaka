use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

#[derive(Clone, Debug)]
pub struct HoyolabAccount {
    pub id: i64,
    pub user_id: String,
    pub hoyolab_token: String,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(untagged)]
pub enum ZenlessResponse {
    Success(ZenlessSuccessResponse),
    Failure(ZenlessFailureResponse),
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct ZenlessSuccessResponse {
    name: String,
    amount: i64,
    icon: String,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct ZenlessFailureResponse {
    pub data: Option<String>,
    pub message: String,
    pub retcode: ZenlessReturnCode,
}

#[derive(Deserialize_repr, Serialize_repr, Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i16)]
pub enum ZenlessReturnCode {
    InternalDatabaseError = -1,
    RateLimited = 10101,
    VisitedTooFrequently = -110,
    AlreadyClaimed = -5003,
    AuthInvalid = -100,
    AuthTimeout = -101,
    OtpRateLimited = -119,
    IncorrectGameAccount = -216,
    IncorrectGamePassword = -202,
    AccountNotExists = -3203,
    VerificationCodeRateLimited = -3206,
    AccountMuted = 2010,
}
