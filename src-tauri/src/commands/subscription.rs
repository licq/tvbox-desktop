use crate::models::Subscription;
use crate::services::{Parser, TvboxConfigParser};
use crate::AppState;
use encoding_rs::{GB18030, GBK};
use flate2::read::GzDecoder;
use std::collections::{HashSet, VecDeque};
use std::io::Read;
use tauri::State;

#[tauri::command]
pub async fn add_subscription(
    name: String,
    url: String,
    state: State<'_, AppState>,
) -> Result<Subscription, String> {
    let storage = state.storage.clone();
    tokio::task::spawn_blocking(move || {
        storage
            .add_subscription(&name, &url)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_subscriptions(state: State<'_, AppState>) -> Result<Vec<Subscription>, String> {
    let storage = state.storage.clone();
    tokio::task::spawn_blocking(move || storage.get_subscriptions().map_err(|e| e.to_string()))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn delete_subscription(id: i64, state: State<'_, AppState>) -> Result<(), String> {
    let storage = state.storage.clone();
    tokio::task::spawn_blocking(move || storage.delete_subscription(id).map_err(|e| e.to_string()))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn refresh_subscription(id: i64, state: State<'_, AppState>) -> Result<(), String> {
    let storage = state.storage.clone();

    // Get subscription info
    let subscription = {
        let storage = storage.clone();
        tokio::task::spawn_blocking(move || storage.get_subscription(id))
            .await
            .map_err(|e| e.to_string())?
            .map_err(|e| e.to_string())?
    };
    let fallback_kind = subscription.kind.clone();

    // Fetch content from URL
    log::info!(
        "开始获取订阅: {} url={}",
        subscription.name,
        subscription.url
    );
    let fetched = match fetch_subscription_content(&subscription.url).await {
        Ok(fetched) => fetched,
        Err(e) => {
            let error = format!("网络请求失败: {}", e);
            persist_refresh_failure(storage.clone(), id, &fallback_kind, &error).await?;
            return Err(error);
        }
    };
    let response_text = fetched.body.clone();
    log::info!("响应长度: {}", response_text.len());

    // Parse JSON
    log::info!(
        "订阅 {} 响应长度: {}",
        subscription.name,
        response_text.len()
    );

    let source_kind = Parser::detect_source_kind(&response_text);

    match source_kind {
        "tvbox_config" => {
            let parsed = match TvboxConfigParser::parse(&response_text) {
                Ok(parsed) => parsed,
                Err(e) => {
                    persist_refresh_failure(storage.clone(), id, &fallback_kind, &e).await?;
                    return Err(e);
                }
            };
            log::info!(
                "TVBox配置解析完成: {} sites, {} parses, {} lives",
                parsed.sites.len(),
                parsed.parses.len(),
                parsed.lives.len()
            );

            let refresh_storage = storage.clone();
            let refresh_result = tokio::task::spawn_blocking(move || {
                refresh_storage
                    .refresh_tvbox_subscription(id, &response_text, &parsed)
                    .map_err(|e| e.to_string())
            })
            .await
            .map_err(|e| e.to_string())?;

            if let Err(error) = refresh_result {
                persist_refresh_failure(storage.clone(), id, &fallback_kind, &error).await?;
                return Err(error);
            }
        }
        _ => {
            let parsed = match Parser::parse_subscription(&response_text) {
                Ok(parsed) => parsed,
                Err(e) => {
                    let error = enrich_simple_parse_error(&e, &fetched);
                    persist_refresh_failure(storage.clone(), id, &fallback_kind, &error).await?;
                    return Err(error);
                }
            };
            log::info!(
                "简单订阅解析完成: {} lives, {} vods",
                parsed.lives.as_ref().map_or(0, |l| l.len()),
                parsed.vods.as_ref().map_or(0, |v| v.len())
            );

            let lives_data: Vec<_> = parsed
                .lives
                .unwrap_or_default()
                .into_iter()
                .map(|live| (live.name, live.logo, live.url, live.category))
                .collect();

            let vods_data: Vec<_> = parsed
                .vods
                .unwrap_or_default()
                .into_iter()
                .map(|vod| {
                    (
                        vod.name,
                        vod.vtype.unwrap_or_default(),
                        vod.poster,
                        vod.description,
                        Parser::parse_episodes(vod.episodes),
                    )
                })
                .collect();

            log::info!(
                "提取简单源数据: {} lives, {} vods",
                lives_data.len(),
                vods_data.len()
            );

            let refresh_storage = storage.clone();
            let refresh_result = tokio::task::spawn_blocking(move || {
                refresh_storage
                    .refresh_subscription(id, lives_data, vods_data)
                    .map_err(|e| e.to_string())
            })
            .await
            .map_err(|e| e.to_string())?;

            if let Err(error) = refresh_result {
                persist_refresh_failure(storage.clone(), id, &fallback_kind, &error).await?;
                return Err(error);
            }
        }
    }

    Ok(())
}

#[derive(Clone)]
struct FetchedContent {
    body: String,
    final_url: String,
    status: u16,
    content_type: Option<String>,
    content_encoding: Option<String>,
    body_hex_preview: String,
    is_image_payload: bool,
}

async fn fetch_subscription_content(url: &str) -> Result<FetchedContent, reqwest::Error> {
    let mut queue = VecDeque::from([(url.to_string(), 0usize)]);
    let mut visited = HashSet::new();
    let mut fallback: Option<FetchedContent> = None;

    while let Some((candidate_url, depth)) = queue.pop_front() {
        if !visited.insert(candidate_url.clone()) {
            continue;
        }

        let fetched = match fetch_text_no_proxy(&candidate_url).await {
            Ok(fetched) => fetched,
            Err(error) => {
                if fallback.is_none() {
                    return Err(error);
                }
                log::warn!("尝试候选配置地址失败: {} ({})", candidate_url, error);
                continue;
            }
        };

        if looks_like_json(&fetched.body) {
            if candidate_url != url {
                log::info!("入口返回非 JSON，已自动跟进候选配置地址: {}", candidate_url);
            }
            return Ok(fetched);
        }

        if let Some(fragment) = extract_json_object_fragment(&fetched.body) {
            if candidate_url != url {
                log::info!("候选地址返回页面，已提取嵌入 JSON: {}", candidate_url);
            }
            return Ok(fetched_content_with_body(&fetched, fragment));
        }

        let next_urls = discover_candidate_urls(url, &candidate_url, &fetched, depth);
        if fallback.is_none() || fallback.as_ref().is_some_and(|content| content.is_image_payload) {
            fallback = Some(fetched);
        }

        for next_url in next_urls {
            if !visited.contains(&next_url) {
                queue.push_back((next_url, depth + 1));
            }
        }
    }

    Ok(fallback.expect("initial subscription fetch should produce fallback content"))
}

fn fetched_content_with_body(fetched: &FetchedContent, body: String) -> FetchedContent {
    FetchedContent {
        body,
        final_url: fetched.final_url.clone(),
        status: fetched.status,
        content_type: fetched.content_type.clone(),
        content_encoding: fetched.content_encoding.clone(),
        body_hex_preview: fetched.body_hex_preview.clone(),
        is_image_payload: false,
    }
}

fn discover_candidate_urls(
    original_url: &str,
    candidate_url: &str,
    fetched: &FetchedContent,
    depth: usize,
) -> Vec<String> {
    if depth >= 2 {
        return Vec::new();
    }

    let mut urls = Vec::new();
    if fetched.is_image_payload {
        log::info!("入口返回图片，尝试探测备选入口: {}", candidate_url);
        urls.extend(build_probe_urls(original_url, &fetched.final_url));
    }

    urls.extend(extract_clipboard_urls(&fetched.body));
    urls.extend(extract_candidate_urls(&fetched.body));

    urls.retain(|url| url != candidate_url && url != &fetched.final_url);
    urls.sort_by_key(|url| candidate_priority(url));
    urls.dedup();
    urls
}

async fn fetch_text_no_proxy(url: &str) -> Result<FetchedContent, reqwest::Error> {
    let client = reqwest::Client::builder()
        .no_proxy()
        .connect_timeout(std::time::Duration::from_secs(6))
        .timeout(std::time::Duration::from_secs(12))
        .build()?;
    let response = client
        .get(url)
        .header(
            reqwest::header::USER_AGENT,
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36",
        )
        .header(
            reqwest::header::ACCEPT,
            "text/html,application/json,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
        )
        .header(
            reqwest::header::ACCEPT_LANGUAGE,
            "zh-CN,zh;q=0.9,en;q=0.8",
        )
        .send()
        .await?;
    let final_url = response.url().to_string();
    let status = response.status().as_u16();
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.to_string());
    let content_encoding = response
        .headers()
        .get(reqwest::header::CONTENT_ENCODING)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.to_string());
    let bytes = response.bytes().await?;
    let body_hex_preview = bytes
        .iter()
        .take(32)
        .map(|byte| format!("{:02x}", byte))
        .collect::<Vec<String>>()
        .join("");
    let is_image_payload = is_image_bytes(&bytes);
    let body = decode_response_body(&bytes, content_type.as_deref(), content_encoding.as_deref());

    Ok(FetchedContent {
        body,
        final_url,
        status,
        content_type,
        content_encoding,
        body_hex_preview,
        is_image_payload,
    })
}

fn looks_like_json(content: &str) -> bool {
    let trimmed = content.trim_start();
    trimmed.starts_with('{') || trimmed.starts_with('[')
}

fn extract_candidate_urls(content: &str) -> Vec<String> {
    let normalized_content = content
        .replace("\\/", "/")
        .replace("\\u003a", ":")
        .replace("\\u002f", "/")
        .replace("\\x3a", ":")
        .replace("\\x2f", "/");

    let decoded_percent_content = decode_common_percent_encoding(&normalized_content);

    let trimmed = content.trim();
    let mut urls: Vec<String> = if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        vec![trimmed.to_string()]
    } else {
        Vec::new()
    };

    let mut extracted: Vec<String> = decoded_percent_content
        .split(|ch: char| {
            ch.is_whitespace()
                || matches!(ch, '"' | '\'' | '<' | '>' | '(' | ')' | '[' | ']' | '{' | '}')
        })
        .filter_map(|raw| {
            let cleaned = raw
                .trim_matches(|ch: char| matches!(ch, ',' | ';' | '"' | '\'' | ')' | ']'))
                .replace("&amp;", "&");
            if cleaned.starts_with("http://") || cleaned.starts_with("https://") {
                Some(cleaned)
            } else {
                None
            }
        })
        .collect();
    urls.append(&mut extracted);

    urls.sort_by_key(|url| {
        let lower = url.to_ascii_lowercase();
        let score = if lower.ends_with(".json") {
            0
        } else if lower.contains("json") || lower.contains("config") || lower.contains("tv") {
            1
        } else {
            2
        };
        (score, url.len())
    });
    urls.dedup();
    urls
}

fn extract_clipboard_urls(content: &str) -> Vec<String> {
    let regex = regex::Regex::new(r#"data-clipboard-text="([^"]+)""#).unwrap();
    regex
        .captures_iter(content)
        .filter_map(|capture| capture.get(1).map(|value| html_escape_decode(value.as_str())))
        .collect()
}

fn html_escape_decode(content: &str) -> String {
    content
        .replace("&amp;", "&")
        .replace("&#x2F;", "/")
        .replace("&#47;", "/")
        .replace("&quot;", "\"")
}

fn candidate_priority(url: &str) -> (usize, usize) {
    let lower = url.to_ascii_lowercase();
    let score = if lower.ends_with(".json") {
        0
    } else if lower.contains("raw.githubusercontent.com")
        || lower.contains("gh-proxy")
        || lower.contains("cdn.")
    {
        1
    } else if lower.ends_with("/tv") || lower.ends_with("/tv/") {
        2
    } else if lower.ends_with(".bmp") {
        3
    } else {
        4
    };
    (score, url.len())
}

fn extract_json_object_fragment(content: &str) -> Option<String> {
    let start = content.find('{')?;
    let end = content.rfind('}')?;
    if end <= start {
        return None;
    }

    let fragment = content[start..=end].trim();
    if looks_like_json(fragment)
        && fragment.contains(':')
        && serde_json::from_str::<serde_json::Value>(fragment).is_ok()
    {
        Some(fragment.to_string())
    } else {
        None
    }
}

fn enrich_simple_parse_error(base_error: &str, fetched: &FetchedContent) -> String {
    if !base_error.starts_with("JSON解析失败") {
        return base_error.to_string();
    }

    let preview: String = fetched
        .body
        .chars()
        .take(160)
        .collect::<String>()
        .replace('\n', " ")
        .replace('\r', " ");

    format!(
        "{} | 响应状态={} | 类型={} | 编码={} | 最终地址={} | 十六进制预览={} | 内容预览={}",
        base_error,
        fetched.status,
        fetched.content_type.as_deref().unwrap_or("unknown"),
        fetched.content_encoding.as_deref().unwrap_or("none"),
        fetched.final_url,
        fetched.body_hex_preview,
        preview
    )
}

fn decode_response_body(bytes: &[u8], content_type: Option<&str>, content_encoding: Option<&str>) -> String {
    let mut normalized = bytes.to_vec();

    if let Some(encoding) = content_encoding.map(|value| value.to_ascii_lowercase()) {
        if encoding.contains("gzip") {
            if let Some(decoded) = decode_gzip(bytes) {
                normalized = decoded;
            }
        } else if encoding.contains("zstd") {
            if let Ok(decoded) = zstd::stream::decode_all(bytes) {
                normalized = decoded;
            }
        }
    } else if bytes.starts_with(&[0x1f, 0x8b]) {
        if let Some(decoded) = decode_gzip(bytes) {
            normalized = decoded;
        }
    } else if bytes.starts_with(&[0x28, 0xb5, 0x2f, 0xfd]) {
        if let Ok(decoded) = zstd::stream::decode_all(bytes) {
            normalized = decoded;
        }
    }

    if let Ok(text) = String::from_utf8(normalized.clone()) {
        return text;
    }

    let charset = extract_charset(content_type);
    if let Some(charset) = charset {
        if charset.contains("gb18030") {
            let (decoded, _, _) = GB18030.decode(&normalized);
            return decoded.into_owned();
        }
        if charset.contains("gbk") || charset.contains("gb2312") {
            let (decoded, _, _) = GBK.decode(&normalized);
            return decoded.into_owned();
        }
    }

    let (decoded_gbk, _, _) = GBK.decode(&normalized);
    let gbk_text = decoded_gbk.into_owned();
    if gbk_text.contains("http://") || gbk_text.contains("https://") || gbk_text.contains("复制") {
        return gbk_text;
    }

    String::from_utf8_lossy(&normalized).into_owned()
}

fn decode_gzip(bytes: &[u8]) -> Option<Vec<u8>> {
    let mut decoder = GzDecoder::new(bytes);
    let mut decoded = Vec::new();
    match decoder.read_to_end(&mut decoded) {
        Ok(_) => Some(decoded),
        Err(_) => None,
    }
}

fn is_image_bytes(bytes: &[u8]) -> bool {
    bytes.starts_with(&[0xff, 0xd8, 0xff])
        || bytes.starts_with(&[0x89, b'P', b'N', b'G'])
        || bytes.starts_with(b"GIF87a")
        || bytes.starts_with(b"GIF89a")
        || bytes.starts_with(b"RIFF")
}

fn build_probe_urls(original_url: &str, final_url: &str) -> Vec<String> {
    let mut urls = vec![original_url.to_string(), final_url.to_string()];

    if let Ok(parsed) = reqwest::Url::parse(final_url) {
        if let Some(host) = parsed.host_str() {
            for scheme in ["https", "http"] {
                urls.push(format!("{}://{}/", scheme, host));
                urls.push(format!("{}://{}/tv", scheme, host));
                urls.push(format!("{}://{}/tv/", scheme, host));
                urls.push(format!("{}://{}/index.php/tv", scheme, host));
            }

            if let Some((name, _tld)) = host.rsplit_once('.') {
                for tld in ["com", "net", "top"] {
                    let alt_host = format!("{}.{}", name, tld);
                    for scheme in ["https", "http"] {
                        urls.push(format!("{}://{}/", scheme, alt_host));
                        urls.push(format!("{}://{}/tv", scheme, alt_host));
                        urls.push(format!("{}://{}/tv/", scheme, alt_host));
                    }
                }
            }
        }
    }

    urls.sort();
    urls.dedup();
    urls
}

fn extract_charset(content_type: Option<&str>) -> Option<String> {
    let content_type = content_type?;
    let lower = content_type.to_ascii_lowercase();
    let marker = "charset=";
    let index = lower.find(marker)?;
    Some(
        lower[index + marker.len()..]
            .split(';')
            .next()
            .unwrap_or_default()
            .trim()
            .to_string(),
    )
}

fn decode_common_percent_encoding(content: &str) -> String {
    let mut result = content.to_string();
    let replacements = [
        ("%3A", ":"),
        ("%3a", ":"),
        ("%2F", "/"),
        ("%2f", "/"),
        ("%5C", "\\"),
        ("%5c", "\\"),
        ("%3D", "="),
        ("%3d", "="),
    ];
    for (from, to) in replacements {
        result = result.replace(from, to);
    }
    result
}

#[cfg(test)]
mod tests {
    use crate::services::{Parser, TvboxConfigParser};

    use super::{
        build_probe_urls, candidate_priority, decode_common_percent_encoding,
        extract_candidate_urls, extract_clipboard_urls, extract_json_object_fragment,
        fetch_subscription_content, is_image_bytes, looks_like_json,
    };

    #[test]
    fn detects_json_like_payload() {
        assert!(looks_like_json("{\"a\":1}"));
        assert!(looks_like_json(" [1,2,3]"));
        assert!(!looks_like_json("<html>"));
    }

    #[test]
    fn extracts_and_prioritizes_candidate_urls() {
        let html = r#"
            <a href="https://example.com/page">page</a>
            <a href="https://example.com/config.json">json</a>
        "#;
        let urls = extract_candidate_urls(html);
        assert_eq!(urls.first().map(String::as_str), Some("https://example.com/config.json"));
    }

    #[test]
    fn extracts_escaped_urls() {
        let html = r#"var data = "https:\/\/example.com\/config.json";"#;
        let urls = extract_candidate_urls(html);
        assert_eq!(urls.first().map(String::as_str), Some("https://example.com/config.json"));
    }

    #[test]
    fn decodes_percent_escaped_http_tokens() {
        let encoded = "https%3A%2F%2Fexample.com%2Ftv";
        let decoded = decode_common_percent_encoding(encoded);
        assert_eq!(decoded, "https://example.com/tv");
    }

    #[test]
    fn extracts_embedded_json_fragment() {
        let html = r#"<html><script>window.cfg={"sites":[],"lives":[]};</script></html>"#;
        let fragment = extract_json_object_fragment(html).expect("fragment expected");
        assert!(fragment.contains("\"sites\""));
    }

    #[test]
    fn extracts_clipboard_urls() {
        let html = r#"<div data-clipboard-text="http://www.饭太硬.net/tv"></div>"#;
        let urls = extract_clipboard_urls(html);
        assert_eq!(urls, vec!["http://www.饭太硬.net/tv"]);
    }

    #[test]
    fn detects_image_magic_bytes() {
        assert!(is_image_bytes(&[0xff, 0xd8, 0xff, 0xe0]));
        assert!(is_image_bytes(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a]));
        assert!(!is_image_bytes(b"{\"sites\":[]}"));
    }

    #[test]
    fn builds_probe_urls_from_final_url() {
        let urls = build_probe_urls(
            "http://www.xn--sss604efuw.net/tv",
            "http://www.xn--sss604efuw.net/tv/",
        );
        assert!(urls.iter().any(|url| url == "https://www.xn--sss604efuw.net/"));
        assert!(urls.iter().any(|url| url == "https://www.xn--sss604efuw.com/"));
        assert!(urls.iter().any(|url| url == "http://www.xn--sss604efuw.net/tv"));
    }

    #[test]
    fn prioritizes_direct_json_candidates() {
        assert!(candidate_priority("http://a.com/config.json") < candidate_priority("http://a.com/tv"));
    }

    #[tokio::test]
    #[ignore = "requires live network and DNS access"]
    async fn analyzes_fantaihard_entry_url() {
        let fetched = fetch_subscription_content("http://www.饭太硬.net/tv")
            .await
            .expect("network fetch should succeed");
        assert!(
            looks_like_json(&fetched.body),
            "expected JSON-like payload, got preview: {}",
            fetched.body_hex_preview
        );

        match Parser::detect_source_kind(&fetched.body) {
            "tvbox_config" => {
                let parsed = TvboxConfigParser::parse(&fetched.body)
                    .expect("fetched TVBox config should parse");
                assert!(
                    !parsed.sites.is_empty() || !parsed.parses.is_empty() || !parsed.lives.is_empty(),
                    "parsed TVBox config should contain usable records"
                );
            }
            "simple_json" => {
                Parser::parse_subscription(&fetched.body)
                    .expect("fetched simple json subscription should parse");
            }
            other => panic!("unexpected source kind: {}", other),
        }
    }
}

async fn persist_refresh_failure(
    storage: crate::services::Storage,
    id: i64,
    kind: &str,
    error: &str,
) -> Result<(), String> {
    let kind = kind.to_string();
    let error = error.to_string();
    tokio::task::spawn_blocking(move || {
        storage
            .record_subscription_refresh_failure(id, &kind, &error)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn toggle_subscription(
    id: i64,
    enabled: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let storage = state.storage.clone();
    tokio::task::spawn_blocking(move || {
        storage
            .toggle_subscription(id, enabled)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
