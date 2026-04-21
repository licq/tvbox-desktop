use crate::models::{PlaybackCandidate, ResolvedPlayback};
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

        if looks_like_xb6v_play_page(input) {
            return resolve_xb6v_play_page(input).await;
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
    {
        return "external";
    }

    if looks_like_zxzj_play_page(input) {
        return "embedded";
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

fn looks_like_zxzj_play_page(input: &str) -> bool {
    (input.contains("zxzjhd.com/") || input.contains("zxzjys.com/")) && input.contains("/vodplay/")
}

async fn resolve_xb6v_play_page(input: &str) -> Result<ResolvedPlayback, String> {
    let client = build_client()?;
    let body = fetch_text(&client, input).await?;
    if let Some(source_url) = extract_aliplayer_source(&body) {
        return Ok(ready_with_candidate(source_url.clone(), detect_kind(&source_url)));
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
    iframe_regex
        .captures(body)
        .and_then(|captures| captures.get(1).map(|value| absolutize_url(page_url, value.as_str())))
}

async fn resolve_embedded_share_page(
    client: &reqwest::Client,
    iframe_url: &str,
) -> Result<ResolvedPlayback, String> {
    let body = fetch_text(client, iframe_url).await?;
    let share_url_regex = Regex::new(r#"const\s+url\s*=\s*"([^"]+)""#).unwrap();
    let Some(source_url) = share_url_regex
        .captures(&body)
        .and_then(|captures| captures.get(1).map(|value| absolutize_url(iframe_url, value.as_str())))
    else {
        return Ok(ResolvedPlayback {
            status: "failed".to_string(),
            candidates: vec![],
            error_message: Some("未能从分享页提取实际视频地址".to_string()),
        });
    };

    Ok(ready_with_candidate(source_url.clone(), detect_kind(&source_url)))
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
        extract_iframe_src, looks_like_xb6v_play_page, looks_like_zxzj_play_page,
        PlaybackResolver,
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

    #[test]
    fn detects_xb6v_play_page() {
        assert!(looks_like_xb6v_play_page(
            "https://www.xb6v.com/e/DownSys/play/?classid=17&id=28598&pathid2=0&bf=1"
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
            classify_playback_target("https://www.zxzjhd.com/vodplay/4627-1-1.html"),
            "embedded"
        );
        assert_eq!(
            classify_playback_target("magnet:?xt=urn:btih:test"),
            "external"
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
            absolutize_url("https://vip.dytt-tvs.com/share/demo", "/20260407/15657/index.m3u8"),
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

        assert_eq!(json.get("errorMessage").and_then(|v| v.as_str()), Some("resolver required"));
        assert!(json.get("error_message").is_none());
    }
}
