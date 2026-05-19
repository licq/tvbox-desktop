use crate::models::{PlaybackCandidate, ResolvedPlayback};
use crate::services::playback_types::{
    playback_source_rank, PlaybackProbeResult, PlaybackProbeStatus, PlaybackTarget,
    PlaybackTargetKind,
};
use crate::services::resolver::{
    build_client, classify_playback_target, is_known_cdn_url, probe_candidate_for_runtime,
    looks_like_zxzj_play_page, PlaybackResolver,
};
use crate::services::storage::{
    playback_cache::{
        get_playback_health, hash_playback_target, list_playback_targets,
        replace_playback_targets, upsert_playback_health, PlaybackHealthRecord,
        PlaybackTargetRecord,
    },
    Storage,
};

/// Health-based sort order for pre-probe prioritization:
/// (a) playable first, (b) failed-transient second, (c) failed-permanent third, (d) no-record last.
/// Ordering is stable so targets with the same health status preserve insertion order.
fn sort_targets_by_health(
    storage: &Storage,
    targets: &[PlaybackTarget],
) -> Vec<PlaybackTarget> {
    let mut indexed: Vec<(usize, HealthRank, PlaybackTarget)> = Vec::with_capacity(targets.len());

    for (i, target) in targets.iter().enumerate() {
        let rank = health_rank(storage, target);
        indexed.push((i, rank, target.clone()));
    }

    indexed.sort_by(|left, right| {
        // Primary: health rank (lower = more preferred)
        left.1.cmp(&right.1)
            // Stable: preserve insertion order for equal rank
            .then_with(|| left.0.cmp(&right.0))
    });

    indexed.into_iter().map(|(_, _, t)| t).collect()
}

/// Returns the health-based sort priority for a single target.
/// Uses classify_probe_failure internally to distinguish transient vs permanent failures.
fn health_rank(storage: &Storage, target: &PlaybackTarget) -> HealthRank {
    let target_hash = hash_playback_target(
        &target.target_url,
        target.headers.as_ref(),
        target.referer.as_deref(),
    );
    let record = match get_playback_health(storage, &target_hash) {
        Ok(Some(r)) => r,
        Ok(None) | Err(_) => return HealthRank::NoRecord,
    };

    if record.status == "playable" {
        return HealthRank::Playable;
    }

    // Classify failure kind using the existing failure classification logic
    let failure_class = classify_health_failure(target, &record);
    match failure_class {
        // Dead-link (404/410) and CORS failures are permanent-ish — probe less often
        ProbeFailureClass::DeadLink | ProbeFailureClass::BrowserCors => {
            HealthRank::FailedPermanent
        }
        // Embedded / external are disabled — treat as permanent
        ProbeFailureClass::Embedded | ProbeFailureClass::ExternalRequired => {
            HealthRank::FailedPermanent
        }
        // Upstream transient and unknown are more likely to recover
        ProbeFailureClass::UpstreamTransient | ProbeFailureClass::Unsupported | ProbeFailureClass::Unknown => {
            HealthRank::FailedTransient
        }
    }
}

/// Classify a failure purely from a cached PlaybackHealthRecord (no live probe needed).
fn classify_health_failure(_target: &PlaybackTarget, record: &PlaybackHealthRecord) -> ProbeFailureClass {
    match record.http_status {
        Some(403 | 404 | 410) => ProbeFailureClass::DeadLink,
        Some(429 | 500 | 502 | 503 | 504) => ProbeFailureClass::UpstreamTransient,
        _ => {
            if !record.cors_ok || record.failure_reason.as_ref().is_some_and(|r| r.to_ascii_lowercase().contains("cors")) {
                ProbeFailureClass::BrowserCors
            } else if record.failure_reason.as_ref().is_some_and(|r| {
                let normalized = r.to_ascii_lowercase();
                normalized.contains("not desktop playable")
                    || normalized.contains("external")
                    || normalized.contains("embedded")
            }) {
                ProbeFailureClass::Unsupported
            } else {
                ProbeFailureClass::Unknown
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum HealthRank {
    Playable = 0,
    FailedTransient = 1,
    FailedPermanent = 2,
    NoRecord = 3,
}
use futures::stream::{FuturesUnordered, StreamExt};
use log::info;
use std::collections::{HashMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Semaphore;

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
    force_refresh: bool,
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
    let resolved = resolve_and_probe_targets(storage, &client, normalized, force_refresh).await?;
    if let Some(episode_id) = episode_id {
        persist_runtime_targets_for_episode(storage, episode_id, &resolved)?;
    }
    Ok(to_resolved_playback(resolved))
}

pub fn discover_initial_targets(input: &str) -> Vec<PlaybackTarget> {
    vec![build_runtime_target(input, "default", None, Some(input))]
}

pub fn build_runtime_target(
    url: &str,
    source_key: &str,
    episode_id: Option<i64>,
    referer: Option<&str>,
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
        referer: referer.map(|s| s.to_string()),
    }
}

pub fn maybe_cached_targets_for_episode(
    storage: &Storage,
    episode_id: i64,
) -> Result<Vec<PlaybackTarget>, String> {
    let targets = list_playback_targets(storage, episode_id)
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|record| {
            let target_kind = normalize_cached_target_kind(&record.target_kind, &record.target_url);
            let resolver_key = match target_kind {
                PlaybackTargetKind::Resolvable => record
                    .resolver_key
                    .or_else(|| Some(record.source_key.clone())),
                _ => record.resolver_key,
            };

            Ok::<PlaybackTarget, String>(PlaybackTarget {
                episode_id: Some(record.episode_id),
                source_key: record.source_key,
                target_url: record.target_url,
                target_kind,
                resolver_key,
                headers: parse_headers_json(record.headers_json.as_deref())?,
                sort_hint: record.sort_hint,
                meta: record.meta_text,
                referer: None,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    Ok(targets)
}

pub async fn resolve_and_probe_targets(
    storage: &Storage,
    client: &reqwest::Client,
    targets: Vec<PlaybackTarget>,
    force_refresh: bool,
) -> Result<Vec<RuntimeResolvedCandidate>, String> {
    let mut resolved = Vec::new();
    let semaphore = Semaphore::new(2);

    // Separate Direct targets (probed concurrently) from other kinds (handled sequentially)
    let (mut direct_targets, other_targets): (Vec<_>, Vec<_>) = targets
        .into_iter()
        .partition(|t| matches!(t.target_kind, PlaybackTargetKind::Direct));

    // Sort Direct targets by cached health so known-good sources are probed first
    let sorted_direct = sort_targets_by_health(storage, &direct_targets);
    let sorted_probe_order: Vec<String> = sorted_direct
        .iter()
        .map(|t| t.source_key.clone())
        .collect();
    info!(
        "[probe_parallel] sorted_probe_order sources={}",
        sorted_probe_order.join(",")
    );
    direct_targets = sorted_direct;

    // Probe Direct targets concurrently with semaphore(2) limiting
    if !direct_targets.is_empty() {
        let mut futures = FuturesUnordered::new();
        for target in direct_targets {
            let permit = semaphore.acquire().await.expect("semaphore not closed");
            let storage = storage.clone();
            let client = client.clone();
            let target_url = target.target_url.clone();

            info!(
                "[probe_parallel] start target={} url={}",
                target.source_key, target_url
            );
            let start = std::time::Instant::now();

            futures.push(async move {
                let probe =
                    cached_or_probed_result(&storage, &client, &target, force_refresh).await;
                drop(permit); // release permit immediately after probe completes
                let duration_ms = start.elapsed().as_millis() as u64;
                let status = probe
                    .as_ref()
                    .map(|p| format!("{:?}", p.status))
                    .unwrap_or_else(|e| format!("error:{}", e));
                info!(
                    "[probe_parallel] done target={} url={} status={} duration_ms={}",
                    target.source_key, target_url, status, duration_ms
                );
                (target, probe)
            });
        }

        while let Some(result) = futures.next().await {
            let (target, probe) = result;
            resolved.push(RuntimeResolvedCandidate { target, probe: probe? });
        }
    }

    // Handle Resolvable and other target kinds sequentially
    for target in other_targets {
        match target.target_kind {
            PlaybackTargetKind::Resolvable => {
                let playback = PlaybackResolver::resolve(&target.target_url).await?;
                resolved.extend(expand_resolved_playback(
                    storage,
                    client,
                    &target,
                    playback,
                    force_refresh,
                )
                .await?);
            }
            PlaybackTargetKind::Embedded | PlaybackTargetKind::ExternalRequired => {
                // Known play pages (like zxzj) can be rendered in an iframe -
                // mark them as playable so they show up as embed candidates
                if matches!(target.target_kind, PlaybackTargetKind::Embedded)
                    && looks_like_zxzj_play_page(&target.target_url)
                {
                    resolved.push(RuntimeResolvedCandidate {
                        target,
                        probe: PlaybackProbeResult::playable(),
                    });
                } else {
                    resolved.push(RuntimeResolvedCandidate {
                        target,
                        probe: PlaybackProbeResult::failed("target kind is not desktop playable", None),
                    });
                }
            }
            PlaybackTargetKind::Direct => {
                // Already handled in the concurrent branch above
                unreachable!()
            }
        }
    }

    let sorted = sort_runtime_candidates(resolved);
    prioritize_recently_successful_candidates(storage, sorted)
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

fn prioritize_recently_successful_candidates(
    storage: &Storage,
    candidates: Vec<RuntimeResolvedCandidate>,
) -> Result<Vec<RuntimeResolvedCandidate>, String> {
    let mut ranked = Vec::with_capacity(candidates.len());

    for (index, candidate) in candidates.into_iter().enumerate() {
        let recent_success = recent_success_timestamp(storage, &candidate.target)?;
        ranked.push((recent_success, index, candidate));
    }

    ranked.sort_by(|left, right| {
        right
            .0
            .cmp(&left.0)
            .then_with(|| left.1.cmp(&right.1))
    });

    Ok(ranked.into_iter().map(|(_, _, candidate)| candidate).collect())
}

fn recent_success_timestamp(
    storage: &Storage,
    target: &PlaybackTarget,
) -> Result<Option<i64>, String> {
    let target_hash = hash_playback_target(
        &target.target_url,
        target.headers.as_ref(),
        target.referer.as_deref(),
    );
    let Some(record) = get_playback_health(storage, &target_hash).map_err(|e| e.to_string())? else {
        return Ok(None);
    };

    if record.status == "playable" {
        return Ok(Some(record.checked_at));
    }

    Ok(None)
}

pub fn filter_presentable_targets(
    candidates: Vec<RuntimeResolvedCandidate>,
) -> Vec<RuntimeResolvedCandidate> {
    let visible: Vec<_> = candidates
        .into_iter()
        .filter(|candidate| {
            candidate.target.is_desktop_playable_kind()
                && (matches!(candidate.probe.status, PlaybackProbeStatus::Playable)
                    || should_trust_bare_direct_hls(&candidate.target))
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

fn should_trust_bare_direct_hls(target: &PlaybackTarget) -> bool {
    if !matches!(target.target_kind, PlaybackTargetKind::Direct) {
        return false;
    }
    if !target.target_url.to_ascii_lowercase().contains(".m3u8") {
        return false;
    }

    let has_headers = target
        .headers
        .as_ref()
        .is_some_and(|headers| !headers.is_empty());
    let has_referer = target
        .referer
        .as_ref()
        .is_some_and(|referer| {
            let normalized = referer.trim();
            !normalized.is_empty() && normalized != target.target_url
        });

    !has_headers && !has_referer
}

pub fn to_resolved_playback(candidates: Vec<RuntimeResolvedCandidate>) -> ResolvedPlayback {
    let failure_message = summarize_runtime_failures(&candidates);

    // Only desktop-playable candidates (no embed fallback — iframe experience is poor)
    let visible = filter_presentable_targets(candidates);
    if !visible.is_empty() {
        return ResolvedPlayback {
            status: "ready".to_string(),
            candidates: visible
                .into_iter()
                .map(to_playback_candidate)
                .collect(),
            error_message: None,
        };
    }

    // No playable candidates
    ResolvedPlayback {
        status: resolved_failure_status(&failure_message).to_string(),
        candidates: vec![],
        error_message: Some(
            failure_message.unwrap_or_else(|| "当前集未找到通过探测的可播线路".to_string()),
        ),
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
    force_refresh: bool,
) -> Result<Vec<RuntimeResolvedCandidate>, String> {
    let mut expanded = Vec::new();

    for (index, candidate) in resolved.candidates.into_iter().enumerate() {
        let candidate_url = candidate.url;
        let candidate_kind = parse_candidate_kind(&candidate.kind, &candidate_url);
        let target = PlaybackTarget {
            episode_id: parent.episode_id,
            source_key: parent.source_key.clone(),
            target_url: candidate_url.clone(),
            target_kind: candidate_kind,
            resolver_key: parent.resolver_key.clone(),
            headers: candidate.headers,
            sort_hint: parent.sort_hint.saturating_add(index as i32),
            meta: Some(candidate.label),
            referer: candidate.referer.or_else(|| parent.referer.clone()),
        };

        let probe = if target.is_desktop_playable_kind() && !is_known_cdn_url(&target.target_url) {
            cached_or_probed_result(storage, client, &target, force_refresh).await?
        } else if is_known_cdn_url(&target.target_url) {
            // Known CDNs work in browser (CORS headers present) but Rust's native-tls
            // probe may falsely fail due to CDN TLS fingerprint detection. Skip probe
            // and trust the URL since hls.js works correctly in the WebView.
            PlaybackProbeResult::playable()
        } else if matches!(target.target_kind, PlaybackTargetKind::Embedded) {
            // Embed candidates are no longer presented to users (iframe experience is poor).
            PlaybackProbeResult::failed("embed playback disabled", None)
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
        "embed" => PlaybackTargetKind::ExternalRequired,
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

fn normalize_cached_target_kind(value: &str, url: &str) -> PlaybackTargetKind {
    let parsed = parse_target_kind(value);
    if matches!(parsed, PlaybackTargetKind::Direct)
        && classify_playback_target(url) == "resolvable"
    {
        return PlaybackTargetKind::Resolvable;
    }

    parsed
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
    force_refresh: bool,
) -> Result<PlaybackProbeResult, String> {
    let target_hash = hash_playback_target(
        &target.target_url,
        target.headers.as_ref(),
        target.referer.as_deref(),
    );
    if !force_refresh {
        if let Some(probe) = cached_probe_result(storage, &target_hash)? {
            return Ok(probe);
        }
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
        let key = hash_playback_target(
            &candidate.target.target_url,
            candidate.target.headers.as_ref(),
            candidate.target.referer.as_deref(),
        );
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
        referer: candidate.target.referer,
    }
}

fn persist_runtime_targets_for_episode(
    storage: &Storage,
    episode_id: i64,
    candidates: &[RuntimeResolvedCandidate],
) -> Result<(), String> {
    let records = candidates
        .iter()
        .enumerate()
        .map(|(index, candidate)| PlaybackTargetRecord {
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
            referer: candidate.target.referer.clone(),
            meta_text: candidate.target.meta.clone(),
            sort_hint: index as i32,
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
        maybe_cached_targets_for_episode, prioritize_recently_successful_candidates,
        persist_runtime_targets_for_episode, playable_probe_ttl_seconds, probe_ttl_seconds,
        resolve_playback_for_input, resolved_failure_status, sort_runtime_candidates,
        sort_targets_by_health, summarize_runtime_failures, target_kind_label,
        to_resolved_playback, RuntimeResolvedCandidate, ProbeFailureClass,
    };
    use crate::services::playback_types::{
        PlaybackProbeResult, PlaybackProbeStatus, PlaybackTarget, PlaybackTargetKind,
    };
    use crate::services::storage::playback_cache::{
        list_playback_targets, replace_playback_targets, upsert_playback_health,
        PlaybackTargetRecord,
    };
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
            referer: None,
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
                    "auete",
                    "https://cdn.example.com/auete/index.m3u8",
                ),
                probe: PlaybackProbeResult::playable(),
            },
        ];

        let filtered = filter_presentable_targets(candidates);
        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().any(|candidate| candidate.target.source_key == "jianpian"));
        assert!(filtered.iter().any(|candidate| candidate.target.source_key == "auete"));
    }

    #[test]
    fn trusts_bare_direct_hls_candidates_even_when_probe_fails() {
        let candidates = vec![RuntimeResolvedCandidate {
            target: target(
                PlaybackTargetKind::Direct,
                "demo",
                "https://cdn.example.com/live/index.m3u8",
            ),
            probe: PlaybackProbeResult::failed("request failed: 404", Some(404)),
        }];

        let resolved = to_resolved_playback(candidates);

        assert_eq!(resolved.status, "ready");
        assert_eq!(resolved.candidates.len(), 1);
        assert_eq!(resolved.candidates[0].url, "https://cdn.example.com/live/index.m3u8");
    }

    #[test]
    fn still_requires_probe_for_direct_hls_candidates_with_referer() {
        let mut direct = target(
            PlaybackTargetKind::Direct,
            "demo",
            "https://cdn.example.com/live/index.m3u8",
        );
        direct.referer = Some("https://example.com/player".to_string());

        let resolved = to_resolved_playback(vec![RuntimeResolvedCandidate {
            target: direct,
            probe: PlaybackProbeResult::failed("request failed: 404", Some(404)),
        }]);

        assert_eq!(resolved.status, "failed");
        assert!(resolved.candidates.is_empty());
    }

    #[test]
    fn trusts_bare_direct_hls_candidates_with_self_referer() {
        let mut direct = target(
            PlaybackTargetKind::Direct,
            "demo",
            "https://cdn.example.com/live/index.m3u8",
        );
        direct.referer = Some("https://cdn.example.com/live/index.m3u8".to_string());

        let resolved = to_resolved_playback(vec![RuntimeResolvedCandidate {
            target: direct,
            probe: PlaybackProbeResult::failed("request failed: 404", Some(404)),
        }]);

        assert_eq!(resolved.status, "ready");
        assert_eq!(resolved.candidates.len(), 1);
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
    fn prioritizes_recently_successful_candidates_ahead_of_current_order() {
        let storage = Storage::new(unique_test_dir()).expect("storage should initialize");
        let first = target(
            PlaybackTargetKind::Direct,
            "jianpian",
            "https://cdn.example.com/first/index.m3u8",
        );
        let second = target(
            PlaybackTargetKind::Direct,
            "libvio",
            "https://cdn.example.com/second/index.m3u8",
        );
        let second_hash = crate::services::storage::playback_cache::hash_playback_target(
            &second.target_url,
            second.headers.as_ref(),
            second.referer.as_deref(),
        );
        upsert_playback_health(
            &storage,
            &second_hash,
            "playable",
            true,
            true,
            true,
            Some(200),
            None,
            3600,
        )
        .expect("playback health should persist");

        let prioritized = prioritize_recently_successful_candidates(
            &storage,
            vec![
                RuntimeResolvedCandidate {
                    target: first,
                    probe: PlaybackProbeResult::playable(),
                },
                RuntimeResolvedCandidate {
                    target: second,
                    probe: PlaybackProbeResult::playable(),
                },
            ],
        )
        .expect("candidate prioritization should succeed");

        assert_eq!(prioritized[0].target.source_key, "libvio");
        assert_eq!(prioritized[1].target.source_key, "jianpian");
    }

    #[test]
    fn keeps_existing_order_when_no_recent_success_history_exists() {
        let storage = Storage::new(unique_test_dir()).expect("storage should initialize");

        let prioritized = prioritize_recently_successful_candidates(
            &storage,
            vec![
                RuntimeResolvedCandidate {
                    target: target(
                        PlaybackTargetKind::Direct,
                        "jianpian",
                        "https://cdn.example.com/first/index.m3u8",
                    ),
                    probe: PlaybackProbeResult::playable(),
                },
                RuntimeResolvedCandidate {
                    target: target(
                        PlaybackTargetKind::Direct,
                        "libvio",
                        "https://cdn.example.com/second/index.m3u8",
                    ),
                    probe: PlaybackProbeResult::playable(),
                },
            ],
        )
        .expect("candidate prioritization should succeed");

        assert_eq!(prioritized[0].target.source_key, "jianpian");
        assert_eq!(prioritized[1].target.source_key, "libvio");
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
        let mut direct = target(
            PlaybackTargetKind::Direct,
            "jianpian",
            "https://cdn.example.com/dead/index.m3u8",
        );
        direct.referer = Some("https://example.com/player".to_string());

        let candidates = vec![RuntimeResolvedCandidate {
            target: direct,
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
            "guard://csp_JPJGuard/%E8%B4%B1%E8%B4%B1/97910/1/1", "guard", Some(1), None,
        );
        let embedded = build_runtime_target(
            "https://www.zxzjhd.com/vodplay/4627-1-1.html", "zxzj", Some(2), None,
        );

        assert_eq!(guard.target_kind, PlaybackTargetKind::Resolvable);
        assert_eq!(embedded.target_kind, PlaybackTargetKind::Resolvable);
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
    fn hashes_playback_targets_with_referer() {
        let without_referer = crate::services::storage::playback_cache::hash_playback_target(
            "https://cdn.example.com/index.m3u8",
            None,
            None,
        );
        let with_referer = crate::services::storage::playback_cache::hash_playback_target(
            "https://cdn.example.com/index.m3u8",
            None,
            Some("https://www.ypanso.com/vod/play/id/1/sid/1/nid/1.html"),
        );

        assert_ne!(without_referer, with_referer);
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
                    referer: None,
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
        assert_eq!(records[0].sort_hint, 0);
    }

    #[test]
    fn persists_runtime_targets_using_current_resolved_order_for_sort_hint() {
        let storage = Storage::new(unique_test_dir()).expect("storage should initialize");

        persist_runtime_targets_for_episode(
            &storage,
            66,
            &[
                RuntimeResolvedCandidate {
                    target: PlaybackTarget {
                        episode_id: Some(66),
                        source_key: "libvio".to_string(),
                        target_url: "https://cdn.example.com/second/index.m3u8".to_string(),
                        target_kind: PlaybackTargetKind::Direct,
                        resolver_key: Some("libvio".to_string()),
                        headers: None,
                        sort_hint: 9,
                        meta: Some("第二条".to_string()),
                        referer: None,
                    },
                    probe: PlaybackProbeResult::playable(),
                },
                RuntimeResolvedCandidate {
                    target: PlaybackTarget {
                        episode_id: Some(66),
                        source_key: "jianpian".to_string(),
                        target_url: "https://cdn.example.com/first/index.m3u8".to_string(),
                        target_kind: PlaybackTargetKind::Direct,
                        resolver_key: Some("jianpian".to_string()),
                        headers: None,
                        sort_hint: 1,
                        meta: Some("第一条".to_string()),
                        referer: None,
                    },
                    probe: PlaybackProbeResult::playable(),
                },
            ],
        )
        .expect("runtime targets should persist");

        let records = list_playback_targets(&storage, 66).expect("target query should succeed");
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].target_url, "https://cdn.example.com/second/index.m3u8");
        assert_eq!(records[0].sort_hint, 0);
        assert_eq!(records[1].target_url, "https://cdn.example.com/first/index.m3u8");
        assert_eq!(records[1].sort_hint, 1);
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
                    referer: None,
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
    fn upgrades_legacy_cached_play_pages_to_resolvable_targets() {
        let storage = Storage::new(unique_test_dir()).expect("storage should initialize");
        let record = PlaybackTargetRecord {
            episode_id: 91,
            source_key: "xb6v".to_string(),
            target_url: "https://www.xb6v.com/e/DownSys/play/?classid=6&id=28451&pathid1=0&bf=0"
                .to_string(),
            target_kind: "direct".to_string(),
            resolver_key: None,
            headers_json: None,
            referer: None,
            meta_text: None,
            sort_hint: 0,
        };

        replace_playback_targets(&storage, 91, vec![record]).expect("runtime targets should persist");

        let restored =
            maybe_cached_targets_for_episode(&storage, 91).expect("cached targets should restore");

        assert_eq!(restored.len(), 1);
        assert_eq!(restored[0].target_kind, PlaybackTargetKind::Resolvable);
        assert_eq!(restored[0].resolver_key.as_deref(), Some("xb6v"));
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

    #[test]
    fn sorts_playable_targets_ahead_of_no_record_targets() {
        let storage = Storage::new(unique_test_dir()).expect("storage should initialize");

        let first_hash =
            crate::services::storage::playback_cache::hash_playback_target(
                "https://cdn.example.com/second/index.m3u8",
                None,
                None,
            );
        upsert_playback_health(
            &storage,
            &first_hash,
            "playable",
            true,
            true,
            true,
            Some(200),
            None,
            3600,
        )
        .expect("health should persist");

        let targets = vec![
            target(
                PlaybackTargetKind::Direct,
                "jianpian",
                "https://cdn.example.com/first/index.m3u8",
            ),
            target(
                PlaybackTargetKind::Direct,
                "jianpian",
                "https://cdn.example.com/second/index.m3u8",
            ),
        ];

        let sorted = sort_targets_by_health(&storage, &targets);
        assert_eq!(sorted[0].target_url, "https://cdn.example.com/second/index.m3u8");
        assert_eq!(sorted[1].target_url, "https://cdn.example.com/first/index.m3u8");
    }

    #[test]
    fn sorts_failed_transient_targets_ahead_of_failed_permanent_targets() {
        let storage = Storage::new(unique_test_dir()).expect("storage should initialize");

        let transient_hash =
            crate::services::storage::playback_cache::hash_playback_target(
                "https://cdn.example.com/transient/index.m3u8",
                None,
                None,
            );
        upsert_playback_health(
            &storage,
            &transient_hash,
            "failed",
            true,
            true,
            false,
            Some(502),
            Some("upstream timeout"),
            600,
        )
        .expect("health should persist");

        let dead_hash =
            crate::services::storage::playback_cache::hash_playback_target(
                "https://cdn.example.com/dead/index.m3u8",
                None,
                None,
            );
        upsert_playback_health(
            &storage,
            &dead_hash,
            "failed",
            false,
            false,
            true,
            Some(404),
            Some("not found"),
            1800,
        )
        .expect("health should persist");

        let targets = vec![
            target(
                PlaybackTargetKind::Direct,
                "jianpian",
                "https://cdn.example.com/dead/index.m3u8",
            ),
            target(
                PlaybackTargetKind::Direct,
                "jianpian",
                "https://cdn.example.com/transient/index.m3u8",
            ),
        ];

        let sorted = sort_targets_by_health(&storage, &targets);
        assert_eq!(sorted[0].target_url, "https://cdn.example.com/transient/index.m3u8");
        assert_eq!(sorted[1].target_url, "https://cdn.example.com/dead/index.m3u8");
    }

    #[test]
    fn sorts_no_record_targets_after_all_others() {
        let storage = Storage::new(unique_test_dir()).expect("storage should initialize");

        let known_hash =
            crate::services::storage::playback_cache::hash_playback_target(
                "https://cdn.example.com/known/index.m3u8",
                None,
                None,
            );
        upsert_playback_health(
            &storage,
            &known_hash,
            "playable",
            true,
            true,
            true,
            Some(200),
            None,
            3600,
        )
        .expect("health should persist");

        let targets = vec![
            target(
                PlaybackTargetKind::Direct,
                "jianpian",
                "https://cdn.example.com/unknown/index.m3u8",
            ),
            target(
                PlaybackTargetKind::Direct,
                "jianpian",
                "https://cdn.example.com/known/index.m3u8",
            ),
        ];

        let sorted = sort_targets_by_health(&storage, &targets);
        assert_eq!(sorted[0].target_url, "https://cdn.example.com/known/index.m3u8");
        assert_eq!(sorted[1].target_url, "https://cdn.example.com/unknown/index.m3u8");
    }

    #[test]
    fn preserves_insertion_order_for_targets_with_equal_health_rank() {
        let storage = Storage::new(unique_test_dir()).expect("storage should initialize");

        let targets = vec![
            target(
                PlaybackTargetKind::Direct,
                "jianpian",
                "https://cdn.example.com/alpha/index.m3u8",
            ),
            target(
                PlaybackTargetKind::Direct,
                "jianpian",
                "https://cdn.example.com/beta/index.m3u8",
            ),
            target(
                PlaybackTargetKind::Direct,
                "jianpian",
                "https://cdn.example.com/gamma/index.m3u8",
            ),
        ];

        let sorted = sort_targets_by_health(&storage, &targets);
        assert_eq!(sorted[0].target_url, "https://cdn.example.com/alpha/index.m3u8");
        assert_eq!(sorted[1].target_url, "https://cdn.example.com/beta/index.m3u8");
        assert_eq!(sorted[2].target_url, "https://cdn.example.com/gamma/index.m3u8");
    }

    #[tokio::test]
    #[ignore = "requires live upstream access"]
    async fn dead_direct_hls_is_filtered_by_runtime() {
        let storage = Storage::new(unique_test_dir()).expect("storage should initialize");
        let resolved = resolve_playback_for_input(
            &storage,
            "https://example.invalid/runtime-dead/index.m3u8",
            None,
            false,
        )
        .await
        .expect("runtime should resolve");

        assert_eq!(resolved.status, "failed");
        assert!(resolved.candidates.is_empty());
    }

    #[tokio::test]
    #[ignore = "requires live upstream access"]
    async fn resolves_live_xb6v_play_page_to_hls() {
        let storage = Storage::new(unique_test_dir()).expect("storage should initialize");
        let play_url =
            "https://www.xb6v.com/e/DownSys/play/?classid=8&id=28379&pathid1=0&bf=0";

        let resolved = resolve_playback_for_input(&storage, play_url, None, false)
            .await
            .expect("xb6v runtime should resolve play page");

        println!("xb6v resolved={resolved:#?}");
        assert_eq!(resolved.status, "ready", "xb6v should resolve to a ready HLS stream, got: {resolved:#?}");
        assert!(!resolved.candidates.is_empty(), "xb6v should have at least one candidate");
        assert!(
            resolved.candidates.iter().any(|c| c.url.contains(".m3u8")),
            "xb6v candidate should be an HLS stream"
        );
    }

    #[tokio::test]
    #[ignore = "requires live upstream access"]
    async fn resolves_live_xb6v_pathid3_play_page_to_hls() {
        let storage = Storage::new(unique_test_dir()).expect("storage should initialize");
        let play_url =
            "https://www.xb6v.com/e/DownSys/play/?classid=2&id=28522&pathid3=0&bf=2";

        let resolved = resolve_playback_for_input(&storage, play_url, None, false)
            .await
            .expect("xb6v runtime should resolve play page");

        println!("xb6v pathid3 resolved={resolved:#?}");
        assert_eq!(
            resolved.status,
            "ready",
            "xb6v pathid3 should resolve to a ready HLS stream, got: {resolved:#?}"
        );
        assert!(
            resolved.candidates.iter().any(|c| c.url.contains(".m3u8")),
            "xb6v pathid3 candidate should be an HLS stream"
        );
    }

    fn unique_test_dir() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("tvbox-playback-runtime-test-{}", nanos))
    }
}
