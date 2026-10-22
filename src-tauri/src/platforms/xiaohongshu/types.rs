use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum XiaohongshuErrorCode {
    InvalidInput,
    UntrustedHost,
    RedirectFailed,
    NotFound,
    Offline,
    AuthenticationRequired,
    AuthenticationExpired,
    RiskControlled,
    Network,
    Upstream,
    NoPlayableStream,
    MalformedResponse,
}

#[derive(Debug, Error, Serialize)]
#[error("{message}")]
pub struct XiaohongshuError {
    pub code: XiaohongshuErrorCode,
    pub message: String,
}

impl XiaohongshuError {
    pub fn new(code: XiaohongshuErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatorSummary {
    pub stable_user_id: String,
    pub red_id: Option<String>,
    pub nickname: String,
    pub avatar_url: String,
    pub current_room_id: Option<String>,
    pub live_status: Option<bool>,
    pub room_title: Option<String>,
}
