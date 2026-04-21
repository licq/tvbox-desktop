use rusqlite::{Connection, Result as SqliteResult};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::models::{
    CatalogDetail, CatalogDetailItem, CatalogEpisode, CatalogEpisodeGroup, ChannelSource,
    DoubanHot, HomeCatalogItem, HomePayload, LiveChannel, LiveChannelGroup, LiveChannelGroupItem,
    MergedLiveChannel, PlayHistory, Subscription, VodItem,
};
use crate::services::tvbox::{
    TvboxConfigRecords, TvboxLiveRecord, TvboxParseRecord, TvboxSiteRecord,
};
use crate::services::xb6v::ScrapedCatalogItem;

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

        let continue_watching = Vec::new();
        let latest_updates = query_home_catalog_items(
            &conn,
            "SELECT ci.id, ci.title, ci.item_type, ci.poster, NULL as progress
             FROM catalog_items ci
             INNER JOIN subscriptions s ON ci.subscription_id = s.id
             WHERE s.enabled = 1
             ORDER BY ci.updated_at DESC, ci.id DESC
             LIMIT 12",
            [],
        )?;
        let featured = query_home_catalog_items(
            &conn,
            "SELECT ci.id, ci.title, ci.item_type, ci.poster, NULL as progress
             FROM catalog_items ci
             INNER JOIN subscriptions s ON ci.subscription_id = s.id
             WHERE s.enabled = 1
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
            grouped.entry(source_name).or_default().push(episode);
        }

        Ok(CatalogDetail {
            item,
            episode_groups: grouped
                .into_iter()
                .map(|(source_name, episodes)| CatalogEpisodeGroup {
                    source_name,
                    episodes,
                })
                .collect(),
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
            "DELETE FROM catalog_items WHERE subscription_id = ?1",
            [subscription_id],
        )?;

        let updated_at = chrono_now();
        for item in items {
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
            for episode in &item.episodes {
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
            }
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

#[cfg(test)]
mod tests {
    use super::Storage;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

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
            .record_subscription_refresh_failure(subscription.id, "tvbox_config", "upstream blocked")
            .expect("failure state should record");

        assert!(
            storage
                .get_live_categories()
                .expect("live categories should query")
                .is_empty()
        );
        assert!(
            storage
                .get_live_channel_groups()
                .expect("live groups should query")
                .is_empty()
        );
        assert!(
            storage
                .get_merged_live_channels()
                .expect("merged live channels should query")
                .is_empty()
        );
    }

    #[test]
    fn library_home_returns_empty_continue_watching_for_now() {
        let storage = Storage::new(unique_test_dir()).expect("storage should initialize");
        let subscription = storage
            .add_subscription("tvbox", "https://example.com/tvbox.json")
            .expect("subscription should be inserted");

        seed_catalog_item(&storage, subscription.id, 101, "示例影片", "movie");
        storage
            .save_play_history("vod", 101, 0.5)
            .expect("legacy play history should insert");

        let home = storage
            .get_library_home()
            .expect("library home should query");

        assert!(home.continue_watching.is_empty());
        assert_eq!(home.latest_updates.len(), 1);
        assert_eq!(home.featured.len(), 1);
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
        let conn = storage.conn.lock().expect("storage lock should succeed");
        conn.execute(
            "INSERT INTO catalog_items (
                id, subscription_id, site_id, source_item_key, title, item_type, poster, summary, detail_json, updated_at
             ) VALUES (?1, ?2, NULL, NULL, ?3, ?4, NULL, NULL, NULL, '2026-04-20 00:00:00')",
            rusqlite::params![id, subscription_id, title, item_type],
        )
        .expect("catalog item should insert");
    }

    fn unique_test_dir() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should advance")
            .as_nanos();
        std::env::temp_dir().join(format!("tvbox-storage-test-{}", nanos))
    }
}
