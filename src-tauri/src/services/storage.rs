use rusqlite::{Connection, Result as SqliteResult};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::models::{
    CatalogDetail, CatalogDetailItem, CatalogEpisode, CatalogEpisodeGroup, ChannelSource,
    DoubanHot, HomeCatalogItem, HomePayload, LiveChannel, LiveChannelGroup, LiveChannelGroupItem,
    MergedLiveChannel, PlayHistory, SourceHealthSummary, Subscription, VodItem,
};
use crate::services::tvbox::{
    TvboxConfigRecords, TvboxLiveRecord, TvboxParseRecord, TvboxSiteRecord,
};
use crate::services::xb6v::{runtime_targets_for_item, ScrapedCatalogItem};
use crate::services::{is_visible_playback_target, playback_sort_rank};
use self::playback_cache::PlaybackTargetRecord;

#[path = "playback_cache.rs"]
pub(crate) mod playback_cache;

pub struct Storage {
    conn: Arc<Mutex<Connection>>,
}

impl Storage {
    pub fn new(app_data_dir: PathBuf) -> SqliteResult<Self> {
        std::fs::create_dir_all(&app_data_dir)
            .map_err(|e| rusqlite::Error::InvalidPath(format!("无法创建目录: {}", e).into()))?;

        let db_path = app_data_dir.join("tvbox.db");
        let conn = Connection::open(db_path)?;
        conn.pragma_update(None, "foreign_keys", "ON")?;

        let storage = Self {
            conn: Arc::new(Mutex::new(conn)),
        };

        storage.init_tables()?;
        Ok(storage)
    }

    fn init_tables(&self) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "CREATE TABLE IF NOT EXISTS subscriptions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                url TEXT NOT NULL,
                enabled INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )",
            [],
        )?;

        conn.execute(
            "ALTER TABLE subscriptions ADD COLUMN kind TEXT NOT NULL DEFAULT 'simple_json'",
            [],
        )
        .ok();
        conn.execute(
            "ALTER TABLE subscriptions ADD COLUMN last_refreshed_at TEXT",
            [],
        )
        .ok();
        conn.execute("ALTER TABLE subscriptions ADD COLUMN last_error TEXT", [])
            .ok();

        conn.execute(
            "CREATE TABLE IF NOT EXISTS source_configs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                subscription_id INTEGER NOT NULL,
                config_kind TEXT NOT NULL,
                raw_content TEXT NOT NULL,
                parsed_at TEXT NOT NULL,
                FOREIGN KEY (subscription_id) REFERENCES subscriptions(id) ON DELETE CASCADE
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS source_sites (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                subscription_id INTEGER NOT NULL,
                site_key TEXT NOT NULL,
                site_name TEXT NOT NULL,
                api TEXT,
                ext TEXT,
                searchable INTEGER NOT NULL DEFAULT 1,
                quick_search INTEGER NOT NULL DEFAULT 0,
                filterable INTEGER NOT NULL DEFAULT 0,
                source_type TEXT NOT NULL,
                raw_json TEXT NOT NULL,
                FOREIGN KEY (subscription_id) REFERENCES subscriptions(id) ON DELETE CASCADE
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS source_parses (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                subscription_id INTEGER NOT NULL,
                parse_name TEXT NOT NULL,
                parse_url TEXT NOT NULL,
                source_type INTEGER,
                header_json TEXT,
                raw_json TEXT NOT NULL,
                FOREIGN KEY (subscription_id) REFERENCES subscriptions(id) ON DELETE CASCADE
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS source_lives (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                subscription_id INTEGER NOT NULL,
                group_name TEXT,
                channel_name TEXT NOT NULL,
                raw_url TEXT NOT NULL,
                normalized_url TEXT,
                source_type INTEGER,
                raw_json TEXT NOT NULL,
                FOREIGN KEY (subscription_id) REFERENCES subscriptions(id) ON DELETE CASCADE
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS live_channels (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                subscription_id INTEGER NOT NULL,
                name TEXT NOT NULL,
                logo TEXT,
                url TEXT NOT NULL,
                category TEXT,
                FOREIGN KEY (subscription_id) REFERENCES subscriptions(id) ON DELETE CASCADE
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS catalog_items (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                subscription_id INTEGER NOT NULL,
                site_id INTEGER,
                source_item_key TEXT,
                title TEXT NOT NULL,
                item_type TEXT NOT NULL,
                poster TEXT,
                summary TEXT,
                detail_json TEXT,
                updated_at TEXT NOT NULL,
                FOREIGN KEY (subscription_id) REFERENCES subscriptions(id) ON DELETE CASCADE
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS catalog_episodes (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                catalog_item_id INTEGER NOT NULL,
                source_name TEXT,
                season_label TEXT,
                episode_label TEXT NOT NULL,
                play_url TEXT NOT NULL,
                order_index INTEGER NOT NULL DEFAULT 0,
                extra_json TEXT,
                FOREIGN KEY (catalog_item_id) REFERENCES catalog_items(id) ON DELETE CASCADE
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS vod_items (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                subscription_id INTEGER NOT NULL,
                name TEXT NOT NULL,
                vtype TEXT NOT NULL,
                poster TEXT,
                description TEXT,
                episodes TEXT NOT NULL,
                FOREIGN KEY (subscription_id) REFERENCES subscriptions(id) ON DELETE CASCADE
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS play_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                item_type TEXT NOT NULL,
                item_id INTEGER NOT NULL,
                progress REAL NOT NULL,
                last_played TEXT NOT NULL,
                UNIQUE(item_type, item_id)
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_live_channels_subscription ON live_channels(subscription_id)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_live_channels_category ON live_channels(category)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_vod_items_subscription ON vod_items(subscription_id)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_vod_items_type ON vod_items(vtype)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_catalog_items_subscription ON catalog_items(subscription_id)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_catalog_items_updated_at ON catalog_items(updated_at DESC)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_catalog_episodes_item ON catalog_episodes(catalog_item_id)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_play_history_item ON play_history(item_type, item_id)",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS playback_targets (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                episode_id INTEGER NOT NULL,
                source_key TEXT NOT NULL,
                target_url TEXT NOT NULL,
                target_kind TEXT NOT NULL,
                resolver_key TEXT,
                headers_json TEXT,
                meta_text TEXT,
                sort_hint INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;
        conn.execute("ALTER TABLE playback_targets ADD COLUMN meta_text TEXT", [])
            .ok();

        conn.execute(
            "CREATE TABLE IF NOT EXISTS playback_health (
                target_hash TEXT PRIMARY KEY,
                status TEXT NOT NULL,
                manifest_ok INTEGER NOT NULL DEFAULT 0,
                segment_ok INTEGER NOT NULL DEFAULT 0,
                cors_ok INTEGER NOT NULL DEFAULT 0,
                http_status INTEGER,
                failure_reason TEXT,
                checked_at INTEGER NOT NULL,
                expires_at INTEGER NOT NULL
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_playback_targets_episode_id
             ON playback_targets(episode_id)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_playback_health_expires_at
             ON playback_health(expires_at)",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS douban_hot (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                year INTEGER,
                poster TEXT,
                rating REAL,
                rank INTEGER NOT NULL,
                updated_at TEXT NOT NULL
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_douban_name ON douban_hot(name)",
            [],
        )?;

        Ok(())
    }

    pub fn update_subscription_refresh_state(
        &self,
        id: i64,
        kind: &str,
        refreshed_at: &str,
        last_error: Option<&str>,
    ) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE subscriptions
             SET kind = ?1, last_refreshed_at = ?2, last_error = ?3, updated_at = ?2
             WHERE id = ?4",
            rusqlite::params![kind, refreshed_at, last_error, id],
        )?;
        Ok(())
    }

    pub fn record_subscription_refresh_failure(
        &self,
        id: i64,
        kind: &str,
        last_error: &str,
    ) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        let now = chrono_now();
        conn.execute(
            "UPDATE subscriptions
             SET kind = ?1, last_error = ?2, updated_at = ?3
             WHERE id = ?4",
            rusqlite::params![kind, last_error, now, id],
        )?;
        Ok(())
    }

    pub fn replace_source_config(
        &self,
        subscription_id: i64,
        config_kind: &str,
        raw_content: &str,
        parsed_at: &str,
    ) -> SqliteResult<()> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        tx.execute(
            "DELETE FROM source_configs WHERE subscription_id = ?1",
            [subscription_id],
        )?;
        tx.execute(
            "INSERT INTO source_configs (subscription_id, config_kind, raw_content, parsed_at)
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![subscription_id, config_kind, raw_content, parsed_at],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn clear_source_config_cache(&self, subscription_id: i64) -> SqliteResult<()> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        tx.execute(
            "DELETE FROM source_configs WHERE subscription_id = ?1",
            [subscription_id],
        )?;
        tx.execute(
            "DELETE FROM source_sites WHERE subscription_id = ?1",
            [subscription_id],
        )?;
        tx.execute(
            "DELETE FROM source_parses WHERE subscription_id = ?1",
            [subscription_id],
        )?;
        tx.execute(
            "DELETE FROM source_lives WHERE subscription_id = ?1",
            [subscription_id],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn replace_tvbox_source_records(
        &self,
        subscription_id: i64,
        parsed: &TvboxConfigRecords,
    ) -> SqliteResult<()> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;

        tx.execute(
            "DELETE FROM source_sites WHERE subscription_id = ?1",
            [subscription_id],
        )?;
        tx.execute(
            "DELETE FROM source_parses WHERE subscription_id = ?1",
            [subscription_id],
        )?;
        tx.execute(
            "DELETE FROM source_lives WHERE subscription_id = ?1",
            [subscription_id],
        )?;

        insert_tvbox_sites(&tx, subscription_id, &parsed.sites)?;
        insert_tvbox_parses(&tx, subscription_id, &parsed.parses)?;
        insert_tvbox_lives(&tx, subscription_id, &parsed.lives)?;

        tx.commit()?;
        Ok(())
    }

    pub fn add_subscription(&self, name: &str, url: &str) -> SqliteResult<Subscription> {
        let conn = self.conn.lock().unwrap();
        let now = chrono_now();

        conn.execute(
            "INSERT INTO subscriptions (name, url, enabled, created_at, updated_at) VALUES (?1, ?2, 1, ?3, ?3)",
            [name, url, &now],
        )?;

        let id = conn.last_insert_rowid();

        Ok(Subscription {
            id,
            name: name.to_string(),
            url: url.to_string(),
            kind: "simple_json".to_string(),
            enabled: true,
            last_refreshed_at: None,
            last_error: None,
            created_at: now.clone(),
            updated_at: now,
        })
    }

    pub fn get_subscription(&self, id: i64) -> SqliteResult<Subscription> {
        let conn = self.conn.lock().unwrap();

        conn.query_row(
            "SELECT id, name, url, kind, enabled, last_refreshed_at, last_error, created_at, updated_at
             FROM subscriptions WHERE id = ?1",
            [id],
            |row| {
                Ok(Subscription {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    url: row.get(2)?,
                    kind: row.get(3)?,
                    enabled: row.get::<_, i32>(4)? != 0,
                    last_refreshed_at: row.get(5)?,
                    last_error: row.get(6)?,
                    created_at: row.get(7)?,
                    updated_at: row.get(8)?,
                })
            },
        )
    }

    pub fn get_subscriptions(&self) -> SqliteResult<Vec<Subscription>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT id, name, url, kind, enabled, last_refreshed_at, last_error, created_at, updated_at
             FROM subscriptions ORDER BY id DESC",
        )?;

        let subscriptions = stmt.query_map([], |row| {
            Ok(Subscription {
                id: row.get(0)?,
                name: row.get(1)?,
                url: row.get(2)?,
                kind: row.get(3)?,
                enabled: row.get::<_, i32>(4)? != 0,
                last_refreshed_at: row.get(5)?,
                last_error: row.get(6)?,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        })?;

        subscriptions.collect()
    }

    pub fn get_source_health_summaries(&self) -> SqliteResult<Vec<SourceHealthSummary>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT s.id, s.name, s.url, s.kind, s.enabled, s.last_refreshed_at, s.last_error,
                    COUNT(DISTINCT sl.id) AS live_channel_count,
                    COUNT(DISTINCT ci.id) AS catalog_item_count,
                    COUNT(DISTINCT ce.id) AS catalog_episode_count
             FROM subscriptions s
             LEFT JOIN source_lives sl ON sl.subscription_id = s.id
             LEFT JOIN catalog_items ci ON ci.subscription_id = s.id
             LEFT JOIN catalog_episodes ce ON ce.catalog_item_id = ci.id
             GROUP BY s.id
             ORDER BY s.id DESC",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(SourceHealthSummary {
                id: row.get(0)?,
                name: row.get(1)?,
                url: row.get(2)?,
                kind: row.get(3)?,
                enabled: row.get::<_, i32>(4)? != 0,
                last_refreshed_at: row.get(5)?,
                last_error: row.get(6)?,
                live_channel_count: row.get(7)?,
                catalog_item_count: row.get(8)?,
                catalog_episode_count: row.get(9)?,
            })
        })?;

        rows.collect()
    }

    pub fn delete_subscription(&self, id: i64) -> SqliteResult<()> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;

        tx.execute(
            "DELETE FROM source_configs WHERE subscription_id = ?1",
            [id],
        )?;
        tx.execute("DELETE FROM source_sites WHERE subscription_id = ?1", [id])?;
        tx.execute("DELETE FROM source_parses WHERE subscription_id = ?1", [id])?;
        tx.execute("DELETE FROM source_lives WHERE subscription_id = ?1", [id])?;
        tx.execute("DELETE FROM live_channels WHERE subscription_id = ?1", [id])?;
        tx.execute("DELETE FROM vod_items WHERE subscription_id = ?1", [id])?;
        tx.execute("DELETE FROM subscriptions WHERE id = ?1", [id])?;

        tx.commit()?;
        Ok(())
    }

    pub fn toggle_subscription(&self, id: i64, enabled: bool) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        let now = chrono_now();

        conn.execute(
            "UPDATE subscriptions SET enabled = ?1, updated_at = ?2 WHERE id = ?3",
            rusqlite::params![enabled as i32, now, id],
        )?;

        Ok(())
    }

    pub fn get_live_channels(&self, category: Option<String>) -> SqliteResult<Vec<LiveChannel>> {
        let conn = self.conn.lock().unwrap();

        if let Some(cat) = category {
            let mut stmt = conn.prepare(
                "SELECT sl.id,
                        sl.subscription_id,
                        sl.channel_name,
                        NULL as logo,
                        sl.raw_url,
                        COALESCE(NULLIF(TRIM(sl.group_name), ''), '其他') AS category
                 FROM source_lives sl
                 INNER JOIN subscriptions s ON sl.subscription_id = s.id
                 WHERE s.enabled = 1
                   AND COALESCE(NULLIF(TRIM(s.last_error), ''), '') = ''
                   AND COALESCE(NULLIF(TRIM(sl.group_name), ''), '其他') = ?1
                 ORDER BY sl.channel_name, sl.id",
            )?;
            let rows = stmt.query_map([cat], |row| {
                Ok(LiveChannel {
                    id: row.get(0)?,
                    subscription_id: row.get(1)?,
                    name: row.get(2)?,
                    logo: row.get(3)?,
                    url: row.get(4)?,
                    category: row.get(5)?,
                })
            })?;
            rows.collect()
        } else {
            let mut stmt = conn.prepare(
                "SELECT sl.id,
                        sl.subscription_id,
                        sl.channel_name,
                        NULL as logo,
                        sl.raw_url,
                        COALESCE(NULLIF(TRIM(sl.group_name), ''), '其他') AS category
                 FROM source_lives sl
                 INNER JOIN subscriptions s ON sl.subscription_id = s.id
                 WHERE s.enabled = 1
                   AND COALESCE(NULLIF(TRIM(s.last_error), ''), '') = ''
                 ORDER BY category, sl.channel_name, sl.id",
            )?;
            let rows = stmt.query_map([], |row| {
                Ok(LiveChannel {
                    id: row.get(0)?,
                    subscription_id: row.get(1)?,
                    name: row.get(2)?,
                    logo: row.get(3)?,
                    url: row.get(4)?,
                    category: row.get(5)?,
                })
            })?;
            rows.collect()
        }
    }

    pub fn get_live_categories(&self) -> SqliteResult<Vec<String>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT DISTINCT COALESCE(NULLIF(TRIM(sl.group_name), ''), '其他') AS category
             FROM source_lives sl
             INNER JOIN subscriptions s ON sl.subscription_id = s.id
             WHERE s.enabled = 1
               AND COALESCE(NULLIF(TRIM(s.last_error), ''), '') = ''
             ORDER BY category",
        )?;

        let categories = stmt.query_map([], |row| row.get(0))?;
        categories.collect()
    }

    pub fn get_live_channel_groups(&self) -> SqliteResult<Vec<LiveChannelGroup>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT COALESCE(NULLIF(TRIM(group_name), ''), '其他') AS category,
                    channel_name,
                    COUNT(*) AS source_count
             FROM source_lives sl
             INNER JOIN subscriptions s ON sl.subscription_id = s.id
             WHERE s.enabled = 1
               AND COALESCE(NULLIF(TRIM(s.last_error), ''), '') = ''
             GROUP BY category, channel_name
             ORDER BY category, channel_name",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                LiveChannelGroupItem {
                    name: row.get(1)?,
                    source_count: row.get(2)?,
                },
            ))
        })?;

        let mut grouped = std::collections::BTreeMap::<String, Vec<LiveChannelGroupItem>>::new();
        for row in rows {
            let (category, channel) = row?;
            grouped.entry(category).or_default().push(channel);
        }

        Ok(grouped
            .into_iter()
            .map(|(category, channels)| LiveChannelGroup { category, channels })
            .collect())
    }

    pub fn clear_channels_for_subscription(&self, subscription_id: i64) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM live_channels WHERE subscription_id = ?1",
            [subscription_id],
        )?;
        Ok(())
    }

    pub fn add_live_channel(
        &self,
        sub_id: i64,
        name: &str,
        logo: Option<&str>,
        url: &str,
        category: Option<&str>,
    ) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO live_channels (subscription_id, name, logo, url, category) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![sub_id, name, logo, url, category],
        )?;
        Ok(())
    }

    pub fn get_vod_items(&self, vtype: Option<String>, page: u32) -> SqliteResult<Vec<VodItem>> {
        let conn = self.conn.lock().unwrap();
        let page_size = 50;
        let offset = page * page_size;

        if let Some(t) = vtype {
            let mut stmt = conn.prepare(
                "SELECT v.id, v.subscription_id, v.name, v.vtype, v.poster, v.description, v.episodes
                 FROM vod_items v
                 INNER JOIN subscriptions s ON v.subscription_id = s.id
                 WHERE s.enabled = 1 AND v.vtype = ?1
                 ORDER BY v.name
                 LIMIT ?2 OFFSET ?3"
            )?;
            let rows = stmt.query_map(rusqlite::params![t, page_size, offset], |row| {
                Ok(VodItem {
                    id: row.get(0)?,
                    subscription_id: row.get(1)?,
                    name: row.get(2)?,
                    vtype: row.get(3)?,
                    poster: row.get(4)?,
                    description: row.get(5)?,
                    episodes: row.get(6)?,
                })
            })?;
            rows.collect()
        } else {
            let mut stmt = conn.prepare(
                "SELECT v.id, v.subscription_id, v.name, v.vtype, v.poster, v.description, v.episodes
                 FROM vod_items v
                 INNER JOIN subscriptions s ON v.subscription_id = s.id
                 WHERE s.enabled = 1
                 ORDER BY v.name
                 LIMIT ?1 OFFSET ?2"
            )?;
            let rows = stmt.query_map(rusqlite::params![page_size, offset], |row| {
                Ok(VodItem {
                    id: row.get(0)?,
                    subscription_id: row.get(1)?,
                    name: row.get(2)?,
                    vtype: row.get(3)?,
                    poster: row.get(4)?,
                    description: row.get(5)?,
                    episodes: row.get(6)?,
                })
            })?;
            rows.collect()
        }
    }

    pub fn get_vod_detail(&self, id: i64) -> SqliteResult<VodItem> {
        let conn = self.conn.lock().unwrap();

        conn.query_row(
            "SELECT id, subscription_id, name, vtype, poster, description, episodes
             FROM vod_items WHERE id = ?1",
            [id],
            |row| {
                Ok(VodItem {
                    id: row.get(0)?,
                    subscription_id: row.get(1)?,
                    name: row.get(2)?,
                    vtype: row.get(3)?,
                    poster: row.get(4)?,
                    description: row.get(5)?,
                    episodes: row.get(6)?,
                })
            },
        )
    }

    pub fn get_library_home(&self) -> SqliteResult<HomePayload> {
        let conn = self.conn.lock().unwrap();

        let continue_watching = query_home_catalog_items(
            &conn,
            "SELECT ci.id, ci.title, ci.item_type, ci.poster, ph.progress, s.name AS source_badge, '继续观看' AS update_badge
             FROM play_history ph
             INNER JOIN catalog_items ci ON ph.item_type = 'vod' AND ph.item_id = ci.id
             INNER JOIN subscriptions s ON ci.subscription_id = s.id
             WHERE s.enabled = 1
               AND COALESCE(json_extract(ci.detail_json, '$.source'), '') != 'zxzj'
             ORDER BY ph.last_played DESC
             LIMIT 12",
            [],
        )?;
        let latest_updates = query_home_catalog_items(
            &conn,
            "SELECT ci.id, ci.title, ci.item_type, ci.poster, NULL as progress, s.name AS source_badge, NULL AS update_badge
             FROM catalog_items ci
             INNER JOIN subscriptions s ON ci.subscription_id = s.id
             WHERE s.enabled = 1
               AND COALESCE(json_extract(ci.detail_json, '$.source'), '') != 'zxzj'
             ORDER BY ci.updated_at DESC, ci.id DESC
             LIMIT 12",
            [],
        )?;
        let featured = query_home_catalog_items(
            &conn,
            "SELECT ci.id, ci.title, ci.item_type, ci.poster, NULL as progress, s.name AS source_badge, NULL AS update_badge
             FROM catalog_items ci
             INNER JOIN subscriptions s ON ci.subscription_id = s.id
             WHERE s.enabled = 1
               AND COALESCE(json_extract(ci.detail_json, '$.source'), '') != 'zxzj'
             ORDER BY ci.id DESC
             LIMIT 12",
            [],
        )?;

        Ok(HomePayload {
            continue_watching,
            latest_updates,
            featured,
        })
    }

    pub fn get_catalog_items(
        &self,
        item_type: Option<String>,
        keyword: Option<String>,
    ) -> SqliteResult<Vec<HomeCatalogItem>> {
        let conn = self.conn.lock().unwrap();
        match (item_type, keyword) {
            (Some(item_type), Some(keyword)) => query_home_catalog_items(
                &conn,
                "SELECT ci.id, ci.title, ci.item_type, ci.poster, NULL as progress, s.name AS source_badge, NULL AS update_badge
                 FROM catalog_items ci
                 INNER JOIN subscriptions s ON ci.subscription_id = s.id
                 WHERE s.enabled = 1
                   AND COALESCE(json_extract(ci.detail_json, '$.source'), '') != 'zxzj'
                   AND ci.item_type = ?1
                   AND ci.title LIKE ?2
                 ORDER BY ci.updated_at DESC, ci.id DESC
                 LIMIT 240",
                rusqlite::params![item_type, format!("%{}%", keyword)],
            ),
            (Some(item_type), None) => query_home_catalog_items(
                &conn,
                "SELECT ci.id, ci.title, ci.item_type, ci.poster, NULL as progress, s.name AS source_badge, NULL AS update_badge
                 FROM catalog_items ci
                 INNER JOIN subscriptions s ON ci.subscription_id = s.id
                 WHERE s.enabled = 1
                   AND COALESCE(json_extract(ci.detail_json, '$.source'), '') != 'zxzj'
                   AND ci.item_type = ?1
                 ORDER BY ci.updated_at DESC, ci.id DESC
                 LIMIT 240",
                [item_type],
            ),
            (None, Some(keyword)) => query_home_catalog_items(
                &conn,
                "SELECT ci.id, ci.title, ci.item_type, ci.poster, NULL as progress, s.name AS source_badge, NULL AS update_badge
                 FROM catalog_items ci
                 INNER JOIN subscriptions s ON ci.subscription_id = s.id
                 WHERE s.enabled = 1
                   AND COALESCE(json_extract(ci.detail_json, '$.source'), '') != 'zxzj'
                   AND ci.title LIKE ?1
                 ORDER BY ci.updated_at DESC, ci.id DESC
                 LIMIT 240",
                [format!("%{}%", keyword)],
            ),
            (None, None) => query_home_catalog_items(
                &conn,
                "SELECT ci.id, ci.title, ci.item_type, ci.poster, NULL as progress, s.name AS source_badge, NULL AS update_badge
                 FROM catalog_items ci
                 INNER JOIN subscriptions s ON ci.subscription_id = s.id
                 WHERE s.enabled = 1
                   AND COALESCE(json_extract(ci.detail_json, '$.source'), '') != 'zxzj'
                 ORDER BY ci.updated_at DESC, ci.id DESC
                 LIMIT 240",
                [],
            ),
        }
    }

    pub fn get_catalog_detail(&self, item_id: i64) -> SqliteResult<CatalogDetail> {
        let conn = self.conn.lock().unwrap();

        let item = conn.query_row(
            "SELECT ci.id, ci.title, ci.item_type, ci.poster, ci.summary, ci.detail_json
             FROM catalog_items ci
             INNER JOIN subscriptions s ON ci.subscription_id = s.id
             WHERE s.enabled = 1 AND ci.id = ?1",
            [item_id],
            |row| {
                Ok(CatalogDetailItem {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    item_type: row.get(2)?,
                    poster: row.get(3)?,
                    summary: row.get(4)?,
                    detail_json: row.get(5)?,
                })
            },
        )?;

        let mut stmt = conn.prepare(
            "SELECT id,
                    COALESCE(NULLIF(TRIM(source_name), ''), '默认来源') AS source_name,
                    episode_label,
                    play_url,
                    order_index
             FROM catalog_episodes
             WHERE catalog_item_id = ?1
             ORDER BY source_name, order_index, id",
        )?;
        let rows = stmt.query_map([item_id], |row| {
            Ok((
                row.get::<_, String>(1)?,
                CatalogEpisode {
                    id: row.get(0)?,
                    episode_label: row.get(2)?,
                    play_url: row.get(3)?,
                    order_index: row.get(4)?,
                },
            ))
        })?;

        let mut grouped = std::collections::BTreeMap::<String, Vec<CatalogEpisode>>::new();
        for row in rows {
            let (source_name, episode) = row?;
            if is_visible_playback_target(&episode.play_url) {
                grouped
                    .entry(source_name.clone())
                    .or_default()
                    .push(episode.clone());
            }
        }

        let mut episode_groups: Vec<_> = grouped
            .into_iter()
            .map(|(source_name, mut episodes)| {
                episodes.sort_by(|left, right| {
                    playback_sort_rank(&left.play_url)
                        .cmp(&playback_sort_rank(&right.play_url))
                        .then(left.order_index.cmp(&right.order_index))
                        .then(left.id.cmp(&right.id))
                });
                CatalogEpisodeGroup {
                    source_name,
                    episodes,
                }
            })
            .collect();

        episode_groups.sort_by(|left, right| {
            let left_rank = left
                .episodes
                .first()
                .map(|episode| playback_sort_rank(&episode.play_url))
                .unwrap_or(i32::MAX);
            let right_rank = right
                .episodes
                .first()
                .map(|episode| playback_sort_rank(&episode.play_url))
                .unwrap_or(i32::MAX);
            left_rank
                .cmp(&right_rank)
                .then(left.source_name.cmp(&right.source_name))
        });

        Ok(CatalogDetail {
            item,
            episode_groups,
        })
    }

    pub fn search_vod(&self, keyword: &str) -> SqliteResult<Vec<VodItem>> {
        let conn = self.conn.lock().unwrap();
        let pattern = format!("%{}%", keyword);

        let mut stmt = conn.prepare(
            "SELECT v.id, v.subscription_id, v.name, v.vtype, v.poster, v.description, v.episodes
             FROM vod_items v
             INNER JOIN subscriptions s ON v.subscription_id = s.id
             WHERE s.enabled = 1 AND (v.name LIKE ?1 OR v.description LIKE ?1)
             ORDER BY v.name",
        )?;

        let items = stmt.query_map([pattern], |row| {
            Ok(VodItem {
                id: row.get(0)?,
                subscription_id: row.get(1)?,
                name: row.get(2)?,
                vtype: row.get(3)?,
                poster: row.get(4)?,
                description: row.get(5)?,
                episodes: row.get(6)?,
            })
        })?;

        items.collect()
    }

    pub fn clear_vod_for_subscription(&self, subscription_id: i64) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM vod_items WHERE subscription_id = ?1",
            [subscription_id],
        )?;
        Ok(())
    }

    pub fn add_vod_item(
        &self,
        sub_id: i64,
        name: &str,
        vtype: &str,
        poster: Option<&str>,
        description: Option<&str>,
        episodes: &str,
    ) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO vod_items (subscription_id, name, vtype, poster, description, episodes)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![sub_id, name, vtype, poster, description, episodes],
        )?;
        Ok(())
    }

    pub fn refresh_subscription(
        &self,
        id: i64,
        lives: Vec<(String, Option<String>, String, Option<String>)>,
        vods: Vec<(String, String, Option<String>, Option<String>, String)>,
    ) -> SqliteResult<()> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        let refreshed_at = chrono_now();

        // Clear old data
        tx.execute("DELETE FROM live_channels WHERE subscription_id = ?1", [id])?;
        tx.execute("DELETE FROM vod_items WHERE subscription_id = ?1", [id])?;
        tx.execute(
            "DELETE FROM source_configs WHERE subscription_id = ?1",
            [id],
        )?;
        tx.execute("DELETE FROM source_sites WHERE subscription_id = ?1", [id])?;
        tx.execute("DELETE FROM source_parses WHERE subscription_id = ?1", [id])?;
        tx.execute("DELETE FROM source_lives WHERE subscription_id = ?1", [id])?;

        // Insert new live channels
        for (name, logo, url, category) in lives {
            let group_name = category.clone();
            tx.execute(
                "INSERT INTO live_channels (subscription_id, name, logo, url, category) VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![id, name, logo, url, category],
            )?;
            tx.execute(
                "INSERT INTO source_lives (
                    subscription_id, group_name, channel_name, raw_url, normalized_url, source_type, raw_json
                 ) VALUES (?1, ?2, ?3, ?4, ?5, NULL, ?6)",
                rusqlite::params![id, group_name, name, url, url, "{}"],
            )?;
        }

        // Insert new vod items
        for (name, vtype, poster, description, episodes) in vods {
            tx.execute(
                "INSERT INTO vod_items (subscription_id, name, vtype, poster, description, episodes) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![id, name, vtype, poster, description, episodes],
            )?;
        }

        tx.execute(
            "UPDATE subscriptions
             SET kind = 'simple_json', last_refreshed_at = ?1, last_error = NULL, updated_at = ?1
             WHERE id = ?2",
            rusqlite::params![refreshed_at, id],
        )?;

        tx.commit()?;
        Ok(())
    }

    pub fn refresh_tvbox_subscription(
        &self,
        id: i64,
        raw_content: &str,
        parsed: &TvboxConfigRecords,
    ) -> SqliteResult<()> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        let refreshed_at = chrono_now();

        tx.execute("DELETE FROM live_channels WHERE subscription_id = ?1", [id])?;
        tx.execute("DELETE FROM vod_items WHERE subscription_id = ?1", [id])?;
        tx.execute(
            "DELETE FROM source_configs WHERE subscription_id = ?1",
            [id],
        )?;
        tx.execute("DELETE FROM source_sites WHERE subscription_id = ?1", [id])?;
        tx.execute("DELETE FROM source_parses WHERE subscription_id = ?1", [id])?;
        tx.execute("DELETE FROM source_lives WHERE subscription_id = ?1", [id])?;

        tx.execute(
            "INSERT INTO source_configs (subscription_id, config_kind, raw_content, parsed_at)
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![id, "tvbox_config", raw_content, refreshed_at],
        )?;

        insert_tvbox_sites(&tx, id, &parsed.sites)?;
        insert_tvbox_parses(&tx, id, &parsed.parses)?;
        insert_tvbox_lives(&tx, id, &parsed.lives)?;

        tx.execute(
            "UPDATE subscriptions
             SET kind = 'tvbox_config', last_refreshed_at = ?1, last_error = NULL, updated_at = ?1
             WHERE id = ?2",
            rusqlite::params![refreshed_at, id],
        )?;

        tx.commit()?;
        Ok(())
    }

    pub fn replace_catalog_for_subscription(
        &self,
        subscription_id: i64,
        items: &[ScrapedCatalogItem],
    ) -> SqliteResult<()> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        tx.execute(
            "DELETE FROM playback_targets
             WHERE episode_id IN (
                SELECT ce.id
                FROM catalog_episodes ce
                INNER JOIN catalog_items ci ON ce.catalog_item_id = ci.id
                WHERE ci.subscription_id = ?1
             )",
            [subscription_id],
        )?;
        tx.execute(
            "DELETE FROM catalog_items WHERE subscription_id = ?1",
            [subscription_id],
        )?;

        let updated_at = chrono_now();
        for item in items {
            let runtime_targets = runtime_targets_for_item(item);
            tx.execute(
                "INSERT INTO catalog_items (
                    subscription_id, site_id, source_item_key, title, item_type, poster, summary, detail_json, updated_at
                 ) VALUES (?1, NULL, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                rusqlite::params![
                    subscription_id,
                    item.source_item_key,
                    item.title,
                    item.item_type,
                    item.poster,
                    item.summary,
                    item.detail_json,
                    updated_at
                ],
            )?;

            let catalog_item_id = tx.last_insert_rowid();
            for (episode, target) in item.episodes.iter().zip(runtime_targets.iter()) {
                tx.execute(
                    "INSERT INTO catalog_episodes (
                        catalog_item_id, source_name, season_label, episode_label, play_url, order_index, extra_json
                     ) VALUES (?1, ?2, NULL, ?3, ?4, ?5, NULL)",
                    rusqlite::params![
                        catalog_item_id,
                        episode.source_name,
                        episode.episode_label,
                        episode.play_url,
                        episode.order_index
                    ],
                )?;
                let episode_id = tx.last_insert_rowid();
                insert_playback_target(&tx, playback_target_record(episode_id, target))?;
            }
        }

        tx.commit()?;
        Ok(())
    }

    pub fn replace_catalog_item_detail(
        &self,
        item_id: i64,
        item: &ScrapedCatalogItem,
    ) -> SqliteResult<()> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        let updated_at = chrono_now();

        tx.execute(
            "UPDATE catalog_items
             SET title = ?2,
                 item_type = ?3,
                 poster = COALESCE(?4, poster),
                 summary = COALESCE(?5, summary),
                 detail_json = COALESCE(?6, detail_json),
                 updated_at = ?7
             WHERE id = ?1",
            rusqlite::params![
                item_id,
                item.title,
                item.item_type,
                item.poster,
                item.summary,
                item.detail_json,
                updated_at
            ],
        )?;
        tx.execute(
            "DELETE FROM playback_targets
             WHERE episode_id IN (
                SELECT id FROM catalog_episodes WHERE catalog_item_id = ?1
             )",
            [item_id],
        )?;
        tx.execute(
            "DELETE FROM catalog_episodes WHERE catalog_item_id = ?1",
            [item_id],
        )?;

        let runtime_targets = runtime_targets_for_item(item);
        for (episode, target) in item.episodes.iter().zip(runtime_targets.iter()) {
            tx.execute(
                "INSERT INTO catalog_episodes (
                    catalog_item_id, source_name, season_label, episode_label, play_url, order_index, extra_json
                 ) VALUES (?1, ?2, NULL, ?3, ?4, ?5, NULL)",
                rusqlite::params![
                    item_id,
                    episode.source_name,
                    episode.episode_label,
                    episode.play_url,
                    episode.order_index
                ],
            )?;
            let episode_id = tx.last_insert_rowid();
            insert_playback_target(&tx, playback_target_record(episode_id, target))?;
        }

        tx.commit()?;
        Ok(())
    }

    pub fn save_play_history(
        &self,
        item_type: &str,
        item_id: i64,
        progress: f64,
    ) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        let now = chrono_now();

        conn.execute(
            "INSERT INTO play_history (item_type, item_id, progress, last_played)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(item_type, item_id) DO UPDATE SET progress = ?3, last_played = ?4",
            rusqlite::params![item_type, item_id, progress, now],
        )?;

        Ok(())
    }

    pub fn get_play_history(&self) -> SqliteResult<Vec<PlayHistory>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT id, item_type, item_id, progress, last_played FROM play_history ORDER BY last_played DESC LIMIT 100",
        )?;

        let history = stmt.query_map([], |row| {
            Ok(PlayHistory {
                id: row.get(0)?,
                item_type: row.get(1)?,
                item_id: row.get(2)?,
                progress: row.get(3)?,
                last_played: row.get(4)?,
            })
        })?;

        history.collect()
    }

    pub fn get_douban_hot(&self) -> SqliteResult<Vec<DoubanHot>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, year, poster, rating, rank, updated_at FROM douban_hot ORDER BY rank LIMIT 100"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(DoubanHot {
                id: row.get(0)?,
                name: row.get(1)?,
                year: row.get(2)?,
                poster: row.get(3)?,
                rating: row.get(4)?,
                rank: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })?;
        rows.collect()
    }

    pub fn upsert_douban_hot(&self, items: &[DoubanHot]) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        for item in items {
            conn.execute(
                "INSERT OR REPLACE INTO douban_hot (name, year, poster, rating, rank, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![
                    item.name,
                    item.year,
                    item.poster,
                    item.rating,
                    item.rank,
                    item.updated_at
                ],
            )?;
        }
        Ok(())
    }

    pub fn clear_douban_hot(&self) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM douban_hot", [])?;
        Ok(())
    }

    pub fn get_merged_live_channels(&self) -> SqliteResult<Vec<MergedLiveChannel>> {
        let conn = self.conn.lock().unwrap();
        query_merged_live_channels(&conn, None)
    }

    pub fn get_merged_live_channels_by_category(
        &self,
        category: &str,
    ) -> SqliteResult<Vec<MergedLiveChannel>> {
        let conn = self.conn.lock().unwrap();
        query_merged_live_channels(&conn, Some(category))
    }
}

fn query_home_catalog_items(
    conn: &Connection,
    sql: &str,
    params: impl rusqlite::Params,
) -> SqliteResult<Vec<HomeCatalogItem>> {
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map(params, |row| {
        Ok(HomeCatalogItem {
            id: row.get(0)?,
            title: row.get(1)?,
            item_type: row.get(2)?,
            poster: row.get(3)?,
            progress: row.get(4)?,
            source_badge: row.get(5)?,
            update_badge: row.get(6)?,
        })
    })?;
    rows.collect()
}

fn query_merged_live_channels(
    conn: &Connection,
    category: Option<&str>,
) -> SqliteResult<Vec<MergedLiveChannel>> {
    let sql = if category.is_some() {
        "SELECT sl.channel_name,
                COALESCE(NULLIF(TRIM(sl.group_name), ''), '其他') AS category,
                GROUP_CONCAT(sl.raw_url || '|' || sl.subscription_id) AS sources
         FROM source_lives sl
         INNER JOIN subscriptions s ON sl.subscription_id = s.id
         WHERE s.enabled = 1
           AND COALESCE(NULLIF(TRIM(s.last_error), ''), '') = ''
           AND COALESCE(NULLIF(TRIM(sl.group_name), ''), '其他') = ?1
         GROUP BY sl.channel_name, category
         ORDER BY sl.channel_name"
    } else {
        "SELECT sl.channel_name,
                COALESCE(NULLIF(TRIM(sl.group_name), ''), '其他') AS category,
                GROUP_CONCAT(sl.raw_url || '|' || sl.subscription_id) AS sources
         FROM source_lives sl
         INNER JOIN subscriptions s ON sl.subscription_id = s.id
         WHERE s.enabled = 1
           AND COALESCE(NULLIF(TRIM(s.last_error), ''), '') = ''
         GROUP BY sl.channel_name, category
         ORDER BY category, sl.channel_name"
    };

    let mut stmt = conn.prepare(sql)?;
    let rows = match category {
        Some(category) => stmt.query_map([category], map_merged_live_channel_row)?,
        None => stmt.query_map([], map_merged_live_channel_row)?,
    };
    rows.collect()
}

fn map_merged_live_channel_row(row: &rusqlite::Row<'_>) -> SqliteResult<MergedLiveChannel> {
    let sources_str: String = row.get(2)?;
    let name: String = row.get(0)?;
    let category = Some(row.get::<_, String>(1)?);

    let sources = sources_str
        .split(',')
        .filter_map(|source| {
            let parts: Vec<&str> = source.split('|').collect();
            if parts.len() == 2 {
                Some(ChannelSource {
                    url: parts[0].to_string(),
                    subscription_id: parts[1].parse().unwrap_or(0),
                })
            } else {
                None
            }
        })
        .collect();

    Ok(MergedLiveChannel {
        id: generate_channel_id(&name, category.as_deref()),
        name,
        logo: None,
        category,
        sources,
    })
}

/// Generate a deterministic ID from channel name and category
/// Uses FNV hash for simplicity and speed
fn generate_channel_id(name: &str, category: Option<&str>) -> i64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    name.hash(&mut hasher);
    if let Some(cat) = category {
        cat.hash(&mut hasher);
    }
    hasher.finish() as i64
}

impl Clone for Storage {
    fn clone(&self) -> Self {
        Storage {
            conn: Arc::clone(&self.conn),
        }
    }
}

fn chrono_now() -> String {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let total_secs = duration.as_secs();

    // Calculate days since 1970-01-01
    let days_since_epoch = total_secs / 86400;

    // Calculate year, month, day using Zeller-like algorithm
    // Using the formula for Gregorian calendar
    let mut year = 1970;
    let mut remaining_days = days_since_epoch as i64;

    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if remaining_days < days_in_year {
            break;
        }
        remaining_days -= days_in_year;
        year += 1;
    }

    let days_in_months = if is_leap_year(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut month = 1;
    for (i, &days) in days_in_months.iter().enumerate() {
        if remaining_days < days as i64 {
            month = i + 1;
            break;
        }
        remaining_days -= days as i64;
    }

    let day = remaining_days + 1;

    let secs_in_day = total_secs % 86400;
    let hour = secs_in_day / 3600;
    let minute = (secs_in_day % 3600) / 60;
    let second = secs_in_day % 60;

    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        year, month, day, hour, minute, second
    )
}

fn is_leap_year(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

fn insert_tvbox_sites(
    tx: &rusqlite::Transaction<'_>,
    subscription_id: i64,
    sites: &[TvboxSiteRecord],
) -> SqliteResult<()> {
    for site in sites {
        tx.execute(
            "INSERT INTO source_sites (
                subscription_id, site_key, site_name, api, ext,
                searchable, quick_search, filterable, source_type, raw_json
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            rusqlite::params![
                subscription_id,
                site.site_key,
                site.site_name,
                site.api,
                site.ext,
                site.searchable as i32,
                site.quick_search as i32,
                site.filterable as i32,
                site.source_type,
                site.raw_json
            ],
        )?;
    }
    Ok(())
}

fn insert_tvbox_parses(
    tx: &rusqlite::Transaction<'_>,
    subscription_id: i64,
    parses: &[TvboxParseRecord],
) -> SqliteResult<()> {
    for parse in parses {
        tx.execute(
            "INSERT INTO source_parses (
                subscription_id, parse_name, parse_url, source_type, header_json, raw_json
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                subscription_id,
                parse.name,
                parse.url,
                parse.source_type,
                parse.header_json,
                parse.raw_json
            ],
        )?;
    }
    Ok(())
}

fn insert_tvbox_lives(
    tx: &rusqlite::Transaction<'_>,
    subscription_id: i64,
    lives: &[TvboxLiveRecord],
) -> SqliteResult<()> {
    for live in lives {
        tx.execute(
            "INSERT INTO source_lives (
                subscription_id, group_name, channel_name, raw_url, normalized_url, source_type, raw_json
             ) VALUES (?1, ?2, ?3, ?4, ?4, ?5, ?6)",
            rusqlite::params![
                subscription_id,
                live.group_name,
                live.name,
                live.url,
                live.source_type,
                live.raw_json
            ],
        )?;
    }
    Ok(())
}

fn playback_target_record(
    episode_id: i64,
    target: &crate::services::PlaybackTarget,
) -> PlaybackTargetRecord {
    PlaybackTargetRecord {
        episode_id,
        source_key: target.source_key.clone(),
        target_url: target.target_url.clone(),
        target_kind: target_kind_label(&target.target_kind).to_string(),
        resolver_key: target.resolver_key.clone(),
        headers_json: target
            .headers
            .as_ref()
            .and_then(|headers| serde_json::to_string(headers).ok()),
        meta_text: target.meta.clone(),
        sort_hint: target.sort_hint,
    }
}

fn insert_playback_target(
    tx: &rusqlite::Transaction<'_>,
    target: PlaybackTargetRecord,
) -> SqliteResult<()> {
    tx.execute(
        r#"
        INSERT INTO playback_targets (
            episode_id,
            source_key,
            target_url,
            target_kind,
            resolver_key,
            headers_json,
            meta_text,
            sort_hint
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
        "#,
        rusqlite::params![
            target.episode_id,
            target.source_key,
            target.target_url,
            target.target_kind,
            target.resolver_key,
            target.headers_json,
            target.meta_text,
            target.sort_hint,
        ],
    )?;
    Ok(())
}

fn target_kind_label(kind: &crate::services::PlaybackTargetKind) -> &'static str {
    match kind {
        crate::services::PlaybackTargetKind::Direct => "direct",
        crate::services::PlaybackTargetKind::Resolvable => "resolvable",
        crate::services::PlaybackTargetKind::Embedded => "embedded",
        crate::services::PlaybackTargetKind::ExternalRequired => "external_required",
    }
}

#[cfg(test)]
mod tests {
    use super::{playback_target_record, target_kind_label, Storage};
    use super::playback_cache::{
        get_playback_health, list_playback_targets, replace_playback_targets,
        upsert_playback_health, PlaybackTargetRecord,
    };
    use crate::services::playback_runtime::build_runtime_target;
    use crate::services::{
        PlaybackTargetKind, ScrapedCatalogEpisode, ScrapedCatalogItem,
    };
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn persists_playback_health_with_ttl() {
        let storage = Storage::new(unique_test_dir()).expect("storage should initialize");

        upsert_playback_health(
            &storage,
            "hash-1",
            "playable",
            true,
            true,
            true,
            Some(200),
            None,
            1800,
        )
        .expect("health should save");

        let health = get_playback_health(&storage, "hash-1")
            .expect("health query should succeed")
            .expect("health row should exist");

        assert_eq!(health.status, "playable");
        assert!(health.expires_at > health.checked_at);
    }

    #[test]
    fn persists_playback_targets_for_episode() {
        let storage = Storage::new(unique_test_dir()).expect("storage should initialize");

        replace_playback_targets(
            &storage,
            77,
            vec![PlaybackTargetRecord {
                episode_id: 77,
                source_key: "jianpian".to_string(),
                target_url: "https://cdn.example.com/ok/index.m3u8".to_string(),
                target_kind: "direct".to_string(),
                resolver_key: None,
                headers_json: None,
                meta_text: Some("荐片线路".to_string()),
                sort_hint: 0,
            }],
        )
        .expect("targets should save");

        let targets = list_playback_targets(&storage, 77).expect("target query should succeed");
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].meta_text.as_deref(), Some("荐片线路"));
        assert_eq!(targets[0].target_kind, "direct");
    }

    #[test]
    fn playback_target_record_preserves_resolver_key_for_resolvable_sources() {
        let mut target = build_runtime_target(
            "guard://csp_JPJGuard/%E8%B4%B1%E8%B4%B1/97910/1/1",
            "csp_JPJGuard",
            Some(88),
        );
        target.sort_hint = 0;
        let record = playback_target_record(88, &target);

        assert_eq!(record.target_kind, "resolvable");
        assert_eq!(record.resolver_key.as_deref(), Some("csp_JPJGuard"));
        assert_eq!(record.headers_json, None);
    }

    #[test]
    fn target_kind_label_matches_runtime_kind() {
        assert_eq!(target_kind_label(&PlaybackTargetKind::Direct), "direct");
        assert_eq!(target_kind_label(&PlaybackTargetKind::Embedded), "embedded");
    }

    #[test]
    fn replace_catalog_item_detail_persists_runtime_targets_for_new_episodes() {
        let storage = Storage::new(unique_test_dir()).expect("storage should initialize");
        let subscription = storage
            .add_subscription("tvbox", "https://example.com/tvbox.json")
            .expect("subscription should be inserted");

        seed_catalog_item_with_source(&storage, subscription.id, 301, "缓存影片", "series", "guard");

        let scraped = ScrapedCatalogItem {
            source_item_key: "guard:荐片:97910".to_string(),
            title: "缓存影片".to_string(),
            item_type: "series".to_string(),
            poster: None,
            summary: None,
            detail_json: Some(
                r#"{"source":"guard","guard_key":"csp_JPJGuard","site_key":"贱贱","item_id":"97910","item_type":"series"}"#
                    .to_string(),
            ),
            episodes: vec![ScrapedCatalogEpisode {
                source_name: "荐片".to_string(),
                episode_label: "第1集".to_string(),
                play_url: "guard://csp_JPJGuard/%E8%B4%B1%E8%B4%B1/97910/1/1".to_string(),
                order_index: 1,
            }],
        };

        storage
            .replace_catalog_item_detail(301, &scraped)
            .expect("catalog detail should update");

        let detail = storage
            .get_catalog_detail(301)
            .expect("catalog detail should query");
        let episode_id = detail.episode_groups[0].episodes[0].id;
        let targets =
            list_playback_targets(&storage, episode_id).expect("runtime targets should query");

        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].source_key, "csp_JPJGuard");
        assert_eq!(targets[0].resolver_key.as_deref(), Some("csp_JPJGuard"));
        assert_eq!(targets[0].meta_text.as_deref(), Some("荐片:第1集"));
        assert_eq!(targets[0].target_kind, "resolvable");
        assert!(targets[0].target_url.starts_with("guard://"));
    }

    #[test]
    fn records_refresh_failure_without_overwriting_last_success_timestamp() {
        let storage = Storage::new(unique_test_dir()).expect("storage should initialize");
        let subscription = storage
            .add_subscription("demo", "https://example.com/sub.json")
            .expect("subscription should be inserted");

        storage
            .record_subscription_refresh_failure(subscription.id, "simple_json", "payload invalid")
            .expect("failure state should be recorded");

        let refreshed = storage
            .get_subscription(subscription.id)
            .expect("subscription should load");

        assert_eq!(refreshed.kind, "simple_json");
        assert_eq!(refreshed.last_error.as_deref(), Some("payload invalid"));
        assert_eq!(refreshed.last_refreshed_at, None);
    }

    #[test]
    fn groups_live_channels_and_returns_source_counts() {
        let storage = Storage::new(unique_test_dir()).expect("storage should initialize");
        let subscription = storage
            .add_subscription("tvbox", "https://example.com/tvbox.json")
            .expect("subscription should be inserted");

        seed_live_source(
            &storage,
            subscription.id,
            Some("央视频道"),
            "CCTV-1",
            "https://a.example/live.m3u8",
        );
        seed_live_source(
            &storage,
            subscription.id,
            Some("央视频道"),
            "CCTV-1",
            "https://b.example/live.m3u8",
        );
        seed_live_source(
            &storage,
            subscription.id,
            None,
            "测试频道",
            "https://c.example/live.m3u8",
        );

        let groups = storage
            .get_live_channel_groups()
            .expect("live groups should query");

        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].category, "其他");
        assert_eq!(groups[0].channels[0].name, "测试频道");
        assert_eq!(groups[0].channels[0].source_count, 1);
        assert_eq!(groups[1].category, "央视频道");
        assert_eq!(groups[1].channels[0].name, "CCTV-1");
        assert_eq!(groups[1].channels[0].source_count, 2);
    }

    #[test]
    fn live_category_and_drill_down_queries_use_source_lives() {
        let storage = Storage::new(unique_test_dir()).expect("storage should initialize");
        let subscription = storage
            .add_subscription("tvbox", "https://example.com/tvbox.json")
            .expect("subscription should be inserted");

        seed_live_source(
            &storage,
            subscription.id,
            Some("卫视频道"),
            "湖南卫视",
            "https://a.example/hnws.m3u8",
        );
        seed_live_source(
            &storage,
            subscription.id,
            Some("卫视频道"),
            "湖南卫视",
            "https://b.example/hnws.m3u8",
        );

        let categories = storage
            .get_live_categories()
            .expect("live categories should query");
        let channels = storage
            .get_merged_live_channels_by_category("卫视频道")
            .expect("live drill-down should query");

        assert_eq!(categories, vec!["卫视频道"]);
        assert_eq!(channels.len(), 1);
        assert_eq!(channels[0].name, "湖南卫视");
        assert_eq!(channels[0].category.as_deref(), Some("卫视频道"));
        assert_eq!(channels[0].sources.len(), 2);
    }

    #[test]
    fn live_queries_hide_subscriptions_with_refresh_errors() {
        let storage = Storage::new(unique_test_dir()).expect("storage should initialize");
        let subscription = storage
            .add_subscription("tvbox", "https://example.com/tvbox.json")
            .expect("subscription should be inserted");

        seed_live_source(
            &storage,
            subscription.id,
            Some("央视频道"),
            "CCTV-1",
            "https://a.example/live.m3u8",
        );
        storage
            .record_subscription_refresh_failure(
                subscription.id,
                "tvbox_config",
                "upstream blocked",
            )
            .expect("failure state should record");

        assert!(storage
            .get_live_categories()
            .expect("live categories should query")
            .is_empty());
        assert!(storage
            .get_live_channel_groups()
            .expect("live groups should query")
            .is_empty());
        assert!(storage
            .get_merged_live_channels()
            .expect("merged live channels should query")
            .is_empty());
    }

    #[test]
    fn source_health_summaries_count_live_catalog_and_episode_rows() {
        let storage = Storage::new(unique_test_dir()).expect("storage should initialize");
        let subscription = storage
            .add_subscription("饭太硬", "https://example.com/tvbox.json")
            .expect("subscription should be inserted");

        seed_live_source(
            &storage,
            subscription.id,
            Some("央视频道"),
            "CCTV-1",
            "https://live.example/cctv1.m3u8",
        );
        seed_live_source(
            &storage,
            subscription.id,
            Some("央视频道"),
            "CCTV-2",
            "https://live.example/cctv2.m3u8",
        );
        seed_catalog_item_with_source(
            &storage,
            subscription.id,
            201,
            "示例电影",
            "movie",
            "jianpian",
        );
        seed_catalog_item_with_source(
            &storage,
            subscription.id,
            202,
            "示例剧集",
            "series",
            "jianpian",
        );
        seed_catalog_episode(
            &storage,
            201,
            "荐片线路",
            "第01集",
            "https://media.example/movie-1.m3u8",
            0,
        );
        seed_catalog_episode(
            &storage,
            202,
            "荐片线路",
            "第01集",
            "https://media.example/series-1.m3u8",
            0,
        );
        seed_catalog_episode(
            &storage,
            202,
            "荐片线路",
            "第02集",
            "https://media.example/series-2.m3u8",
            1,
        );

        let summaries = storage
            .get_source_health_summaries()
            .expect("source summaries should query");

        let summary = summaries
            .iter()
            .find(|summary| summary.id == subscription.id)
            .expect("subscription summary should exist");
        assert_eq!(summary.name, "饭太硬");
        assert_eq!(summary.live_channel_count, 2);
        assert_eq!(summary.catalog_item_count, 2);
        assert_eq!(summary.catalog_episode_count, 3);
        assert!(summary.enabled);
    }

    #[test]
    fn library_home_returns_continue_watching_from_vod_history() {
        let storage = Storage::new(unique_test_dir()).expect("storage should initialize");
        let subscription = storage
            .add_subscription("荐片", "https://example.com/tvbox.json")
            .expect("subscription should be inserted");

        seed_catalog_item_with_source(&storage, subscription.id, 101, "示例影片", "movie", "jianpian");
        storage
            .save_play_history("vod", 101, 42.0)
            .expect("play history should insert");

        let home = storage
            .get_library_home()
            .expect("library home should query");

        assert_eq!(home.continue_watching.len(), 1);
        assert_eq!(home.continue_watching[0].id, 101);
        assert_eq!(home.continue_watching[0].title, "示例影片");
        assert_eq!(home.continue_watching[0].item_type, "movie");
        assert_eq!(home.continue_watching[0].progress, Some(42.0));
        assert_eq!(home.continue_watching[0].source_badge.as_deref(), Some("荐片"));
    }

    #[test]
    fn library_queries_hide_embedded_only_catalog_sources() {
        let storage = Storage::new(unique_test_dir()).expect("storage should initialize");
        let subscription = storage
            .add_subscription("tvbox", "https://example.com/tvbox.json")
            .expect("subscription should be inserted");

        seed_catalog_item_with_source(&storage, subscription.id, 111, "可播影片", "movie", "auete");
        seed_catalog_item_with_source(&storage, subscription.id, 112, "嵌页影片", "series", "zxzj");

        let home = storage
            .get_library_home()
            .expect("library home should query");
        let catalog = storage
            .get_catalog_items(None, None)
            .expect("catalog items should query");

        assert_eq!(home.latest_updates.len(), 1);
        assert_eq!(home.featured.len(), 1);
        assert_eq!(home.latest_updates[0].title, "可播影片");
        assert_eq!(catalog.len(), 1);
        assert_eq!(catalog[0].title, "可播影片");
    }

    #[test]
    fn library_queries_keep_wencai_and_jianpian_visible_while_excluding_zxzj() {
        let storage = Storage::new(unique_test_dir()).expect("storage should initialize");
        let subscription = storage
            .add_subscription("tvbox", "https://example.com/tvbox.json")
            .expect("subscription should be inserted");

        let conn = storage.conn.lock().expect("storage lock should succeed");
        for (id, title, item_type, source) in [
            (211_i64, "文采电影", "movie", "wencai"),
            (212_i64, "荐片剧场", "series", "jianpian"),
            (213_i64, "嵌页剧场", "series", "zxzj"),
        ] {
            conn.execute(
                "INSERT INTO catalog_items (
                    id, subscription_id, site_id, source_item_key, title, item_type, poster, summary, detail_json, updated_at
                 ) VALUES (?1, ?2, NULL, NULL, ?3, ?4, NULL, NULL, ?5, '2026-04-20 00:00:00')",
                rusqlite::params![
                    id,
                    subscription.id,
                    title,
                    item_type,
                    Some(format!(r#"{{"source":"{}"}}"#, source))
                ],
            )
            .expect("catalog item should insert");
        }
        drop(conn);

        let home = storage
            .get_library_home()
            .expect("library home should query");
        let catalog = storage
            .get_catalog_items(None, None)
            .expect("catalog items should query");
        let series_catalog = storage
            .get_catalog_items(Some("series".to_string()), None)
            .expect("series catalog should query");
        let keyword_catalog = storage
            .get_catalog_items(None, Some("文采".to_string()))
            .expect("keyword catalog should query");

        let latest_titles: Vec<&str> = home
            .latest_updates
            .iter()
            .map(|item| item.title.as_str())
            .collect();
        let featured_titles: Vec<&str> = home
            .featured
            .iter()
            .map(|item| item.title.as_str())
            .collect();
        let catalog_titles: Vec<&str> = catalog.iter().map(|item| item.title.as_str()).collect();

        assert_eq!(latest_titles, vec!["荐片剧场", "文采电影"]);
        assert_eq!(featured_titles, vec!["荐片剧场", "文采电影"]);
        assert_eq!(catalog_titles, vec!["荐片剧场", "文采电影"]);
        assert_eq!(series_catalog.len(), 1);
        assert_eq!(series_catalog[0].title, "荐片剧场");
        assert_eq!(keyword_catalog.len(), 1);
        assert_eq!(keyword_catalog[0].title, "文采电影");
    }

    #[test]
    fn simple_json_refresh_keeps_source_lives_in_sync() {
        let storage = Storage::new(unique_test_dir()).expect("storage should initialize");
        let subscription = storage
            .add_subscription("demo", "https://example.com/sub.json")
            .expect("subscription should be inserted");

        storage
            .refresh_subscription(
                subscription.id,
                vec![
                    (
                        "CCTV-1".to_string(),
                        None,
                        "https://a.example/cctv1.m3u8".to_string(),
                        Some("央视频道".to_string()),
                    ),
                    (
                        "CCTV-1".to_string(),
                        None,
                        "https://b.example/cctv1.m3u8".to_string(),
                        Some("央视频道".to_string()),
                    ),
                    (
                        "测试频道".to_string(),
                        None,
                        "https://c.example/test.m3u8".to_string(),
                        None,
                    ),
                ],
                Vec::new(),
            )
            .expect("simple_json refresh should succeed");

        let categories = storage
            .get_live_categories()
            .expect("live categories should query");
        let grouped = storage
            .get_live_channel_groups()
            .expect("live groups should query");
        let channels = storage
            .get_merged_live_channels_by_category("央视频道")
            .expect("live drill-down should query");

        assert_eq!(categories, vec!["其他", "央视频道"]);
        assert_eq!(grouped.len(), 2);
        assert_eq!(grouped[0].category, "其他");
        assert_eq!(grouped[0].channels[0].name, "测试频道");
        assert_eq!(grouped[1].category, "央视频道");
        assert_eq!(grouped[1].channels[0].name, "CCTV-1");
        assert_eq!(grouped[1].channels[0].source_count, 2);
        assert_eq!(channels.len(), 1);
        assert_eq!(channels[0].name, "CCTV-1");
        assert_eq!(channels[0].sources.len(), 2);
    }

    #[test]
    fn catalog_detail_hides_embedded_and_external_lines_and_sorts_playable_first() {
        let storage = Storage::new(unique_test_dir()).expect("storage should initialize");
        let subscription = storage
            .add_subscription("tvbox", "https://example.com/tvbox.json")
            .expect("subscription should be inserted");

        seed_catalog_item(&storage, subscription.id, 201, "示例影片", "movie");
        seed_catalog_episode(
            &storage,
            201,
            "嵌入线路",
            "第01集",
            "https://www.zxzjhd.com/vodplay/4627-1-1.html",
            0,
        );
        seed_catalog_episode(
            &storage,
            201,
            "直链线路",
            "第01集",
            "https://media.example.com/demo/index.m3u8",
            1,
        );
        seed_catalog_episode(
            &storage,
            201,
            "可解析线路",
            "第01集",
            "https://www.xb6v.com/e/DownSys/play/?classid=2&id=28522&pathid2=0&bf=1",
            2,
        );
        seed_catalog_episode(
            &storage,
            201,
            "外部线路",
            "全集",
            "magnet:?xt=urn:btih:test",
            3,
        );

        let detail = storage
            .get_catalog_detail(201)
            .expect("catalog detail should query");

        assert_eq!(detail.episode_groups.len(), 2);
        assert_eq!(detail.episode_groups[0].source_name, "直链线路");
        assert_eq!(
            detail.episode_groups[0].episodes[0].play_url,
            "https://media.example.com/demo/index.m3u8"
        );
        assert_eq!(detail.episode_groups[1].source_name, "可解析线路");
        assert!(detail.episode_groups.iter().all(
            |group| !group.source_name.contains("嵌入") && !group.source_name.contains("外部")
        ));
    }

    fn seed_live_source(
        storage: &Storage,
        subscription_id: i64,
        group_name: Option<&str>,
        channel_name: &str,
        raw_url: &str,
    ) {
        let conn = storage.conn.lock().expect("storage lock should succeed");
        conn.execute(
            "INSERT INTO source_lives (
                subscription_id, group_name, channel_name, raw_url, normalized_url, source_type, raw_json
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                subscription_id,
                group_name,
                channel_name,
                raw_url,
                raw_url,
                Option::<i64>::None,
                "{}"
            ],
        )
        .expect("source live should insert");
    }

    fn seed_catalog_item(
        storage: &Storage,
        subscription_id: i64,
        id: i64,
        title: &str,
        item_type: &str,
    ) {
        seed_catalog_item_with_source(storage, subscription_id, id, title, item_type, "");
    }

    fn seed_catalog_item_with_source(
        storage: &Storage,
        subscription_id: i64,
        id: i64,
        title: &str,
        item_type: &str,
        source: &str,
    ) {
        let conn = storage.conn.lock().expect("storage lock should succeed");
        conn.execute(
            "INSERT INTO catalog_items (
                id, subscription_id, site_id, source_item_key, title, item_type, poster, summary, detail_json, updated_at
             ) VALUES (?1, ?2, NULL, NULL, ?3, ?4, NULL, NULL, ?5, '2026-04-20 00:00:00')",
            rusqlite::params![
                id,
                subscription_id,
                title,
                item_type,
                if source.is_empty() {
                    None::<String>
                } else {
                    Some(format!(r#"{{"source":"{}"}}"#, source))
                }
            ],
        )
        .expect("catalog item should insert");
    }

    fn seed_catalog_episode(
        storage: &Storage,
        catalog_item_id: i64,
        source_name: &str,
        episode_label: &str,
        play_url: &str,
        order_index: i64,
    ) {
        let conn = storage.conn.lock().expect("storage lock should succeed");
        conn.execute(
            "INSERT INTO catalog_episodes (
                catalog_item_id, source_name, season_label, episode_label, play_url, order_index, extra_json
             ) VALUES (?1, ?2, NULL, ?3, ?4, ?5, NULL)",
            rusqlite::params![
                catalog_item_id,
                source_name,
                episode_label,
                play_url,
                order_index
            ],
        )
        .expect("catalog episode should insert");
    }

    fn unique_test_dir() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should advance")
            .as_nanos();
        std::env::temp_dir().join(format!("tvbox-storage-test-{}", nanos))
    }
}
