use std::collections::HashSet;

use reqwest::header::{COOKIE, REFERER, USER_AGENT};
use serde::Serialize;
use serde_json::{json, Value};
use url::Url;

use super::client::{authorized_headers, client, refresh_creator, response_error};
use super::credential::{CredentialVault, ParsedCookie};
use super::signing::user_agent;
use super::types::{XiaohongshuError, XiaohongshuErrorCode};

const ROOM_INFO_URI: &str = "/api/sns/red/live/h5/v1/room/current_room_info";
const ROOM_INFO_URL: &str =
    "https://live-room.xiaohongshu.com/api/sns/red/live/h5/v1/room/current_room_info";

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum StreamFormat {
    Flv,
    Hls,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamVariant {
    pub url: String,
    pub format: StreamFormat,
    pub label: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct XiaohongshuPlayback {
    pub stable_user_id: String,
    pub current_room_id: String,
    pub title: String,
    pub nickname: String,
    pub avatar_url: String,
    pub primary_url: String,
    pub variants: Vec<StreamVariant>,
}

fn trusted_stream_url(raw: &str) -> Option<(String, StreamFormat)> {
    let url = Url::parse(raw).ok()?;
    if !matches!(url.scheme(), "http" | "https") {
        return None;
    }
    let host = url.host_str()?.to_ascii_lowercase();
    if !(host == "xiaohongshu.com"
        || host.ends_with(".xiaohongshu.com")
        || host == "xhscdn.com"
        || host.ends_with(".xhscdn.com"))
    {
        return None;
    }
    let lower = raw.to_ascii_lowercase();
    let format = if lower.contains(".flv") {
        StreamFormat::Flv
    } else if lower.contains(".m3u8") || lower.contains("format=hls") {
        StreamFormat::Hls
    } else {
        return None;
    };
    Some((raw.to_string(), format))
}

fn collect_streams(value: &Value, path: &mut Vec<String>, output: &mut Vec<StreamVariant>) {
    match value {
        Value::String(raw) => {
            if let Some((url, format)) = trusted_stream_url(raw) {
                let label = path
                    .iter()
                    .rev()
                    .find(|part| !part.chars().all(|character| character.is_ascii_digit()))
                    .cloned()
                    .unwrap_or_else(|| match format {
                        StreamFormat::Flv => "FLV".into(),
                        StreamFormat::Hls => "HLS".into(),
                    });
                output.push(StreamVariant { url, format, label });
            }
        }
        Value::Array(values) => {
            for (index, value) in values.iter().enumerate() {
                path.push(index.to_string());
                collect_streams(value, path, output);
                path.pop();
            }
        }
        Value::Object(object) => {
            for (key, value) in object {
                path.push(key.clone());
                collect_streams(value, path, output);
                path.pop();
            }
        }
        _ => {}
    }
}

pub fn response_stream_variants(value: &Value) -> Vec<StreamVariant> {
    let mut variants = Vec::new();
    collect_streams(value, &mut Vec::new(), &mut variants);
    let mut seen = HashSet::new();
    variants.retain(|variant| seen.insert(variant.url.clone()));
    variants
}

fn string_at_keys(value: &Value, keys: &[&str]) -> Option<String> {
    match value {
        Value::Object(object) => {
            for key in keys {
                if let Some(Value::String(found)) = object.get(*key) {
                    if !found.trim().is_empty() {
                        return Some(found.clone());
                    }
                }
            }
            object
                .values()
                .find_map(|value| string_at_keys(value, keys))
        }
        Value::Array(values) => values.iter().find_map(|value| string_at_keys(value, keys)),
        _ => None,
    }
}

fn initial_state(html: &str) -> Option<Value> {
    let marker = "window.__INITIAL_STATE__=";
    let start = html.find(marker)? + marker.len();
    let end = html[start..].find("</script>")? + start;
    serde_json::from_str(&html[start..end].replace("undefined", "null")).ok()
}

async fn room_responses(
    room_id: &str,
    stable_user_id: &str,
    secret: &super::credential::SecretCookie,
) -> Result<Vec<Value>, XiaohongshuError> {
    ParsedCookie::parse(secret.expose()).map_err(|_| {
        XiaohongshuError::new(
            XiaohongshuErrorCode::AuthenticationExpired,
            "小红书 Cookie 格式无效。",
        )
    })?;
    let payload = json!({"room_id": room_id, "source": "share_out_of_app"});
    let query = format!(
        "room_id={}&source=share_out_of_app",
        urlencoding::encode(room_id)
    );
    let response = client()?
        .get(format!("{ROOM_INFO_URL}?{query}"))
        .headers(authorized_headers("GET", ROOM_INFO_URI, &payload, secret)?)
        .send()
        .await
        .map_err(|_| {
            XiaohongshuError::new(XiaohongshuErrorCode::Network, "无法读取小红书直播间。")
        })?;
    let status = response.status();
    let api_value: Value = response.json().await.map_err(|_| {
        XiaohongshuError::new(
            XiaohongshuErrorCode::MalformedResponse,
            "直播间接口返回了无法解析的数据。",
        )
    })?;
    if !status.is_success() || api_value.get("success").and_then(Value::as_bool) == Some(false) {
        return Err(response_error(status, &api_value));
    }

    let page_url =
        format!("https://www.xiaohongshu.com/livestream/{room_id}?host_id={stable_user_id}");
    let html = client()?
        .get(page_url)
        .header(USER_AGENT, user_agent())
        .header(REFERER, "https://www.xiaohongshu.com/")
        .header(COOKIE, secret.expose())
        .send()
        .await
        .map_err(|_| {
            XiaohongshuError::new(XiaohongshuErrorCode::Network, "无法读取小红书直播页。")
        })?
        .text()
        .await
        .map_err(|_| {
            XiaohongshuError::new(
                XiaohongshuErrorCode::MalformedResponse,
                "无法读取小红书直播页内容。",
            )
        })?;
    let mut values = vec![api_value];
    if let Some(state) = initial_state(&html) {
        values.push(state);
    }
    Ok(values)
}

pub async fn resolve_playback(
    stable_user_id: &str,
    red_id: Option<&str>,
    preferred_format: Option<&str>,
    vault: &CredentialVault,
) -> Result<XiaohongshuPlayback, XiaohongshuError> {
    let creator = refresh_creator(stable_user_id, red_id, vault).await?;
    match creator.live_status {
        Some(true) => {}
        Some(false) => {
            return Err(XiaohongshuError::new(
                XiaohongshuErrorCode::Offline,
                "主播未开播。",
            ))
        }
        None => {
            return Err(XiaohongshuError::new(
                XiaohongshuErrorCode::MalformedResponse,
                "直播状态字段缺失，暂时无法取流。",
            ))
        }
    }
    let room_id = creator.current_room_id.clone().ok_or_else(|| {
        XiaohongshuError::new(
            XiaohongshuErrorCode::MalformedResponse,
            "直播状态缺少当前房间 ID。",
        )
    })?;
    let secret = vault.load().map_err(|_| {
        XiaohongshuError::new(
            XiaohongshuErrorCode::AuthenticationRequired,
            "请先配置有效的小红书 Cookie。",
        )
    })?;
    let values = room_responses(&room_id, stable_user_id, &secret).await?;
    let mut variants = Vec::new();
    for value in &values {
        variants.extend(response_stream_variants(value));
    }
    let mut seen = HashSet::new();
    variants.retain(|variant| seen.insert(variant.url.clone()));
    if variants.is_empty() {
        return Err(XiaohongshuError::new(
            XiaohongshuErrorCode::NoPlayableStream,
            "直播响应中没有可验证的播放地址。",
        ));
    }
    let preferred = preferred_format.unwrap_or("flv").to_ascii_lowercase();
    variants.sort_by_key(|variant| {
        let format = match variant.format {
            StreamFormat::Flv => "flv",
            StreamFormat::Hls => "hls",
        };
        format != preferred
    });
    let primary_url = variants[0].url.clone();
    let title = values
        .iter()
        .find_map(|value| string_at_keys(value, &["room_title", "title"]))
        .unwrap_or_default();
    Ok(XiaohongshuPlayback {
        stable_user_id: stable_user_id.to_string(),
        current_room_id: room_id,
        title,
        nickname: creator.nickname,
        avatar_url: creator.avatar_url,
        primary_url,
        variants,
    })
}

#[tauri::command]
pub async fn get_xiaohongshu_playback(
    stable_user_id: String,
    red_id: Option<String>,
    preferred_format: Option<String>,
    vault: tauri::State<'_, CredentialVault>,
) -> Result<XiaohongshuPlayback, XiaohongshuError> {
    resolve_playback(
        &stable_user_id,
        red_id.as_deref(),
        preferred_format.as_deref(),
        &vault,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variants_are_response_derived_deduplicated_and_host_allowlisted() {
        let body = json!({
            "data": {"streams": [
                {"quality":"origin", "flv":"https://live-source-play.xhscdn.com/live/a.flv?token=1"},
                {"backup":"https://live-source-play.xhscdn.com/live/a.flv?token=1"},
                {"hls":"https://live-source-play.xhscdn.com/live/a.m3u8?token=2"},
                {"evil":"https://evil.example/live/a.flv"}
            ]}
        });
        let variants = response_stream_variants(&body);
        assert_eq!(variants.len(), 2);
        assert!(variants
            .iter()
            .any(|variant| variant.format == StreamFormat::Flv));
        assert!(variants
            .iter()
            .any(|variant| variant.format == StreamFormat::Hls));
        assert!(variants
            .iter()
            .all(|variant| !variant.url.contains("evil.example")));
    }

    #[test]
    fn missing_streams_do_not_construct_a_cdn_url() {
        let body = json!({"data":{"room_info":{"room_id":"123"}}});
        assert!(response_stream_variants(&body).is_empty());
    }
}
