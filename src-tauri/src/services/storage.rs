use rusqlite::{Connection, Result as SqliteResult};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::models::{LiveChannel, Subscription, VodItem, PlayHistory, DoubanHot, ChannelSource, MergedLiveChannel};

pub struct Storage {
    conn: Arc<Mutex<Connection>>,
}

impl Storage {
    pub fn new(app_data_dir: PathBuf) -> SqliteResult<Self> {
        std::fs::create_dir_all(&app_data_dir).map_err(|e| {
            rusqlite::Error::InvalidPath(format!("无法创建目录: {}", e).into())
        })?;

        let db_path = app_data_dir.join("tvbox.db");
        let conn = Connection::open(db_path)?;

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
            enabled: true,
            created_at: now.clone(),
            updated_at: now,
        })
    }

    pub fn get_subscription(&self, id: i64) -> SqliteResult<Subscription> {
        let conn = self.conn.lock().unwrap();

        conn.query_row(
            "SELECT id, name, url, enabled, created_at, updated_at FROM subscriptions WHERE id = ?1",
            [id],
            |row| {
                Ok(Subscription {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    url: row.get(2)?,
                    enabled: row.get::<_, i32>(3)? != 0,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                })
            },
        )
    }

    pub fn get_subscriptions(&self) -> SqliteResult<Vec<Subscription>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT id, name, url, enabled, created_at, updated_at FROM subscriptions ORDER BY id DESC",
        )?;

        let subscriptions = stmt.query_map([], |row| {
            Ok(Subscription {
                id: row.get(0)?,
                name: row.get(1)?,
                url: row.get(2)?,
                enabled: row.get::<_, i32>(3)? != 0,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })?;

        subscriptions.collect()
    }

    pub fn delete_subscription(&self, id: i64) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute("DELETE FROM live_channels WHERE subscription_id = ?1", [id])?;
        conn.execute("DELETE FROM vod_items WHERE subscription_id = ?1", [id])?;
        conn.execute("DELETE FROM subscriptions WHERE id = ?1", [id])?;

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
                "SELECT lc.id, lc.subscription_id, lc.name, lc.logo, lc.url, lc.category
                 FROM live_channels lc
                 INNER JOIN subscriptions s ON lc.subscription_id = s.id
                 WHERE s.enabled = 1 AND lc.category = ?1
                 ORDER BY lc.name"
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
                "SELECT lc.id, lc.subscription_id, lc.name, lc.logo, lc.url, lc.category
                 FROM live_channels lc
                 INNER JOIN subscriptions s ON lc.subscription_id = s.id
                 WHERE s.enabled = 1
                 ORDER BY lc.name"
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
            "SELECT DISTINCT lc.category FROM live_channels lc
             INNER JOIN subscriptions s ON lc.subscription_id = s.id
             WHERE s.enabled = 1 AND lc.category IS NOT NULL AND lc.category != ''
             ORDER BY lc.category",
        )?;

        let categories = stmt.query_map([], |row| row.get(0))?;
        categories.collect()
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

    pub fn refresh_subscription(&self, id: i64, lives: Vec<(String, Option<String>, String, Option<String>)>, vods: Vec<(String, String, Option<String>, Option<String>, String)>) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();

        // Clear old data
        conn.execute("DELETE FROM live_channels WHERE subscription_id = ?1", [id])?;
        conn.execute("DELETE FROM vod_items WHERE subscription_id = ?1", [id])?;

        // Insert new live channels
        for (name, logo, url, category) in lives {
            conn.execute(
                "INSERT INTO live_channels (subscription_id, name, logo, url, category) VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![id, name, logo, url, category],
            )?;
        }

        // Insert new vod items
        for (name, vtype, poster, description, episodes) in vods {
            conn.execute(
                "INSERT INTO vod_items (subscription_id, name, vtype, poster, description, episodes) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![id, name, vtype, poster, description, episodes],
            )?;
        }

        // Update subscription timestamp
        let now = chrono_now();
        conn.execute(
            "UPDATE subscriptions SET updated_at = ?1 WHERE id = ?2",
            rusqlite::params![now, id],
        )?;

        Ok(())
    }

    pub fn save_play_history(&self, item_type: &str, item_id: i64, progress: f64) -> SqliteResult<()> {
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
                rusqlite::params![item.name, item.year, item.poster, item.rating, item.rank, item.updated_at],
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

        // Get all channels grouped by name+category
        let mut stmt = conn.prepare(
            "SELECT lc.name, lc.logo, lc.category,
                    GROUP_CONCAT(lc.url || '|' || lc.subscription_id) as sources
             FROM live_channels lc
             INNER JOIN subscriptions s ON lc.subscription_id = s.id
             WHERE s.enabled = 1
             GROUP BY lc.name, lc.category
             ORDER BY lc.category, lc.name"
        )?;

        let rows = stmt.query_map([], |row| {
            let sources_str: String = row.get(3)?;
            let name: String = row.get(0)?;
            let category: Option<String> = row.get(2)?;

            let sources: Vec<ChannelSource> = sources_str
                .split(',')
                .filter_map(|s| {
                    let parts: Vec<&str> = s.split('|').collect();
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

            // Generate deterministic ID from name + category
            let id = generate_channel_id(&name, category.as_deref());

            Ok(MergedLiveChannel {
                id,
                name,
                logo: row.get(1)?,
                category,
                sources,
            })
        })?;
        rows.collect()
    }

    pub fn get_merged_live_channels_by_category(&self, category: &str) -> SqliteResult<Vec<MergedLiveChannel>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT lc.name, lc.logo, lc.category,
                    GROUP_CONCAT(lc.url || '|' || lc.subscription_id) as sources
             FROM live_channels lc
             INNER JOIN subscriptions s ON lc.subscription_id = s.id
             WHERE s.enabled = 1 AND lc.category = ?1
             GROUP BY lc.name, lc.category
             ORDER BY lc.name"
        )?;

        let rows = stmt.query_map([category], |row| {
            let sources_str: String = row.get(3)?;
            let name: String = row.get(0)?;
            let category_str: Option<String> = row.get(2)?;

            let sources: Vec<ChannelSource> = sources_str
                .split(',')
                .filter_map(|s| {
                    let parts: Vec<&str> = s.split('|').collect();
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

            let id = generate_channel_id(&name, category_str.as_deref());

            Ok(MergedLiveChannel {
                id,
                name,
                logo: row.get(1)?,
                category: category_str,
                sources,
            })
        })?;
        rows.collect()
    }
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
