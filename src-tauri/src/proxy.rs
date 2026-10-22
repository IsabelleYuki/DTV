use actix_web::{dev::ServerHandle, web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use bytes::Bytes;
use futures_util::TryStreamExt;
use reqwest::Client;
// awc removed for now due to API differences; using reqwest streaming
use crate::flv_relay;
use crate::platforms::xiaohongshu::credential::CredentialVault;
use crate::StreamUrlStore;
use serde::Deserialize;
use std::io::ErrorKind;
use std::net::TcpStream;
use std::sync::Mutex as StdMutex;
use std::time::Duration;
use tauri::{AppHandle, State};
use tokio::sync::mpsc;

// Define a struct to hold the server handle in a Tauri managed state
#[derive(Default)]
pub struct ProxyServerHandle(pub StdMutex<Option<ServerHandle>>);

// Align with pure_live-master's Huya playback UA
const HUYA_HYSDK_UA: &str =
    "HYSDK(Windows,30000002)_APP(pc_exe&7080000&official)_SDK(trans&2.34.0.5795)";

async fn find_free_port() -> u16 {
    // Using a fixed port as requested by the user for easier debugging
    34719
}

#[derive(Deserialize)]
struct ImageQuery {
    url: String,
}

#[derive(Deserialize)]
struct HlsQuery {
    url: String,
}

fn safe_upstream_label(raw: &str) -> String {
    url::Url::parse(raw)
        .ok()
        .and_then(|url| {
            let host = url.host_str()?;
            Some(format!("{}://{}{}", url.scheme(), host, url.path()))
        })
        .unwrap_or_else(|| "invalid-upstream-url".to_string())
}

fn local_hls_url(raw: &str) -> String {
    format!("/hls?url={}", urlencoding::encode(raw))
}

fn rewrite_hls_uri_attributes(line: &str, base: &url::Url) -> String {
    let mut output = line.to_string();
    let mut search_from = 0;
    while let Some(relative_start) = output[search_from..].find("URI=\"") {
        let value_start = search_from + relative_start + 5;
        let Some(relative_end) = output[value_start..].find('"') else {
            break;
        };
        let value_end = value_start + relative_end;
        let raw = &output[value_start..value_end];
        let Ok(resolved) = base.join(raw) else {
            search_from = value_end + 1;
            continue;
        };
        let replacement = local_hls_url(resolved.as_str());
        output.replace_range(value_start..value_end, &replacement);
        search_from = value_start + replacement.len() + 1;
    }
    output
}

fn rewrite_hls_manifest(manifest: &str, base: &url::Url) -> String {
    manifest
        .lines()
        .map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                String::new()
            } else if trimmed.starts_with('#') {
                rewrite_hls_uri_attributes(line, base)
            } else {
                base.join(trimmed)
                    .map(|resolved| local_hls_url(resolved.as_str()))
                    .unwrap_or_else(|_| line.to_string())
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

async fn hls_proxy_handler(
    query: web::Query<HlsQuery>,
    client: web::Data<Client>,
    credential_vault: web::Data<CredentialVault>,
) -> impl Responder {
    let upstream_url = query.url.trim();
    let Some(headers) = flv_relay::xiaohongshu_relay_headers(upstream_url, None) else {
        return HttpResponse::BadRequest().body("Unsupported HLS upstream host.");
    };
    let secret = if flv_relay::xiaohongshu_cookie_required(upstream_url) {
        match credential_vault.load() {
            Ok(secret) => Some(secret),
            Err(_) => {
                return HttpResponse::Unauthorized()
                    .body("Xiaohongshu credential is required for this upstream host.")
            }
        }
    } else {
        None
    };
    let mut request = client
        .get(upstream_url)
        .header("User-Agent", DEFAULT_STREAM_UA)
        .header("Referer", headers.referer)
        .header("Origin", headers.origin);
    if let Some(secret) = secret.as_ref() {
        request = request.header("Cookie", secret.expose());
    }
    let response = match request.send().await {
        Ok(response) => response,
        Err(_) => return HttpResponse::BadGateway().body("Unable to connect to HLS upstream."),
    };
    let status = response.status();
    if !status.is_success() {
        let actix_status = actix_web::http::StatusCode::from_u16(status.as_u16())
            .unwrap_or(actix_web::http::StatusCode::BAD_GATEWAY);
        return HttpResponse::build(actix_status).body("HLS upstream rejected the request.");
    }
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("application/octet-stream")
        .to_string();
    let bytes = match response.bytes().await {
        Ok(bytes) => bytes,
        Err(_) => return HttpResponse::BadGateway().body("Unable to read HLS upstream response."),
    };
    let is_manifest = upstream_url.to_ascii_lowercase().contains(".m3u8")
        || content_type.to_ascii_lowercase().contains("mpegurl");
    if is_manifest {
        let Ok(base) = url::Url::parse(upstream_url) else {
            return HttpResponse::BadRequest().body("Invalid HLS manifest URL.");
        };
        let Ok(manifest) = std::str::from_utf8(&bytes) else {
            return HttpResponse::BadGateway().body("HLS manifest is not valid UTF-8.");
        };
        return HttpResponse::Ok()
            .content_type("application/vnd.apple.mpegurl")
            .insert_header(("Cache-Control", "no-store"))
            .body(rewrite_hls_manifest(manifest, &base));
    }
    HttpResponse::Ok()
        .content_type(content_type)
        .insert_header(("Cache-Control", "no-store"))
        .body(bytes)
}

async fn image_proxy_handler(
    query: web::Query<ImageQuery>,
    client: web::Data<Client>,
) -> impl Responder {
    let url = query.url.clone();
    if url.is_empty() {
        return HttpResponse::BadRequest().body("Missing url query parameter");
    }

    let mut req = client
        .get(&url)
        .header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
        )
        .header(
            "Accept",
            "image/avif,image/webp,image/apng,image/*;q=0.8,*/*;q=0.5",
        );

    // Set a Referer to bypass hotlink protections
    if url.contains("hdslb.com") || url.contains("bilibili.com") {
        req = req
            .header("Referer", "https://live.bilibili.com/")
            .header("Origin", "https://live.bilibili.com");
    } else if url.contains("huya.com") {
        req = req
            .header("Referer", "https://www.huya.com/")
            .header("Origin", "https://www.huya.com");
    } else if url.contains("douyin") || url.contains("douyinpic.com") {
        req = req.header("Referer", "https://www.douyin.com/");
    }

    match req.send().await {
        Ok(upstream_response) => {
            let content_type = upstream_response
                .headers()
                .get(reqwest::header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .unwrap_or("application/octet-stream")
                .to_string();

            // 为避免 Windows 下 chunked 传输的 Early-EOF，改为一次性读取 bytes 并返回
            if upstream_response.status().is_success() {
                match upstream_response.bytes().await {
                    Ok(bytes) => HttpResponse::Ok()
                        .content_type(content_type)
                        .insert_header(("Content-Length", bytes.len().to_string()))
                        // Allow the WebView to cache proxied images to avoid re-downloading covers/avatars
                        // when switching routes or scrolling lists.
                        .insert_header(("Cache-Control", "public, max-age=86400, immutable"))
                        .body(bytes),
                    Err(e) => {
                        eprintln!("[Rust/proxy.rs image] Failed to read bytes: {}", e);
                        HttpResponse::InternalServerError()
                            .body(format!("Failed to read image bytes: {}", e))
                    }
                }
            } else {
                let status_from_reqwest = upstream_response.status();
                let error_text = upstream_response
                    .text()
                    .await
                    .unwrap_or_else(|e| format!("Failed to read error body from upstream: {}", e));
                eprintln!(
                    "[Rust/proxy.rs image] Upstream request to {} failed with status: {}. Body: {}",
                    url, status_from_reqwest, error_text
                );
                let actix_status_code =
                    actix_web::http::StatusCode::from_u16(status_from_reqwest.as_u16())
                        .unwrap_or(actix_web::http::StatusCode::INTERNAL_SERVER_ERROR);

                HttpResponse::build(actix_status_code).body(format!(
                    "Error fetching IMAGE from upstream (reqwest): {}. Status: {}. Details: {}",
                    url, status_from_reqwest, error_text
                ))
            }
        }
        Err(e) => {
            eprintln!(
                "[Rust/proxy.rs image] Failed to send request to upstream {}: {}",
                url, e
            );
            HttpResponse::InternalServerError()
                .body(format!("Error connecting to upstream IMAGE {}: {}", url, e))
        }
    }
}

const DEFAULT_STREAM_UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

// Your actual proxy logic - this is a simplified placeholder
async fn flv_proxy_handler(
    _req: HttpRequest,
    stream_url_store: web::Data<StreamUrlStore>,
    client: web::Data<Client>,
    credential_vault: web::Data<CredentialVault>,
) -> impl Responder {
    let url = stream_url_store.url.lock().unwrap().clone();
    if url.is_empty() {
        return HttpResponse::NotFound().body("Stream URL is not set or empty.");
    }
    // 仅斗鱼会登记续流上下文；为空时 relay 退化为一次性直通，行为与改造前一致。
    let renew_ctx = stream_url_store.renew.lock().unwrap().clone();

    let upstream_label = safe_upstream_label(&url);
    println!(
        "[Rust/proxy.rs handler] Incoming FLV proxy request -> {} (renew: {})",
        upstream_label,
        if renew_ctx.is_some() { "on" } else { "off" }
    );

    // 首次连接同步完成，便于把上游的错误状态原样透传给前端。
    let xiaohongshu_secret = if flv_relay::xiaohongshu_cookie_required(&url) {
        match credential_vault.load() {
            Ok(secret) => Some(secret),
            Err(_) => {
                return HttpResponse::Unauthorized()
                    .body("Xiaohongshu credential is required for this upstream host.")
            }
        }
    } else {
        None
    };
    let xiaohongshu_cookie = xiaohongshu_secret.as_ref().map(|secret| secret.expose());
    let first_body = match flv_relay::open_stream(
        &client,
        &url,
        DEFAULT_STREAM_UA,
        HUYA_HYSDK_UA,
        xiaohongshu_cookie,
    )
    .await
    {
        Ok(body) => body,
        Err(flv_relay::OpenError::Status { status, .. }) => {
            eprintln!(
                "[Rust/proxy.rs handler] Upstream request to {} failed with status: {}.",
                upstream_label, status
            );
            let actix_status_code = actix_web::http::StatusCode::from_u16(status.as_u16())
                .unwrap_or(actix_web::http::StatusCode::INTERNAL_SERVER_ERROR);
            return HttpResponse::build(actix_status_code).body(format!(
                "Error fetching FLV stream from upstream. Status: {}.",
                status
            ));
        }
        Err(flv_relay::OpenError::Connect(_)) => {
            eprintln!(
                "[Rust/proxy.rs handler] Failed to connect to upstream {}.",
                upstream_label
            );
            return HttpResponse::BadGateway().body("Error connecting to upstream FLV stream.");
        }
    };

    // 每个 item 是一个上游字节块。容量不宜大，否则会在播放器之前多压一层缓冲、推高直播延迟。
    let (tx, rx) = mpsc::channel::<Result<Bytes, std::io::Error>>(32);
    tauri::async_runtime::spawn(flv_relay::run(
        tx,
        first_body,
        renew_ctx,
        client.get_ref().clone(),
        DEFAULT_STREAM_UA.to_string(),
        HUYA_HYSDK_UA.to_string(),
    ));

    let byte_stream = futures_util::stream::unfold(rx, |mut rx| async move {
        rx.recv().await.map(|item| (item, rx))
    })
    .map_err(|e| {
        eprintln!("[Rust/proxy.rs handler] Relay stream error: {}", e);
        actix_web::error::ErrorInternalServerError(format!("Upstream stream error: {}", e))
    });

    HttpResponse::Ok()
        .content_type("video/x-flv")
        .insert_header(("Connection", "keep-alive"))
        .insert_header(("Cache-Control", "no-store"))
        .insert_header(("Accept-Ranges", "bytes"))
        .streaming(byte_stream)
}

#[tauri::command]
pub async fn start_proxy(
    _app_handle: AppHandle,
    server_handle_state: State<'_, ProxyServerHandle>,
    stream_url_store: State<'_, StreamUrlStore>,
    credential_vault: State<'_, CredentialVault>,
) -> Result<String, String> {
    let port = find_free_port().await;
    let current_stream_url = stream_url_store.url.lock().unwrap().clone();

    if current_stream_url.is_empty() {
        return Err("Stream URL is not set in store. Cannot start proxy.".to_string());
    }

    // stream_url_data_for_actix can be created once and cloned, as StreamUrlStore is Arc based and Send + Sync
    let stream_url_data_for_actix = web::Data::new(stream_url_store.inner().clone());
    let credential_vault_data_for_actix = web::Data::new(credential_vault.inner().clone());
    // REMOVED: let awc_client_for_actix = web::Data::new(Client::default());

    // Ensure MutexGuard is dropped before .await
    let existing_handle_to_stop = { server_handle_state.0.lock().unwrap().take() };
    if let Some(existing_handle) = existing_handle_to_stop {
        existing_handle.stop(false).await;
    }

    let server = match HttpServer::new(move || {
        let app_data_stream_url = stream_url_data_for_actix.clone();
        let app_data_credential_vault = credential_vault_data_for_actix.clone();
        // Create reqwest::Client inside the closure for each worker thread (for images)
        let app_data_reqwest_client = web::Data::new(
            Client::builder()
                .no_proxy()
                .http1_only()
                .gzip(false)
                .brotli(false)
                .no_deflate()
                .pool_idle_timeout(None)
                .pool_max_idle_per_host(4)
                .tcp_keepalive(Duration::from_secs(60))
                .timeout(Duration::from_secs(7200))
                .build()
                .expect("failed to build client"),
        );
        App::new()
            .app_data(app_data_stream_url)
            .app_data(app_data_credential_vault)
            .app_data(app_data_reqwest_client)
            .wrap(actix_cors::Cors::permissive())
            .route("/live.flv", web::get().to(flv_proxy_handler))
            .route("/hls", web::get().to(hls_proxy_handler))
            .route("/image", web::get().to(image_proxy_handler))
    })
    .keep_alive(Duration::from_secs(120))
    .bind(("127.0.0.1", port))
    {
        Ok(srv) => srv,
        Err(e) => {
            let err_msg = format!(
                "[Rust/proxy.rs] Failed to bind server to port {}: {}",
                port, e
            );
            eprintln!("{}", err_msg);
            return Err(err_msg);
        }
    }
    .run();

    let server_handle_for_state = server.handle();
    *server_handle_state.0.lock().unwrap() = Some(server_handle_for_state);

    // Use tauri::async_runtime::spawn directly
    tauri::async_runtime::spawn(async move {
        if let Err(e) = server.await {
            eprintln!("[Rust/proxy.rs] Proxy server run error: {}", e);
        } else {
            println!("[Rust/proxy.rs] Proxy server on port {} shut down.", port);
        }
    });

    let proxy_url = format!("http://127.0.0.1:{}/live.flv", port);
    Ok(proxy_url)
}

#[tauri::command]
pub async fn start_static_proxy_server(
    _app_handle: AppHandle,
    stream_url_store: State<'_, StreamUrlStore>,
    credential_vault: State<'_, CredentialVault>,
) -> Result<String, String> {
    // Use a dedicated port for static image proxy to avoid interfering with FLV stream proxy
    let port: u16 = 34721;

    // If the server is already running, just return the base URL (idempotent behavior)
    if TcpStream::connect(("127.0.0.1", port)).is_ok() {
        return Ok(format!("http://127.0.0.1:{}", port));
    }

    let stream_url_data_for_actix = web::Data::new(stream_url_store.inner().clone());
    let credential_vault_data_for_actix = web::Data::new(credential_vault.inner().clone());

    let server = match HttpServer::new(move || {
        let app_data_stream_url = stream_url_data_for_actix.clone();
        let app_data_credential_vault = credential_vault_data_for_actix.clone();
        let app_data_reqwest_client = web::Data::new(
            Client::builder()
                .no_proxy()
                .http1_only()
                .gzip(false)
                .brotli(false)
                .no_deflate()
                .pool_idle_timeout(None)
                .pool_max_idle_per_host(4)
                .tcp_keepalive(Duration::from_secs(60))
                .timeout(Duration::from_secs(7200))
                .build()
                .expect("failed to build client"),
        );
        App::new()
            .app_data(app_data_stream_url)
            .app_data(app_data_credential_vault)
            .app_data(app_data_reqwest_client)
            .wrap(actix_cors::Cors::permissive())
            .route("/live.flv", web::get().to(flv_proxy_handler))
            .route("/hls", web::get().to(hls_proxy_handler))
            .route("/image", web::get().to(image_proxy_handler))
    })
    .keep_alive(Duration::from_secs(120))
    .bind(("127.0.0.1", port))
    {
        Ok(srv) => srv,
        Err(e) => {
            // If address already in use, assume server is running and return OK base URL
            if e.kind() == ErrorKind::AddrInUse {
                eprintln!(
                    "[Rust/proxy.rs] Port {} already in use; assuming static proxy running.",
                    port
                );
                return Ok(format!("http://127.0.0.1:{}", port));
            }
            let err_msg = format!(
                "[Rust/proxy.rs] Failed to bind server to port {}: {}",
                port, e
            );
            eprintln!("{}", err_msg);
            return Err(err_msg);
        }
    }
    .run();

    // Do NOT overwrite the main proxy server handle; run static proxy independently

    tauri::async_runtime::spawn(async move {
        if let Err(e) = server.await {
            eprintln!("[Rust/proxy.rs] Proxy server run error: {}", e);
        } else {
            println!("[Rust/proxy.rs] Proxy server on port {} shut down.", port);
        }
    });

    Ok(format!("http://127.0.0.1:{}", port))
}

#[tauri::command]
pub async fn stop_proxy(server_handle_state: State<'_, ProxyServerHandle>) -> Result<(), String> {
    // Ensure MutexGuard is dropped before .await
    let handle_to_stop = { server_handle_state.0.lock().unwrap().take() };

    if let Some(handle) = handle_to_stop {
        handle.stop(false).await; // Changed to non-graceful shutdown
        println!("[Rust/proxy.rs] stop_proxy: Initiated non-graceful shutdown.");
    } else {
        println!("[Rust/proxy.rs] stop_proxy command: No proxy server was running or handle already taken.");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_label_removes_query_tokens() {
        let label = safe_upstream_label(
            "https://live-source-play.xhscdn.com/live/a.flv?token=secret&expires=1",
        );
        assert_eq!(label, "https://live-source-play.xhscdn.com/live/a.flv");
        assert!(!label.contains("secret"));
    }

    #[test]
    fn hls_manifest_rewrites_relative_segments_and_key_uris() {
        let base = url::Url::parse(
            "https://live-source-play.xhscdn.com/live/master/index.m3u8?token=secret",
        )
        .unwrap();
        let manifest = "#EXTM3U\n#EXT-X-KEY:METHOD=AES-128,URI=\"keys/key.bin\"\nsegments/one.ts\n";
        let rewritten = rewrite_hls_manifest(manifest, &base);
        assert!(rewritten.contains(
            "/hls?url=https%3A%2F%2Flive-source-play.xhscdn.com%2Flive%2Fmaster%2Fkeys%2Fkey.bin"
        ));
        assert!(rewritten.contains("/hls?url=https%3A%2F%2Flive-source-play.xhscdn.com%2Flive%2Fmaster%2Fsegments%2Fone.ts"));
        assert!(!rewritten.contains("token%3Dsecret"));
    }
}
