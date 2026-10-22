use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, COOKIE, LOCATION, REFERER, USER_AGENT};
use reqwest::{redirect::Policy, Client, StatusCode};
use serde_json::{json, Value};

use super::credential::{CredentialVault, ParsedCookie, SecretCookie};
use super::input::{parse_creator_input, parse_redirect_target, CreatorInput};
use super::signing::{signed_headers, user_agent};
use super::types::{CreatorSummary, XiaohongshuError, XiaohongshuErrorCode};

const SEARCH_URI: &str = "/api/sns/web/v1/search/usersearch";
const SEARCH_URL: &str = "https://edith.xiaohongshu.com/api/sns/web/v1/search/usersearch";
const USER_ME_URI: &str = "/api/sns/web/v2/user/me";
const USER_ME_URL: &str = "https://edith.xiaohongshu.com/api/sns/web/v2/user/me";

fn client() -> Result<Client, XiaohongshuError> {
    Client::builder()
        .redirect(Policy::none())
        .connect_timeout(std::time::Duration::from_secs(15))
        .timeout(std::time::Duration::from_secs(25))
        .no_proxy()
        .build()
        .map_err(|_| {
            XiaohongshuError::new(XiaohongshuErrorCode::Network, "无法创建小红书网络客户端。")
        })
}

fn credential(vault: &CredentialVault) -> Result<SecretCookie, XiaohongshuError> {
    vault.load().map_err(|_| {
        XiaohongshuError::new(
            XiaohongshuErrorCode::AuthenticationRequired,
            "请先配置有效的小红书 Cookie。",
        )
    })
}

fn header(value: &str) -> Result<HeaderValue, XiaohongshuError> {
    HeaderValue::from_str(value).map_err(|_| {
        XiaohongshuError::new(XiaohongshuErrorCode::InvalidInput, "请求标头格式无效。")
    })
}

fn authorized_headers(
    method: &str,
    uri: &str,
    payload: &Value,
    secret: &SecretCookie,
) -> Result<HeaderMap, XiaohongshuError> {
    let parsed = ParsedCookie::parse(secret.expose()).map_err(|_| {
        XiaohongshuError::new(
            XiaohongshuErrorCode::AuthenticationExpired,
            "小红书 Cookie 格式无效。",
        )
    })?;
    let signed = signed_headers(method, uri, &parsed, payload);
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static(user_agent()));
    headers.insert(
        ACCEPT,
        HeaderValue::from_static("application/json, text/plain, */*"),
    );
    headers.insert(
        REFERER,
        HeaderValue::from_static("https://www.xiaohongshu.com/"),
    );
    headers.insert(COOKIE, header(secret.expose())?);
    headers.insert("X-s", header(&signed.x_s)?);
    headers.insert("X-t", header(&signed.x_t)?);
    headers.insert("X-S-Common", header(&signed.x_s_common)?);
    headers.insert("x-b3-traceid", header(&signed.b3_trace_id)?);
    headers.insert("x-xray-traceid", header(&signed.xray_trace_id)?);
    Ok(headers)
}

fn response_error(status: StatusCode, body: &Value) -> XiaohongshuError {
    let message = body
        .get("msg")
        .or_else(|| body.get("message"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    let lowered = message.to_ascii_lowercase();
    if status == StatusCode::UNAUTHORIZED
        || status == StatusCode::FORBIDDEN
        || lowered.contains("登录")
    {
        XiaohongshuError::new(
            XiaohongshuErrorCode::AuthenticationExpired,
            "小红书 Cookie 已失效，请重新配置。",
        )
    } else if status == StatusCode::TOO_MANY_REQUESTS
        || message.contains("频繁")
        || message.contains("异常")
        || lowered.contains("risk")
    {
        XiaohongshuError::new(
            XiaohongshuErrorCode::RiskControlled,
            "请求触发了小红书风控，请稍后再试。",
        )
    } else {
        XiaohongshuError::new(XiaohongshuErrorCode::Upstream, "小红书服务暂时不可用。")
    }
}

fn value_string(value: &Value, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| value.get(*key))
        .and_then(|value| match value {
            Value::String(value) => Some(value.clone()),
            Value::Number(value) => Some(value.to_string()),
            _ => None,
        })
        .filter(|value| !value.trim().is_empty())
}

fn creator_from_search(value: &Value) -> Option<CreatorSummary> {
    let stable_user_id = value_string(value, &["id", "user_id", "userId", "userid"])?;
    let live = value.get("live_info").or_else(|| value.get("liveInfo"));
    Some(CreatorSummary {
        stable_user_id,
        red_id: value_string(value, &["red_id", "redId"]),
        nickname: value_string(value, &["name", "nickname", "nick_name"])
            .unwrap_or_else(|| "未知主播".into()),
        avatar_url: value_string(value, &["image", "avatar", "avatar_url"]).unwrap_or_default(),
        current_room_id: live.and_then(|value| value_string(value, &["room_id", "roomId"])),
        live_status: live
            .and_then(|value| value.get("status"))
            .and_then(Value::as_i64)
            .map(|status| status == 2),
    })
}

pub fn creators_from_search_response(
    body: &Value,
    keyword: &str,
) -> Result<Vec<CreatorSummary>, XiaohongshuError> {
    if body.get("success").and_then(Value::as_bool) == Some(false) {
        return Err(response_error(StatusCode::OK, body));
    }
    let users = body
        .pointer("/data/users")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            XiaohongshuError::new(
                XiaohongshuErrorCode::MalformedResponse,
                "搜索响应缺少用户列表。",
            )
        })?;
    let mut creators = users
        .iter()
        .filter_map(creator_from_search)
        .collect::<Vec<_>>();
    let exact = creators
        .iter()
        .filter(|creator| {
            creator.red_id.as_deref() == Some(keyword) || creator.stable_user_id == keyword
        })
        .cloned()
        .collect::<Vec<_>>();
    if !exact.is_empty() {
        return Ok(exact);
    }
    creators.sort_by_key(|creator| creator.nickname != keyword);
    Ok(creators)
}

async fn resolve_short_link(url: url::Url) -> Result<CreatorInput, XiaohongshuError> {
    let client = client()?;
    let mut current = url;
    for _ in 0..5 {
        let response = client
            .get(current.clone())
            .header(USER_AGENT, user_agent())
            .send()
            .await
            .map_err(|_| {
                XiaohongshuError::new(XiaohongshuErrorCode::Network, "无法解析小红书短链接。")
            })?;
        let location = response
            .headers()
            .get(LOCATION)
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| {
                XiaohongshuError::new(
                    XiaohongshuErrorCode::RedirectFailed,
                    "短链接没有返回有效跳转地址。",
                )
            })?;
        let target = current.join(location).map_err(|_| {
            XiaohongshuError::new(XiaohongshuErrorCode::RedirectFailed, "短链接跳转地址无效。")
        })?;
        match parse_creator_input(target.as_str())? {
            CreatorInput::ShortLink(next) => current = next,
            _ => return parse_redirect_target(target.as_str()),
        }
    }
    Err(XiaohongshuError::new(
        XiaohongshuErrorCode::RedirectFailed,
        "短链接跳转次数过多。",
    ))
}

async fn search(
    keyword: &str,
    secret: &SecretCookie,
) -> Result<Vec<CreatorSummary>, XiaohongshuError> {
    let request_id = format!(
        "{}-{}",
        chrono::Utc::now().timestamp(),
        uuid::Uuid::new_v4().simple()
    );
    let payload = json!({
        "search_user_request": {
            "keyword": keyword,
            "search_id": uuid::Uuid::new_v4().simple().to_string(),
            "page": 1,
            "page_size": 15,
            "biz_type": "web_search_user",
            "request_id": request_id
        }
    });
    let headers = authorized_headers("POST", SEARCH_URI, &payload, secret)?;

    let response = client()?
        .post(SEARCH_URL)
        .headers(headers)
        .json(&payload)
        .send()
        .await
        .map_err(|_| XiaohongshuError::new(XiaohongshuErrorCode::Network, "无法连接小红书。"))?;
    let status = response.status();
    let body: Value = response.json().await.map_err(|_| {
        XiaohongshuError::new(
            XiaohongshuErrorCode::MalformedResponse,
            "小红书返回了无法解析的数据。",
        )
    })?;
    if !status.is_success() {
        return Err(response_error(status, &body));
    }
    creators_from_search_response(&body, keyword)
}

pub async fn test_credential(secret: &SecretCookie) -> Result<(), XiaohongshuError> {
    let payload = json!({});
    let response = client()?
        .get(USER_ME_URL)
        .headers(authorized_headers("GET", USER_ME_URI, &payload, secret)?)
        .send()
        .await
        .map_err(|_| XiaohongshuError::new(XiaohongshuErrorCode::Network, "无法连接小红书。"))?;
    let status = response.status();
    let body: Value = response.json().await.map_err(|_| {
        XiaohongshuError::new(
            XiaohongshuErrorCode::MalformedResponse,
            "小红书返回了无法解析的数据。",
        )
    })?;
    if !status.is_success() || body.get("success").and_then(Value::as_bool) == Some(false) {
        return Err(response_error(status, &body));
    }
    Ok(())
}

fn find_basic_info(value: &Value) -> Option<&Value> {
    if value.get("redId").is_some()
        && (value.get("nickname").is_some() || value.get("name").is_some())
    {
        return Some(value);
    }
    match value {
        Value::Object(object) => object.values().find_map(find_basic_info),
        Value::Array(values) => values.iter().find_map(find_basic_info),
        _ => None,
    }
}

async fn profile(stable_user_id: &str) -> Result<CreatorSummary, XiaohongshuError> {
    let url = format!("https://www.xiaohongshu.com/user/profile/{stable_user_id}");
    let text = client()?
        .get(url)
        .header(USER_AGENT, user_agent())
        .header(REFERER, "https://www.xiaohongshu.com/")
        .send()
        .await
        .map_err(|_| XiaohongshuError::new(XiaohongshuErrorCode::Network, "无法读取小红书主页。"))?
        .error_for_status()
        .map_err(|_| {
            XiaohongshuError::new(XiaohongshuErrorCode::Upstream, "小红书主页暂时不可用。")
        })?
        .text()
        .await
        .map_err(|_| {
            XiaohongshuError::new(
                XiaohongshuErrorCode::MalformedResponse,
                "无法读取小红书主页内容。",
            )
        })?;
    let marker = "window.__INITIAL_STATE__=";
    let start = text
        .find(marker)
        .map(|index| index + marker.len())
        .ok_or_else(|| {
            XiaohongshuError::new(
                XiaohongshuErrorCode::MalformedResponse,
                "主页缺少主播资料。",
            )
        })?;
    let end = text[start..]
        .find("</script>")
        .map(|index| start + index)
        .ok_or_else(|| {
            XiaohongshuError::new(XiaohongshuErrorCode::MalformedResponse, "主页资料不完整。")
        })?;
    let state: Value = serde_json::from_str(&text[start..end].replace("undefined", "null"))
        .map_err(|_| {
            XiaohongshuError::new(
                XiaohongshuErrorCode::MalformedResponse,
                "无法解析主页主播资料。",
            )
        })?;
    let basic = find_basic_info(&state).ok_or_else(|| {
        XiaohongshuError::new(XiaohongshuErrorCode::NotFound, "主页中未找到主播资料。")
    })?;
    Ok(CreatorSummary {
        stable_user_id: stable_user_id.to_string(),
        red_id: value_string(basic, &["redId", "red_id"]),
        nickname: value_string(basic, &["nickname", "name"])
            .unwrap_or_else(|| stable_user_id.to_string()),
        avatar_url: value_string(basic, &["imageb", "images", "avatar", "image"])
            .unwrap_or_default(),
        current_room_id: None,
        live_status: None,
    })
}

pub async fn resolve_creators(
    input: &str,
    vault: &CredentialVault,
) -> Result<Vec<CreatorSummary>, XiaohongshuError> {
    let parsed = parse_creator_input(input)?;
    let parsed = match parsed {
        CreatorInput::ShortLink(url) => resolve_short_link(url).await?,
        parsed => parsed,
    };
    match parsed {
        CreatorInput::StableUserId(stable_user_id) => Ok(vec![profile(&stable_user_id).await?]),
        CreatorInput::Search(keyword) => search(&keyword, &credential(vault)?).await,
        CreatorInput::ShortLink(_) => Err(XiaohongshuError::new(
            XiaohongshuErrorCode::RedirectFailed,
            "短链接解析失败。",
        )),
    }
}

#[tauri::command]
pub async fn search_xiaohongshu_creators(
    input: String,
    vault: tauri::State<'_, CredentialVault>,
) -> Result<Vec<CreatorSummary>, XiaohongshuError> {
    resolve_creators(&input, &vault).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_red_id_wins_over_fuzzy_results() {
        let body = json!({"success": true, "data": {"users": [
            {"id":"stable-a", "red_id":"target", "name":"Same Name", "image":"a"},
            {"id":"stable-b", "red_id":"target-2", "name":"target", "image":"b"}
        ]}});
        let creators = creators_from_search_response(&body, "target").unwrap();
        assert_eq!(creators.len(), 1);
        assert_eq!(creators[0].stable_user_id, "stable-a");
    }

    #[test]
    fn ambiguous_same_name_results_remain_selectable() {
        let body = json!({"success": true, "data": {"users": [
            {"id":"stable-a", "red_id":"one", "name":"Same Name", "image":"a"},
            {"id":"stable-b", "red_id":"two", "name":"Same Name", "image":"b"}
        ]}});
        let creators = creators_from_search_response(&body, "Same Name").unwrap();
        assert_eq!(creators.len(), 2);
        assert_ne!(creators[0].stable_user_id, creators[1].stable_user_id);
    }
}
