use crate::models::{PlaybackCandidate, ResolvedPlayback};
use crate::services::{
    extract_auete_player_url, extract_jianpian_player_url, extract_libvio_player_url,
    extract_wencai_player_url,
};
use regex::Regex;

pub struct PlaybackResolver;

impl PlaybackResolver {
    pub async fn resolve(input: &str) -> Result<ResolvedPlayback, String> {
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
        if looks_like_libvio_play_page(input) {
            return resolve_libvio_play_page(input).await;
        }
        if looks_like_auete_play_page(input) {
            return resolve_auete_play_page(input).await;
        }
        if looks_like_wencai_play_page(input) {
            return resolve_wencai_play_page(input).await;
        }
        if looks_like_jianpian_play_page(input) {
            return resolve_jianpian_play_page(input).await;
        }
        if looks_like_zxzj_play_page(input) {
            return Ok(ready_with_candidate(input.to_string(), "embed"));
        }

        Ok(ready_with_candidate(input.to_string(), detect_kind(input)))
    }
}

pub fn classify_playback_target(input: &str) -> &'static str {
    if input.starts_with("drpy://")
        || input.starts_with("magnet:")
        || input.starts_with("ed2k://")
        || input.starts_with("thunder://")
        || looks_like_cloud_disk_link(input)
    {
        return "external";
    }

    if looks_like_zxzj_play_page(input) {
        return "embedded";
    }

    if looks_like_xb6v_play_page(input)
        || looks_like_libvio_play_page(input)
        || looks_like_auete_play_page(input)
        || looks_like_wencai_play_page(input)
        || looks_like_jianpian_play_page(input)
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

    "direct"
}

pub fn is_visible_playback_target(input: &str) -> bool {
    matches!(classify_playback_target(input), "direct" | "resolvable")
}

pub fn playback_sort_rank(input: &str) -> i32 {
    match classify_playback_target(input) {
        "direct" => 0,
        "resolvable" => 1,
        "embedded" => 2,
        _ => 3,
    }
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

fn looks_like_libvio_play_page(input: &str) -> bool {
    (input.contains("libvio.") || input.contains("libvio.me/")) && input.contains("/play/")
}

fn looks_like_auete_play_page(input: &str) -> bool {
    (input.contains("auete.") || input.contains("au1080.com/") || input.contains("auete.top/"))
        && input.contains("/play-")
        && input.ends_with(".html")
}

fn looks_like_wencai_play_page(input: &str) -> bool {
    input.contains("/play/")
        && (input.contains("wencai")
            || input.contains("deeyy.com")
            || input.contains("%E6%96%87%E9%87%87"))
}

fn looks_like_jianpian_play_page(input: &str) -> bool {
    (input.contains("/vodplay/") || input.contains("/play/"))
        && (input.contains("jianpian")
            || input.contains("jpys")
            || input.contains("jpvod.com")
            || input.contains("%E8%8D%90%E7%89%87"))
}

fn looks_like_zxzj_play_page(input: &str) -> bool {
    (input.contains("zxzjhd.com/") || input.contains("zxzjys.com/")) && input.contains("/vodplay/")
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

async fn resolve_libvio_play_page(input: &str) -> Result<ResolvedPlayback, String> {
    let client = build_client()?;
    let body = fetch_text(&client, input).await?;
    let Some(source_url) = extract_libvio_player_url(&body) else {
        return Ok(ResolvedPlayback {
            status: "failed".to_string(),
            candidates: vec![],
            error_message: Some("未能从 LIBVIO 播放页提取实际视频地址".to_string()),
        });
    };

    Ok(ready_with_candidate(
        source_url.clone(),
        detect_kind(&source_url),
    ))
}

async fn resolve_auete_play_page(input: &str) -> Result<ResolvedPlayback, String> {
    let client = build_client()?;
    let body = fetch_text(&client, input).await?;
    let play_pages = extract_auete_play_page_candidates(input, &body);
    if play_pages.is_empty() {
        return Ok(ResolvedPlayback {
            status: "failed".to_string(),
            candidates: vec![],
            error_message: Some("未能从 Auete 播放页提取实际视频地址".to_string()),
        });
    }

    let mut candidates = Vec::new();
    for (index, play_page) in play_pages.into_iter().enumerate() {
        let page_body = if index == 0 && play_page.url == input {
            body.clone()
        } else {
            match fetch_text(&client, &play_page.url).await {
                Ok(value) => value,
                Err(_) => continue,
            }
        };

        let Some(source_url) = extract_auete_player_url(&page_body) else {
            continue;
        };
        if probe_media_candidate(&client, &source_url).await.is_err() {
            continue;
        }

        candidates.push(PlaybackCandidate {
            url: source_url.clone(),
            label: play_page.label,
            kind: detect_kind(&source_url).to_string(),
            headers: None,
        });
    }

    if candidates.is_empty() {
        return Ok(ResolvedPlayback {
            status: "failed".to_string(),
            candidates: vec![],
            error_message: Some("Auete 当前集数未找到可直播线路".to_string()),
        });
    }

    Ok(ResolvedPlayback {
        status: "ready".to_string(),
        candidates,
        error_message: None,
    })
}

async fn resolve_wencai_play_page(input: &str) -> Result<ResolvedPlayback, String> {
    resolve_multi_candidate_page(
        input,
        extract_wencai_play_page_candidates,
        extract_wencai_player_url,
        "WenCai 当前集数未找到可直播线路",
    )
    .await
}

async fn resolve_jianpian_play_page(input: &str) -> Result<ResolvedPlayback, String> {
    resolve_multi_candidate_page(
        input,
        extract_jianpian_play_page_candidates,
        extract_jianpian_player_url,
        "荐片当前集数未找到可直播线路",
    )
    .await
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PlayPageCandidate {
    url: String,
    label: String,
}

async fn resolve_multi_candidate_page(
    input: &str,
    extract_candidates: fn(&str, &str) -> Vec<PlayPageCandidate>,
    extract_player_url: fn(&str) -> Option<String>,
    failed_message: &str,
) -> Result<ResolvedPlayback, String> {
    let client = build_client()?;
    let body = fetch_text(&client, input).await?;
    let play_pages = extract_candidates(input, &body);
    if play_pages.is_empty() {
        return Ok(ResolvedPlayback {
            status: "failed".to_string(),
            candidates: vec![],
            error_message: Some(failed_message.to_string()),
        });
    }

    let mut candidates = Vec::new();
    for (index, play_page) in play_pages.into_iter().enumerate() {
        let page_body = if index == 0 && play_page.url == input {
            body.clone()
        } else {
            match fetch_text(&client, &play_page.url).await {
                Ok(value) => value,
                Err(_) => continue,
            }
        };

        let Some(source_url) = extract_player_url(&page_body) else {
            continue;
        };
        if probe_media_candidate(&client, &source_url).await.is_err() {
            continue;
        }

        candidates.push(PlaybackCandidate {
            url: source_url.clone(),
            label: play_page.label,
            kind: detect_kind(&source_url).to_string(),
            headers: None,
        });
    }

    if candidates.is_empty() {
        return Ok(ResolvedPlayback {
            status: "failed".to_string(),
            candidates: vec![],
            error_message: Some(failed_message.to_string()),
        });
    }

    Ok(ResolvedPlayback {
        status: "ready".to_string(),
        candidates,
        error_message: None,
    })
}

fn extract_auete_play_page_candidates(page_url: &str, body: &str) -> Vec<PlayPageCandidate> {
    let current_label_regex = Regex::new(r#"var\s+playp='([^']+)'"#).unwrap();
    let section_heading_regex = Regex::new(r#"』([^：<]+)："#).unwrap();
    let anchor_regex =
        Regex::new(r#"<a class="btn btn-orange" title="([^"]+)" href="([^"]+)""#).unwrap();

    let Some(current_label) = current_label_regex
        .captures(body)
        .and_then(|capture| capture.get(1))
        .map(|value| value.as_str().trim().to_string())
    else {
        return vec![PlayPageCandidate {
            url: page_url.to_string(),
            label: "默认线路".to_string(),
        }];
    };

    let mut candidates = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for section in body.split(r#"<div id="player_list""#).skip(1) {
        let Some(source_name) = section_heading_regex
            .captures(section)
            .and_then(|capture| capture.get(1))
            .map(|value| value.as_str().trim().to_string())
        else {
            continue;
        };

        for capture in anchor_regex.captures_iter(section) {
            let Some(label) = capture.get(1).map(|value| value.as_str().trim()) else {
                continue;
            };
            if label != current_label {
                continue;
            }
            let Some(href) = capture.get(2).map(|value| value.as_str()) else {
                continue;
            };
            let absolute = absolutize_url(page_url, href);
            if !seen.insert(absolute.clone()) {
                continue;
            }
            candidates.push(PlayPageCandidate {
                url: absolute,
                label: source_name.clone(),
            });
        }
    }

    if candidates.is_empty() {
        candidates.push(PlayPageCandidate {
            url: page_url.to_string(),
            label: "默认线路".to_string(),
        });
    }

    if let Some(index) = candidates
        .iter()
        .position(|candidate| candidate.url == page_url)
    {
        if index != 0 {
            let current = candidates.remove(index);
            candidates.insert(0, current);
        }
    }

    candidates
}

fn extract_wencai_play_page_candidates(page_url: &str, body: &str) -> Vec<PlayPageCandidate> {
    extract_split_play_page_candidates(
        page_url,
        body,
        &["<div class=\"module-tab-item\"", "<div class=\"line\""],
        is_wencai_play_link,
        "默认线路",
    )
}

fn extract_jianpian_play_page_candidates(page_url: &str, body: &str) -> Vec<PlayPageCandidate> {
    if body.contains("vod-play-list-box") && body.contains("jpvod.com") {
        return extract_jpvod_play_page_candidates(page_url, body);
    }
    extract_split_play_page_candidates(
        page_url,
        body,
        &["<div class=\"switch-box-item\"", "<div class=\"from\""],
        is_jianpian_play_link,
        "默认线路",
    )
}

fn extract_jpvod_play_page_candidates(page_url: &str, body: &str) -> Vec<PlayPageCandidate> {
    let section_regex =
        Regex::new(r#"(?s)<section class="[^"]*vod-play-list-box[^"]*"[^>]*>(.*?)</section>"#)
            .unwrap();
    let title_regex = Regex::new(r#"<h2 class="title">([^<]+)</h2>"#).unwrap();
    let anchor_regex = Regex::new(r#"<a[^>]+href="([^"]+)"[^>]*>(.*?)</a>"#).unwrap();

    let mut current_label = None;
    let mut grouped = Vec::new();

    for section in section_regex.captures_iter(body) {
        let Some(content) = section.get(1).map(|value| value.as_str()) else {
            continue;
        };
        let Some(source_name) = title_regex
            .captures(content)
            .and_then(|capture| capture.get(1))
            .map(|value| strip_html(value.as_str()).trim().to_string())
        else {
            continue;
        };

        let mut entries = Vec::new();
        for anchor in anchor_regex.captures_iter(content) {
            let Some(href) = anchor.get(1).map(|value| value.as_str()) else {
                continue;
            };
            let absolute = absolutize_url(page_url, href);
            if !is_jianpian_play_link(&absolute) {
                continue;
            }
            let Some(label) = anchor
                .get(2)
                .map(|value| strip_html(value.as_str()).trim().to_string())
            else {
                continue;
            };
            if absolute == page_url {
                current_label = Some(label.clone());
            }
            entries.push((absolute, label));
        }
        if !entries.is_empty() {
            grouped.push((source_name, entries));
        }
    }

    let mut candidates = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for (source_name, entries) in grouped {
        for (url, label) in entries {
            if current_label.as_deref().is_some_and(|current| current != label) {
                continue;
            }
            if !seen.insert(url.clone()) {
                continue;
            }
            candidates.push(PlayPageCandidate {
                url,
                label: source_name.clone(),
            });
        }
    }

    if candidates.is_empty() {
        candidates.push(PlayPageCandidate {
            url: page_url.to_string(),
            label: "默认线路".to_string(),
        });
    }

    candidates
}

fn extract_split_play_page_candidates(
    page_url: &str,
    body: &str,
    section_markers: &[&str],
    is_valid_play_url: fn(&str) -> bool,
    default_label: &str,
) -> Vec<PlayPageCandidate> {
    let current_label_regex = Regex::new(r#"var\s+playp='([^']+)'"#).unwrap();
    let source_regex = Regex::new(r#"(?s)^.*?>(.*?)</div>"#).unwrap();
    let anchor_regex = Regex::new(r#"<a[^>]+href="([^"]+)"[^>]*>(.*?)</a>"#).unwrap();
    let current_label = current_label_regex
        .captures(body)
        .and_then(|capture| capture.get(1))
        .map(|value| strip_html(value.as_str()).trim().to_string());

    let mut candidates = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for marker in section_markers {
        for section in body.split(marker).skip(1) {
            let Some(source_name) = source_regex
                .captures(section)
                .and_then(|capture| capture.get(1))
                .map(|value| strip_html(value.as_str()).trim().to_string())
            else {
                continue;
            };

            for anchor in anchor_regex.captures_iter(section) {
                let Some(label) = anchor
                    .get(2)
                    .map(|value| strip_html(value.as_str()).trim().to_string())
                else {
                    continue;
                };
                if current_label.as_deref().is_some_and(|current| current != label) {
                    continue;
                }
                let Some(href) = anchor.get(1).map(|value| value.as_str()) else {
                    continue;
                };
                let absolute = absolutize_url(page_url, href);
                if !is_valid_play_url(&absolute) || !seen.insert(absolute.clone()) {
                    continue;
                }
                candidates.push(PlayPageCandidate {
                    url: absolute,
                    label: source_name.clone(),
                });
            }
        }
    }

    if candidates.is_empty() {
        candidates.push(PlayPageCandidate {
            url: page_url.to_string(),
            label: default_label.to_string(),
        });
    }

    if let Some(index) = candidates
        .iter()
        .position(|candidate| candidate.url == page_url)
    {
        if index != 0 {
            let current = candidates.remove(index);
            candidates.insert(0, current);
        }
    }

    candidates
}

fn is_wencai_play_link(url: &str) -> bool {
    url.contains("/play/")
}

fn is_jianpian_play_link(url: &str) -> bool {
    url.contains("/vodplay/") || url.contains("/play/")
}

fn strip_html(value: &str) -> String {
    let tag_regex = Regex::new(r"<[^>]+>").unwrap();
    let decoded = value
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&nbsp;", " ");
    tag_regex.replace_all(&decoded, " ").to_string()
}

async fn probe_media_candidate(client: &reqwest::Client, url: &str) -> Result<(), String> {
    if url.contains(".m3u8") {
        return probe_hls_playlist(client, url).await;
    }

    let response = client
        .get(url)
        .header(reqwest::header::RANGE, "bytes=0-1")
        .header(
            reqwest::header::USER_AGENT,
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36",
        )
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !response.status().is_success() && response.status() != reqwest::StatusCode::PARTIAL_CONTENT
    {
        return Err(format!("stream probe failed: {}", response.status()));
    }

    Ok(())
}

async fn probe_hls_playlist(client: &reqwest::Client, url: &str) -> Result<(), String> {
    let body = fetch_text(client, url).await?;
    if !body.contains("#EXTM3U") {
        return Err("playlist missing EXTM3U header".to_string());
    }

    let media_playlist_url = if body.contains("#EXT-X-STREAM-INF") {
        let variant_url = first_playlist_resource(url, &body)
            .ok_or_else(|| "master playlist missing variant url".to_string())?;
        let variant_body = fetch_text(client, &variant_url).await?;
        if !variant_body.contains("#EXTM3U") {
            return Err("variant playlist missing EXTM3U header".to_string());
        }
        probe_hls_media_playlist(client, &variant_url, &variant_body).await?;
        variant_url
    } else {
        probe_hls_media_playlist(client, url, &body).await?;
        url.to_string()
    };

    if media_playlist_url.is_empty() {
        return Err("media playlist probe failed".to_string());
    }

    Ok(())
}

async fn probe_hls_media_playlist(
    client: &reqwest::Client,
    playlist_url: &str,
    body: &str,
) -> Result<(), String> {
    if let Some(key_url) = extract_hls_key_url(playlist_url, body) {
        probe_binary_resource(client, &key_url).await?;
    }

    let segment_url = first_playlist_resource(playlist_url, body)
        .ok_or_else(|| "media playlist missing segment url".to_string())?;
    probe_binary_resource(client, &segment_url).await
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

async fn probe_binary_resource(client: &reqwest::Client, url: &str) -> Result<(), String> {
    let response = client
        .get(url)
        .header(reqwest::header::RANGE, "bytes=0-1")
        .header(
            reqwest::header::USER_AGENT,
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36",
        )
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.status().is_success() || response.status() == reqwest::StatusCode::PARTIAL_CONTENT {
        Ok(())
    } else {
        Err(format!("resource probe failed: {}", response.status()))
    }
}

fn build_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .no_proxy()
        .connect_timeout(std::time::Duration::from_secs(20))
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|e| e.to_string())
}

async fn fetch_text(client: &reqwest::Client, input: &str) -> Result<String, String> {
    client
        .get(input)
        .header(
            reqwest::header::USER_AGENT,
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36",
        )
        .send()
        .await
        .map_err(|e| e.to_string())?
        .text()
        .await
        .map_err(|e| e.to_string())
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

#[cfg(test)]
mod tests {
    use super::{
        absolutize_url, classify_playback_target, detect_kind, extract_aliplayer_source,
        extract_auete_play_page_candidates, extract_hls_key_url, extract_iframe_src,
        extract_jianpian_play_page_candidates, extract_jpvod_play_page_candidates,
        extract_wencai_play_page_candidates,
        first_playlist_resource, looks_like_auete_play_page, looks_like_jianpian_play_page,
        looks_like_libvio_play_page, looks_like_wencai_play_page, looks_like_xb6v_play_page,
        looks_like_zxzj_play_page, PlaybackResolver,
    };
    use crate::models::ResolvedPlayback;

    #[tokio::test]
    async fn marks_hls_url_as_ready_candidate() {
        let resolved = PlaybackResolver::resolve("https://example.com/live.m3u8")
            .await
            .unwrap();
        assert_eq!(resolved.status, "ready");
        assert_eq!(resolved.candidates[0].kind, "hls");
    }

    #[tokio::test]
    async fn marks_unknown_scheme_as_external_required() {
        let resolved = PlaybackResolver::resolve("drpy://source/detail")
            .await
            .unwrap();
        assert_eq!(resolved.status, "external_required");
    }

    #[tokio::test]
    async fn marks_magnet_as_external_required() {
        let resolved = PlaybackResolver::resolve("magnet:?xt=urn:btih:test")
            .await
            .unwrap();
        assert_eq!(resolved.status, "external_required");
        assert_eq!(resolved.candidates[0].kind, "external");
    }

    #[tokio::test]
    async fn marks_cloud_disk_urls_as_external_required() {
        let resolved = PlaybackResolver::resolve("https://pan.baidu.com/s/example")
            .await
            .unwrap();
        assert_eq!(resolved.status, "external_required");
        assert_eq!(resolved.candidates[0].kind, "external");
        assert_eq!(resolved.candidates[0].url, "https://pan.baidu.com/s/example");
    }

    #[test]
    fn detects_xb6v_play_page() {
        assert!(looks_like_xb6v_play_page(
            "https://www.xb6v.com/e/DownSys/play/?classid=17&id=28598&pathid2=0&bf=1"
        ));
        assert!(looks_like_libvio_play_page(
            "https://www.libvio.me/play/714891197-1-1.html"
        ));
        assert!(looks_like_auete_play_page(
            "https://auete.top/Movie/dzp/xunlongjuemizong/play-0-0.html"
        ));
        assert!(looks_like_wencai_play_page(
            "https://www.wencai.example/play/123-1-1.html"
        ));
        assert!(looks_like_wencai_play_page(
            "https://www.deeyy.com/vod/play/id/1419/sid/1/nid/1.html"
        ));
        assert!(looks_like_jianpian_play_page(
            "https://jpvod.com/play/123-1-1.html"
        ));
        assert!(looks_like_zxzj_play_page(
            "https://www.zxzjhd.com/vodplay/4627-1-1.html"
        ));
        assert_eq!(detect_kind("https://video.example.com/index.m3u8"), "hls");
    }

    #[tokio::test]
    async fn treats_zxzj_play_page_as_embed_candidate() {
        let resolved = PlaybackResolver::resolve("https://www.zxzjhd.com/vodplay/4627-1-1.html")
            .await
            .unwrap();
        assert_eq!(resolved.status, "ready");
        assert_eq!(resolved.candidates[0].kind, "embed");
        assert_eq!(
            resolved.candidates[0].url,
            "https://www.zxzjhd.com/vodplay/4627-1-1.html"
        );
    }

    #[test]
    fn classifies_playback_targets_for_visibility_and_sorting() {
        assert_eq!(
            classify_playback_target("https://media.example.com/video/index.m3u8"),
            "direct"
        );
        assert_eq!(
            classify_playback_target(
                "https://www.xb6v.com/e/DownSys/play/?classid=2&id=28522&pathid2=0&bf=1"
            ),
            "resolvable"
        );
        assert_eq!(
            classify_playback_target("https://www.libvio.me/play/714891197-1-1.html"),
            "resolvable"
        );
        assert_eq!(
            classify_playback_target("https://auete.top/Movie/dzp/xunlongjuemizong/play-0-0.html"),
            "resolvable"
        );
        assert_eq!(
            classify_playback_target("https://www.wencai.example/play/123-1-1.html"),
            "resolvable"
        );
        assert_eq!(
            classify_playback_target("https://jpvod.com/play/888-1-1.html"),
            "resolvable"
        );
        assert_eq!(
            classify_playback_target("https://www.zxzjhd.com/vodplay/4627-1-1.html"),
            "embedded"
        );
        assert_eq!(
            classify_playback_target("magnet:?xt=urn:btih:test"),
            "external"
        );
        assert_eq!(
            classify_playback_target("https://pan.baidu.com/s/example"),
            "external"
        );
    }

    #[test]
    fn classifies_wencai_and_jianpian_play_pages_as_resolvable() {
        assert_eq!(
            classify_playback_target("https://www.wencai.example/play/123-1-1.html"),
            "resolvable"
        );
        assert_eq!(
            classify_playback_target("https://jpvod.com/play/888-1-1.html"),
            "resolvable"
        );
    }

    #[test]
    fn extracts_same_episode_candidates_from_wencai_page() {
        let body = r#"
            <script>var playp='正片';</script>
            <div class="line">文采A</div><a href="/play/1-1-1.html">正片</a>
            <div class="line">文采B</div><a href="/play/1-2-1.html">正片</a>
            <div class="line">文采网盘</div><a href="https://pan.quark.cn/s/demo">合集</a>
        "#;

        let candidates = extract_wencai_play_page_candidates(
            "https://www.wencai.example/play/1-1-1.html",
            body,
        );

        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].label, "文采A");
        assert_eq!(
            candidates[0].url,
            "https://www.wencai.example/play/1-1-1.html"
        );
        assert_eq!(candidates[1].label, "文采B");
        assert_eq!(
            candidates[1].url,
            "https://www.wencai.example/play/1-2-1.html"
        );
    }

    #[test]
    fn extracts_same_episode_candidates_from_jianpian_page() {
        let body = r#"
            <script>var playp='正片';</script>
            <div class="from">荐片A</div><a href="/play/2-1-1.html">正片</a>
            <div class="from">荐片B</div><a href="/play/2-2-1.html">正片</a>
            <div class="from">荐片下载</div><a href="magnet:?xt=urn:btih:test">合集</a>
        "#;

        let candidates = extract_jianpian_play_page_candidates(
            "https://jpvod.com/play/2-1-1.html",
            body,
        );

        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].label, "荐片A");
        assert_eq!(
            candidates[0].url,
            "https://jpvod.com/play/2-1-1.html"
        );
        assert_eq!(candidates[1].label, "荐片B");
        assert_eq!(
            candidates[1].url,
            "https://jpvod.com/play/2-2-1.html"
        );
    }

    #[test]
    fn extracts_same_episode_candidates_from_jpvod_page() {
        let body = r#"
            <section class="section-ryhd6l vod-play-list-box vod-play-list-1 active">
              <div class="section-head-ryhd6l justify-content-start">
                <h2 class="title">金牌资源</h2>
              </div>
              <div class="section-content-ryhd6l">
                <a class="w-100 btn btn-secondary active" href="/play/97910-1-1.html" title="播放第1集">第1集</a>
                <a class="w-100 btn btn-secondary" href="/play/97910-1-2.html" title="播放第2集">第2集</a>
              </div>
            </section>
            <section class="section-ryhd6l vod-play-list-box vod-play-list-2">
              <div class="section-head-ryhd6l justify-content-start">
                <h2 class="title">无尽线路</h2>
              </div>
              <div class="section-content-ryhd6l">
                <a class="w-100 btn btn-secondary" href="/play/97910-2-1.html" title="播放第1集">第1集</a>
                <a class="w-100 btn btn-secondary" href="/play/97910-2-2.html" title="播放第2集">第2集</a>
              </div>
            </section>
        "#;

        let candidates =
            extract_jpvod_play_page_candidates("https://jpvod.com/play/97910-1-1.html", body);

        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].label, "金牌资源");
        assert_eq!(candidates[0].url, "https://jpvod.com/play/97910-1-1.html");
        assert_eq!(candidates[1].label, "无尽线路");
        assert_eq!(candidates[1].url, "https://jpvod.com/play/97910-2-1.html");
    }

    #[test]
    fn extracts_auete_same_episode_play_page_candidates() {
        let body = r#"
            <script>var playp='HD中字';</script>
            <div id="player_list" class="clearfix mt-0">
              <h2>『秒速5厘米真人版』云播D线：</h2>
              <ul><li><a class="btn btn-orange" title="HD中字" href="/Movie/demo/play-0-0.html">HD中字</a></li></ul>
            </div>
            <div id="player_list" class="clearfix mt-0">
              <h2>『秒速5厘米真人版』云播M线：</h2>
              <ul><li><a class="btn btn-orange" title="HD中字" href="/Movie/demo/play-1-0.html">HD中字</a></li></ul>
            </div>
            <div id="player_list" class="clearfix mt-0">
              <h2>『秒速5厘米真人版』网盘：</h2>
              <ul><li><a class="btn btn-orange" title="合集" href="/Movie/demo/play-2-0.html">合集</a></li></ul>
            </div>
        "#;

        let candidates =
            extract_auete_play_page_candidates("https://auete.top/Movie/demo/play-0-0.html", body);

        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].label, "云播D线");
        assert_eq!(
            candidates[0].url,
            "https://auete.top/Movie/demo/play-0-0.html"
        );
        assert_eq!(candidates[1].label, "云播M线");
        assert_eq!(
            candidates[1].url,
            "https://auete.top/Movie/demo/play-1-0.html"
        );
    }

    #[test]
    fn extracts_first_playlist_resource_from_master_and_media_playlists() {
        let master = "#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=800000\n2000k/hls/mixed.m3u8\n";
        let media = "#EXTM3U\n#EXTINF:4.0,\nsegment-001.ts\n";

        assert_eq!(
            first_playlist_resource("https://example.com/root/index.m3u8", master).as_deref(),
            Some("https://example.com/root/2000k/hls/mixed.m3u8")
        );
        assert_eq!(
            first_playlist_resource("https://example.com/root/2000k/hls/index.m3u8", media)
                .as_deref(),
            Some("https://example.com/root/2000k/hls/segment-001.ts")
        );
    }

    #[test]
    fn extracts_hls_key_url() {
        let media = "#EXTM3U\n#EXT-X-KEY:METHOD=AES-128,URI=\"/demo/key.key\"\n#EXTINF:4.0,\nsegment-001.ts\n";
        assert_eq!(
            extract_hls_key_url("https://example.com/root/index.m3u8", media).as_deref(),
            Some("https://example.com/demo/key.key")
        );
    }

    #[test]
    fn extracts_aliplayer_source() {
        let body = r#"var player = new Aliplayer({ "source": "https://example.com/index.m3u8" });"#;
        assert_eq!(
            extract_aliplayer_source(body).as_deref(),
            Some("https://example.com/index.m3u8")
        );
    }

    #[test]
    fn extracts_iframe_and_absolutizes_relative_url() {
        let body = r#"<div class="video"><iframe src="https://vip.dytt-tvs.com/share/demo"></iframe></div>"#;
        assert_eq!(
            extract_iframe_src(
                "https://www.xb6v.com/e/DownSys/play/?classid=6&id=28503&pathid1=0&bf=0",
                body
            )
            .as_deref(),
            Some("https://vip.dytt-tvs.com/share/demo")
        );
        assert_eq!(
            absolutize_url(
                "https://vip.dytt-tvs.com/share/demo",
                "/20260407/15657/index.m3u8"
            ),
            "https://vip.dytt-tvs.com/20260407/15657/index.m3u8"
        );
    }

    #[test]
    fn serializes_error_message_in_camel_case() {
        let resolved = ResolvedPlayback {
            status: "external_required".to_string(),
            candidates: vec![],
            error_message: Some("resolver required".to_string()),
        };

        let json = serde_json::to_value(resolved).unwrap();

        assert_eq!(
            json.get("errorMessage").and_then(|v| v.as_str()),
            Some("resolver required")
        );
        assert!(json.get("error_message").is_none());
    }
}
