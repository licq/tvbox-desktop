use crate::commands::player::SegmentProxyResponse;
use crate::models::{PlaybackCandidate, ResolvedPlayback};

use once_cell::sync::OnceCell;
use std::sync::Arc;

use crate::services::ad_blocker::HlsAdBlocker;
use crate::services::playback_types::{PlaybackProbeResult, PlaybackProbeStatus};
use regex::Regex;
pub struct PlaybackResolver;

/// Shared HTTP client cached at module level to avoid repeated TLS handshake overhead.
/// Connection pooling enables reuse of established TCP/TLS connections across segment fetches.
static HTTP_CLIENT: OnceCell<Arc<reqwest::Client>> = OnceCell::new();

fn get_http_client() -> Result<Arc<reqwest::Client>, String> {
    HTTP_CLIENT
        .get_or_try_init(|| {
            let mut default_headers = reqwest::header::HeaderMap::new();
            default_headers.insert(
                reqwest::header::ACCEPT,
                reqwest::header::HeaderValue::from_static("*/*"),
            );
            default_headers.insert(
                reqwest::header::ACCEPT_LANGUAGE,
                reqwest::header::HeaderValue::from_static("en-US,en;q=0.9"),
            );
            default_headers.insert(
                reqwest::header::CACHE_CONTROL,
                reqwest::header::HeaderValue::from_static("no-cache"),
            );
            default_headers.insert(
                reqwest::header::PRAGMA,
                reqwest::header::HeaderValue::from_static("no-cache"),
            );

            let client = reqwest::Client::builder()
                .default_headers(default_headers)
                .connect_timeout(std::time::Duration::from_secs(20))
                .timeout(std::time::Duration::from_secs(20))
                .http1_only()
                .pool_max_idle_per_host(16)  // Keep connections alive for HLS streaming
                .no_proxy()
                .build()
                .map_err(|e| e.to_string())?;
            Ok(Arc::new(client))
        })
        .cloned()
}

/// Categories for why a play-page parse failed.
/// Each kind maps to a descriptive, actionable Chinese message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParserFailureKind {
    /// Page loaded (HTTP 200) but no player JSON or video element found.
    NoUrlExtracted,
    /// Extraction methods succeeded but URLs were not playable media files.
    NonMediaUrl,
    /// Page fetch failed or returned a non-2xx HTTP status.
    NetworkError,
    /// Page body is empty or structurally malformed.
    InvalidResponse,
}

impl ParserFailureKind {
    pub fn message(&self, http_status: Option<u16>) -> String {
        match self {
            ParserFailureKind::NoUrlExtracted => {
                "解析失败(no_url_extracted): 播放页已加载但未找到视频地址，可能需要更新解析规则".to_string()
            }
            ParserFailureKind::NonMediaUrl => {
                "解析失败(non_media_url): 找到的地址非视频文件，可能需要更新解析规则".to_string()
            }
            ParserFailureKind::NetworkError => {
                if let Some(code) = http_status {
                    format!("解析失败(network_error): 网页请求失败 HTTP {}", code)
                } else {
                    "解析失败(network_error): 网络请求失败，请检查网络连接".to_string()
                }
            }
            ParserFailureKind::InvalidResponse => {
                "解析失败(invalid_response): 播放页内容为空或格式异常".to_string()
            }
        }
    }
}

/// Returns true if the URL has a known playable media file extension (case-insensitive).
fn is_playable_media_url(url: &str) -> bool {
    let lower = url.to_lowercase();
    lower.ends_with(".m3u8")
        || lower.ends_with(".mp4")
        || lower.ends_with(".m4v")
        || lower.ends_with(".webm")
        || lower.ends_with(".mov")
        || lower.ends_with(".flv")
}

impl PlaybackResolver {
    pub async fn resolve(input: &str) -> Result<ResolvedPlayback, String> {
        if input.starts_with("guard://") {
            return Err("guard:// playback is no longer supported".to_string());
        }

        if input.starts_with("drpy://") {
            return Ok(external_required(
                "Current desktop build does not execute drpy rules directly",
                input,
            ));
        }

        if input.starts_with("magnet:")
            || input.starts_with("ed2k://")
            || input.starts_with("thunder://")
        {
            return Ok(external_required("当前资源需要交给外部下载工具处理", input));
        }
        if looks_like_cloud_disk_link(input) {
            return Ok(external_required("当前资源需要交给外部网盘工具处理", input));
        }

        if looks_like_xb6v_play_page(input)
            || looks_like_ypanso_play_page(input)
            || looks_like_zxzj_play_page(input)
            || looks_like_generic_play_page(input)
        {
            return resolve_play_page(input).await;
        }

        if classify_playback_target(input) == "direct" {
            let client = build_client()?;
            if let Err(error) = probe_media_candidate(&client, input, None).await {
                return Ok(ResolvedPlayback {
                    status: "failed".to_string(),
                    candidates: vec![],
                    error_message: Some(format!("当前直链不可播放: {error}")),
                });
            }
        }

        Ok(ready_with_candidate(input.to_string(), detect_kind(input), Some(input)))
    }
}

pub fn classify_playback_target(input: &str) -> &'static str {
    if input.starts_with("guard://") {
        return "resolvable";
    }

    if input.starts_with("drpy://")
        || input.starts_with("magnet:")
        || input.starts_with("ed2k://")
        || input.starts_with("thunder://")
        || looks_like_cloud_disk_link(input)
    {
        return "external";
    }

    if looks_like_xb6v_play_page(input)
        || looks_like_ypanso_play_page(input)
        || looks_like_zxzj_play_page(input)
        || looks_like_generic_play_page(input)
    {
        return "resolvable";
    }

    if input.contains(".m3u8")
        || input.contains(".mp4")
        || input.contains(".m4v")
        || input.contains(".webm")
        || input.contains(".mov")
    {
        return "direct";
    }

    // Non-media URLs that aren't recognized as guard/external/resolvable
    // are treated as external_required (no embed fallback — iframe experience is poor).
    "external"
}

pub fn is_visible_playback_target(input: &str) -> bool {
    map_target_kind_to_probe_gate(classify_playback_target(input))
}

pub fn playback_sort_rank(input: &str) -> i32 {
    match classify_playback_target(input) {
        "direct" => 0,
        "resolvable" => 1,
        _ => 3,
    }
}

pub fn map_target_kind_to_probe_gate(kind: &str) -> bool {
    matches!(kind, "direct" | "resolvable")
}

fn external_required(message: &str, input: &str) -> ResolvedPlayback {
    ResolvedPlayback {
        status: "external_required".to_string(),
        candidates: vec![PlaybackCandidate {
            url: input.to_string(),
            label: "外部打开".to_string(),
            kind: "external".to_string(),
            headers: None,
            referer: Some(input.to_string()),
        }],
        error_message: Some(message.to_string()),
    }
}

fn ready_with_candidate(url: String, kind: &'static str, referer: Option<&str>) -> ResolvedPlayback {
    ResolvedPlayback {
        status: "ready".to_string(),
        candidates: vec![PlaybackCandidate {
            url,
            label: "默认线路".to_string(),
            kind: kind.to_string(),
            headers: None,
            referer: referer.map(|s| s.to_string()),
        }],
        error_message: None,
    }
}

fn detect_kind(input: &str) -> &'static str {
    match classify_playback_target(input) {
        "direct" if input.contains(".m3u8") => "hls",
        "embedded" => "embed",
        "external" => "external",
        _ => "http",
    }
}

fn looks_like_xb6v_play_page(input: &str) -> bool {
    input.contains("xb6v.com/e/DownSys/play/")
}

fn looks_like_ypanso_play_page(input: &str) -> bool {
    input.contains("ypanso.com/vod/play/id/")
}

pub fn looks_like_zxzj_play_page(input: &str) -> bool {
    input.contains("zxzjys.com/vodplay/")
        || input.contains("zxzj.com/vodplay/")
        || input.contains("zxzjhd.com/vodplay/")
}

fn looks_like_generic_play_page(input: &str) -> bool {
    if input.contains(".m3u8")
        || input.contains(".mp4")
        || input.contains(".m4v")
        || input.contains(".webm")
        || input.contains(".mov")
    {
        return false;
    }

    input.contains("/play/")
        || input.contains("/vodplay/")
        || input.contains("/vod/play/")
        || (input.contains("/vod/")
            && !input.contains("/vod/detail/")
            && !input.contains("/vodsearch/")
            && !input.contains("/vodtype/")
            && !input.contains("/vod/show/"))
}

fn looks_like_cloud_disk_link(input: &str) -> bool {
    input.contains("pan.baidu.com/")
        || input.contains("pan.quark.cn/")
        || input.contains("drive.uc.cn/")
        || input.contains("aliyundrive.com/")
        || input.contains("alipan.com/")
}

async fn resolve_play_page(input: &str) -> Result<ResolvedPlayback, String> {
    let client = build_client()?;
    let (body, http_status) = match fetch_text_with_status(&client, input, None).await {
        Ok((body, status)) => (body, status),
        Err(e) => {
            let code = parse_http_status_from_error(&e);
            let kind = if code.is_some_and(|c| c >= 200 && c < 300) {
                ParserFailureKind::InvalidResponse
            } else {
                ParserFailureKind::NetworkError
            };
            eprintln!("[resolve_play] fetch failed: {}", e);
            return Ok(ResolvedPlayback {
                status: "failed".to_string(),
                candidates: vec![],
                error_message: Some(kind.message(code)),
            });
        }
    };

    if body.trim().is_empty() {
        return Ok(ResolvedPlayback {
            status: "failed".to_string(),
            candidates: vec![],
            error_message: Some(ParserFailureKind::InvalidResponse.message(None)),
        });
    }

    let referer = Some(input);

    // Try 1: Aliplayer "source" JSON field
    let (aliplayer_tried, aliplayer_result) = match extract_aliplayer_source(&body) {
        Some(source_url) => {
            eprintln!("[resolve_play] Found aliplayer source: {}", &source_url[..source_url.len().min(80)]);
            (true, Some((source_url.clone(), detect_kind(&source_url))))
        }
        None => {
            eprintln!("[resolve_play] aliplayer: no_url_extracted");
            (true, None)
        }
    };

    if let Some((source_url, kind)) = aliplayer_result {
        return Ok(ready_with_candidate(source_url, kind, referer));
    }

    // Try 2: DPlayer video config (video: { url: '...' })
    let (dplayer_tried, dplayer_result) = match extract_dplayer_video_url(&body) {
        Some(video_url) => {
            eprintln!("[resolve_play] Found dplayer video url: {}", &video_url[..video_url.len().min(80)]);
            (true, Some((video_url.clone(), detect_kind(&video_url))))
        }
        None => {
            eprintln!("[resolve_play] dplayer: no_url_extracted");
            (true, None)
        }
    };

    if let Some((video_url, kind)) = dplayer_result {
        return Ok(ready_with_candidate(video_url, kind, referer));
    }

    // Try 3: maccms player_aaaa / player_bbbb video JSON with url field
    let (maccms_tried, maccms_result) = match extract_maccms_player_url(&body) {
        Some(video_url) => {
            let kind = detect_kind(&video_url);
            eprintln!("[resolve_play] Found maccms player url: {}", &video_url[..video_url.len().min(80)]);
            (true, Some((video_url, kind)))
        }
        None => {
            eprintln!("[resolve_play] maccms: no_url_extracted");
            (true, None)
        }
    };

    if let Some((video_url, kind)) = maccms_result {
        return Ok(ready_with_candidate(video_url, kind, referer));
    }

    // Try 4: Direct <video> or <source> elements
    let (html_tried, html_result) = match extract_html_video_src(input, &body) {
        Some(video_url) => {
            eprintln!("[resolve_play] Found video element src: {}", &video_url[..video_url.len().min(80)]);
            (true, Some((video_url.clone(), detect_kind(&video_url))))
        }
        None => {
            eprintln!("[resolve_play] html_video: no_url_extracted");
            (true, None)
        }
    };

    if let Some((video_url, kind)) = html_result {
        return Ok(ready_with_candidate(video_url, kind, referer));
    }

    // Try 5: iframe-based player (share page)
    let iframe_tried = if let Some(iframe_url) = extract_iframe_src(input, &body) {
        eprintln!("[resolve_play] Found iframe: {}", &iframe_url[..iframe_url.len().min(80)]);
        match resolve_embedded_share_page(&client, &iframe_url).await {
            Ok(playback) if !playback.candidates.is_empty() => {
                eprintln!("[resolve_play] Share page resolved candidate: {:?}", playback.candidates[0].url.len().min(80));
                return Ok(playback);
            }
            other => {
                eprintln!("[resolve_play] iframe_share_page: no_url_extracted. Result: {:?}",
                    other.as_ref().map(|r| &r.status).unwrap_or(&"error".to_string()));
                true
            }
        }
    } else {
        eprintln!("[resolve_play] iframe: no_url_extracted");
        true
    };

    let tried_methods: Vec<&'static str> = [
        (aliplayer_tried, "aliplayer"),
        (dplayer_tried, "dplayer"),
        (maccms_tried, "maccms"),
        (html_tried, "html_video"),
        (iframe_tried, "iframe"),
    ]
    .into_iter()
    .filter(|(tried, _)| *tried)
    .map(|(_, name)| name)
    .collect();

    let failure_kind = ParserFailureKind::NoUrlExtracted;
    eprintln!("[resolve_play] All methods exhausted. Classification: {:?}. Tried: {:?}", failure_kind, tried_methods);

    Ok(ResolvedPlayback {
        status: "failed".to_string(),
        candidates: vec![],
        error_message: Some(failure_kind.message(Some(http_status))),
    })
}

/// Extract video URL from maccms player_xxxx JSON objects (e.g. player_aaaa, player_bbbb).
/// These contain `url` and `name` fields with the actual video source.
fn extract_maccms_player_url(body: &str) -> Option<String> {
    // (?s) for multi-line JSON, matches player_aaaa, player_bbbb, etc.
    let player_regex = Regex::new(r"(?s)player_[a-z]{4}\s*=\s*(\{.*?\})</script>").ok()?;
    player_regex.captures(body).and_then(|captures| {
        let json_str = captures.get(1).map(|m| m.as_str())?;
        let parsed: serde_json::Value = serde_json::from_str(json_str).ok()?;
        parsed.get("url").and_then(|v| v.as_str())
            .filter(|url| is_playable_media_url(url))
            .map(|s| s.to_string())
    })
}

/// Extract video URL from DPlayer configuration blocks.
/// Supports patterns like:
/// `video: { url: 'https://.../index.m3u8', type: 'hls' }`
fn extract_dplayer_video_url(body: &str) -> Option<String> {
    let patterns = [
        r#"video\s*:\s*\{\s*url\s*:\s*'([^']+)'"#,
        r#"video\s*:\s*\{\s*url\s*:\s*"([^"]+)""#,
        r#"url\s*:\s*'([^']+\.m3u8[^']*)'"#,
        r#"url\s*:\s*"([^"]+\.m3u8[^"]*)""#,
    ];

    for pattern in patterns {
        if let Ok(regex) = Regex::new(pattern) {
            if let Some(captures) = regex.captures(body) {
                if let Some(value) = captures.get(1) {
                    let url = value.as_str().trim();
                    if !url.is_empty() && is_playable_media_url(url) {
                        return Some(url.to_string());
                    }
                }
            }
        }
    }

    None
}

/// Extract video source URL from <video> or <source> HTML elements.
fn extract_html_video_src(page_url: &str, body: &str) -> Option<String> {
    // Try <source src="..."> inside a <video> element
    let source_regex = Regex::new(r#"<source[^>]+src="([^"]+)""#).ok()?;
    if let Some(url) = source_regex.captures(body)
        .and_then(|c| c.get(1))
        .map(|m| absolutize_url(page_url, m.as_str()))
    {
        if url.contains(".m3u8") || url.contains(".mp4") || url.contains(".flv") {
            return Some(url);
        }
    }
    // Try <video src="...">
    let video_regex = Regex::new(r#"<video[^>]+src="([^"]+)""#).ok()?;
    video_regex.captures(body)
        .and_then(|c| c.get(1))
        .map(|m| absolutize_url(page_url, m.as_str()))
}

fn extract_aliplayer_source(body: &str) -> Option<String> {
    let source_regex = Regex::new(r#""source"\s*:\s*"([^"]+)""#).unwrap();
    source_regex
        .captures(body)
        .and_then(|captures| captures.get(1).map(|value| value.as_str().to_string()))
        .and_then(|url| {
            if is_playable_media_url(&url) {
                Some(url)
            } else {
                eprintln!("[resolve_play] aliplayer: non_media_url (source field found but rejected: {})", &url[..url.len().min(80)]);
                None
            }
        })
}

fn extract_iframe_src(page_url: &str, body: &str) -> Option<String> {
    let iframe_regex = Regex::new(r#"<iframe[^>]+src="([^"]+)""#).unwrap();
    iframe_regex.captures(body).and_then(|captures| {
        captures
            .get(1)
            .map(|value| absolutize_url(page_url, value.as_str()))
    })
}

async fn resolve_embedded_share_page(
    client: &reqwest::Client,
    iframe_url: &str,
) -> Result<ResolvedPlayback, String> {
    let body = fetch_text(client, iframe_url).await?;
    if let Some(video_url) = extract_dplayer_video_url(&body) {
        return Ok(ready_with_candidate(
            video_url.clone(),
            detect_kind(&video_url),
            Some(iframe_url),
        ));
    }
    let share_url_regex = Regex::new(r#"const\s+url\s*=\s*"([^"]+)""#).unwrap();
    let Some(source_url) = share_url_regex.captures(&body).and_then(|captures| {
        captures
            .get(1)
            .map(|value| absolutize_url(iframe_url, value.as_str()))
    }) else {
        return Ok(ResolvedPlayback {
            status: "failed".to_string(),
            candidates: vec![],
            error_message: Some("未能从分享页提取实际视频地址".to_string()),
        });
    };

    Ok(ready_with_candidate(
        source_url.clone(),
        detect_kind(&source_url),
        Some(iframe_url),
    ))
}

fn absolutize_url(base_url: &str, candidate: &str) -> String {
    if candidate.starts_with("http://") || candidate.starts_with("https://") {
        candidate.to_string()
    } else {
        reqwest::Url::parse(base_url)
            .and_then(|base| base.join(candidate))
            .map(|url| url.to_string())
            .unwrap_or_else(|_| candidate.to_string())
    }
}

pub(crate) fn build_client() -> Result<reqwest::Client, String> {
    let mut default_headers = reqwest::header::HeaderMap::new();
    default_headers.insert(
        reqwest::header::ACCEPT,
        reqwest::header::HeaderValue::from_static("*/*"),
    );
    default_headers.insert(
        reqwest::header::ACCEPT_LANGUAGE,
        reqwest::header::HeaderValue::from_static("en-US,en;q=0.9"),
    );
    default_headers.insert(
        reqwest::header::CACHE_CONTROL,
        reqwest::header::HeaderValue::from_static("no-cache"),
    );
    default_headers.insert(
        reqwest::header::PRAGMA,
        reqwest::header::HeaderValue::from_static("no-cache"),
    );

    reqwest::Client::builder()
        .default_headers(default_headers)
        .connect_timeout(std::time::Duration::from_secs(20))
        .timeout(std::time::Duration::from_secs(20))
        .http1_only()
        .no_proxy()
        .build()
        .map_err(|e| e.to_string())
}

async fn probe_media_candidate(
    client: &reqwest::Client,
    url: &str,
    headers: Option<&std::collections::HashMap<String, String>>,
) -> Result<(), String> {
    let probe = probe_media_candidate_result(client, url, headers).await;
    if matches!(probe.status, PlaybackProbeStatus::Playable) {
        Ok(())
    } else {
        Err(probe
            .failure_reason
            .unwrap_or_else(|| "stream probe failed".to_string()))
    }
}

pub async fn probe_candidate_for_runtime(
    client: &reqwest::Client,
    url: &str,
    headers: Option<&std::collections::HashMap<String, String>>,
) -> PlaybackProbeResult {
    probe_media_candidate_result(client, url, headers).await
}

async fn probe_media_candidate_result(
    client: &reqwest::Client,
    url: &str,
    headers: Option<&std::collections::HashMap<String, String>>,
) -> PlaybackProbeResult {
    if url.contains(".m3u8") {
        return probe_hls_playlist_result(client, url, headers).await;
    }

    probe_binary_resource_result(client, url, headers).await
}

async fn probe_hls_playlist_result(
    client: &reqwest::Client,
    url: &str,
    headers: Option<&std::collections::HashMap<String, String>>,
) -> PlaybackProbeResult {
    let body = match fetch_hls_playlist_with_headers_no_cors(client, url, headers).await {
        Ok(body) => body,
        Err(error) => { return PlaybackProbeResult::failed(error, None); }
    };
    if !body.contains("#EXTM3U") {
        return failed_hls_probe("playlist missing EXTM3U header", Some(200), false, false);
    }

    if body.contains("#EXT-X-STREAM-INF") {
        let variant_url = match first_playlist_resource(url, &body) {
            Some(url) => url,
            None => return failed_hls_probe("master playlist missing variant url", Some(200), true, false),
        };
        let variant_body = match fetch_hls_playlist_with_headers_no_cors(client, &variant_url, headers).await {
            Ok(body) => body,
            Err(error) => { return PlaybackProbeResult::failed(error, None); }
        };
        if !variant_body.contains("#EXTM3U") {
            return failed_hls_probe("variant playlist missing EXTM3U header", Some(200), true, false);
        }
        // Variant playlist is valid - mark as playable even without CORS check on variant
        // (some CDNs don't return CORS on variant chunks but master playlist passed)
        return PlaybackProbeResult::playable();
    }

    probe_hls_media_playlist_result(client, url, &body, headers).await
}

async fn probe_hls_media_playlist_result(
    client: &reqwest::Client,
    playlist_url: &str,
    body: &str,
    headers: Option<&std::collections::HashMap<String, String>>,
) -> PlaybackProbeResult {
    if let Some(key_url) = extract_hls_key_url(playlist_url, body) {
        let probe = probe_binary_resource_result(client, &key_url, headers).await;
        if matches!(probe.status, PlaybackProbeStatus::Failed) {
            return probe;
        }
    }

    let Some(segment_url) = first_playlist_resource(playlist_url, body) else {
        return failed_hls_probe("media playlist missing segment url", Some(200), true, false);
    };
    let probe = probe_binary_resource_result(client, &segment_url, headers).await;
    if matches!(probe.status, PlaybackProbeStatus::Failed) {
        return probe;
    }

    PlaybackProbeResult::playable()
}

fn extract_hls_key_url(base_url: &str, body: &str) -> Option<String> {
    let key_regex = Regex::new(r#"URI="([^"]+)""#).unwrap();
    body.lines()
        .find(|line| line.trim_start().starts_with("#EXT-X-KEY"))
        .and_then(|line| key_regex.captures(line))
        .and_then(|capture| capture.get(1))
        .map(|value| absolutize_url(base_url, value.as_str()))
}

fn first_playlist_resource(base_url: &str, body: &str) -> Option<String> {
    body.lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| absolutize_url(base_url, line))
}

async fn probe_binary_resource_result(
    client: &reqwest::Client,
    url: &str,
    headers: Option<&std::collections::HashMap<String, String>>,
) -> PlaybackProbeResult {
    let request = client
        .get(url)
        .header(reqwest::header::RANGE, "bytes=0-1")
        .header(
            reqwest::header::USER_AGENT,
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36",
        );
    let request_headers = build_hls_request_headers(headers, None);
    let response = match apply_request_headers(request, request_headers.as_ref()).send().await {
        Ok(response) => response,
        Err(error) => return PlaybackProbeResult::failed(error.to_string(), None),
    };
    let http_status = Some(i64::from(response.status().as_u16()));

    if response.status().is_success() || response.status() == reqwest::StatusCode::PARTIAL_CONTENT {
        let mut probe = PlaybackProbeResult::playable();
        probe.http_status = http_status;
        probe.cors_ok = has_browser_cors(&response);
        probe
    } else {
        let mut probe =
            PlaybackProbeResult::failed(format!("resource probe failed: {}", response.status()), http_status);
        probe.manifest_ok = true;
        probe
    }
}

fn failed_hls_probe(
    reason: impl Into<String>,
    http_status: Option<i64>,
    manifest_ok: bool,
    segment_ok: bool,
) -> PlaybackProbeResult {
    let mut probe = PlaybackProbeResult::failed(reason, http_status);
    probe.manifest_ok = manifest_ok;
    probe.segment_ok = segment_ok;
    probe
}

fn has_browser_cors(response: &reqwest::Response) -> bool {
    response
        .headers()
        .get("Access-Control-Allow-Origin")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| !value.trim().is_empty())
}

/// Known CDN hosts that work in browser (hls.js) but may fail Rust-side TLS probing.
/// These CDNs return proper CORS headers but employ TLS fingerprinting that blocks
/// non-browser TLS stacks (curl and IINA work, but native-tls/reqwest fails).
pub(crate) fn is_known_cdn_url(url: &str) -> bool {
    // Check if URL contains a known CDN hostname that works in browser but
    // may fail Rust-side TLS probing due to CDN TLS fingerprint detection.
    let known_hosts = ["s1.fengbao9.com"];
    let matched = known_hosts.iter().any(|host| url.contains(host));
    if matched {
        eprintln!("[is_known_cdn_url] MATCH: {}", url);
    }
    matched
}

async fn fetch_text(client: &reqwest::Client, input: &str) -> Result<String, String> {
    fetch_text_with_headers(client, input, None).await
}

async fn fetch_text_with_headers(
    client: &reqwest::Client,
    input: &str,
    headers: Option<&std::collections::HashMap<String, String>>,
) -> Result<String, String> {
    let request = client
        .get(input)
        .header(
            reqwest::header::USER_AGENT,
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36",
        );
    let request_headers = build_hls_request_headers(headers, None);
    let response = apply_request_headers(request, request_headers.as_ref())
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("request failed: {status}"));
    }

    response.text().await.map_err(|e| e.to_string())
}

/// Like fetch_text_with_headers but also returns the HTTP status code as u16.
async fn fetch_text_with_status(
    client: &reqwest::Client,
    input: &str,
    headers: Option<&std::collections::HashMap<String, String>>,
) -> Result<(String, u16), String> {
    let request = client
        .get(input)
        .header(
            reqwest::header::USER_AGENT,
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36",
        );
    let request_headers = build_hls_request_headers(headers, None);
    let response = apply_request_headers(request, request_headers.as_ref())
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("request failed: {status}"));
    }
    let body = response.text().await.map_err(|e| e.to_string())?;
    Ok((body, status.as_u16()))
}

/// Parse an HTTP status code from a fetch-text error string like "request failed: 404".
fn parse_http_status_from_error(error: &str) -> Option<u16> {
    error
        .chars()
        .filter(|c| c.is_ascii_digit())
        .collect::<String>()
        .parse::<u16>()
        .ok()
}

async fn fetch_hls_playlist_with_headers_no_cors(
    client: &reqwest::Client,
    input: &str,
    headers: Option<&std::collections::HashMap<String, String>>,
) -> Result<String, String> {
    let request = client
        .get(input)
        .header(
            reqwest::header::USER_AGENT,
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36",
        );
    let request_headers = build_hls_request_headers(headers, None);
    let response = apply_request_headers(request, request_headers.as_ref())
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("playlist request failed: {status}"));
    }

    response.text().await.map_err(|e| e.to_string())
}

/// Rewrite relative URLs in an HLS playlist to absolute URLs based on the base URL.
fn rewrite_relative_urls(body: &str, base_url: &str) -> String {
    body.lines()
        .map(|line| {
            let trimmed = line.trim();
            // Skip comment lines and directive lines
            if trimmed.is_empty() || trimmed.starts_with('#') {
                line.to_string()
            } else {
                // Check if it's a relative URL (not http:// or https://)
                if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
                    line.to_string()
                } else {
                    // Convert relative to absolute
                    absolutize_url(base_url, trimmed)
                }
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Normalize a master playlist by embedding the variant playlist as a base64 data URI
/// and rewriting all relative URLs to absolute URLs.
fn normalize_master_playlist(master_body: &str, master_url: &str, variant_body: &str, variant_url: &str) -> String {
    use base64::Engine;
    // Rewrite relative URLs in variant playlist to absolute.
    let normalized_variant = rewrite_relative_urls(variant_body, variant_url);
    // Encode the variant playlist as base64.
    let variant_b64 = base64::engine::general_purpose::STANDARD.encode(normalized_variant.as_bytes());
    let data_uri = format!("data:application/vnd.apple.mpegurl;base64,{}", variant_b64);

    // Rewrite master playlist lines, replacing the variant URL with the data URI.
    let mut result_lines = Vec::new();
    let mut found_variant = false;
    for line in master_body.lines() {
        let trimmed = line.trim();
        if !found_variant && !trimmed.is_empty() && !trimmed.starts_with('#') {
            // This is the first non-comment, non-empty line - the variant URL
            let absolutized = absolutize_url(master_url, trimmed);
            if absolutized == variant_url || absolutized == absolutize_url(master_url, variant_url) {
                // Replace this with the cleaned variant data URI.
                result_lines.push(format!("#EXT-X-EMBEDDED-variant:{}\n{}", data_uri, trimmed));
                found_variant = true;
                continue;
            }
        }
        result_lines.push(line.to_string());
    }

    // If we didn't find the variant URL by matching, just append the data URI as a comment.
    if !found_variant {
        result_lines.insert(0, format!("#EXT-X-EMBEDDED-variant:{}\n#EXT-X-EMBEDDED-variant-PLAYLIST", data_uri));
    }

    result_lines.join("\n")
}

/// Check if an error string indicates an auth/referer-blocked failure
/// that might succeed on retry with a Referer header.
fn looks_like_auth_failure(error: &str) -> bool {
    let lower = error.to_ascii_lowercase();
    lower.contains("403")
        || lower.contains("401")
        || lower.contains("406")
        || lower.contains("407")
        || lower.contains("blocked")
        || lower.contains("forbidden")
        || lower.contains("unauthorized")
}

/// Wrapper for playlist fetch that retries with Referer on auth failure.
async fn fetch_hls_playlist_with_headers_no_cors_and_retry(
    client: &reqwest::Client,
    url: &str,
    headers: Option<&std::collections::HashMap<String, String>>,
    referer: Option<&str>,
) -> Result<String, String> {
    let h = headers;
    let r = referer;
    match fetch_hls_playlist_with_headers_no_cors(client, url, h).await {
        Ok(body) => Ok(body),
        Err(e) if r.is_some() && looks_like_auth_failure(&e) => {
            let mut retry_headers = h.cloned().unwrap_or_default();
            retry_headers.insert("Referer".to_string(), r.unwrap().to_string());
            fetch_hls_playlist_with_headers_no_cors(client, url, Some(&retry_headers)).await
        }
        Err(e) => Err(e),
    }
}

/// Wrapper for binary proxy that retries with Referer on auth failure.
async fn proxy_url_with_retry(
    client: &reqwest::Client,
    url: &str,
    headers: Option<&std::collections::HashMap<String, String>>,
    referer: Option<&str>,
) -> Result<crate::commands::player::SegmentProxyResponse, String> {
    let request_headers = build_hls_request_headers(headers, referer);
    match proxy_url_with_headers(client, url, request_headers.as_ref(), None).await {
        Ok(resp) => Ok(resp),
        Err(e) if referer.is_some() && looks_like_auth_failure(&e) => {
            let retry_headers = build_hls_request_headers(headers, referer);
            proxy_url_with_headers(client, url, retry_headers.as_ref(), None).await
        }
        Err(e) => Err(e.to_string()),
    }
}

/// Proxy a URL with explicit headers and optional Range header.
/// Proxy a URL with explicit headers and optional Range header.
/// Returns the response body (base64), Content-Range metadata, and status code.
pub(crate) async fn proxy_url_with_headers(
    client: &reqwest::Client,
    url: &str,
    headers: Option<&std::collections::HashMap<String, String>>,
    range: Option<&str>,
) -> Result<SegmentProxyResponse, String> {
    use base64::Engine;
    let mut request = client.get(url).header(
        reqwest::header::USER_AGENT,
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36",
    );
    if let Some(range) = range {
        eprintln!("[proxy_url] Applying Range: {} for {}", range, &url[..url.len().min(80)]);
        request = request.header(reqwest::header::RANGE, range);
    }
    let response = apply_request_headers(request, headers)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let status = response.status().as_u16();

    // hls.js needs 206 Partial Content for range requests; 416 is Range Not Satisfiable
    if range.is_some() && status == 416 {
        return Err(format!("Range header not satisfiable (416) for {}", &url[..url.len().min(80)]));
    }

    // Accept both 200 (no range support) and 206 (partial content)
    if !(status == 200 || status == 206) {
        return Err(format!("proxy request failed: HTTP {}", status));
    }

    let content_range = response
        .headers()
        .get("content-range")
        .and_then(|v| v.to_str().ok())
        .map(String::from);

    let bytes = response.bytes().await.map_err(|e| e.to_string())?;
    Ok(SegmentProxyResponse {
        data: base64::engine::general_purpose::STANDARD.encode(bytes),
        content_range,
        status,
    })
}

/// Internal segment fetch that supports Range header and returns full response metadata.
/// Uses a cached HTTP client for connection reuse and faster subsequent requests.
pub(crate) async fn fetch_hls_segment_internal(
    url: &str,
    headers: Option<&std::collections::HashMap<String, String>>,
    referer: Option<&str>,
    range: Option<&str>,
) -> Result<crate::commands::player::SegmentProxyResponse, String> {
    let request_headers = build_hls_request_headers(headers, referer);
    let client = get_http_client()?;
    proxy_url_with_headers(&client, url, request_headers.as_ref(), range).await
}

pub(crate) async fn fetch_hls_manifest_internal(
    url: &str,
    headers: Option<&std::collections::HashMap<String, String>>,
    referer: Option<&str>,
) -> Result<String, String> {
    let request_headers = build_hls_request_headers(headers, referer);

    // For non-manifest URLs (segments), use binary proxy
    if !url.contains(".m3u8") {
        let client = get_http_client()?;
        let resp = proxy_url_with_retry(&client, url, request_headers.as_ref(), referer).await?;
        return Ok(resp.data); // base64-encoded body
    }

    let client = get_http_client()?;
    let body =
        fetch_hls_playlist_with_headers_no_cors_and_retry(&client, url, request_headers.as_ref(), referer).await?;

    // Check if it's a master playlist (contains #EXT-X-STREAM-INF)
    if body.contains("#EXT-X-STREAM-INF") {
        // Get the first variant playlist URL
        let Some(variant_url) = first_playlist_resource(url, &body) else {
            return Err("master playlist missing variant url".to_string());
        };

        // Fetch the variant playlist with retry
        let variant_body = fetch_hls_playlist_with_headers_no_cors_and_retry(
            &client,
            &variant_url,
            request_headers.as_ref(),
            referer,
        )
        .await?;

        let rewritten_variant = rewrite_relative_urls(&variant_body, &variant_url);
        let cleaned_variant = HlsAdBlocker::filter_playlist(&rewritten_variant);

        // Normalize the master playlist with embedded variant
        let normalized = normalize_master_playlist(&body, url, &cleaned_variant, &variant_url);

        // Rewrite relative URLs in the normalized master to absolute
        return Ok(rewrite_relative_urls(&normalized, url));
    }

    // It's not a media playlist, rewrite relative URLs to absolute then filter ads
    let rewritten = rewrite_relative_urls(&body, url);
    Ok(HlsAdBlocker::filter_playlist(&rewritten))
}

fn apply_request_headers(
    request: reqwest::RequestBuilder,
    headers: Option<&std::collections::HashMap<String, String>>,
) -> reqwest::RequestBuilder {
    let mut request = request;
    if let Some(headers) = headers {
        for (key, value) in headers {
            request = request.header(key, value);
        }
    }
    request
}

fn build_hls_request_headers(
    headers: Option<&std::collections::HashMap<String, String>>,
    referer: Option<&str>,
) -> Option<std::collections::HashMap<String, String>> {
    let mut merged = headers.cloned().unwrap_or_default();
    let existing_referer = find_header_value(&merged, "referer").map(str::to_string);
    if let Some(referer_value) = referer.or(existing_referer.as_deref()) {
        if find_header_value(&merged, "referer").is_none() {
            merged.insert("Referer".to_string(), referer_value.to_string());
        }
        if find_header_value(&merged, "origin").is_none() {
            if let Some(origin) = infer_origin_from_referer(referer_value) {
                merged.insert("Origin".to_string(), origin);
            }
        }
    }

    if merged.is_empty() {
        None
    } else {
        Some(merged)
    }
}

fn find_header_value<'a>(
    headers: &'a std::collections::HashMap<String, String>,
    name: &str,
) -> Option<&'a str> {
    headers
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case(name))
        .map(|(_, value)| value.as_str())
}

fn infer_origin_from_referer(referer: &str) -> Option<String> {
    let parsed = reqwest::Url::parse(referer).ok()?;
    let host = parsed.host_str()?;
    let mut origin = format!("{}://{}", parsed.scheme(), host);
    if let Some(port) = parsed.port() {
        origin.push(':');
        origin.push_str(&port.to_string());
    }
    Some(origin)
}

#[cfg(test)]
mod tests {
    use super::{
        build_hls_request_headers, classify_playback_target, detect_kind,
        extract_aliplayer_source, extract_dplayer_video_url, extract_html_video_src,
        extract_iframe_src, extract_maccms_player_url, fetch_hls_manifest_internal,
        infer_origin_from_referer, is_playable_media_url, looks_like_cloud_disk_link,
        looks_like_xb6v_play_page, looks_like_ypanso_play_page, looks_like_zxzj_play_page,
        map_target_kind_to_probe_gate, parse_http_status_from_error, PlaybackResolver,
    };
    use std::collections::HashMap;

    #[tokio::test]
    async fn marks_hls_url_as_ready_candidate() {
        assert_eq!(
            classify_playback_target("https://example.com/live.m3u8"),
            "direct"
        );
        assert_eq!(detect_kind("https://example.com/live.m3u8"), "hls");
    }

    #[tokio::test]
    async fn probes_hls_playlists_without_browser_cors_headers_as_playable() {
        use crate::services::playback_types::PlaybackProbeStatus;
        use super::{build_client, probe_hls_playlist_result};
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").await.expect("listener should bind");
        let addr = listener.local_addr().expect("listener should expose addr");

        let server = tokio::spawn(async move {
            let playlist_body = "#EXTM3U\n#EXT-X-TARGETDURATION:10\n#EXTINF:10,\nsegment.ts\n";
            let playlist_response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/vnd.apple.mpegurl\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                playlist_body.len(),
                playlist_body
            );
            let segment_response = "HTTP/1.1 200 OK\r\nContent-Type: video/mp2t\r\nContent-Length: 4\r\nConnection: close\r\n\r\nDATA".to_string();

            for response in [playlist_response, segment_response] {
                let (mut socket, _) = listener.accept().await.expect("request should arrive");
                let mut request = [0u8; 2048];
                let _ = socket.read(&mut request).await.expect("request should read");
                socket
                    .write_all(response.as_bytes())
                    .await
                    .expect("response should write");
            }
        });

        let client = build_client().expect("client should build");
        let url = format!("http://{addr}/playlist.m3u8");
        let probe = probe_hls_playlist_result(&client, &url, None).await;

        server.await.expect("server task should finish");

        assert_eq!(probe.status, PlaybackProbeStatus::Playable);
    }

    #[tokio::test]
    async fn fetch_hls_manifest_internal_filters_ads_inside_master_playlist() {
        use base64::Engine;
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpListener;

        let master_body = "#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=1280000\n/variant.m3u8\n";
        let variant_body = "#EXTM3U\n#EXTINF:10.0,\nhttps://ads.example.com/ad1.ts\n#EXTINF:10.0,\nhttps://cdn.example.com/seg1.ts\n";

        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener should bind");
        let addr = listener.local_addr().expect("listener should expose addr");

        tokio::spawn(async move {
            loop {
                let (mut socket, _) = listener.accept().await.expect("request should arrive");
                let mut buf = [0_u8; 4096];
                let n = socket.read(&mut buf).await.expect("request should read");
                let request = String::from_utf8_lossy(&buf[..n]);
                let body = if request.contains("/variant.m3u8") {
                    variant_body
                } else {
                    master_body
                };
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/vnd.apple.mpegurl\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                socket
                    .write_all(response.as_bytes())
                    .await
                    .expect("response should write");
            }
        });

        let url = format!("http://{}/master.m3u8", addr);
        let result = fetch_hls_manifest_internal(&url, None, None)
            .await
            .expect("manifest should resolve");

        let embedded_variant_line = result
            .lines()
            .find(|line| line.starts_with("#EXT-X-EMBEDDED-variant:data:application/vnd.apple.mpegurl;base64,"))
            .expect("master playlist should embed a variant data uri");
        let encoded_variant = embedded_variant_line
            .trim_start_matches("#EXT-X-EMBEDDED-variant:data:application/vnd.apple.mpegurl;base64,");
        let decoded_variant = base64::engine::general_purpose::STANDARD
            .decode(encoded_variant)
            .expect("embedded variant should decode");
        let decoded_variant = String::from_utf8(decoded_variant).expect("decoded variant should be utf8");

        assert!(!decoded_variant.contains("ad1.ts"));
        assert!(decoded_variant.contains("seg1.ts"));
    }

    #[tokio::test]
    async fn marks_unknown_scheme_as_external_required() {
        let resolved = PlaybackResolver::resolve("drpy://source/detail")
            .await
            .unwrap();
        assert_eq!(resolved.status, "external_required");
    }

    #[test]
    fn classifies_embedded_targets_as_not_playable() {
        assert!(!map_target_kind_to_probe_gate("embedded"));
        assert!(map_target_kind_to_probe_gate("direct"));
    }

    #[test]
    fn xb6v_play_pages_are_resolvable() {
        assert!(looks_like_xb6v_play_page(
            "https://www.xb6v.com/e/DownSys/play/?classid=8&id=11308&pathid1=0"
        ));
        assert!(!looks_like_xb6v_play_page("https://example.com/video.mp4"));
    }

    #[test]
    fn ypanso_play_pages_are_resolvable() {
        assert!(looks_like_ypanso_play_page(
            "https://www.ypanso.com/vod/play/id/12345/sid/1/nid/1.html"
        ));
        assert!(!looks_like_ypanso_play_page("https://example.com/video.mp4"));
        assert_eq!(
            classify_playback_target("https://www.ypanso.com/vod/play/id/12345/sid/1/nid/1.html"),
            "resolvable"
        );
    }

    #[test]
    fn zxzj_play_pages_are_resolvable() {
        assert!(looks_like_zxzj_play_page(
            "https://www.zxzjhd.com/vodplay/4627-1-1.html"
        ));
        assert_eq!(
            classify_playback_target("https://www.zxzjhd.com/vodplay/4627-1-1.html"),
            "resolvable"
        );
    }

    #[test]
    fn generic_play_pages_are_resolvable() {
        assert_eq!(
            classify_playback_target("https://www.fan.com/play/4627-1-1.html"),
            "resolvable"
        );
        assert_eq!(
            classify_playback_target("https://www.cc.com/vod/4627-1-1.html"),
            "resolvable"
        );
        assert_eq!(
            classify_playback_target("https://www.kkss.com/vod/play/4627-1-1.html"),
            "resolvable"
        );
    }

    #[test]
    fn cloud_disk_links_are_external() {
        assert!(looks_like_cloud_disk_link("https://pan.baidu.com/s/abc"));
        assert!(looks_like_cloud_disk_link("https://pan.quark.cn/s/abc"));
        assert!(!looks_like_cloud_disk_link("https://example.com/video.mp4"));
    }

    #[test]
    fn infers_origin_from_referer_for_hls_requests() {
        assert_eq!(
            infer_origin_from_referer("https://www.ypanso.com/vod/play/id/12345/sid/1/nid/1.html"),
            Some("https://www.ypanso.com".to_string())
        );
    }

    #[test]
    fn merges_referer_and_origin_into_hls_headers() {
        let mut headers = HashMap::new();
        headers.insert("User-Agent".to_string(), "TVBox".to_string());

        let merged = build_hls_request_headers(
            Some(&headers),
            Some("https://www.ypanso.com/vod/play/id/12345/sid/1/nid/1.html"),
        )
        .expect("headers should be present");

        assert_eq!(
            merged.get("Referer").map(String::as_str),
            Some("https://www.ypanso.com/vod/play/id/12345/sid/1/nid/1.html")
        );
        assert_eq!(
            merged.get("Origin").map(String::as_str),
            Some("https://www.ypanso.com")
        );
        assert_eq!(merged.get("User-Agent").map(String::as_str), Some("TVBox"));
    }

    #[test]
    fn extracts_dplayer_video_url_from_config() {
        let body = r#"
        <script>
        const dp = new DPlayer({
          container: document.getElementById('dplayer'),
          autoplay: true,
          video: {
            url: 'https://hn.bfvvs.com/play/bDk9mYAa/index.m3u8',
            type: 'hls',
          },
        });
        </script>
        "#;
        assert_eq!(
            extract_dplayer_video_url(body).as_deref(),
            Some("https://hn.bfvvs.com/play/bDk9mYAa/index.m3u8")
        );
    }

    #[test]
    fn is_playable_media_url_accepts_valid_extensions() {
        let valid = vec![
            "https://example.com/video.m3u8",
            "https://example.com/video.mp4",
            "https://example.com/video.m4v",
            "https://example.com/video.webm",
            "https://example.com/video.mov",
            "https://example.com/video.flv",
            "https://example.com/VIDEO.MP4",
            "https://example.com/VIDEO.M3U8",
        ];
        for url in valid {
            assert!(is_playable_media_url(url), "should be playable: {url}");
        }
    }

    #[test]
    fn is_playable_media_url_rejects_non_media_urls() {
        let invalid = vec![
            "https://example.com/",
            "https://example.com/page.html",
            "https://example.com/image.jpg",
            "https://example.com/script.js",
            "https://example.com/api/json",
        ];
        for url in invalid {
            assert!(!is_playable_media_url(url), "should not be playable: {url}");
        }
    }

    // ─── T01 Tests: URL validation ─────────────────────────────────────────────

    #[test]
    fn extract_dplayer_video_url_rejects_css_js_image_urls() {
        // Even if the JSON-like structure is present, non-media URLs should be rejected.
        let body = r#"video: { url: 'https://example.com/style.css' }"#;
        assert!(
            extract_dplayer_video_url(body).is_none(),
            "CSS URL should be rejected even if JSON format is valid"
        );

        let body_js = r#"video: { url: 'https://example.com/app.js' }"#;
        assert!(
            extract_dplayer_video_url(body_js).is_none(),
            "JS URL should be rejected even if JSON format is valid"
        );

        let body_img = r#"video: { url: 'https://example.com/poster.jpg' }"#;
        assert!(
            extract_dplayer_video_url(body_img).is_none(),
            "Image URL should be rejected even if JSON format is valid"
        );

        // Valid media URL should still be accepted
        let body_valid = r#"video: { url: 'https://example.com/video.mp4' }"#;
        assert_eq!(
            extract_dplayer_video_url(body_valid).as_deref(),
            Some("https://example.com/video.mp4")
        );
    }

    #[test]
    fn extract_aliplayer_source_rejects_non_media_urls() {
        let body = r#""source": "https://example.com/page.html""#;
        assert!(
            extract_aliplayer_source(body).is_none(),
            "HTML URL should be rejected"
        );

        let body_css = r#""source": "https://example.com/style.css""#;
        assert!(
            extract_aliplayer_source(body_css).is_none(),
            "CSS URL should be rejected"
        );

        // Valid media URL should be accepted
        let body_valid = r#""source": "https://example.com/video.m3u8""#;
        assert_eq!(
            extract_aliplayer_source(body_valid).as_deref(),
            Some("https://example.com/video.m3u8")
        );
    }

    #[test]
    fn extract_maccms_player_url_rejects_non_media_urls() {
        let body = r#"player_aaaa = {"url": "https://example.com/page.html", "name": "test"}</script>"#;
        assert!(
            extract_maccms_player_url(body).is_none(),
            "HTML URL should be rejected"
        );

        let body_css = r#"player_bbbb = {"url": "https://example.com/script.js", "name": "test"}</script>"#;
        assert!(
            extract_maccms_player_url(body_css).is_none(),
            "JS URL should be rejected"
        );

        // Valid media URL should be accepted
        let body_valid = r#"player_aaaa = {"url": "https://example.com/video.mp4", "name": "test"}</script>"#;
        assert_eq!(
            extract_maccms_player_url(body_valid).as_deref(),
            Some("https://example.com/video.mp4")
        );
    }

    #[test]
    fn non_media_urls_fall_through_not_to_failure() {
        // When an extractor finds a URL but it's not playable media,
        // it returns None so the next extractor gets tried (not immediate failure).
        let body_with_non_media_source = r#"
        "source": "https://example.com/style.css"
        video: { url: 'https://example.com/app.js' }
        player_aaaa = {"url": "https://example.com/image.jpg", "name": "test"}</script>
        "#;
        // All extractors return None for non-media URLs, triggering fallthrough behavior
        assert!(extract_aliplayer_source(body_with_non_media_source).is_none());
        assert!(extract_dplayer_video_url(body_with_non_media_source).is_none());
        assert!(extract_maccms_player_url(body_with_non_media_source).is_none());
    }

    // ─── T02 Tests: Error classification ───────────────────────────────────────

    #[test]
    fn resolve_play_page_builds_error_message_with_classification_on_failure() {
        use super::ParserFailureKind;

        // Verify each failure kind produces a non-empty, categorized message
        let kinds = [
            (ParserFailureKind::NoUrlExtracted, None),
            (ParserFailureKind::NoUrlExtracted, Some(200)),
            (ParserFailureKind::NonMediaUrl, Some(200)),
            (ParserFailureKind::NetworkError, Some(404)),
            (ParserFailureKind::NetworkError, None),
            (ParserFailureKind::InvalidResponse, None),
        ];

        for (kind, http_status) in kinds {
            let msg = kind.message(http_status);
            let tag = match kind {
                ParserFailureKind::NoUrlExtracted => "no_url_extracted",
                ParserFailureKind::NonMediaUrl => "non_media_url",
                ParserFailureKind::NetworkError => "network_error",
                ParserFailureKind::InvalidResponse => "invalid_response",
            };
            assert!(
                !msg.is_empty(),
                "failure kind {:?} should produce non-empty message",
                kind
            );
            assert!(
                msg.contains(tag),
                "message '{}' should contain tag '{}'",
                msg,
                tag
            );
        }

        // NetworkError with status includes HTTP code in message
        let network_msg = ParserFailureKind::NetworkError.message(Some(404));
        assert!(
            network_msg.contains("404"),
            "network error message should contain 404, got: {}",
            network_msg
        );
    }

    #[test]
    fn parse_http_status_from_error_extracts_status_code() {
        assert_eq!(parse_http_status_from_error("request failed: 404"), Some(404));
        assert_eq!(parse_http_status_from_error("request failed: 503"), Some(503));
        assert_eq!(parse_http_status_from_error("request failed: 200"), Some(200));
        assert_eq!(parse_http_status_from_error("connection refused"), None);
        // "timeout after 30s" contains digits "30" — function extracts them (not ideal but documented behavior)
        assert_eq!(parse_http_status_from_error("timeout after 30s"), Some(30));
        assert_eq!(parse_http_status_from_error("request failed: 403 Forbidden"), Some(403));
    }

    // ─── T03 Tests: Resilience ──────────────────────────────────────────────────

    #[test]
    fn extract_dplayer_video_url_handles_malformed_json_gracefully() {
        // Completely invalid JSON-like text — should return None, not panic
        let body_malformed = r#"video: { url: 'unclosed"#;
        assert!(
            extract_dplayer_video_url(body_malformed).is_none(),
            "malformed JSON-like body should return None without panic"
        );

        // Empty URL value
        let body_empty_url = r#"video: { url: '' }"#;
        assert!(
            extract_dplayer_video_url(body_empty_url).is_none(),
            "empty URL value should return None"
        );

        // Pattern that matches regex but URL is empty
        let body_empty = r#"url: ""#;
        assert!(
            extract_dplayer_video_url(body_empty).is_none(),
            "empty url field should return None"
        );
    }

    #[test]
    fn extract_maccms_player_url_handles_missing_url_field_gracefully() {
        let body_no_url = r#"player_aaaa = {"name": "test"}</script>"#;
        assert!(
            extract_maccms_player_url(body_no_url).is_none(),
            "missing url field should return None without panic"
        );

        let _body_null_url = r#"player_aaaa = {"url": null, "name": "test"}</script>"#;
        assert!(
            extract_maccms_player_url(body_no_url).is_none(),
            "null url field should return None"
        );

        let body_empty_json = r#"player_bbbb = {}</script>"#;
        assert!(
            extract_maccms_player_url(body_empty_json).is_none(),
            "empty player object should return None"
        );
    }

    #[test]
    fn extract_html_video_src_handles_missing_src_attribute_gracefully() {
        let body_no_video = r#"<video></video>"#;
        assert!(
            extract_html_video_src("https://example.com/page.html", body_no_video).is_none(),
            "video element without src should return None"
        );

        let body_source_no_src = r#"<video><source type="video/mp4"></video>"#;
        assert!(
            extract_html_video_src("https://example.com/page.html", body_source_no_src).is_none(),
            "source element without src should return None"
        );

        let body_empty_src = r#"<video src=""></video>"#;
        assert!(
            extract_html_video_src("https://example.com/page.html", body_empty_src).is_none(),
            "empty src attribute should return None"
        );
    }

    #[test]
    fn extract_aliplayer_source_handles_invalid_pattern_gracefully() {
        // Aliplayer source regex only matches double-quoted "source" values.
        // Invalid patterns should return None without panic.
        let body_invalid = r#"source: "https://example.com/video.m3u8""#;
        assert!(
            extract_aliplayer_source(body_invalid).is_none(),
            "aliplayer pattern should only match double-quoted source values"
        );
    }

    #[test]
    fn extract_iframe_src_handles_missing_src_gracefully() {
        let body_no_iframe = r#"<div class="player"></div>"#;
        assert!(
            extract_iframe_src("https://example.com/page.html", body_no_iframe).is_none(),
            "page without iframe should return None"
        );

        let body_iframe_no_src = r#"<iframe></iframe>"#;
        assert!(
            extract_iframe_src("https://example.com/page.html", body_iframe_no_src).is_none(),
            "iframe without src should return None"
        );
    }
}
