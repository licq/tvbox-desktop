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

        Ok(ready_with_candidate(input.to_string(), detect_kind(input)))
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
    if input.contains(".m3u8") {
        "hls"
    } else {
        "http"
    }
}

fn looks_like_xb6v_play_page(input: &str) -> bool {
    input.contains("xb6v.com/e/DownSys/play/")
}

async fn resolve_xb6v_play_page(input: &str) -> Result<ResolvedPlayback, String> {
    let client = reqwest::Client::builder()
        .no_proxy()
        .connect_timeout(std::time::Duration::from_secs(20))
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|e| e.to_string())?;
    let body = client
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
        .map_err(|e| e.to_string())?;

    let source_regex = Regex::new(r#""source"\s*:\s*"([^"]+)""#).unwrap();
    let Some(source_url) = source_regex
        .captures(&body)
        .and_then(|captures| captures.get(1).map(|value| value.as_str().to_string()))
    else {
        return Ok(ResolvedPlayback {
            status: "failed".to_string(),
            candidates: vec![],
            error_message: Some("未能从播放页提取实际视频地址".to_string()),
        });
    };

    Ok(ready_with_candidate(source_url.clone(), detect_kind(&source_url)))
}

#[cfg(test)]
mod tests {
    use super::{detect_kind, looks_like_xb6v_play_page, PlaybackResolver};
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
        assert_eq!(detect_kind("https://video.example.com/index.m3u8"), "hls");
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
