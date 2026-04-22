use super::Storage;
use rusqlite::{params, Result as SqliteResult};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaybackHealthRecord {
    pub target_hash: String,
    pub status: String,
    pub manifest_ok: bool,
    pub segment_ok: bool,
    pub cors_ok: bool,
    pub http_status: Option<i64>,
    pub failure_reason: Option<String>,
    pub checked_at: i64,
    pub expires_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaybackTargetRecord {
    pub episode_id: i64,
    pub source_key: String,
    pub target_url: String,
    pub target_kind: String,
    pub resolver_key: Option<String>,
    pub headers_json: Option<String>,
    pub sort_hint: i32,
}

pub fn replace_playback_targets(
    storage: &Storage,
    episode_id: i64,
    targets: Vec<PlaybackTargetRecord>,
) -> SqliteResult<()> {
    let mut conn = storage.conn.lock().unwrap();
    let tx = conn.transaction()?;

    tx.execute(
        "DELETE FROM playback_targets WHERE episode_id = ?1",
        [episode_id],
    )?;

    for target in targets {
        tx.execute(
            r#"
            INSERT INTO playback_targets (
                episode_id,
                source_key,
                target_url,
                target_kind,
                resolver_key,
                headers_json,
                sort_hint
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            "#,
            params![
                episode_id,
                target.source_key,
                target.target_url,
                target.target_kind,
                target.resolver_key,
                target.headers_json,
                target.sort_hint,
            ],
        )?;
    }

    tx.commit()?;
    Ok(())
}

pub fn get_playback_health(
    storage: &Storage,
    target_hash: &str,
) -> SqliteResult<Option<PlaybackHealthRecord>> {
    let conn = storage.conn.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT target_hash, status, manifest_ok, segment_ok, cors_ok, http_status, failure_reason, checked_at, expires_at
         FROM playback_health
         WHERE target_hash = ?1",
    )?;

    let mut rows = stmt.query([target_hash])?;
    if let Some(row) = rows.next()? {
        Ok(Some(PlaybackHealthRecord {
            target_hash: row.get(0)?,
            status: row.get(1)?,
            manifest_ok: row.get::<_, i64>(2)? != 0,
            segment_ok: row.get::<_, i64>(3)? != 0,
            cors_ok: row.get::<_, i64>(4)? != 0,
            http_status: row.get(5)?,
            failure_reason: row.get(6)?,
            checked_at: row.get(7)?,
            expires_at: row.get(8)?,
        }))
    } else {
        Ok(None)
    }
}

pub fn upsert_playback_health(
    storage: &Storage,
    target_hash: &str,
    status: &str,
    manifest_ok: bool,
    segment_ok: bool,
    cors_ok: bool,
    http_status: Option<i64>,
    failure_reason: Option<&str>,
    ttl_seconds: i64,
) -> SqliteResult<()> {
    let conn = storage.conn.lock().unwrap();
    let checked_at = now_epoch_seconds();
    let expires_at = checked_at.saturating_add(ttl_seconds);

    conn.execute(
        r#"
        INSERT INTO playback_health (
            target_hash,
            status,
            manifest_ok,
            segment_ok,
            cors_ok,
            http_status,
            failure_reason,
            checked_at,
            expires_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
        ON CONFLICT(target_hash) DO UPDATE SET
            status = excluded.status,
            manifest_ok = excluded.manifest_ok,
            segment_ok = excluded.segment_ok,
            cors_ok = excluded.cors_ok,
            http_status = excluded.http_status,
            failure_reason = excluded.failure_reason,
            checked_at = excluded.checked_at,
            expires_at = excluded.expires_at
        "#,
        params![
            target_hash,
            status,
            manifest_ok as i32,
            segment_ok as i32,
            cors_ok as i32,
            http_status,
            failure_reason,
            checked_at,
            expires_at,
        ],
    )?;

    Ok(())
}

pub fn list_playback_targets(
    storage: &Storage,
    episode_id: i64,
) -> SqliteResult<Vec<PlaybackTargetRecord>> {
    let conn = storage.conn.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT episode_id, source_key, target_url, target_kind, resolver_key, headers_json, sort_hint
         FROM playback_targets
         WHERE episode_id = ?1
         ORDER BY sort_hint ASC, id ASC",
    )?;

    let rows = stmt.query_map([episode_id], |row| {
        Ok(PlaybackTargetRecord {
            episode_id: row.get(0)?,
            source_key: row.get(1)?,
            target_url: row.get(2)?,
            target_kind: row.get(3)?,
            resolver_key: row.get(4)?,
            headers_json: row.get(5)?,
            sort_hint: row.get(6)?,
        })
    })?;

    rows.collect()
}

fn now_epoch_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_secs() as i64
}
