use crate::models::{PlaybackCandidate, ResolvedPlayback};
use crate::services::playback_types::{
    PlaybackProbeResult, PlaybackProbeStatus, PlaybackTarget, PlaybackTargetKind,
};
use crate::services::resolver::{
    build_client, classify_playback_target, probe_candidate_for_runtime, PlaybackResolver,
};
use crate::services::storage::{
    playback_cache::{
        get_playback_health, hash_playback_target, list_playback_targets, upsert_playback_health,
    },
    Storage,
};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct RuntimeResolvedCandidate {
    pub target: PlaybackTarget,
    pub probe: PlaybackProbeResult,
}

pub async fn resolve_playback_for_input(
    storage: &Storage,
    input: &str,
) -> Result<ResolvedPlayback, String> {
    let client = build_client()?;
    let normalized = discover_initial_targets(input);
    let resolved = resolve_and_probe_targets(storage, &client, normalized).await?;
    Ok(to_resolved_playback(resolved))
}

pub fn discover_initial_targets(input: &str) -> Vec<PlaybackTarget> {
    vec![build_runtime_target(input, "default", None)]
}

pub fn build_runtime_target(
    url: &str,
    source_key: &str,
    episode_id: Option<i64>,
) -> PlaybackTarget {
    let target_kind = match classify_playback_target(url) {
        "direct" => PlaybackTargetKind::Direct,
        "resolvable" => PlaybackTargetKind::Resolvable,
        "embedded" => PlaybackTargetKind::Embedded,
        _ => PlaybackTargetKind::ExternalRequired,
    };

    PlaybackTarget {
        episode_id,
        source_key: source_key.to_string(),
        target_url: url.to_string(),
        target_kind: target_kind.clone(),
        resolver_key: matches!(target_kind, PlaybackTargetKind::Resolvable)
            .then_some(source_key.to_string()),
        headers: None,
        sort_hint: 0,
        meta: None,
    }
}

pub fn maybe_cached_targets_for_episode(
    storage: &Storage,
    episode_id: i64,
) -> Result<Vec<PlaybackTarget>, String> {
    list_playback_targets(storage, episode_id)
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|record| {
            Ok(PlaybackTarget {
                episode_id: Some(record.episode_id),
                source_key: record.source_key,
                target_url: record.target_url,
                target_kind: parse_target_kind(&record.target_kind),
                resolver_key: record.resolver_key,
                headers: parse_headers_json(record.headers_json.as_deref())?,
                sort_hint: record.sort_hint,
                meta: None,
            })
        })
        .collect()
}

pub async fn resolve_and_probe_targets(
    storage: &Storage,
    client: &reqwest::Client,
    targets: Vec<PlaybackTarget>,
) -> Result<Vec<RuntimeResolvedCandidate>, String> {
    let mut resolved = Vec::new();

    for target in targets {
        match target.target_kind {
            PlaybackTargetKind::Direct => {
                let probe = cached_or_probed_result(storage, client, &target).await?;
                resolved.push(RuntimeResolvedCandidate { target, probe });
            }
            PlaybackTargetKind::Resolvable => {
                let playback = PlaybackResolver::resolve(&target.target_url).await?;
                resolved.extend(expand_resolved_playback(storage, client, &target, playback).await?);
            }
            PlaybackTargetKind::Embedded | PlaybackTargetKind::ExternalRequired => {
                resolved.push(RuntimeResolvedCandidate {
                    target,
                    probe: PlaybackProbeResult::failed("target kind is not desktop playable", None),
                });
            }
        }
    }

    Ok(sort_runtime_candidates(resolved))
}

pub fn sort_runtime_candidates(
    candidates: Vec<RuntimeResolvedCandidate>,
) -> Vec<RuntimeResolvedCandidate> {
    let ranked = crate::services::rank_targets(
        candidates
            .iter()
            .cloned()
            .map(|candidate| (candidate.target, candidate.probe.status))
            .collect(),
    );

    let mut indexed: HashMap<(String, Option<i64>, i32), Vec<RuntimeResolvedCandidate>> = HashMap::new();
    for candidate in candidates {
        indexed
            .entry(runtime_candidate_key(&candidate))
            .or_default()
            .push(candidate);
    }

    let mut sorted = Vec::new();
    for (target, _) in ranked {
        if let Some(bucket) = indexed.get_mut(&(target.target_url, target.episode_id, target.sort_hint)) {
            if let Some(candidate) = bucket.pop() {
                sorted.push(candidate);
            }
        }
    }

    sorted
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
            .map(to_playback_candidate)
            .collect(),
        error_message: None,
    }
}

fn runtime_candidate_key(candidate: &RuntimeResolvedCandidate) -> (String, Option<i64>, i32) {
    (
        candidate.target.target_url.clone(),
        candidate.target.episode_id,
        candidate.target.sort_hint,
    )
}

async fn expand_resolved_playback(
    storage: &Storage,
    client: &reqwest::Client,
    parent: &PlaybackTarget,
    resolved: ResolvedPlayback,
) -> Result<Vec<RuntimeResolvedCandidate>, String> {
    let mut expanded = Vec::new();

    for (index, candidate) in resolved.candidates.into_iter().enumerate() {
        let candidate_url = candidate.url;
        let target = PlaybackTarget {
            episode_id: parent.episode_id,
            source_key: parent.source_key.clone(),
            target_url: candidate_url.clone(),
            target_kind: parse_candidate_kind(&candidate.kind, &candidate_url),
            resolver_key: parent.resolver_key.clone(),
            headers: candidate.headers,
            sort_hint: parent.sort_hint.saturating_add(index as i32),
            meta: Some(candidate.label),
        };

        let probe = if target.is_desktop_playable_kind() {
            cached_or_probed_result(storage, client, &target).await?
        } else {
            PlaybackProbeResult::failed("target kind is not desktop playable", None)
        };

        expanded.push(RuntimeResolvedCandidate { target, probe });
    }

    Ok(expanded)
}

fn parse_candidate_kind(value: &str, url: &str) -> PlaybackTargetKind {
    match value {
        "hls" | "http" => {
            if classify_playback_target(url) == "direct" {
                PlaybackTargetKind::Direct
            } else {
                PlaybackTargetKind::Resolvable
            }
        }
        "embed" => PlaybackTargetKind::Embedded,
        "external" => PlaybackTargetKind::ExternalRequired,
        _ => PlaybackTargetKind::Resolvable,
    }
}

fn parse_target_kind(value: &str) -> PlaybackTargetKind {
    match value {
        "direct" => PlaybackTargetKind::Direct,
        "resolvable" => PlaybackTargetKind::Resolvable,
        "embedded" => PlaybackTargetKind::Embedded,
        _ => PlaybackTargetKind::ExternalRequired,
    }
}

fn parse_headers_json(value: Option<&str>) -> Result<Option<HashMap<String, String>>, String> {
    match value {
        Some(raw) if !raw.trim().is_empty() => serde_json::from_str(raw)
            .map(Some)
            .map_err(|e| e.to_string()),
        _ => Ok(None),
    }
}

async fn cached_or_probed_result(
    storage: &Storage,
    client: &reqwest::Client,
    target: &PlaybackTarget,
) -> Result<PlaybackProbeResult, String> {
    let target_hash = hash_playback_target(&target.target_url, target.headers.as_ref());
    if let Some(probe) = cached_probe_result(storage, &target_hash)? {
        return Ok(probe);
    }

    let probe = probe_candidate_for_runtime(client, &target.target_url, target.headers.as_ref()).await;
    persist_probe_result(storage, &target_hash, &probe)?;
    Ok(probe)
}

fn cached_probe_result(storage: &Storage, target_hash: &str) -> Result<Option<PlaybackProbeResult>, String> {
    let Some(record) = get_playback_health(storage, target_hash).map_err(|e| e.to_string())? else {
        return Ok(None);
    };
    if record.expires_at <= now_epoch_seconds() {
        return Ok(None);
    }

    Ok(Some(PlaybackProbeResult {
        status: if record.status == "playable" {
            PlaybackProbeStatus::Playable
        } else {
            PlaybackProbeStatus::Failed
        },
        manifest_ok: record.manifest_ok,
        segment_ok: record.segment_ok,
        cors_ok: record.cors_ok,
        http_status: record.http_status,
        failure_reason: record.failure_reason,
    }))
}

fn persist_probe_result(
    storage: &Storage,
    target_hash: &str,
    probe: &PlaybackProbeResult,
) -> Result<(), String> {
    let ttl_seconds = match probe.status {
        PlaybackProbeStatus::Playable => 3600,
        PlaybackProbeStatus::Failed => 600,
    };

    upsert_playback_health(
        storage,
        target_hash,
        if matches!(probe.status, PlaybackProbeStatus::Playable) {
            "playable"
        } else {
            "failed"
        },
        probe.manifest_ok,
        probe.segment_ok,
        probe.cors_ok,
        probe.http_status,
        probe.failure_reason.as_deref(),
        ttl_seconds,
    )
    .map_err(|e| e.to_string())
}

fn now_epoch_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_secs() as i64
}

fn to_playback_candidate(candidate: RuntimeResolvedCandidate) -> PlaybackCandidate {
    let kind = match candidate.target.target_kind {
        PlaybackTargetKind::Direct if candidate.target.target_url.contains(".m3u8") => "hls",
        PlaybackTargetKind::Direct => "http",
        PlaybackTargetKind::Resolvable => "http",
        PlaybackTargetKind::Embedded => "embed",
        PlaybackTargetKind::ExternalRequired => "external",
    };

    PlaybackCandidate {
        url: candidate.target.target_url,
        label: candidate
            .target
            .meta
            .unwrap_or_else(|| "默认线路".to_string()),
        kind: kind.to_string(),
        headers: candidate.target.headers,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_runtime_target, filter_presentable_targets, parse_headers_json,
        sort_runtime_candidates, to_resolved_playback, RuntimeResolvedCandidate,
    };
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

    #[test]
    fn builds_resolvable_and_embedded_runtime_targets() {
        let guard = build_runtime_target(
            "guard://csp_JPJGuard/%E8%B4%B1%E8%B4%B1/97910/1/1",
            "guard",
            Some(1),
        );
        let embedded = build_runtime_target(
            "https://www.zxzjhd.com/vodplay/4627-1-1.html",
            "zxzj",
            Some(2),
        );

        assert_eq!(guard.target_kind, PlaybackTargetKind::Resolvable);
        assert_eq!(embedded.target_kind, PlaybackTargetKind::Embedded);
    }

    #[test]
    fn sorts_playable_direct_candidates_ahead_of_failed_resolvable_candidates() {
        let ranked = sort_runtime_candidates(vec![
            RuntimeResolvedCandidate {
                target: target(
                    PlaybackTargetKind::Resolvable,
                    "guard",
                    "guard://csp_JPJGuard/%E8%B4%B1%E8%B4%B1/97910/1/1",
                ),
                probe: PlaybackProbeResult::failed("upstream failed", Some(502)),
            },
            RuntimeResolvedCandidate {
                target: target(
                    PlaybackTargetKind::Direct,
                    "jianpian",
                    "https://cdn.example.com/play/index.m3u8",
                ),
                probe: PlaybackProbeResult::playable(),
            },
        ]);

        assert_eq!(ranked[0].target.target_kind, PlaybackTargetKind::Direct);
        assert_eq!(ranked[1].probe.status, PlaybackProbeStatus::Failed);
    }

    #[test]
    fn parses_optional_headers_json_for_cached_targets() {
        let headers = parse_headers_json(Some(r#"{"Referer":"https://jpvod.com/"}"#))
            .expect("headers should parse")
            .expect("headers should be present");

        assert_eq!(
            headers.get("Referer").map(String::as_str),
            Some("https://jpvod.com/")
        );
    }

    #[test]
    fn prefers_cached_playable_target_over_failing_candidate() {
        let cached_ok = RuntimeResolvedCandidate {
            target: target(
                PlaybackTargetKind::Direct,
                "jianpian",
                "https://cdn.example.com/ok/index.m3u8",
            ),
            probe: PlaybackProbeResult::playable(),
        };

        let failing = RuntimeResolvedCandidate {
            target: target(
                PlaybackTargetKind::Direct,
                "jianpian",
                "https://cdn.example.com/bad/index.m3u8",
            ),
            probe: PlaybackProbeResult::failed("manifest failed", Some(404)),
        };

        let ranked = sort_runtime_candidates(vec![failing, cached_ok]);
        assert!(ranked[0].probe.failure_reason.is_none());
    }
}
