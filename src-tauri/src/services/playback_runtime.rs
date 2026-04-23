use crate::models::{PlaybackCandidate, ResolvedPlayback};
use crate::services::playback_types::{
    playback_source_rank, PlaybackProbeResult, PlaybackProbeStatus, PlaybackTarget,
    PlaybackTargetKind,
};
use crate::services::resolver::{
    build_client, classify_playback_target, probe_candidate_for_runtime, PlaybackResolver,
};
use crate::services::storage::{
    playback_cache::{
        get_playback_health, hash_playback_target, list_playback_targets,
        replace_playback_targets, upsert_playback_health, PlaybackTargetRecord,
    },
    Storage,
};
use std::collections::{HashMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct RuntimeResolvedCandidate {
    pub target: PlaybackTarget,
    pub probe: PlaybackProbeResult,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProbeFailureClass {
    Embedded,
    ExternalRequired,
    BrowserCors,
    DeadLink,
    UpstreamTransient,
    Unsupported,
    Unknown,
}

pub async fn resolve_playback_for_input(
    storage: &Storage,
    input: &str,
    episode_id: Option<i64>,
) -> Result<ResolvedPlayback, String> {
    let client = build_client()?;
    let normalized = if let Some(episode_id) = episode_id {
        let cached = maybe_cached_targets_for_episode(storage, episode_id)?;
        if cached.is_empty() {
            discover_initial_targets(input)
        } else {
            cached
        }
    } else {
        discover_initial_targets(input)
    };
    let resolved = resolve_and_probe_targets(storage, &client, normalized).await?;
    if let Some(episode_id) = episode_id {
        persist_runtime_targets_for_episode(storage, episode_id, &resolved)?;
    }
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
                meta: record.meta_text,
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
    let visible: Vec<_> = candidates
        .into_iter()
        .filter(|candidate| {
            candidate.target.is_desktop_playable_kind()
                && matches!(candidate.probe.status, PlaybackProbeStatus::Playable)
        })
        .collect();

    let Some(best_source_rank) = visible
        .iter()
        .map(|candidate| playback_source_rank(&candidate.target.source_key))
        .min()
    else {
        return visible;
    };

    let source_filtered: Vec<_> = visible
        .into_iter()
        .filter(|candidate| {
            playback_source_rank(&candidate.target.source_key) == best_source_rank
        })
        .collect();

    dedupe_presentable_targets(source_filtered)
}

pub fn to_resolved_playback(candidates: Vec<RuntimeResolvedCandidate>) -> ResolvedPlayback {
    let failure_message = summarize_runtime_failures(&candidates);
    let visible = filter_presentable_targets(candidates);
    if visible.is_empty() {
        return ResolvedPlayback {
            status: resolved_failure_status(&failure_message).to_string(),
            candidates: vec![],
            error_message: Some(
                failure_message.unwrap_or_else(|| "当前集未找到通过探测的可播线路".to_string()),
            ),
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
    persist_probe_result(storage, target, &target_hash, &probe)?;
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
    target: &PlaybackTarget,
    target_hash: &str,
    probe: &PlaybackProbeResult,
) -> Result<(), String> {
    let ttl_seconds = probe_ttl_seconds(target, probe);

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

fn probe_ttl_seconds(target: &PlaybackTarget, probe: &PlaybackProbeResult) -> i64 {
    match probe.status {
        PlaybackProbeStatus::Playable => playable_probe_ttl_seconds(&target.source_key),
        PlaybackProbeStatus::Failed => failed_probe_ttl_seconds(target, probe),
    }
}

fn playable_probe_ttl_seconds(source_key: &str) -> i64 {
    let normalized = source_key.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "libvio" | "auete" | "wencai" | "jianpian" | "csp_jpysguard" | "csp_jpjguard" => 7200,
        "xb6v" => 5400,
        "zxzj" => 1800,
        _ => 3600,
    }
}

fn failed_probe_ttl_seconds(target: &PlaybackTarget, probe: &PlaybackProbeResult) -> i64 {
    match classify_probe_failure(target, probe) {
        ProbeFailureClass::DeadLink | ProbeFailureClass::BrowserCors => {
            failed_probe_penalty_ttl_seconds(&target.source_key)
        }
        ProbeFailureClass::Embedded | ProbeFailureClass::ExternalRequired => 3600,
        ProbeFailureClass::UpstreamTransient
        | ProbeFailureClass::Unsupported
        | ProbeFailureClass::Unknown => 600,
    }
}

fn failed_probe_penalty_ttl_seconds(source_key: &str) -> i64 {
    let normalized = source_key.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "libvio" | "auete" | "wencai" | "jianpian" | "csp_jpysguard" | "csp_jpjguard" => 1800,
        "zxzj" => 900,
        _ => 1200,
    }
}

fn classify_probe_failure(target: &PlaybackTarget, probe: &PlaybackProbeResult) -> ProbeFailureClass {
    if matches!(target.target_kind, PlaybackTargetKind::Embedded) {
        return ProbeFailureClass::Embedded;
    }
    if matches!(target.target_kind, PlaybackTargetKind::ExternalRequired) {
        return ProbeFailureClass::ExternalRequired;
    }

    if matches!(probe.http_status, Some(403 | 404 | 410)) {
        return ProbeFailureClass::DeadLink;
    }
    if matches!(probe.http_status, Some(429 | 500 | 502 | 503 | 504))
        || probe
            .failure_reason
            .as_deref()
            .is_some_and(|reason| {
                let normalized = reason.to_ascii_lowercase();
                normalized.contains("timeout")
                    || normalized.contains("timed out")
                    || normalized.contains("tempor")
                    || normalized.contains("upstream")
            })
    {
        return ProbeFailureClass::UpstreamTransient;
    }
    if !probe.cors_ok
        || probe
            .failure_reason
            .as_deref()
            .is_some_and(|reason| reason.to_ascii_lowercase().contains("cors"))
    {
        return ProbeFailureClass::BrowserCors;
    }
    if probe
        .failure_reason
        .as_deref()
        .is_some_and(|reason| {
            let normalized = reason.to_ascii_lowercase();
            normalized.contains("not desktop playable")
                || normalized.contains("external")
                || normalized.contains("embedded")
        })
    {
        return ProbeFailureClass::Unsupported;
    }

    ProbeFailureClass::Unknown
}

fn summarize_runtime_failures(candidates: &[RuntimeResolvedCandidate]) -> Option<String> {
    if candidates.is_empty() {
        return None;
    }

    let mut dead_link = 0;
    let mut browser_cors = 0;
    let mut transient = 0;
    let mut embedded = 0;
    let mut external = 0;
    let mut unsupported = 0;
    let mut unknown = 0;

    for candidate in candidates {
        match classify_probe_failure(&candidate.target, &candidate.probe) {
            ProbeFailureClass::DeadLink => dead_link += 1,
            ProbeFailureClass::BrowserCors => browser_cors += 1,
            ProbeFailureClass::UpstreamTransient => transient += 1,
            ProbeFailureClass::Embedded => embedded += 1,
            ProbeFailureClass::ExternalRequired => external += 1,
            ProbeFailureClass::Unsupported => unsupported += 1,
            ProbeFailureClass::Unknown => unknown += 1,
        }
    }

    let classified_total =
        dead_link + browser_cors + transient + embedded + external + unsupported + unknown;
    if classified_total == 0 {
        return None;
    }

    let top = [
        (
            dead_link,
            "当前集可解析到的线路大多已失效",
        ),
        (
            browser_cors,
            "当前集线路返回了资源，但浏览器环境无法直接访问",
        ),
        (
            transient,
            "当前集线路暂时不可用，可能是上游波动",
        ),
        (
            embedded,
            "当前集只有站内嵌页线路，桌面端未直接展示",
        ),
        (
            external,
            "当前集只有外部工具线路，桌面端未直接展示",
        ),
        (
            unsupported,
            "当前集线路类型当前桌面端不支持直接播放",
        ),
        (
            unknown,
            "当前集未找到通过探测的可播线路",
        ),
    ]
    .into_iter()
    .max_by_key(|(count, _)| *count)?;

    Some(top.1.to_string())
}

fn dedupe_presentable_targets(
    candidates: Vec<RuntimeResolvedCandidate>,
) -> Vec<RuntimeResolvedCandidate> {
    let mut seen = HashSet::new();
    let mut deduped = Vec::new();

    for candidate in candidates {
        let key = hash_playback_target(&candidate.target.target_url, candidate.target.headers.as_ref());
        if seen.insert(key) {
            deduped.push(candidate);
        }
    }

    deduped
}

fn resolved_failure_status(failure_message: &Option<String>) -> &'static str {
    match failure_message.as_deref() {
        Some("当前集只有外部工具线路，桌面端未直接展示") => "external_required",
        _ => "failed",
    }
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

fn persist_runtime_targets_for_episode(
    storage: &Storage,
    episode_id: i64,
    candidates: &[RuntimeResolvedCandidate],
) -> Result<(), String> {
    let records = candidates
        .iter()
        .map(|candidate| PlaybackTargetRecord {
            episode_id,
            source_key: candidate.target.source_key.clone(),
            target_url: candidate.target.target_url.clone(),
            target_kind: target_kind_label(&candidate.target.target_kind).to_string(),
            resolver_key: candidate.target.resolver_key.clone(),
            headers_json: candidate
                .target
                .headers
                .as_ref()
                .and_then(|headers| serde_json::to_string(headers).ok()),
            meta_text: candidate.target.meta.clone(),
            sort_hint: candidate.target.sort_hint,
        })
        .collect();

    replace_playback_targets(storage, episode_id, records).map_err(|e| e.to_string())
}

fn target_kind_label(kind: &PlaybackTargetKind) -> &'static str {
    match kind {
        PlaybackTargetKind::Direct => "direct",
        PlaybackTargetKind::Resolvable => "resolvable",
        PlaybackTargetKind::Embedded => "embedded",
        PlaybackTargetKind::ExternalRequired => "external_required",
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_runtime_target, classify_probe_failure, dedupe_presentable_targets,
        filter_presentable_targets, parse_headers_json, failed_probe_ttl_seconds,
        maybe_cached_targets_for_episode,
        persist_runtime_targets_for_episode, playable_probe_ttl_seconds, probe_ttl_seconds,
        resolved_failure_status, summarize_runtime_failures, target_kind_label, ProbeFailureClass,
        resolve_playback_for_input, sort_runtime_candidates, to_resolved_playback,
        RuntimeResolvedCandidate,
    };
    use crate::services::playback_types::{
        PlaybackProbeResult, PlaybackProbeStatus, PlaybackTarget, PlaybackTargetKind,
    };
    use crate::services::storage::playback_cache::list_playback_targets;
    use crate::services::Storage;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

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
    fn prefers_best_playable_source_family_when_multiple_candidates_are_playable() {
        let candidates = vec![
            RuntimeResolvedCandidate {
                target: target(
                    PlaybackTargetKind::Direct,
                    "default",
                    "https://cdn.example.com/default/index.m3u8",
                ),
                probe: PlaybackProbeResult::playable(),
            },
            RuntimeResolvedCandidate {
                target: target(
                    PlaybackTargetKind::Direct,
                    "jianpian",
                    "https://cdn.example.com/jianpian/index.m3u8",
                ),
                probe: PlaybackProbeResult::playable(),
            },
        ];

        let filtered = filter_presentable_targets(candidates);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].target.source_key, "jianpian");
    }

    #[test]
    fn keeps_multiple_playable_candidates_from_same_best_source_tier() {
        let candidates = vec![
            RuntimeResolvedCandidate {
                target: target(
                    PlaybackTargetKind::Direct,
                    "jianpian",
                    "https://cdn.example.com/jianpian/index.m3u8",
                ),
                probe: PlaybackProbeResult::playable(),
            },
            RuntimeResolvedCandidate {
                target: target(
                    PlaybackTargetKind::Direct,
                    "libvio",
                    "https://cdn.example.com/libvio/index.m3u8",
                ),
                probe: PlaybackProbeResult::playable(),
            },
        ];

        let filtered = filter_presentable_targets(candidates);
        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().any(|candidate| candidate.target.source_key == "jianpian"));
        assert!(filtered.iter().any(|candidate| candidate.target.source_key == "libvio"));
    }

    #[test]
    fn dedupes_presentable_candidates_with_same_url_and_headers() {
        let candidates = vec![
            RuntimeResolvedCandidate {
                target: target(
                    PlaybackTargetKind::Direct,
                    "jianpian",
                    "https://cdn.example.com/shared/index.m3u8",
                ),
                probe: PlaybackProbeResult::playable(),
            },
            RuntimeResolvedCandidate {
                target: target(
                    PlaybackTargetKind::Direct,
                    "libvio",
                    "https://cdn.example.com/shared/index.m3u8",
                ),
                probe: PlaybackProbeResult::playable(),
            },
        ];

        let deduped = dedupe_presentable_targets(candidates);
        assert_eq!(deduped.len(), 1);
        assert_eq!(deduped[0].target.target_url, "https://cdn.example.com/shared/index.m3u8");
    }

    #[test]
    fn keeps_candidates_with_same_url_but_different_headers() {
        let mut first = target(
            PlaybackTargetKind::Direct,
            "jianpian",
            "https://cdn.example.com/shared/index.m3u8",
        );
        let mut second = target(
            PlaybackTargetKind::Direct,
            "libvio",
            "https://cdn.example.com/shared/index.m3u8",
        );
        let mut first_headers = std::collections::HashMap::new();
        first_headers.insert("Referer".to_string(), "https://jpvod.com/".to_string());
        let mut second_headers = std::collections::HashMap::new();
        second_headers.insert("Referer".to_string(), "https://www.libvio.me/".to_string());
        first.headers = Some(first_headers);
        second.headers = Some(second_headers);

        let deduped = dedupe_presentable_targets(vec![
            RuntimeResolvedCandidate {
                target: first,
                probe: PlaybackProbeResult::playable(),
            },
            RuntimeResolvedCandidate {
                target: second,
                probe: PlaybackProbeResult::playable(),
            },
        ]);

        assert_eq!(deduped.len(), 2);
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
    fn summarizes_dead_link_failures_for_failed_runtime_result() {
        let candidates = vec![RuntimeResolvedCandidate {
            target: target(
                PlaybackTargetKind::Direct,
                "jianpian",
                "https://cdn.example.com/dead/index.m3u8",
            ),
            probe: PlaybackProbeResult::failed("manifest failed", Some(404)),
        }];

        assert_eq!(
            summarize_runtime_failures(&candidates).as_deref(),
            Some("当前集可解析到的线路大多已失效")
        );

        let resolved = to_resolved_playback(candidates);
        assert_eq!(
            resolved.error_message.as_deref(),
            Some("当前集可解析到的线路大多已失效")
        );
    }

    #[test]
    fn summarizes_embedded_only_failures_for_failed_runtime_result() {
        let candidates = vec![RuntimeResolvedCandidate {
            target: target(
                PlaybackTargetKind::Embedded,
                "zxzj",
                "https://www.zxzjhd.com/vodplay/4627-1-1.html",
            ),
            probe: PlaybackProbeResult::failed("target kind is not desktop playable", None),
        }];

        assert_eq!(
            summarize_runtime_failures(&candidates).as_deref(),
            Some("当前集只有站内嵌页线路，桌面端未直接展示")
        );
    }

    #[test]
    fn maps_external_only_failures_to_external_required_status() {
        let candidates = vec![RuntimeResolvedCandidate {
            target: target(
                PlaybackTargetKind::ExternalRequired,
                "default",
                "magnet:?xt=urn:btih:test",
            ),
            probe: PlaybackProbeResult::failed("external", None),
        }];

        let failure_message = summarize_runtime_failures(&candidates);
        assert_eq!(
            failure_message.as_deref(),
            Some("当前集只有外部工具线路，桌面端未直接展示")
        );
        assert_eq!(resolved_failure_status(&failure_message), "external_required");

        let resolved = to_resolved_playback(candidates);
        assert_eq!(resolved.status, "external_required");
    }

    #[test]
    fn keeps_embedded_only_failures_as_failed_status() {
        let candidates = vec![RuntimeResolvedCandidate {
            target: target(
                PlaybackTargetKind::Embedded,
                "zxzj",
                "https://www.zxzjhd.com/vodplay/4627-1-1.html",
            ),
            probe: PlaybackProbeResult::failed("target kind is not desktop playable", None),
        }];

        let failure_message = summarize_runtime_failures(&candidates);
        assert_eq!(resolved_failure_status(&failure_message), "failed");
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

    #[test]
    fn persists_runtime_targets_with_headers_for_episode_cache() {
        let storage = Storage::new(unique_test_dir()).expect("storage should initialize");
        let mut headers = std::collections::HashMap::new();
        headers.insert("Referer".to_string(), "https://jpvod.com/".to_string());

        persist_runtime_targets_for_episode(
            &storage,
            55,
            &[RuntimeResolvedCandidate {
                target: PlaybackTarget {
                    episode_id: Some(55),
                    source_key: "jianpian".to_string(),
                    target_url: "https://cdn.example.com/play/index.m3u8".to_string(),
                    target_kind: PlaybackTargetKind::Direct,
                    resolver_key: Some("jianpian".to_string()),
                    headers: Some(headers),
                    sort_hint: 1,
                    meta: Some("无尽线路".to_string()),
                },
                probe: PlaybackProbeResult::playable(),
            }],
        )
        .expect("runtime targets should persist");

        let records = list_playback_targets(&storage, 55).expect("target query should succeed");
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].target_kind, "direct");
        assert_eq!(records[0].resolver_key.as_deref(), Some("jianpian"));
        assert!(records[0]
            .headers_json
            .as_deref()
            .is_some_and(|value| value.contains("Referer")));
        assert_eq!(records[0].meta_text.as_deref(), Some("无尽线路"));
    }

    #[test]
    fn restores_cached_runtime_target_metadata_for_episode_cache() {
        let storage = Storage::new(unique_test_dir()).expect("storage should initialize");

        persist_runtime_targets_for_episode(
            &storage,
            88,
            &[RuntimeResolvedCandidate {
                target: PlaybackTarget {
                    episode_id: Some(88),
                    source_key: "wencai".to_string(),
                    target_url: "https://cdn.example.com/play/index.m3u8".to_string(),
                    target_kind: PlaybackTargetKind::Direct,
                    resolver_key: Some("wencai".to_string()),
                    headers: None,
                    sort_hint: 2,
                    meta: Some("文采线路A:第1集".to_string()),
                },
                probe: PlaybackProbeResult::playable(),
            }],
        )
        .expect("runtime targets should persist");

        let restored =
            maybe_cached_targets_for_episode(&storage, 88).expect("cached targets should restore");

        assert_eq!(restored.len(), 1);
        assert_eq!(restored[0].meta.as_deref(), Some("文采线路A:第1集"));
        assert_eq!(restored[0].resolver_key.as_deref(), Some("wencai"));
    }

    #[test]
    fn maps_target_kind_to_storage_label() {
        assert_eq!(target_kind_label(&PlaybackTargetKind::Direct), "direct");
        assert_eq!(
            target_kind_label(&PlaybackTargetKind::ExternalRequired),
            "external_required"
        );
    }

    #[test]
    fn gives_high_confidence_source_families_longer_playable_probe_ttl() {
        assert_eq!(playable_probe_ttl_seconds("csp_JPJGuard"), 7200);
        assert_eq!(playable_probe_ttl_seconds("libvio"), 7200);
        assert_eq!(playable_probe_ttl_seconds("default"), 3600);
    }

    #[test]
    fn gives_dead_links_longer_failure_ttl_for_high_confidence_source_families() {
        let target = target(
            PlaybackTargetKind::Direct,
            "jianpian",
            "https://cdn.example.com/dead/index.m3u8",
        );
        let probe = PlaybackProbeResult::failed("manifest failed", Some(404));

        assert_eq!(failed_probe_ttl_seconds(&target, &probe), 1800);
        assert_eq!(probe_ttl_seconds(&target, &probe), 1800);
    }

    #[test]
    fn keeps_transient_failures_on_short_ttl() {
        let target = target(
            PlaybackTargetKind::Direct,
            "default",
            "https://cdn.example.com/transient/index.m3u8",
        );
        let probe = PlaybackProbeResult::failed("upstream timeout", Some(502));

        assert_eq!(failed_probe_ttl_seconds(&target, &probe), 600);
        assert_eq!(probe_ttl_seconds(&target, &probe), 600);
    }

    #[test]
    fn classifies_dead_links_by_http_status() {
        let target = target(
            PlaybackTargetKind::Direct,
            "default",
            "https://cdn.example.com/dead/index.m3u8",
        );
        let probe = PlaybackProbeResult::failed("manifest failed", Some(404));

        assert_eq!(classify_probe_failure(&target, &probe), ProbeFailureClass::DeadLink);
    }

    #[test]
    fn classifies_browser_cors_failures_from_probe_metadata() {
        let target = target(
            PlaybackTargetKind::Direct,
            "default",
            "https://cdn.example.com/cors/index.m3u8",
        );
        let probe = PlaybackProbeResult {
            status: PlaybackProbeStatus::Failed,
            manifest_ok: true,
            segment_ok: true,
            cors_ok: false,
            http_status: Some(200),
            failure_reason: Some("resource probe missing browser CORS headers".to_string()),
        };

        assert_eq!(classify_probe_failure(&target, &probe), ProbeFailureClass::BrowserCors);
    }

    #[test]
    fn classifies_upstream_timeouts_as_transient() {
        let target = target(
            PlaybackTargetKind::Direct,
            "default",
            "https://cdn.example.com/transient/index.m3u8",
        );
        let probe = PlaybackProbeResult::failed("upstream timeout", Some(502));

        assert_eq!(
            classify_probe_failure(&target, &probe),
            ProbeFailureClass::UpstreamTransient
        );
    }

    #[test]
    fn classifies_embedded_targets_as_embedded_failures() {
        let target = target(
            PlaybackTargetKind::Embedded,
            "zxzj",
            "https://www.zxzjhd.com/vodplay/4627-1-1.html",
        );
        let probe = PlaybackProbeResult::failed("target kind is not desktop playable", None);

        assert_eq!(classify_probe_failure(&target, &probe), ProbeFailureClass::Embedded);
    }

    #[tokio::test]
    #[ignore = "requires live upstream access"]
    async fn dead_direct_hls_is_filtered_by_runtime() {
        let storage = Storage::new(unique_test_dir()).expect("storage should initialize");
        let resolved = resolve_playback_for_input(
            &storage,
            "https://example.invalid/runtime-dead/index.m3u8",
            None,
        )
        .await
        .expect("runtime should resolve");

        assert_eq!(resolved.status, "failed");
        assert!(resolved.candidates.is_empty());
    }

    fn unique_test_dir() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("tvbox-playback-runtime-test-{}", nanos))
    }
}
