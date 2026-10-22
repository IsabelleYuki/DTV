use regex::Regex;
use url::Url;

use super::types::{XiaohongshuError, XiaohongshuErrorCode};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CreatorInput {
    StableUserId(String),
    ShortLink(Url),
    Search(String),
}

fn trusted_host(host: &str) -> bool {
    host == "xiaohongshu.com"
        || host.ends_with(".xiaohongshu.com")
        || host == "xhslink.com"
        || host.ends_with(".xhslink.com")
}

fn is_short_host(host: &str) -> bool {
    host == "xhslink.com" || host.ends_with(".xhslink.com")
}

fn candidate_url(input: &str) -> Option<&str> {
    static URL_RE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    URL_RE
        .get_or_init(|| Regex::new(r#"https?://[^\s<>\"']+"#).expect("valid URL regex"))
        .find(input)
        .map(|value| {
            value
                .as_str()
                .trim_end_matches(['。', '，', ',', ')', '）'])
        })
}

fn stable_id_from_url(url: &Url) -> Option<String> {
    if let Some(value) = url
        .query_pairs()
        .find_map(|(key, value)| (key == "host_id").then(|| value.into_owned()))
        .filter(|value| !value.trim().is_empty())
    {
        return Some(value);
    }

    let segments = url.path_segments()?.collect::<Vec<_>>();
    segments
        .windows(2)
        .find(|window| window[0] == "profile")
        .map(|window| window[1].trim().to_string())
        .filter(|value| !value.is_empty())
}

pub fn parse_creator_input(input: &str) -> Result<CreatorInput, XiaohongshuError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(XiaohongshuError::new(
            XiaohongshuErrorCode::InvalidInput,
            "请输入主页、分享链接、UID、小红书号或昵称。",
        ));
    }

    if let Some(raw_url) =
        candidate_url(trimmed).or_else(|| trimmed.starts_with("http").then_some(trimmed))
    {
        let url = Url::parse(raw_url).map_err(|_| {
            XiaohongshuError::new(XiaohongshuErrorCode::InvalidInput, "链接格式无效。")
        })?;
        let host = url.host_str().unwrap_or_default().to_ascii_lowercase();
        if !trusted_host(&host) {
            return Err(XiaohongshuError::new(
                XiaohongshuErrorCode::UntrustedHost,
                "只接受小红书官方域名链接。",
            ));
        }
        if is_short_host(&host) {
            return Ok(CreatorInput::ShortLink(url));
        }
        if let Some(stable_id) = stable_id_from_url(&url) {
            return Ok(CreatorInput::StableUserId(stable_id));
        }
        return Err(XiaohongshuError::new(
            XiaohongshuErrorCode::InvalidInput,
            "链接中没有可识别的主播 UID。",
        ));
    }

    let is_uid = trimmed.len() == 24 && trimmed.bytes().all(|byte| byte.is_ascii_hexdigit());
    if is_uid {
        return Ok(CreatorInput::StableUserId(trimmed.to_ascii_lowercase()));
    }
    Ok(CreatorInput::Search(trimmed.to_string()))
}

pub fn parse_redirect_target(location: &str) -> Result<CreatorInput, XiaohongshuError> {
    match parse_creator_input(location)? {
        CreatorInput::ShortLink(_) => Err(XiaohongshuError::new(
            XiaohongshuErrorCode::RedirectFailed,
            "短链接仍指向另一个短链接。",
        )),
        resolved => Ok(resolved),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_profile_uid_and_live_share_host_id() {
        assert_eq!(
            parse_creator_input(
                "https://www.xiaohongshu.com/user/profile/0123456789abcdef01234567?xsec_token=t"
            )
            .unwrap(),
            CreatorInput::StableUserId("0123456789abcdef01234567".into())
        );
        assert_eq!(
            parse_creator_input("看看直播 https://www.xiaohongshu.com/livestream/999?host_id=fedcba987654321001234567").unwrap(),
            CreatorInput::StableUserId("fedcba987654321001234567".into())
        );
    }

    #[test]
    fn classifies_uid_red_id_and_short_link() {
        assert!(matches!(
            parse_creator_input("0123456789abcdef01234567").unwrap(),
            CreatorInput::StableUserId(_)
        ));
        assert_eq!(
            parse_creator_input("my-red-id").unwrap(),
            CreatorInput::Search("my-red-id".into())
        );
        assert!(matches!(
            parse_creator_input("https://xhslink.com/a/abc").unwrap(),
            CreatorInput::ShortLink(_)
        ));
    }

    #[test]
    fn rejects_lookalike_and_foreign_hosts() {
        for url in [
            "https://xiaohongshu.com.evil.example/user/profile/0123456789abcdef01234567",
            "https://example.com/user/profile/0123456789abcdef01234567",
        ] {
            assert_eq!(
                parse_creator_input(url).unwrap_err().code,
                XiaohongshuErrorCode::UntrustedHost
            );
        }
    }

    #[test]
    fn redirect_target_must_resolve_to_stable_identity() {
        assert_eq!(
            parse_redirect_target(
                "https://www.xiaohongshu.com/livestream/123?host_id=0123456789abcdef01234567"
            )
            .unwrap(),
            CreatorInput::StableUserId("0123456789abcdef01234567".into())
        );
        assert_eq!(
            parse_redirect_target("https://xhslink.com/a/again")
                .unwrap_err()
                .code,
            XiaohongshuErrorCode::RedirectFailed
        );
    }
}
