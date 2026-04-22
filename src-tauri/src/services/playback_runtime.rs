use crate::models::{PlaybackCandidate, ResolvedPlayback};
use crate::services::playback_types::{
    PlaybackProbeResult, PlaybackProbeStatus, PlaybackTarget, PlaybackTargetKind,
};

#[derive(Debug, Clone)]
pub struct RuntimeResolvedCandidate {
    pub target: PlaybackTarget,
    pub probe: PlaybackProbeResult,
}

pub fn filter_presentable_targets(
    candidates: Vec<RuntimeResolvedCandidate>,
) -> Vec<RuntimeResolvedCandidate> {
    candidates
        .into_iter()
        .filter(|candidate| {
            candidate.target.is_desktop_playable_kind()
                && matches!(candidate.probe.status, PlaybackProbeStatus::Playable)
        })
        .collect()
}

pub fn to_resolved_playback(candidates: Vec<RuntimeResolvedCandidate>) -> ResolvedPlayback {
    let visible = filter_presentable_targets(candidates);
    if visible.is_empty() {
        return ResolvedPlayback {
            status: "failed".to_string(),
            candidates: vec![],
            error_message: Some("当前集未找到通过探测的可播线路".to_string()),
        };
    }

    ResolvedPlayback {
        status: "ready".to_string(),
        candidates: visible
            .into_iter()
            .map(|candidate| PlaybackCandidate {
                url: candidate.target.target_url,
                label: "默认线路".to_string(),
                kind: match candidate.target.target_kind {
                    PlaybackTargetKind::Direct => "hls".to_string(),
                    PlaybackTargetKind::Resolvable => "http".to_string(),
                    PlaybackTargetKind::Embedded => "embed".to_string(),
                    PlaybackTargetKind::ExternalRequired => "external".to_string(),
                },
                headers: candidate.target.headers,
            })
            .collect(),
        error_message: None,
    }
}

#[cfg(test)]
mod tests {
    use super::{filter_presentable_targets, to_resolved_playback, RuntimeResolvedCandidate};
    use crate::services::playback_types::{
        PlaybackProbeResult, PlaybackProbeStatus, PlaybackTarget, PlaybackTargetKind,
    };

    fn target(kind: PlaybackTargetKind, source_key: &str, url: &str) -> PlaybackTarget {
        PlaybackTarget {
            episode_id: Some(1),
            source_key: source_key.to_string(),
            target_url: url.to_string(),
            target_kind: kind,
            resolver_key: None,
            headers: None,
            sort_hint: 0,
            meta: None,
        }
    }

    #[test]
    fn filters_out_embedded_and_external_targets() {
        let candidates = vec![
            RuntimeResolvedCandidate {
                target: target(
                    PlaybackTargetKind::Embedded,
                    "zxzj",
                    "https://www.zxzjhd.com/vodplay/4627-1-1.html",
                ),
                probe: PlaybackProbeResult::failed("embedded", None),
            },
            RuntimeResolvedCandidate {
                target: target(
                    PlaybackTargetKind::ExternalRequired,
                    "magnet",
                    "magnet:?xt=urn:btih:test",
                ),
                probe: PlaybackProbeResult::failed("external", None),
            },
            RuntimeResolvedCandidate {
                target: target(
                    PlaybackTargetKind::Direct,
                    "jianpian",
                    "https://cdn.example.com/play/index.m3u8",
                ),
                probe: PlaybackProbeResult {
                    status: PlaybackProbeStatus::Playable,
                    manifest_ok: true,
                    segment_ok: true,
                    cors_ok: true,
                    http_status: Some(200),
                    failure_reason: None,
                },
            },
        ];

        let filtered = filter_presentable_targets(candidates);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].target.source_key, "jianpian");
    }

    #[test]
    fn maps_empty_runtime_result_to_failed_resolved_playback() {
        let resolved = to_resolved_playback(vec![]);
        assert_eq!(resolved.status, "failed");
        assert_eq!(
            resolved.error_message.as_deref(),
            Some("当前集未找到通过探测的可播线路")
        );
        assert!(resolved.candidates.is_empty());
    }
}
