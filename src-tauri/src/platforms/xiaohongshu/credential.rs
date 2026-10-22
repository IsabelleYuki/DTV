use std::fmt;
use std::sync::{Arc, Mutex};

use serde::Serialize;
use thiserror::Error;

const KEYRING_SERVICE: &str = "com.chenzeong.dtv.xiaohongshu";
const KEYRING_ACCOUNT: &str = "cookie";

#[derive(Clone)]
pub struct SecretCookie(String);

impl SecretCookie {
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for SecretCookie {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SecretCookie([REDACTED])")
    }
}

pub struct ParsedCookie {
    raw: SecretCookie,
    a1: String,
    web_session: String,
}

impl ParsedCookie {
    pub fn parse(raw: &str) -> Result<Self, CredentialError> {
        let trimmed = raw.trim();
        if trimmed.is_empty() || !trimmed.contains('=') {
            return Err(CredentialError::new(
                CredentialErrorCode::Malformed,
                "Cookie 格式无效，请粘贴完整请求 Cookie。",
            ));
        }

        let mut a1 = None;
        let mut web_session = None;
        for item in trimmed.split(';') {
            let Some((name, value)) = item.trim().split_once('=') else {
                continue;
            };
            let value = value.trim();
            match name.trim() {
                "a1" if !value.is_empty() => a1 = Some(value.to_string()),
                "web_session" if !value.is_empty() => web_session = Some(value.to_string()),
                _ => {}
            }
        }

        let a1 = a1.ok_or_else(|| {
            CredentialError::new(CredentialErrorCode::MissingA1, "Cookie 缺少 a1 字段。")
        })?;
        if a1.len() != 52 {
            return Err(CredentialError::new(
                CredentialErrorCode::InvalidA1,
                "Cookie 中的 a1 字段长度无效。",
            ));
        }
        let web_session = web_session.ok_or_else(|| {
            CredentialError::new(
                CredentialErrorCode::MissingWebSession,
                "Cookie 缺少 web_session 字段。",
            )
        })?;

        Ok(Self {
            raw: SecretCookie(trimmed.to_string()),
            a1,
            web_session,
        })
    }

    pub fn a1(&self) -> &str {
        &self.a1
    }

    pub fn web_session(&self) -> &str {
        &self.web_session
    }

    pub fn into_secret(self) -> SecretCookie {
        self.raw
    }
}

impl fmt::Debug for ParsedCookie {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ParsedCookie")
            .field("a1", &"[REDACTED]")
            .field("web_session", &"[REDACTED]")
            .finish()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CredentialErrorCode {
    Malformed,
    MissingA1,
    InvalidA1,
    MissingWebSession,
    NotConfigured,
    SecureStoreUnavailable,
    SecureStoreError,
}

#[derive(Debug, Error, Serialize)]
#[error("{message}")]
pub struct CredentialError {
    code: CredentialErrorCode,
    message: String,
}

impl CredentialError {
    fn new(code: CredentialErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub fn code(&self) -> CredentialErrorCode {
        self.code
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CredentialStorage {
    Absent,
    Session,
    Secure,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CredentialSaveMode {
    Session,
    Secure,
}

impl CredentialSaveMode {
    pub fn parse(value: &str) -> Result<Self, CredentialError> {
        match value.trim().to_ascii_lowercase().as_str() {
            "session" => Ok(Self::Session),
            "secure" => Ok(Self::Secure),
            _ => Err(CredentialError::new(
                CredentialErrorCode::Malformed,
                "凭据保存方式无效。",
            )),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct CredentialStatus {
    pub storage: CredentialStorage,
    pub valid: Option<bool>,
    pub message: Option<String>,
    pub error_code: Option<CredentialErrorCode>,
}

trait SecureCredentialStore: Send + Sync {
    fn load(&self) -> Result<Option<SecretCookie>, CredentialError>;
    fn save(&self, cookie: &SecretCookie) -> Result<(), CredentialError>;
    fn clear(&self) -> Result<(), CredentialError>;
}

struct NativeCredentialStore;

impl NativeCredentialStore {
    fn entry() -> Result<keyring::Entry, CredentialError> {
        keyring::Entry::new(KEYRING_SERVICE, KEYRING_ACCOUNT).map_err(|_| {
            CredentialError::new(
                CredentialErrorCode::SecureStoreUnavailable,
                "系统安全凭据存储不可用，可改用仅本次会话。",
            )
        })
    }
}

impl SecureCredentialStore for NativeCredentialStore {
    fn load(&self) -> Result<Option<SecretCookie>, CredentialError> {
        match Self::entry()?.get_password() {
            Ok(value) => Ok(Some(SecretCookie(value))),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(_) => Err(CredentialError::new(
                CredentialErrorCode::SecureStoreError,
                "无法读取系统安全凭据存储。",
            )),
        }
    }

    fn save(&self, cookie: &SecretCookie) -> Result<(), CredentialError> {
        Self::entry()?.set_password(cookie.expose()).map_err(|_| {
            CredentialError::new(
                CredentialErrorCode::SecureStoreError,
                "无法写入系统安全凭据存储。",
            )
        })
    }

    fn clear(&self) -> Result<(), CredentialError> {
        match Self::entry()?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(_) => Err(CredentialError::new(
                CredentialErrorCode::SecureStoreError,
                "无法清除系统安全凭据存储。",
            )),
        }
    }
}

#[derive(Clone)]
pub struct CredentialVault {
    session: Arc<Mutex<Option<SecretCookie>>>,
    secure: Option<Arc<dyn SecureCredentialStore>>,
}

impl Default for CredentialVault {
    fn default() -> Self {
        Self {
            session: Arc::new(Mutex::new(None)),
            secure: Some(Arc::new(NativeCredentialStore)),
        }
    }
}

impl CredentialVault {
    #[cfg(test)]
    fn new_session_only() -> Self {
        Self {
            session: Arc::new(Mutex::new(None)),
            secure: None,
        }
    }

    pub fn set_session(&self, raw: &str) -> Result<(), CredentialError> {
        let cookie = ParsedCookie::parse(raw)?.into_secret();
        *self.session.lock().map_err(|_| {
            CredentialError::new(
                CredentialErrorCode::SecureStoreError,
                "会话凭据状态不可用。",
            )
        })? = Some(cookie);
        Ok(())
    }

    pub fn set_secure(&self, raw: &str) -> Result<(), CredentialError> {
        let cookie = ParsedCookie::parse(raw)?.into_secret();
        let secure = self.secure.as_ref().ok_or_else(|| {
            CredentialError::new(
                CredentialErrorCode::SecureStoreUnavailable,
                "系统安全凭据存储不可用，可改用仅本次会话。",
            )
        })?;
        secure.save(&cookie)?;
        self.clear_session();
        Ok(())
    }

    pub fn load(&self) -> Result<SecretCookie, CredentialError> {
        if let Some(cookie) = self
            .session
            .lock()
            .map_err(|_| {
                CredentialError::new(
                    CredentialErrorCode::SecureStoreError,
                    "会话凭据状态不可用。",
                )
            })?
            .clone()
        {
            return Ok(cookie);
        }
        if let Some(secure) = &self.secure {
            if let Some(cookie) = secure.load()? {
                ParsedCookie::parse(cookie.expose())?;
                return Ok(cookie);
            }
        }
        Err(CredentialError::new(
            CredentialErrorCode::NotConfigured,
            "尚未配置小红书 Cookie。",
        ))
    }

    pub fn status(&self) -> CredentialStatus {
        let session_present = self
            .session
            .lock()
            .map(|value| value.is_some())
            .unwrap_or(false);
        if session_present {
            return CredentialStatus {
                storage: CredentialStorage::Session,
                valid: None,
                message: None,
                error_code: None,
            };
        }
        if let Some(secure) = &self.secure {
            return match secure.load() {
                Ok(Some(cookie)) => match ParsedCookie::parse(cookie.expose()) {
                    Ok(_) => CredentialStatus {
                        storage: CredentialStorage::Secure,
                        valid: None,
                        message: None,
                        error_code: None,
                    },
                    Err(error) => CredentialStatus {
                        storage: CredentialStorage::Secure,
                        valid: Some(false),
                        message: Some(error.to_string()),
                        error_code: Some(error.code()),
                    },
                },
                Ok(None) => CredentialStatus {
                    storage: CredentialStorage::Absent,
                    valid: None,
                    message: None,
                    error_code: None,
                },
                Err(error) => CredentialStatus {
                    storage: CredentialStorage::Absent,
                    valid: None,
                    message: Some(error.to_string()),
                    error_code: Some(error.code()),
                },
            };
        }
        CredentialStatus {
            storage: CredentialStorage::Absent,
            valid: None,
            message: None,
            error_code: None,
        }
    }

    pub fn clear_session(&self) {
        if let Ok(mut value) = self.session.lock() {
            *value = None;
        }
    }

    pub fn clear_all(&self) -> Result<(), CredentialError> {
        self.clear_session();
        if let Some(secure) = &self.secure {
            secure.clear()?;
        }
        Ok(())
    }
}

#[tauri::command]
pub fn xiaohongshu_credential_status(vault: tauri::State<'_, CredentialVault>) -> CredentialStatus {
    vault.status()
}

#[tauri::command]
pub fn xiaohongshu_set_cookie(
    cookie: String,
    storage: String,
    vault: tauri::State<'_, CredentialVault>,
) -> Result<CredentialStatus, CredentialError> {
    match CredentialSaveMode::parse(&storage)? {
        CredentialSaveMode::Session => vault.set_session(&cookie)?,
        CredentialSaveMode::Secure => vault.set_secure(&cookie)?,
    }
    Ok(vault.status())
}

#[tauri::command]
pub fn xiaohongshu_clear_cookie(
    vault: tauri::State<'_, CredentialVault>,
) -> Result<CredentialStatus, CredentialError> {
    vault.clear_all()?;
    Ok(vault.status())
}

#[tauri::command]
pub async fn xiaohongshu_test_cookie(
    cookie: Option<String>,
    vault: tauri::State<'_, CredentialVault>,
) -> Result<CredentialStatus, super::types::XiaohongshuError> {
    let (secret, storage) = if let Some(raw) = cookie.filter(|value| !value.trim().is_empty()) {
        let parsed = ParsedCookie::parse(&raw).map_err(|error| {
            super::types::XiaohongshuError::new(
                super::types::XiaohongshuErrorCode::AuthenticationExpired,
                error.to_string(),
            )
        })?;
        (parsed.into_secret(), CredentialStorage::Session)
    } else {
        let status = vault.status();
        let secret = vault.load().map_err(|error| {
            super::types::XiaohongshuError::new(
                super::types::XiaohongshuErrorCode::AuthenticationRequired,
                error.to_string(),
            )
        })?;
        (secret, status.storage)
    };
    super::client::test_credential(&secret).await?;
    Ok(CredentialStatus {
        storage,
        valid: Some(true),
        message: Some("连接正常。".to_string()),
        error_code: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct UnavailableSecureStore;

    impl SecureCredentialStore for UnavailableSecureStore {
        fn load(&self) -> Result<Option<SecretCookie>, CredentialError> {
            Err(CredentialError::new(
                CredentialErrorCode::SecureStoreUnavailable,
                "test store unavailable",
            ))
        }

        fn save(&self, _cookie: &SecretCookie) -> Result<(), CredentialError> {
            Err(CredentialError::new(
                CredentialErrorCode::SecureStoreUnavailable,
                "test store unavailable",
            ))
        }

        fn clear(&self) -> Result<(), CredentialError> {
            Err(CredentialError::new(
                CredentialErrorCode::SecureStoreUnavailable,
                "test store unavailable",
            ))
        }
    }

    #[test]
    fn parses_required_cookie_fields_without_exposing_other_values() {
        let parsed = ParsedCookie::parse("foo=ignored; a1=abcdefghijklmnopqrstuvwxyz0123456789abcdefghijklmnop; web_session=session-value")
            .expect("valid cookie");

        assert_eq!(
            parsed.a1(),
            "abcdefghijklmnopqrstuvwxyz0123456789abcdefghijklmnop"
        );
        assert_eq!(parsed.web_session(), "session-value");
        assert!(!format!("{parsed:?}").contains("session-value"));
    }

    #[test]
    fn rejects_missing_required_fields_with_specific_codes() {
        let missing_a1 = ParsedCookie::parse("web_session=session-value").unwrap_err();
        assert_eq!(missing_a1.code(), CredentialErrorCode::MissingA1);

        let missing_session =
            ParsedCookie::parse("a1=abcdefghijklmnopqrstuvwxyz0123456789abcdefghijklmnop")
                .unwrap_err();
        assert_eq!(
            missing_session.code(),
            CredentialErrorCode::MissingWebSession
        );
    }

    #[test]
    fn keeps_session_credentials_in_memory_and_redacts_debug_output() {
        let vault = CredentialVault::new_session_only();
        vault
            .set_session("a1=abcdefghijklmnopqrstuvwxyz0123456789abcdefghijklmnop; web_session=session-value")
            .expect("session credential saved");

        assert_eq!(vault.status().storage, CredentialStorage::Session);
        assert!(vault
            .load()
            .expect("credential available")
            .expose()
            .contains("web_session="));
        assert!(!format!("{:?}", vault.load().unwrap()).contains("session-value"));

        vault.clear_session();
        assert_eq!(vault.status().storage, CredentialStorage::Absent);
    }

    #[test]
    fn accepts_only_explicit_secure_or_session_save_modes() {
        assert_eq!(
            CredentialSaveMode::parse("secure").unwrap(),
            CredentialSaveMode::Secure
        );
        assert_eq!(
            CredentialSaveMode::parse("session").unwrap(),
            CredentialSaveMode::Session
        );
        assert_eq!(
            CredentialSaveMode::parse("localStorage")
                .unwrap_err()
                .code(),
            CredentialErrorCode::Malformed
        );
    }

    #[test]
    fn reports_secure_store_failures_without_silent_plaintext_fallback() {
        let vault = CredentialVault {
            session: Arc::new(Mutex::new(None)),
            secure: Some(Arc::new(UnavailableSecureStore)),
        };
        let cookie =
            "a1=abcdefghijklmnopqrstuvwxyz0123456789abcdefghijklmnop; web_session=session-value";

        let error = vault.set_secure(cookie).unwrap_err();
        assert_eq!(error.code(), CredentialErrorCode::SecureStoreUnavailable);
        assert_eq!(vault.status().storage, CredentialStorage::Absent);
        assert_eq!(
            vault.status().error_code,
            Some(CredentialErrorCode::SecureStoreUnavailable)
        );
        assert!(vault.load().is_err());
    }
}
