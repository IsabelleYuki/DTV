//! Xiaohongshu web request signing.
//!
//! Protocol constants and layout are derived from the MIT-licensed xhshow-js
//! project (https://github.com/renmu123/xhshow-js). No credential is logged or
//! included in an error value.

use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine;
use md5::{Digest, Md5};
use rand::Rng;
use serde_json::{json, Value};

use super::credential::ParsedCookie;

const STANDARD_BASE64: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const CUSTOM_BASE64: &str = "ZmserbBoHQtNP+wOcza/LpngG8yJq42KWYj0DSfdikx3VT16IlUAFM97hECvuRX5";
const X3_BASE64: &str = "MfgqrsbcyzPQRStuvC7mn501HIJBo2DEFTKdeNOwxWXYZap89+/A4UVLhijkl63G";
const HEX_KEY: &str = "71a302257793271ddd273bcee3e4b98d9d7935e1da33f5765e2ea8afb6dc77a51a499d23b67c20660025860cbf13d4540d92497f58686c574e508f46e1956344f39139bf4faf22a3eef120b79258145b2feb5193b6478669961298e79bedca646e1a693a926154a5a7a1bd1cf0dedb742f917a747a1e388b234f2277";
const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/142.0.0.0 Safari/537.36 Edg/142.0.0.0";

#[derive(Clone, Debug)]
pub struct SignedHeaders {
    pub x_s: String,
    pub x_t: String,
    pub x_s_common: String,
    pub b3_trace_id: String,
    pub xray_trace_id: String,
}

fn translated_base64(data: &[u8], alphabet: &str) -> String {
    base64::engine::general_purpose::STANDARD
        .encode(data)
        .chars()
        .map(|character| {
            STANDARD_BASE64
                .find(character)
                .and_then(|index| alphabet.as_bytes().get(index).copied())
                .map(char::from)
                .unwrap_or(character)
        })
        .collect()
}

fn timestamp_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn random_hex(length: usize) -> String {
    const HEX: &[u8] = b"abcdef0123456789";
    let mut rng = rand::thread_rng();
    (0..length)
        .map(|_| char::from(HEX[rng.gen_range(0..HEX.len())]))
        .collect()
}

fn quote_query(value: &str) -> String {
    url::form_urlencoded::byte_serialize(value.as_bytes())
        .collect::<String>()
        .replace("%2C", ",")
        .replace('+', "%20")
}

fn content_string(method: &str, uri: &str, payload: &Value) -> String {
    if method.eq_ignore_ascii_case("POST") {
        return format!(
            "{uri}{}",
            serde_json::to_string(payload).unwrap_or_else(|_| "{}".into())
        );
    }
    let Some(object) = payload.as_object() else {
        return uri.to_string();
    };
    let sorted = object
        .iter()
        .map(|(key, value)| {
            let value = match value {
                Value::String(value) => value.clone(),
                Value::Array(values) => values
                    .iter()
                    .map(|value| {
                        value
                            .as_str()
                            .map(str::to_string)
                            .unwrap_or_else(|| value.to_string())
                    })
                    .collect::<Vec<_>>()
                    .join(","),
                value => value.to_string(),
            };
            (key, format!("{key}={}", quote_query(&value)))
        })
        .collect::<BTreeMap<_, _>>()
        .into_values()
        .collect::<Vec<_>>();
    if sorted.is_empty() {
        uri.to_string()
    } else {
        format!("{uri}?{}", sorted.join("&"))
    }
}

fn push_le(target: &mut Vec<u8>, value: u64, length: usize) {
    target.extend((0..length).map(|shift| ((value >> (shift * 8)) & 0xff) as u8));
}

fn sign_xs(method: &str, uri: &str, a1: &str, payload: &Value, timestamp: u64) -> String {
    let content = content_string(method, uri, payload);
    let digest = Md5::digest(content.as_bytes());
    let mut rng = rand::thread_rng();
    let seed = rng.gen::<u32>();
    let seed_byte = seed as u8;
    let mut bytes = vec![119, 104, 96, 41];
    push_le(&mut bytes, seed as u64, 4);

    let mut fingerprint_a = timestamp.to_le_bytes();
    fingerprint_a[0] = fingerprint_a[1..5]
        .iter()
        .chain(&fingerprint_a[5..8])
        .fold(0u8, |sum, byte| sum.wrapping_add(*byte));
    bytes.extend(fingerprint_a.map(|byte| byte ^ 41));
    let offset = rng.gen_range(10u64..=50);
    bytes.extend(timestamp.saturating_sub(offset).to_le_bytes());
    push_le(&mut bytes, rng.gen_range(15u64..=50), 4);
    push_le(&mut bytes, rng.gen_range(900u64..=1200), 4);
    push_le(&mut bytes, content.len() as u64, 4);
    bytes.extend(digest[..8].iter().map(|byte| byte ^ seed_byte));
    bytes.push(52);
    let mut a1_bytes = [0u8; 52];
    for (target, source) in a1_bytes.iter_mut().zip(a1.as_bytes()) {
        *target = *source;
    }
    bytes.extend(a1_bytes);
    bytes.push(10);
    bytes.extend(b"xhs-pc-web");
    bytes.extend([1, 1, seed_byte ^ 115]);
    bytes.extend([
        249, 65, 103, 103, 201, 181, 131, 99, 94, 7, 68, 250, 132, 21,
    ]);

    let key = hex::decode(HEX_KEY).expect("valid x-s xor key");
    for (index, byte) in bytes.iter_mut().enumerate().take(key.len()) {
        *byte ^= key[index];
    }
    bytes.truncate(124);
    let x3 = format!("mns0301_{}", translated_base64(&bytes, X3_BASE64));
    let wrapper = serde_json::to_vec(&json!({
        "x0": "4.2.6",
        "x1": "xhs-pc-web",
        "x2": "Windows",
        "x3": x3,
        "x4": ""
    }))
    .expect("serializable signature wrapper");
    format!("XYS_{}", translated_base64(&wrapper, CUSTOM_BASE64))
}

fn rc4(data: &[u8], key: &[u8]) -> Vec<u8> {
    let mut state = [0u8; 256];
    for (index, value) in state.iter_mut().enumerate() {
        *value = index as u8;
    }
    let mut j = 0usize;
    for index in 0..256 {
        j = (j + state[index] as usize + key[index % key.len()] as usize) & 255;
        state.swap(index, j);
    }
    let (mut i, mut j) = (0usize, 0usize);
    data.iter()
        .map(|byte| {
            i = (i + 1) & 255;
            j = (j + state[i] as usize) & 255;
            state.swap(i, j);
            byte ^ state[(state[i] as usize + state[j] as usize) & 255]
        })
        .collect()
}

fn quote_ciphertext_like_xhshow(data: &[u8]) -> Vec<u8> {
    let mut encoded = String::new();
    for byte in data {
        let safe = byte.is_ascii_alphanumeric() || b".!*'()~_-".contains(byte);
        if safe {
            encoded.push(char::from(*byte));
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    let mut output = Vec::new();
    for part in encoded.split('%').skip(1) {
        if part.len() < 2 {
            continue;
        }
        if let Ok(byte) = u8::from_str_radix(&part[..2], 16) {
            output.push(byte);
        }
        output.extend_from_slice(&part.as_bytes()[2..]);
    }
    output
}

fn crc32_js_int(data: &[u8]) -> i32 {
    let mut crc = 0xffff_ffffu32;
    for byte in data {
        crc ^= *byte as u32;
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xedb8_8320 & 0u32.wrapping_sub(crc & 1));
        }
    }
    let computed = crc ^ 0xffff_ffff;
    (0xffff_ffff ^ computed ^ 0xedb8_8320) as i32
}

fn sign_xs_common(a1: &str, timestamp: u64) -> String {
    let fingerprint = json!({
        "x33": "0", "x34": "0", "x35": "0", "x36": rand::thread_rng().gen_range(1..=20).to_string(),
        "x37": "0|0|0|0|0|0|0|0|0|1|0|0|0|0|0|0|0|0|1|0|0|0|0|0",
        "x38": "0|0|1|0|1|0|0|0|0|0|1|0|1|0|1|0|0|0|0|0|0|0|0|0|0|0|0|0|0|0|0|0|0|0|0|0|0|0|0",
        "x39": 0, "x42": "3.4.4", "x43": "742cc32c", "x44": timestamp.to_string(),
        "x45": "__SEC_CAV__1-1-1-1-1|__SEC_WSA__|", "x46": "false", "x48": "", "x49": "{list:[],type:}",
        "x50": "", "x51": "", "x52": "", "x82": "_0x17a2|_0x1954"
    });
    let encrypted = rc4(
        serde_json::to_string(&fingerprint)
            .expect("serializable fingerprint")
            .as_bytes(),
        b"xhswebmplfbt",
    );
    let b1 = translated_base64(&quote_ciphertext_like_xhshow(&encrypted), CUSTOM_BASE64);
    let common = serde_json::to_vec(&json!({
        "s0": 5, "s1": "", "x0": "1", "x1": "4.2.6", "x2": "Windows", "x3": "xhs-pc-web",
        "x4": "4.86.0", "x5": a1, "x6": "", "x7": "", "x8": b1,
        "x9": crc32_js_int(b1.as_bytes()), "x10": 0, "x11": "normal"
    }))
    .expect("serializable x-s-common");
    translated_base64(&common, CUSTOM_BASE64)
}

pub fn signed_headers(
    method: &str,
    uri: &str,
    cookie: &ParsedCookie,
    payload: &Value,
) -> SignedHeaders {
    // Parsing requires web_session as well as a1. It is transported only in
    // the Cookie header; xhshow's current signature payload derives from a1.
    let _web_session_present = !cookie.web_session().is_empty();
    let timestamp = timestamp_millis();
    let sequence = rand::thread_rng().gen_range(0u64..=8_388_607);
    SignedHeaders {
        x_s: sign_xs(method, uri, cookie.a1(), payload, timestamp),
        x_t: timestamp.to_string(),
        x_s_common: sign_xs_common(cookie.a1(), timestamp),
        b3_trace_id: random_hex(16),
        xray_trace_id: format!("{:016x}{}", (timestamp << 23) | sequence, random_hex(16)),
    }
}

pub fn user_agent() -> &'static str {
    USER_AGENT
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signed_headers_have_expected_protocol_shapes_without_cookie_values() {
        let cookie = ParsedCookie::parse(
            "a1=abcdefghijklmnopqrstuvwxyz0123456789abcdefghijklmnop; web_session=secret-session",
        )
        .unwrap();
        let headers = signed_headers(
            "POST",
            "/api/sns/web/v1/search/usersearch",
            &cookie,
            &json!({"q":"test"}),
        );
        assert!(headers.x_s.starts_with("XYS_"));
        assert!(!headers.x_s.contains("secret-session"));
        assert!(!headers.x_s_common.contains("secret-session"));
        assert_eq!(headers.b3_trace_id.len(), 16);
        assert_eq!(headers.xray_trace_id.len(), 32);
    }

    #[test]
    fn query_content_is_sorted_and_comma_safe() {
        assert_eq!(
            content_string("GET", "/path", &json!({"z":"a,b", "a":"hello world"})),
            "/path?a=hello%20world&z=a,b"
        );
    }
}
