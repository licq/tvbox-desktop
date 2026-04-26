use crate::models::{PlaybackCandidate, ResolvedPlayback};
use crate::services::playback_types::{PlaybackProbeResult, PlaybackProbeStatus};
use regex::Regex;

pub struct PlaybackResolver;

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

        if looks_like_xb6v_play_page(input) {
            return resolve_xb6v_play_page(input).await;
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

        Ok(ready_with_candidate(input.to_string(), detect_kind(input)))
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

    if looks_like_xb6v_play_page(input) {
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

    "direct"
}

pub fn is_visible_playback_target(input: &str) -> bool {
    map_target_kind_to_probe_gate(classify_playback_target(input))
}

pub fn playback_sort_rank(input: &str) -> i32 {
    match classify_playback_target(input) {
        "direct" => 0,
        "resolvable" => 1,
        "embedded" => 2,
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
        }],
        error_message: Some(message.to_string()),
    }
}

fn ready_with_candidate(url: String, kind: &'static str) -> ResolvedPlayback {
    ResolvedPlayback {
        status: "ready".to_string(),
        candidates: vec![PlaybackCandidate {
            url,
            label: "默认线路".to_string(),
            kind: kind.to_string(),
            headers: None,
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

fn looks_like_cloud_disk_link(input: &str) -> bool {
    input.contains("pan.baidu.com/")
        || input.contains("pan.quark.cn/")
        || input.contains("drive.uc.cn/")
        || input.contains("aliyundrive.com/")
        || input.contains("alipan.com/")
}

async fn resolve_xb6v_play_page(input: &str) -> Result<ResolvedPlayback, String> {
    let client = build_client()?;
    let body = fetch_text(&client, input).await?;
    if let Some(source_url) = extract_aliplayer_source(&body) {
        return Ok(ready_with_candidate(
            source_url.clone(),
            detect_kind(&source_url),
        ));
    }
    if let Some(iframe_url) = extract_iframe_src(input, &body) {
        return resolve_embedded_share_page(&client, &iframe_url).await;
    }

    Ok(ResolvedPlayback {
        status: "failed".to_string(),
        candidates: vec![],
        error_message: Some("未能从播放页提取实际视频地址".to_string()),
    })
}

fn extract_aliplayer_source(body: &str) -> Option<String> {
    let source_regex = Regex::new(r#""source"\s*:\s*"([^"]+)""#).unwrap();
    source_regex
        .captures(body)
        .and_then(|captures| captures.get(1).map(|value| value.as_str().to_string()))
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
    reqwest::Client::builder()
        .no_proxy()
        .connect_timeout(std::time::Duration::from_secs(20))
        .timeout(std::time::Duration::from_secs(20))
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
    let body = match fetch_hls_playlist_with_headers(client, url, headers).await {
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
        let variant_body = match fetch_hls_playlist_with_headers(client, &variant_url, headers).await {
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
    let response = match apply_request_headers(request, headers).send().await {
        Ok(response) => response,
        Err(error) => return PlaybackProbeResult::failed(error.to_string(), None),
    };
    let http_status = Some(i64::from(response.status().as_u16()));

    if response.status().is_success() || response.status() == reqwest::StatusCode::PARTIAL_CONTENT {
        if !has_browser_cors(&response) {
            let mut probe =
                PlaybackProbeResult::failed("resource probe missing browser CORS headers", http_status);
            probe.manifest_ok = true;
            probe.segment_ok = true;
            return probe;
        }
        let mut probe = PlaybackProbeResult::playable();
        probe.http_status = http_status;
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
    let response = apply_request_headers(request, headers)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("request failed: {status}"));
    }

    response.text().await.map_err(|e| e.to_string())
}

async fn fetch_hls_playlist_with_headers(
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
    let response = apply_request_headers(request, headers)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("playlist request failed: {status}"));
    }
    if !has_browser_cors(&response) {
        return Err("playlist missing browser CORS headers".to_string());
    }

    response.text().await.map_err(|e| e.to_string())
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
    let response = apply_request_headers(request, headers)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("playlist request failed: {status}"));
    }

    response.text().await.map_err(|e| e.to_string())
}

/// Fetches arbitrary URL content (for proxying segment requests).
/// Does not require CORS headers, returns raw bytes as base64 string.
async fn proxy_url(client: &reqwest::Client, url: &str) -> Result<String, String> {
    use base64::Engine;
    let request = client
        .get(url)
        .header(
            reqwest::header::USER_AGENT,
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36",
        );
    let response = request.send().await.map_err(|e| e.to_string())?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("proxy request failed: {status}"));
    }
    let bytes = response.bytes().await.map_err(|e| e.to_string())?;
    // Encode as base64 so we can send binary over JSON
    Ok(base64::engine::general_purpose::STANDARD.encode(bytes))
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
    // Rewrite relative URLs in variant playlist to absolute
    let normalized_variant = rewrite_relative_urls(variant_body, variant_url);
    // Encode the variant playlist as base64
    let variant_b64 = base64::engine::general_purpose::STANDARD.encode(normalized_variant.as_bytes());
    let data_uri = format!("data:application/vnd.apple.mpegurl;base64,{}", variant_b64);

    // Rewrite master playlist lines, replacing the variant URL with the data URI
    let mut result_lines = Vec::new();
    let mut found_variant = false;
    for line in master_body.lines() {
        let trimmed = line.trim();
        if !found_variant && !trimmed.is_empty() && !trimmed.starts_with('#') {
            // This is the first non-comment, non-empty line - the variant URL
            let absolutized = absolutize_url(master_url, trimmed);
            if absolutized == variant_url || absolutized == absolutize_url(master_url, variant_url) {
                // Replace this with the data URI
                result_lines.push(format!("#EXT-X-EMBEDDED-variant:{}\n{}", data_uri, trimmed));
                found_variant = true;
                continue;
            }
        }
        result_lines.push(line.to_string());
    }

    // If we didn't find the variant URL by matching, just append the data URI as a comment
    if !found_variant {
        result_lines.insert(0, format!("#EXT-X-EMBEDDED-variant:{}\n#EXT-X-EMBEDDED-variant-PLAYLIST", data_uri));
    }

    result_lines.join("\n")
}

pub(crate) async fn fetch_hls_manifest_internal(
    url: &str,
    headers: Option<&std::collections::HashMap<String, String>>,
) -> Result<String, String> {
    // For non-manifest URLs (segments), use binary proxy
    if !url.contains(".m3u8") {
        let client = build_client()?;
        return proxy_url(&client, url).await;
    }

    let client = build_client()?;
    let body = fetch_hls_playlist_with_headers_no_cors(&client, url, headers).await?;

    // Check if it's a master playlist (contains #EXT-X-STREAM-INF)
    if body.contains("#EXT-X-STREAM-INF") {
        // Get the first variant playlist URL
        let Some(variant_url) = first_playlist_resource(url, &body) else {
            return Err("master playlist missing variant url".to_string());
        };

        // Fetch the variant playlist without CORS check
        let variant_body = fetch_hls_playlist_with_headers_no_cors(&client, &variant_url, headers).await?;

        // Normalize the master playlist with embedded variant
        let normalized = normalize_master_playlist(&body, url, &variant_body, &variant_url);

        // Rewrite relative URLs in the normalized master to absolute
        return Ok(rewrite_relative_urls(&normalized, url));
    }

    // It's not a master playlist, just rewrite relative URLs to absolute
    Ok(rewrite_relative_urls(&body, url))
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

#[cfg(test)]
mod tests {
    use super::{
        classify_playback_target, detect_kind, looks_like_cloud_disk_link,
        looks_like_xb6v_play_page, map_target_kind_to_probe_gate, PlaybackResolver,
    };

    #[tokio::test]
    async fn marks_hls_url_as_ready_candidate() {
        assert_eq!(
            classify_playback_target("https://example.com/live.m3u8"),
            "direct"
        );
        assert_eq!(detect_kind("https://example.com/live.m3u8"), "hls");
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
    fn cloud_disk_links_are_external() {
        assert!(looks_like_cloud_disk_link("https://pan.baidu.com/s/abc"));
        assert!(looks_like_cloud_disk_link("https://pan.quark.cn/s/abc"));
        assert!(!looks_like_cloud_disk_link("https://example.com/video.mp4"));
    }
}
