use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

pub mod daily;
pub mod geetest;

#[derive(Clone, Debug)]
pub struct HoyolabAccount {
    pub id: i64,
    pub user_id: String,
    pub hoyolab_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HoyolabResponse<T> {
    pub data: Option<T>,
    pub message: String,
    pub retcode: ZenlessReturnCode,
}

impl<T> HoyolabResponse<T> {
    pub fn is_success(&self) -> bool {
        self.retcode == ZenlessReturnCode::Success
    }

    pub fn is_error(&self) -> bool {
        self.retcode != ZenlessReturnCode::Success
    }

    pub fn data(&self) -> Option<&T> {
        if self.is_success() {
            self.data.as_ref()
        } else {
            None
        }
    }

    pub fn into_result(self) -> Result<T, String> {
        if self.is_success() {
            self.data
                .ok_or_else(|| "Success response missing data".to_string())
        } else {
            Err(self.message)
        }
    }
}

#[derive(Deserialize_repr, Serialize_repr, Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i16)]
pub enum ZenlessReturnCode {
    Success = 0,
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
