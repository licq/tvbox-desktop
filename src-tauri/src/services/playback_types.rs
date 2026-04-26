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
            http_status: None,
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
        let source_rank = playback_source_rank(&target.source_key);
        (probe_rank, kind_rank, source_rank, target.sort_hint)
    });
    entries
}

pub fn playback_source_rank(source_key: &str) -> i32 {
    let normalized = source_key.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "auete" | "wencai" | "jianpian" | "csp_jpysguard" | "csp_jpjguard" => 0,
        "libvio" => 1,
        "xb6v" => 2,
        "default" | "guard" => 3,
        "zxzj" => 4,
        // Dynamic sources from TVBox config: rank based on known prefixes
        s if s.starts_with("csp_") => 0, // All CSP sources ranked as preferred
        _ => 5, // Unknown sources ranked lowest
    }
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
    fn ranks_preferred_source_families_ahead_of_generic_sources() {
        let mut generic = target(PlaybackTargetKind::Direct, 0);
        generic.source_key = "default".to_string();

        let mut preferred = target(PlaybackTargetKind::Direct, 0);
        preferred.source_key = "csp_JPJGuard".to_string();

        let ranked = rank_targets(vec![
            (generic, PlaybackProbeStatus::Playable),
            (preferred, PlaybackProbeStatus::Playable),
        ]);

        assert_eq!(ranked[0].0.source_key, "csp_JPJGuard");
        assert_eq!(ranked[1].0.source_key, "default");
    }

    #[test]
    fn ranks_zxzj_after_preferred_source_families_when_probe_status_matches() {
        let mut preferred = target(PlaybackTargetKind::Resolvable, 0);
        preferred.source_key = "libvio".to_string();

        let mut zxzj = target(PlaybackTargetKind::Resolvable, 0);
        zxzj.source_key = "zxzj".to_string();

        let ranked = rank_targets(vec![
            (zxzj, PlaybackProbeStatus::Playable),
            (preferred, PlaybackProbeStatus::Playable),
        ]);

        assert_eq!(ranked[0].0.source_key, "libvio");
        assert_eq!(ranked[1].0.source_key, "zxzj");
    }

    #[test]
    fn ranks_guard_and_auete_ahead_of_libvio() {
        let mut top = target(PlaybackTargetKind::Direct, 0);
        top.source_key = "csp_JPJGuard".to_string();

        let mut lower = target(PlaybackTargetKind::Direct, 0);
        lower.source_key = "libvio".to_string();

        let ranked = rank_targets(vec![
            (lower, PlaybackProbeStatus::Playable),
            (top, PlaybackProbeStatus::Playable),
        ]);

        assert_eq!(ranked[0].0.source_key, "csp_JPJGuard");
        assert_eq!(ranked[1].0.source_key, "libvio");
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
        assert_eq!(probe.http_status, None);
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
