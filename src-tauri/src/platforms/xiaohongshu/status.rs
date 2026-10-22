use serde::Serialize;

use super::client::refresh_creator;
use super::credential::CredentialVault;
use super::types::{CreatorSummary, XiaohongshuError, XiaohongshuErrorCode};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CreatorLiveStatus {
    Live,
    Offline,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatorRefresh {
    pub stable_user_id: String,
    pub red_id: Option<String>,
    pub nickname: String,
    pub avatar_url: String,
    pub current_room_id: Option<String>,
    pub live_status: CreatorLiveStatus,
    pub room_title: Option<String>,
}

pub fn refresh_from_creator(
    requested_stable_id: &str,
    creator: CreatorSummary,
) -> Result<CreatorRefresh, XiaohongshuError> {
    if creator.stable_user_id != requested_stable_id {
        return Err(XiaohongshuError::new(
            XiaohongshuErrorCode::NotFound,
            "搜索结果与已关注主播身份不一致。",
        ));
    }
    Ok(CreatorRefresh {
        stable_user_id: requested_stable_id.to_string(),
        red_id: creator.red_id,
        nickname: creator.nickname,
        avatar_url: creator.avatar_url,
        current_room_id: creator.current_room_id,
        live_status: match creator.live_status {
            Some(true) => CreatorLiveStatus::Live,
            Some(false) => CreatorLiveStatus::Offline,
            None => {
                return Err(XiaohongshuError::new(
                    XiaohongshuErrorCode::MalformedResponse,
                    "直播状态字段缺失，暂时无法判断主播是否开播。",
                ))
            }
        },
        room_title: creator.room_title,
    })
}

#[tauri::command]
pub async fn refresh_xiaohongshu_creator(
    stable_user_id: String,
    red_id: Option<String>,
    vault: tauri::State<'_, CredentialVault>,
) -> Result<CreatorRefresh, XiaohongshuError> {
    let creator = refresh_creator(&stable_user_id, red_id.as_deref(), &vault).await?;
    refresh_from_creator(&stable_user_id, creator)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn creator(room: Option<&str>) -> CreatorSummary {
        CreatorSummary {
            stable_user_id: "stable-user".into(),
            red_id: Some("red-id".into()),
            nickname: "主播".into(),
            avatar_url: "avatar".into(),
            current_room_id: room.map(str::to_string),
            live_status: Some(room.is_some()),
            room_title: room.map(|_| "本场直播".to_string()),
        }
    }

    #[test]
    fn dynamic_room_can_change_without_changing_stable_identity() {
        let first = refresh_from_creator("stable-user", creator(Some("room-one"))).unwrap();
        let second = refresh_from_creator("stable-user", creator(Some("room-two"))).unwrap();
        assert_eq!(first.stable_user_id, second.stable_user_id);
        assert_ne!(first.current_room_id, second.current_room_id);
    }

    #[test]
    fn mismatched_search_identity_is_an_error_not_offline() {
        let mut wrong = creator(None);
        wrong.stable_user_id = "another-user".into();
        assert_eq!(
            refresh_from_creator("stable-user", wrong).unwrap_err().code,
            XiaohongshuErrorCode::NotFound
        );
    }

    #[test]
    fn missing_live_status_is_unknown_error_not_offline() {
        let mut unknown = creator(None);
        unknown.live_status = None;
        assert_eq!(
            refresh_from_creator("stable-user", unknown)
                .unwrap_err()
                .code,
            XiaohongshuErrorCode::MalformedResponse
        );
    }
}
