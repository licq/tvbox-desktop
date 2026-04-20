use crate::models::{PlaybackCandidate, ResolvedPlayback};

pub struct PlaybackResolver;

impl PlaybackResolver {
    pub fn resolve(input: &str) -> Result<ResolvedPlayback, String> {
        if input.starts_with("drpy://") {
            return Ok(ResolvedPlayback {
                status: "external_required".to_string(),
                candidates: vec![],
                error_message: Some(
                    "Current desktop build does not execute drpy rules directly".to_string(),
                ),
            });
        }

        let kind = if input.contains(".m3u8") { "hls" } else { "http" };

        Ok(ResolvedPlayback {
            status: "ready".to_string(),
            candidates: vec![PlaybackCandidate {
                url: input.to_string(),
                label: "默认线路".to_string(),
                kind: kind.to_string(),
                headers: None,
            }],
            error_message: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::PlaybackResolver;
    use crate::models::ResolvedPlayback;

    #[test]
    fn marks_hls_url_as_ready_candidate() {
        let resolved = PlaybackResolver::resolve("https://example.com/live.m3u8").unwrap();
        assert_eq!(resolved.status, "ready");
        assert_eq!(resolved.candidates[0].kind, "hls");
    }

    #[test]
    fn marks_unknown_scheme_as_external_required() {
        let resolved = PlaybackResolver::resolve("drpy://source/detail").unwrap();
        assert_eq!(resolved.status, "external_required");
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
