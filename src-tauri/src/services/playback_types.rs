use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlaybackTargetKind {
    Direct,
    Resolvable,
    Embedded,
    ExternalRequired,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlaybackProbeStatus {
    Playable,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaybackProbeResult {
    pub status: PlaybackProbeStatus,
    pub manifest_ok: bool,
    pub segment_ok: bool,
    pub cors_ok: bool,
    pub http_status: Option<i64>,
    pub failure_reason: Option<String>,
}

impl PlaybackProbeResult {
    pub fn playable() -> Self {
        Self {
            status: PlaybackProbeStatus::Playable,
            manifest_ok: true,
            segment_ok: true,
            cors_ok: true,
            http_status: Some(200),
            failure_reason: None,
        }
    }

    pub fn failed(reason: impl Into<String>, http_status: Option<i64>) -> Self {
        Self {
            status: PlaybackProbeStatus::Failed,
            manifest_ok: false,
            segment_ok: false,
            cors_ok: false,
            http_status,
            failure_reason: Some(reason.into()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaybackTarget {
    pub episode_id: Option<i64>,
    pub source_key: String,
    pub target_url: String,
    pub target_kind: PlaybackTargetKind,
    pub resolver_key: Option<String>,
    pub headers: Option<HashMap<String, String>>,
    pub sort_hint: i32,
    pub meta: Option<String>,
}

impl PlaybackTarget {
    pub fn is_desktop_playable_kind(&self) -> bool {
        self.target_kind.is_probe_gate()
    }
}

impl PlaybackTargetKind {
    pub fn is_probe_gate(&self) -> bool {
        matches!(self, PlaybackTargetKind::Direct | PlaybackTargetKind::Resolvable)
    }
}

pub fn rank_targets(
    mut entries: Vec<(PlaybackTarget, PlaybackProbeStatus)>,
) -> Vec<(PlaybackTarget, PlaybackProbeStatus)> {
    entries.sort_by_key(|(target, status)| {
        let kind_rank = match target.target_kind {
            PlaybackTargetKind::Direct => 0,
            PlaybackTargetKind::Resolvable => 1,
            PlaybackTargetKind::Embedded => 2,
            PlaybackTargetKind::ExternalRequired => 3,
        };
        let probe_rank = match status {
            PlaybackProbeStatus::Playable => 0,
            PlaybackProbeStatus::Failed => 1,
        };
        (probe_rank, kind_rank, target.sort_hint)
    });
    entries
}

#[cfg(test)]
mod tests {
    use super::{
        PlaybackProbeResult, PlaybackProbeStatus, PlaybackTarget, PlaybackTargetKind, rank_targets,
    };

    fn target(kind: PlaybackTargetKind, sort_hint: i32) -> PlaybackTarget {
        PlaybackTarget {
            episode_id: Some(1),
            source_key: "source".to_string(),
            target_url: "https://example.com/stream.m3u8".to_string(),
            target_kind: kind,
            resolver_key: None,
            headers: None,
            sort_hint,
            meta: None,
        }
    }

    #[test]
    fn ranks_playable_direct_targets_ahead_of_resolvable_targets() {
        let ranked = rank_targets(vec![
            (
                target(PlaybackTargetKind::Resolvable, 0),
                PlaybackProbeStatus::Playable,
            ),
            (
                target(PlaybackTargetKind::Direct, 0),
                PlaybackProbeStatus::Playable,
            ),
        ]);

        assert_eq!(ranked[0].0.target_kind, PlaybackTargetKind::Direct);
        assert_eq!(ranked[1].0.target_kind, PlaybackTargetKind::Resolvable);
    }

    #[test]
    fn marks_embedded_targets_as_never_playable() {
        let embedded = target(PlaybackTargetKind::Embedded, 0);

        assert!(!embedded.is_desktop_playable_kind());
    }

    #[test]
    fn ranks_failed_targets_after_playable_targets() {
        let ranked = rank_targets(vec![
            (target(PlaybackTargetKind::Direct, 0), PlaybackProbeStatus::Failed),
            (
                target(PlaybackTargetKind::Resolvable, 0),
                PlaybackProbeStatus::Playable,
            ),
        ]);

        assert_eq!(ranked[0].1, PlaybackProbeStatus::Playable);
        assert_eq!(ranked[1].1, PlaybackProbeStatus::Failed);
    }

    #[test]
    fn ranks_external_required_targets_after_embedded_targets() {
        let ranked = rank_targets(vec![
            (
                target(PlaybackTargetKind::ExternalRequired, 0),
                PlaybackProbeStatus::Playable,
            ),
            (
                target(PlaybackTargetKind::Embedded, 0),
                PlaybackProbeStatus::Playable,
            ),
        ]);

        assert_eq!(ranked[0].0.target_kind, PlaybackTargetKind::Embedded);
        assert_eq!(ranked[1].0.target_kind, PlaybackTargetKind::ExternalRequired);
    }

    #[test]
    fn marks_embedded_target_kind_as_not_probeable() {
        assert!(!PlaybackTargetKind::Embedded.is_probe_gate());
        assert!(PlaybackTargetKind::Direct.is_probe_gate());
    }

    #[test]
    fn builds_playable_probe_result_with_success_metadata() {
        let probe = PlaybackProbeResult::playable();

        assert_eq!(probe.status, PlaybackProbeStatus::Playable);
        assert!(probe.manifest_ok);
        assert!(probe.segment_ok);
        assert!(probe.cors_ok);
        assert_eq!(probe.http_status, Some(200));
        assert_eq!(probe.failure_reason, None);
    }

    #[test]
    fn builds_failed_probe_result_with_failure_metadata() {
        let probe = PlaybackProbeResult::failed("embedded", Some(403));

        assert_eq!(probe.status, PlaybackProbeStatus::Failed);
        assert!(!probe.manifest_ok);
        assert!(!probe.segment_ok);
        assert!(!probe.cors_ok);
        assert_eq!(probe.http_status, Some(403));
        assert_eq!(probe.failure_reason.as_deref(), Some("embedded"));
    }
}
