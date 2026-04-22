use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlaybackTargetKind {
    Direct,
    Resolvable,
    Embedded,
    ExternalRequired,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlaybackProbeStatus {
    Playable,
    Failed,
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
        matches!(
            self.target_kind,
            PlaybackTargetKind::Direct | PlaybackTargetKind::Resolvable
        )
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
    use super::{PlaybackProbeStatus, PlaybackTarget, PlaybackTargetKind, rank_targets};

    #[test]
    fn ranks_playable_direct_targets_ahead_of_resolvable_targets() {
        let direct = PlaybackTarget {
            episode_id: Some(1),
            source_key: "jianpian".to_string(),
            target_url: "https://cdn.example.com/ok/index.m3u8".to_string(),
            target_kind: PlaybackTargetKind::Direct,
            resolver_key: None,
            headers: None,
            sort_hint: 0,
            meta: None,
        };
        let resolvable = PlaybackTarget {
            episode_id: Some(1),
            source_key: "guard".to_string(),
            target_url: "guard://csp_JPJGuard/%E8%B4%B1%E8%B4%B1/97910/1/1".to_string(),
            target_kind: PlaybackTargetKind::Resolvable,
            resolver_key: Some("guard".to_string()),
            headers: None,
            sort_hint: 0,
            meta: None,
        };

        let ranked = rank_targets(vec![
            (resolvable, PlaybackProbeStatus::Playable),
            (direct, PlaybackProbeStatus::Playable),
        ]);

        assert_eq!(ranked[0].0.target_kind, PlaybackTargetKind::Direct);
        assert_eq!(ranked[1].0.target_kind, PlaybackTargetKind::Resolvable);
    }

    #[test]
    fn marks_embedded_targets_as_never_playable() {
        let embedded = PlaybackTarget {
            episode_id: Some(2),
            source_key: "zxzj".to_string(),
            target_url: "https://www.zxzjhd.com/vodplay/4627-1-1.html".to_string(),
            target_kind: PlaybackTargetKind::Embedded,
            resolver_key: None,
            headers: None,
            sort_hint: 0,
            meta: None,
        };

        assert!(!embedded.is_desktop_playable_kind());
    }
}
