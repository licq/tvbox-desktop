use crate::models::Subscription;
use crate::services::tvbox::TvboxLiveRecord;
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
    let fetched = match fetch_effective_subscription_content(&subscription.name, &subscription.url).await {
        Ok(fetched) => fetched,
        Err(error) => {
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

            let expanded_lives = expand_tvbox_live_records(&parsed.lives).await;
            let mut parsed = parsed;
            parsed.lives = expanded_lives;
            log::info!("TVBox直播展开完成: {} channels", parsed.lives.len());

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

#[derive(Debug, Clone, PartialEq, Eq)]
struct ClipboardCandidate {
    label: String,
    url: String,
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

async fn fetch_effective_subscription_content(
    subscription_name: &str,
    url: &str,
) -> Result<FetchedContent, String> {
    let fetched = fetch_subscription_content(url)
        .await
        .map_err(|e| format!("网络请求失败: {}", e))?;
    if detect_upstream_blocked(&fetched).is_none() {
        return Ok(fetched);
    }

    if let Some(snapshot) = try_brand_snapshot_fallback(subscription_name, url).await {
        log::warn!(
            "订阅 {} 官方入口不可用，已回退到公开快照: {}",
            subscription_name,
            snapshot.final_url
        );
        return Ok(snapshot);
    }

    Err(detect_upstream_blocked(&fetched).unwrap_or_else(|| "上游源不可用".to_string()))
}

async fn try_brand_snapshot_fallback(
    subscription_name: &str,
    original_url: &str,
) -> Option<FetchedContent> {
    let haystack = format!("{} {}", subscription_name, original_url);
    if !haystack.contains("饭太硬") {
        return None;
    }

    for snapshot_url in known_brand_snapshot_urls(subscription_name, original_url) {
        match fetch_text_no_proxy(&snapshot_url).await {
            Ok(fetched) if looks_like_json(&fetched.body) => {
                return Some(augment_brand_snapshot_content(
                    subscription_name,
                    original_url,
                    &snapshot_url,
                    fetched,
                ));
            }
            Ok(_) => {}
            Err(error) => {
                log::warn!("饭太硬快照回退失败: {} ({})", snapshot_url, error);
            }
        }
    }

    None
}

fn known_brand_snapshot_urls(subscription_name: &str, original_url: &str) -> Vec<String> {
    let haystack = format!("{} {}", subscription_name, original_url);
    if haystack.contains("饭太硬") {
        vec!["https://cdn.jsdelivr.net/gh/qist/tvbox@master/0826.json".to_string()]
    } else {
        Vec::new()
    }
}

fn augment_brand_snapshot_content(
    subscription_name: &str,
    original_url: &str,
    snapshot_url: &str,
    mut fetched: FetchedContent,
) -> FetchedContent {
    let haystack = format!("{} {}", subscription_name, original_url);
    if haystack.contains("饭太硬") && snapshot_url.contains("qist/tvbox@master/0826.json") {
        if let Some(body) = inject_live_snapshot_entry(
            &fetched.body,
            "饭太硬快照直播",
            "https://cdn.jsdelivr.net/gh/qist/tvbox@master/tvboxtv.txt",
        ) {
            fetched.body = body;
        }
    }
    fetched
}

fn inject_live_snapshot_entry(content: &str, name: &str, url: &str) -> Option<String> {
    let mut root = serde_json::from_str::<serde_json::Value>(content).ok()?;
    let object = root.as_object_mut()?;
    let lives = object
        .entry("lives")
        .or_insert_with(|| serde_json::Value::Array(Vec::new()))
        .as_array_mut()?;

    let already_exists = lives.iter().any(|value| {
        value
            .as_object()
            .and_then(|item| item.get("url"))
            .and_then(|value| value.as_str())
            == Some(url)
    });
    if !already_exists {
        lives.push(serde_json::json!({
            "name": name,
            "type": 0,
            "url": url
        }));
    }

    serde_json::to_string(&root).ok()
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

async fn expand_tvbox_live_records(records: &[TvboxLiveRecord]) -> Vec<TvboxLiveRecord> {
    let mut expanded = Vec::new();

    for record in records {
        if should_expand_live_playlist(record) {
            match expand_live_playlist_record(record).await {
                Ok(items) if !items.is_empty() => {
                    expanded.extend(items);
                    continue;
                }
                Ok(_) => {}
                Err(error) => {
                    log::warn!("展开直播列表失败: {} ({})", record.url, error);
                }
            }
            continue;
        }

        expanded.push(record.clone());
    }

    expanded
}

fn should_expand_live_playlist(record: &TvboxLiveRecord) -> bool {
    if record.source_type != Some(0) {
        return false;
    }

    let lower = record.url.to_ascii_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://")
}

async fn expand_live_playlist_record(
    record: &TvboxLiveRecord,
) -> Result<Vec<TvboxLiveRecord>, reqwest::Error> {
    for candidate_url in candidate_live_playlist_urls(&record.url) {
        let fetched = fetch_text_no_proxy(&candidate_url).await?;
        let content = fetched.body.trim();
        let entries = if looks_like_m3u_playlist(content) {
            parse_m3u_live_playlist(content, record)
        } else {
            parse_txt_live_playlist(content, record)
        };
        if !entries.is_empty() {
            return Ok(entries);
        }
    }

    Ok(Vec::new())
}

fn looks_like_m3u_playlist(content: &str) -> bool {
    content.contains("#EXTM3U") || content.contains("#EXTINF:")
}

fn candidate_live_playlist_urls(url: &str) -> Vec<String> {
    let mut candidates = vec![url.to_string()];
    if let Some(embedded) = extract_embedded_http_url(url) {
        candidates.insert(0, embedded);
    }
    candidates.dedup();
    candidates
}

fn extract_embedded_http_url(url: &str) -> Option<String> {
    let http_index = url.find("http://");
    let https_index = url.find("https://");
    let first_index = match (http_index, https_index) {
        (Some(http), Some(https)) => Some(http.min(https)),
        (Some(http), None) => Some(http),
        (None, Some(https)) => Some(https),
        (None, None) => None,
    }?;

    let tail = &url[first_index + 1..];
    let second_http = tail.find("http://").map(|index| index + first_index + 1);
    let second_https = tail.find("https://").map(|index| index + first_index + 1);
    let embedded_index = match (second_http, second_https) {
        (Some(http), Some(https)) => Some(http.min(https)),
        (Some(http), None) => Some(http),
        (None, Some(https)) => Some(https),
        (None, None) => None,
    }?;

    Some(url[embedded_index..].to_string())
}

fn parse_txt_live_playlist(content: &str, record: &TvboxLiveRecord) -> Vec<TvboxLiveRecord> {
    let mut group_name = record.group_name.clone();
    let mut entries = Vec::new();

    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        if line.contains(",#genre#") {
            group_name = line.split_once(',').map(|(name, _)| name.trim().to_string());
            continue;
        }
        if line.starts_with('#') {
            continue;
        }

        let Some((name, url)) = line.split_once(',') else {
            continue;
        };
        let name = name.trim();
        let url = url.trim();
        if name.is_empty() || url.is_empty() {
            continue;
        }

        entries.push(TvboxLiveRecord {
            group_name: group_name.clone(),
            name: name.to_string(),
            url: url.to_string(),
            source_type: record.source_type,
            raw_json: record.raw_json.clone(),
        });
    }

    entries
}

fn parse_m3u_live_playlist(content: &str, record: &TvboxLiveRecord) -> Vec<TvboxLiveRecord> {
    let mut entries = Vec::new();
    let mut pending_name: Option<String> = None;
    let mut pending_group = record.group_name.clone();
    let group_regex = regex::Regex::new(r#"group-title="([^"]*)""#).unwrap();

    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with("#EXTINF:") {
            pending_name = line
                .rsplit_once(',')
                .map(|(_, name)| name.trim().to_string())
                .filter(|name| !name.is_empty());
            pending_group = group_regex
                .captures(line)
                .and_then(|capture| capture.get(1).map(|value| value.as_str().trim().to_string()))
                .filter(|value| !value.is_empty())
                .or_else(|| record.group_name.clone());
            continue;
        }
        if line.starts_with('#') {
            continue;
        }

        let Some(name) = pending_name.take() else {
            continue;
        };
        entries.push(TvboxLiveRecord {
            group_name: pending_group.clone(),
            name,
            url: line.to_string(),
            source_type: record.source_type,
            raw_json: record.raw_json.clone(),
        });
    }

    entries
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

    let clipboard_candidates = extract_clipboard_candidates(&fetched.body);
    let preferred_clipboard_urls = filter_preferred_clipboard_urls(original_url, &clipboard_candidates);
    if !preferred_clipboard_urls.is_empty() {
        urls.extend(preferred_clipboard_urls);
    } else {
        urls.extend(clipboard_candidates.into_iter().map(|candidate| candidate.url));
        urls.extend(extract_candidate_urls(&fetched.body));
    }

    urls.retain(|url| url != candidate_url && url != &fetched.final_url);
    urls.sort_by_key(|url| candidate_priority(url));
    urls.dedup();
    urls
}

async fn fetch_text_no_proxy(url: &str) -> Result<FetchedContent, reqwest::Error> {
    let client = reqwest::Client::builder()
        .no_proxy()
        .connect_timeout(std::time::Duration::from_secs(20))
        .timeout(std::time::Duration::from_secs(20))
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

fn extract_clipboard_candidates(content: &str) -> Vec<ClipboardCandidate> {
    let regex = regex::Regex::new(
        r#"data-clipboard-text="([^"]+)"[^>]*>\s*<span>([^<]+)</span>"#,
    )
    .unwrap();
    regex
        .captures_iter(content)
        .filter_map(|capture| {
            let url = capture.get(1).map(|value| html_escape_decode(value.as_str()))?;
            let label = capture
                .get(2)
                .map(|value| html_escape_decode(value.as_str().trim()))
                .unwrap_or_default();
            Some(ClipboardCandidate { label, url })
        })
        .collect()
}

fn filter_preferred_clipboard_urls(
    original_url: &str,
    candidates: &[ClipboardCandidate],
) -> Vec<String> {
    let brand_tokens = extract_brand_tokens(original_url);
    if brand_tokens.is_empty() {
        return Vec::new();
    }

    let mut preferred_urls = Vec::new();
    for candidate in candidates {
        let haystack = format!("{} {}", candidate.label, candidate.url);
        if brand_tokens.iter().any(|token| haystack.contains(token)) {
            preferred_urls.push(candidate.url.clone());
        }
    }

    preferred_urls.sort_by_key(|url| candidate_priority(url));
    preferred_urls.dedup();
    preferred_urls
}

fn extract_brand_tokens(original_url: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    for segment in original_url
        .split(|ch: char| matches!(ch, '/' | ':' | '.' | '?' | '&' | '=' | '-'))
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
    {
        if segment.chars().any(|ch| !ch.is_ascii()) {
            tokens.push(segment.to_string());
        }
    }
    tokens.sort();
    tokens.dedup();
    tokens
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

fn detect_upstream_blocked(fetched: &FetchedContent) -> Option<String> {
    let body = fetched.body.trim();
    let blocked = fetched.status == 423
        || body.contains("The Repository has been blocked")
        || body.contains("Not Found Project");
    if !blocked {
        return None;
    }

    Some(format!(
        "上游源不可用: 响应状态={} 最终地址={} 内容预览={}",
        fetched.status,
        fetched.final_url,
        body.chars().take(120).collect::<String>().replace('\n', " ")
    ))
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
        candidate_live_playlist_urls, detect_upstream_blocked, extract_candidate_urls,
        extract_clipboard_candidates, extract_embedded_http_url, extract_json_object_fragment,
        extract_brand_tokens, fetch_effective_subscription_content, filter_preferred_clipboard_urls,
        inject_live_snapshot_entry, is_image_bytes, looks_like_json, parse_m3u_live_playlist,
        parse_txt_live_playlist, TvboxLiveRecord, fetch_text_no_proxy, augment_brand_snapshot_content,
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
    fn extracts_clipboard_candidates_with_labels() {
        let html = r#"<div class="inner copy-btn" data-clipboard-text="http://www.饭太硬.net/tv"><span>饭太硬备用</span></div>"#;
        let candidates = extract_clipboard_candidates(html);
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].label, "饭太硬备用");
        assert_eq!(candidates[0].url, "http://www.饭太硬.net/tv");
    }

    #[test]
    fn extracts_brand_tokens_from_unicode_source_url() {
        let tokens = extract_brand_tokens("http://www.饭太硬.net/tv");
        assert_eq!(tokens, vec!["饭太硬"]);
    }

    #[test]
    fn prefers_matching_brand_clipboard_candidates() {
        let candidates = vec![
            super::ClipboardCandidate {
                label: "饭太硬".to_string(),
                url: "http://www.饭太硬.com/tv".to_string(),
            },
            super::ClipboardCandidate {
                label: "巧技".to_string(),
                url: "http://cdn.qiaoji8.com/tvbox.json".to_string(),
            },
            super::ClipboardCandidate {
                label: "饭太硬备用".to_string(),
                url: "https://gitee.com/xxoooo/fan/raw/master/in.bmp".to_string(),
            },
        ];

        let preferred = filter_preferred_clipboard_urls("http://www.饭太硬.net/tv", &candidates);
        assert_eq!(
            preferred,
            vec![
                "http://www.饭太硬.com/tv".to_string(),
                "https://gitee.com/xxoooo/fan/raw/master/in.bmp".to_string(),
            ]
        );
    }

    #[test]
    fn detects_image_magic_bytes() {
        assert!(is_image_bytes(&[0xff, 0xd8, 0xff, 0xe0]));
        assert!(is_image_bytes(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a]));
        assert!(!is_image_bytes(b"{\"sites\":[]}"));
    }

    #[test]
    fn detects_blocked_upstream_payloads() {
        let fetched = super::FetchedContent {
            body: "[session-1e40c19b] Route error: The Repository has been blocked.".to_string(),
            final_url: "https://gitee.com/xxoooo/fan/raw/master/in.bmp".to_string(),
            status: 423,
            content_type: Some("text/plain".to_string()),
            content_encoding: None,
            body_hex_preview: String::new(),
            is_image_payload: false,
        };

        let error = detect_upstream_blocked(&fetched).expect("blocked upstream should be detected");
        assert!(error.contains("上游源不可用"));
        assert!(error.contains("423"));
    }

    #[test]
    fn parses_txt_live_playlist_into_channels() {
        let record = TvboxLiveRecord {
            group_name: None,
            name: "直播".to_string(),
            url: "https://example.com/live.txt".to_string(),
            source_type: Some(0),
            raw_json: "{}".to_string(),
        };
        let content = "央视频道,#genre#\nCCTV1,https://a.example/cctv1.m3u8\nCCTV2,https://a.example/cctv2.m3u8\n";
        let entries = parse_txt_live_playlist(content, &record);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].group_name.as_deref(), Some("央视频道"));
        assert_eq!(entries[0].name, "CCTV1");
    }

    #[test]
    fn parses_m3u_live_playlist_into_channels() {
        let record = TvboxLiveRecord {
            group_name: Some("默认分组".to_string()),
            name: "直播".to_string(),
            url: "https://example.com/live.m3u".to_string(),
            source_type: Some(0),
            raw_json: "{}".to_string(),
        };
        let content = "#EXTM3U\n#EXTINF:-1 group-title=\"央视频道\",CCTV1\nhttps://a.example/cctv1.m3u8\n#EXTINF:-1,CCTV2\nhttps://a.example/cctv2.m3u8\n";
        let entries = parse_m3u_live_playlist(content, &record);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].group_name.as_deref(), Some("央视频道"));
        assert_eq!(entries[1].group_name.as_deref(), Some("默认分组"));
        assert_eq!(entries[1].name, "CCTV2");
    }

    #[test]
    fn extracts_embedded_live_playlist_url_from_proxy_wrapper() {
        let wrapped = "https://gh-proxy.net/https://raw.githubusercontent.com/fanmingming/live/refs/heads/main/tv/m3u/ipv6.m3u";
        assert_eq!(
            extract_embedded_http_url(wrapped).as_deref(),
            Some("https://raw.githubusercontent.com/fanmingming/live/refs/heads/main/tv/m3u/ipv6.m3u")
        );
        assert_eq!(
            candidate_live_playlist_urls(wrapped),
            vec![
                "https://raw.githubusercontent.com/fanmingming/live/refs/heads/main/tv/m3u/ipv6.m3u".to_string(),
                wrapped.to_string(),
            ]
        );
    }

    #[test]
    fn injects_fantaihard_live_snapshot_entry_once() {
        let input = r#"{"lives":[{"name":"live","type":0,"url":"https://example.com/live.m3u"}]}"#;
        let updated = inject_live_snapshot_entry(
            input,
            "饭太硬快照直播",
            "https://cdn.jsdelivr.net/gh/qist/tvbox@master/tvboxtv.txt",
        )
        .expect("snapshot entry should inject");
        let value: serde_json::Value = serde_json::from_str(&updated).expect("json should parse");
        let lives = value["lives"].as_array().expect("lives should be array");
        assert_eq!(lives.len(), 2);

        let updated_again = inject_live_snapshot_entry(
            &updated,
            "饭太硬快照直播",
            "https://cdn.jsdelivr.net/gh/qist/tvbox@master/tvboxtv.txt",
        )
        .expect("snapshot entry should stay injectible");
        let value: serde_json::Value =
            serde_json::from_str(&updated_again).expect("json should parse");
        assert_eq!(value["lives"].as_array().expect("lives").len(), 2);
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
        let fetched = fetch_effective_subscription_content("饭太硬", "http://www.饭太硬.net/tv")
            .await
            .expect("effective source fetch should succeed");
        println!(
            "final_url={} status={} preview={}",
            fetched.final_url,
            fetched.status,
            fetched.body.chars().take(200).collect::<String>().replace('\n', " ")
        );
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

    #[tokio::test]
    #[ignore = "requires live network and DNS access"]
    async fn expands_fantaihard_snapshot_to_more_than_1000_channels() {
        let fetched = fetch_text_no_proxy("https://cdn.jsdelivr.net/gh/qist/tvbox@master/0826.json")
            .await
            .expect("fantaihard snapshot fetch should succeed");
        let fetched = augment_brand_snapshot_content(
            "饭太硬",
            "https://cdn.jsdelivr.net/gh/qist/tvbox@master/0826.json",
            "https://cdn.jsdelivr.net/gh/qist/tvbox@master/0826.json",
            fetched,
        );
        let parsed = TvboxConfigParser::parse(&fetched.body).expect("TVBox config should parse");
        let expanded = super::expand_tvbox_live_records(&parsed.lives).await;
        println!(
            "final_url={} raw_lives={} expanded_channels={}",
            fetched.final_url,
            parsed.lives.len(),
            expanded.len()
        );
        assert!(
            expanded.len() >= 1000,
            "expected fantaihard-compatible source to expand to >=1000 channels, got {}",
            expanded.len()
        );
    }

    #[tokio::test]
    #[ignore = "requires live network and DNS access"]
    async fn parses_fantaihard_live_snapshot_file_to_more_than_1000_channels() {
        let fetched = fetch_text_no_proxy("https://cdn.jsdelivr.net/gh/qist/tvbox@master/tvboxtv.txt")
            .await
            .expect("fantaihard live snapshot file should fetch");
        let record = TvboxLiveRecord {
            group_name: None,
            name: "饭太硬快照直播".to_string(),
            url: "https://cdn.jsdelivr.net/gh/qist/tvbox@master/tvboxtv.txt".to_string(),
            source_type: Some(0),
            raw_json: "{}".to_string(),
        };
        let entries = parse_txt_live_playlist(&fetched.body, &record);
        println!("tvboxtv expanded_channels={}", entries.len());
        assert!(
            entries.len() >= 1000,
            "expected tvboxtv.txt to contain >=1000 channels, got {}",
            entries.len()
        );
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
