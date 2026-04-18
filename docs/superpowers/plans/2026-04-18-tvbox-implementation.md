# TVBox 影视仓 - 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 构建一个基于 Rust + Tauri + Vue 3 的跨平台影视仓应用，支持直播电视和影视点播

**Architecture:** 单体架构 - 前端 Vue 3 + Tauri WebView，后端 Rust 处理所有业务逻辑（订阅解析、频道管理、播放器控制、SQLite 存储）

**Tech Stack:** Tauri 2.x, Vue 3, TypeScript, TailwindCSS, Pinia, rusqlite

---

## 文件结构

```
tvbox/
├── src/                          # Vue 前端
│   ├── assets/
│   ├── components/
│   │   ├── ChannelCard.vue
│   │   ├── VodCard.vue
│   │   ├── Player.vue
│   │   └── SearchBar.vue
│   ├── views/
│   │   ├── Home.vue
│   │   ├── Live.vue
│   │   ├── Vod.vue
│   │   ├── PlayerPage.vue
│   │   ├── Subscriptions.vue
│   │   ├── VodDetail.vue
│   │   └── Settings.vue
│   ├── stores/
│   │   ├── subscription.ts
│   │   ├── live.ts
│   │   ├── vod.ts
│   │   └── player.ts
│   ├── types/
│   │   └── index.ts
│   ├── App.vue
│   └── main.ts
├── src-tauri/
│   ├── src/
│   │   ├── commands/
│   │   │   ├── mod.rs
│   │   │   ├── subscription.rs
│   │   │   ├── live.rs
│   │   │   ├── vod.rs
│   │   │   └── player.rs
│   │   ├── models/
│   │   │   └── mod.rs
│   │   ├── services/
│   │   │   ├── mod.rs
│   │   │   ├── parser.rs
│   │   │   └── storage.rs
│   │   └── main.rs
│   ├── Cargo.toml
│   └── tauri.conf.json
└── package.json
```

---

## Phase 1: 项目搭建 (Tasks 1-4)

### Task 1: 初始化 Tauri + Vue 项目

**Files:**
- Create: `package.json`
- Create: `vite.config.ts`
- Create: `tsconfig.json`
- Create: `tailwind.config.js`
- Create: `src-tauri/Cargo.toml`
- Create: `src-tauri/tauri.conf.json`
- Create: `src/main.ts`
- Create: `src/App.vue`

- [ ] **Step 1: 创建 package.json**

```json
{
  "name": "tvbox",
  "version": "0.1.0",
  "private": true,
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "vue-tsc --noEmit && vite build",
    "preview": "vite preview",
    "tauri": "tauri"
  },
  "dependencies": {
    "vue": "^3.4.21",
    "vue-router": "^4.3.0",
    "pinia": "^2.1.7"
  },
  "devDependencies": {
    "@tauri-apps/cli": "^2.0.0",
    "@tauri-apps/api": "^2.0.0",
    "@vitejs/plugin-vue": "^5.0.4",
    "typescript": "^5.4.0",
    "vite": "^5.2.0",
    "vue-tsc": "^2.0.0",
    "tailwindcss": "^3.4.0",
    "autoprefixer": "^10.4.0",
    "postcss": "^8.4.0"
  }
}
```

- [ ] **Step 2: 创建 vite.config.ts**

```typescript
import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import { resolve } from 'path'

export default defineConfig({
  plugins: [vue()],
  resolve: {
    alias: {
      '@': resolve(__dirname, 'src')
    }
  },
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: {
      ignored: ['**/src-tauri/**']
    }
  }
})
```

- [ ] **Step 3: 创建 TypeScript 配置**

```json
{
  "compilerOptions": {
    "target": "ES2020",
    "useDefineForClassFields": true,
    "module": "ESNext",
    "lib": ["ES2020", "DOM", "DOM.Iterable"],
    "skipLibCheck": true,
    "moduleResolution": "bundler",
    "allowImportingTsExtensions": true,
    "resolveJsonModule": true,
    "isolatedModules": true,
    "noEmit": true,
    "jsx": "preserve",
    "strict": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "noFallthroughCasesInSwitch": true,
    "paths": {
      "@/*": ["./src/*"]
    }
  },
  "include": ["src/**/*.ts", "src/**/*.d.ts", "src/**/*.tsx", "src/**/*.vue"],
  "references": [{ "path": "./tsconfig.node.json" }]
}
```

- [ ] **Step 4: 创建 TailwindCSS 配置**

```javascript
/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{vue,js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        primary: '#3B82F6',
        secondary: '#10B981',
        dark: '#1F2937',
        darker: '#111827'
      }
    },
  },
  plugins: [],
}
```

- [ ] **Step 5: 创建 src-tauri/Cargo.toml**

```toml
[package]
name = "tvbox"
version = "0.1.0"
edition = "2021"

[lib]
name = "tvbox_lib"
crate-type = ["staticlib", "cdylib", "rlib"]

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
tauri = { version = "2", features = [] }
tauri-plugin-shell = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
rusqlite = { version = "0.31", features = ["bundled"] }
reqwest = { version = "0.12", features = ["json"] }
tokio = { version = "1", features = ["full"] }
thiserror = "1"
log = "0.4"
env_logger = "0.11"

[profile.release]
panic = "abort"
codegen-units = 1
lto = true
opt-level = "s"
strip = true
```

- [ ] **Step 6: 创建 src-tauri/tauri.conf.json**

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "TVBox",
  "version": "0.1.0",
  "identifier": "com.tvbox.app",
  "build": {
    "devtools": true,
    "frontendDist": "../dist",
    "devUrl": "http://localhost:1420",
    "beforeDevCommand": "npm run dev",
    "beforeBuildCommand": "npm run build"
  },
  "app": {
    "windows": [
      {
        "title": "TVBox 影视仓",
        "width": 1280,
        "height": 720,
        "resizable": true,
        "fullscreen": false
      }
    ],
    "security": {
      "csp": null
    }
  }
}
```

- [ ] **Step 7: 创建 src/main.ts**

```typescript
import { createApp } from 'vue'
import { createPinia } from 'pinia'
import App from './App.vue'
import router from './router'
import './style.css'

const app = createApp(App)

app.use(createPinia())
app.use(router)

app.mount('#app')
```

- [ ] **Step 8: 创建 src/style.css**

```css
@tailwind base;
@tailwind components;
@tailwind utilities;

:root {
  font-family: Inter, system-ui, Avenir, Helvetica, Arial, sans-serif;
  line-height: 1.5;
  font-weight: 400;
  color: white;
  background-color: #111827;
}

* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}
```

- [ ] **Step 9: 创建 src/App.vue**

```vue
<script setup lang="ts">
import { RouterView } from 'vue-router'
</script>

<template>
  <RouterView />
</template>
```

- [ ] **Step 10: 提交代码**

```bash
git add -A
git commit -m "feat: initialize Tauri + Vue project structure"
```

---

### Task 2: 配置 SQLite 和数据模型

**Files:**
- Create: `src-tauri/src/models/mod.rs`
- Create: `src-tauri/src/services/storage.rs`
- Create: `src-tauri/src/services/mod.rs`
- Modify: `src-tauri/src/main.rs`

- [ ] **Step 1: 创建数据模型 src-tauri/src/models/mod.rs**

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    pub id: i64,
    pub name: String,
    pub url: String,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveChannel {
    pub id: i64,
    pub subscription_id: i64,
    pub name: String,
    pub logo: Option<String>,
    pub url: String,
    pub category: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VodItem {
    pub id: i64,
    pub subscription_id: i64,
    pub name: String,
    pub vtype: String,
    pub poster: Option<String>,
    pub description: Option<String>,
    pub episodes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayHistory {
    pub id: i64,
    pub item_type: String,
    pub item_id: i64,
    pub progress: f64,
    pub last_played: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewSubscription {
    pub name: String,
    pub url: String,
}
```

- [ ] **Step 2: 创建存储服务 src-tauri/src/services/storage.rs**

```rust
use rusqlite::{Connection, Result, params};
use std::sync::Mutex;
use std::path::PathBuf;
use crate::models::*;

pub struct Storage {
    conn: Mutex<Connection>,
}

impl Storage {
    pub fn new(app_data_dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&app_data_dir).ok();
        let db_path = app_data_dir.join("tvbox.db");
        let conn = Connection::open(db_path)?;
        let storage = Storage { conn: Mutex::new(conn) };
        storage.init_tables()?;
        Ok(storage)
    }

    fn init_tables(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "CREATE TABLE IF NOT EXISTS subscriptions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                url TEXT NOT NULL UNIQUE,
                enabled INTEGER DEFAULT 1,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS live_channels (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                subscription_id INTEGER,
                name TEXT NOT NULL,
                logo TEXT,
                url TEXT NOT NULL,
                category TEXT,
                FOREIGN KEY (subscription_id) REFERENCES subscriptions(id)
            )",
            [],
        )?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS vod_items (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                subscription_id INTEGER,
                name TEXT NOT NULL,
                type TEXT,
                poster TEXT,
                description TEXT,
                episodes TEXT,
                FOREIGN KEY (subscription_id) REFERENCES subscriptions(id)
            )",
            [],
        )?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS play_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                item_type TEXT,
                item_id INTEGER,
                progress REAL,
                last_played TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;
        Ok(())
    }

    // Subscription CRUD
    pub fn add_subscription(&self, name: &str, url: &str) -> Result<Subscription> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO subscriptions (name, url) VALUES (?1, ?2)",
            params![name, url],
        )?;
        let id = conn.last_insert_rowid();
        self.get_subscription(id)
    }

    pub fn get_subscription(&self, id: i64) -> Result<Subscription> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, name, url, enabled, created_at, updated_at FROM subscriptions WHERE id = ?1",
            params![id],
            |row| Ok(Subscription {
                id: row.get(0)?,
                name: row.get(1)?,
                url: row.get(2)?,
                enabled: row.get::<_, i32>(3)? == 1,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            }),
        )
    }

    pub fn get_subscriptions(&self) -> Result<Vec<Subscription>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, url, enabled, created_at, updated_at FROM subscriptions"
        )?;
        let rows = stmt.query_map([], |row| Ok(Subscription {
            id: row.get(0)?,
            name: row.get(1)?,
            url: row.get(2)?,
            enabled: row.get::<_, i32>(3)? == 1,
            created_at: row.get(4)?,
            updated_at: row.get(5)?,
        }))?;
        rows.collect()
    }

    pub fn delete_subscription(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM live_channels WHERE subscription_id = ?1", params![id])?;
        conn.execute("DELETE FROM vod_items WHERE subscription_id = ?1", params![id])?;
        conn.execute("DELETE FROM subscriptions WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn toggle_subscription(&self, id: i64, enabled: bool) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE subscriptions SET enabled = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
            params![enabled as i32, id],
        )?;
        Ok(())
    }

    // Live channels
    pub fn get_live_channels(&self, category: Option<String>) -> Result<Vec<LiveChannel>> {
        let conn = self.conn.lock().unwrap();
        let query = match category {
            Some(_) => "SELECT lc.id, lc.subscription_id, lc.name, lc.logo, lc.url, lc.category
                       FROM live_channels lc
                       JOIN subscriptions s ON lc.subscription_id = s.id
                       WHERE s.enabled = 1 AND lc.category = ?1",
            None => "SELECT lc.id, lc.subscription_id, lc.name, lc.logo, lc.url, lc.category
                    FROM live_channels lc
                    JOIN subscriptions s ON lc.subscription_id = s.id
                    WHERE s.enabled = 1",
        };
        let mut stmt = conn.prepare(query)?;
        let rows = match category {
            Some(cat) => stmt.query_map(params![cat], |row| Ok(LiveChannel {
                id: row.get(0)?,
                subscription_id: row.get(1)?,
                name: row.get(2)?,
                logo: row.get(3)?,
                url: row.get(4)?,
                category: row.get(5)?,
            }))?,
            None => stmt.query_map([], |row| Ok(LiveChannel {
                id: row.get(0)?,
                subscription_id: row.get(1)?,
                name: row.get(2)?,
                logo: row.get(3)?,
                url: row.get(4)?,
                category: row.get(5)?,
            }))?,
        };
        rows.collect()
    }

    pub fn get_live_categories(&self) -> Result<Vec<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT DISTINCT lc.category FROM live_channels lc
             JOIN subscriptions s ON lc.subscription_id = s.id
             WHERE s.enabled = 1 AND lc.category IS NOT NULL"
        )?;
        let rows = stmt.query_map([], |row| row.get(0))?;
        rows.collect()
    }

    pub fn clear_channels_for_subscription(&self, subscription_id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM live_channels WHERE subscription_id = ?1",
            params![subscription_id],
        )?;
        Ok(())
    }

    pub fn add_live_channel(&self, sub_id: i64, name: &str, logo: Option<&str>, url: &str, category: Option<&str>) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO live_channels (subscription_id, name, logo, url, category) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![sub_id, name, logo, url, category],
        )?;
        Ok(())
    }

    // VOD items
    pub fn get_vod_items(&self, vtype: Option<String>, page: u32) -> Result<Vec<VodItem>> {
        let conn = self.conn.lock().unwrap();
        let page_size = 20;
        let offset = page * page_size;
        let query = match vtype {
            Some(_) => "SELECT id, subscription_id, name, type, poster, description, episodes
                       FROM vod_items WHERE type = ?1 LIMIT ?2 OFFSET ?3",
            None => "SELECT id, subscription_id, name, type, poster, description, episodes
                    FROM vod_items LIMIT ?1 OFFSET ?2",
        };
        let mut stmt = conn.prepare(query)?;
        let rows = match vtype {
            Some(t) => stmt.query_map(params![t, page_size, offset], |row| Ok(VodItem {
                id: row.get(0)?,
                subscription_id: row.get(1)?,
                name: row.get(2)?,
                vtype: row.get(3)?,
                poster: row.get(4)?,
                description: row.get(5)?,
                episodes: row.get(6)?,
            }))?,
            None => stmt.query_map(params![page_size, offset], |row| Ok(VodItem {
                id: row.get(0)?,
                subscription_id: row.get(1)?,
                name: row.get(2)?,
                vtype: row.get(3)?,
                poster: row.get(4)?,
                description: row.get(5)?,
                episodes: row.get(6)?,
            }))?,
        };
        rows.collect()
    }

    pub fn get_vod_detail(&self, id: i64) -> Result<VodItem> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, subscription_id, name, type, poster, description, episodes FROM vod_items WHERE id = ?1",
            params![id],
            |row| Ok(VodItem {
                id: row.get(0)?,
                subscription_id: row.get(1)?,
                name: row.get(2)?,
                vtype: row.get(3)?,
                poster: row.get(4)?,
                description: row.get(5)?,
                episodes: row.get(6)?,
            }),
        )
    }

    pub fn search_vod(&self, keyword: &str) -> Result<Vec<VodItem>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, subscription_id, name, type, poster, description, episodes
             FROM vod_items WHERE name LIKE ?1"
        )?;
        let pattern = format!("%{}%", keyword);
        let rows = stmt.query_map(params![pattern], |row| Ok(VodItem {
            id: row.get(0)?,
            subscription_id: row.get(1)?,
            name: row.get(2)?,
            vtype: row.get(3)?,
            poster: row.get(4)?,
            description: row.get(5)?,
            episodes: row.get(6)?,
        }))?;
        rows.collect()
    }

    pub fn clear_vod_for_subscription(&self, subscription_id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM vod_items WHERE subscription_id = ?1",
            params![subscription_id],
        )?;
        Ok(())
    }

    pub fn add_vod_item(&self, sub_id: i64, name: &str, vtype: &str, poster: Option<&str>, description: Option<&str>, episodes: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO vod_items (subscription_id, name, type, poster, description, episodes) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![sub_id, name, vtype, poster, description, episodes],
        )?;
        Ok(())
    }

    // Play history
    pub fn save_play_history(&self, item_type: &str, item_id: i64, progress: f64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO play_history (item_type, item_id, progress) VALUES (?1, ?2, ?3)",
            params![item_type, item_id, progress],
        )?;
        Ok(())
    }

    pub fn get_play_history(&self) -> Result<Vec<PlayHistory>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, item_type, item_id, progress, last_played FROM play_history ORDER BY last_played DESC LIMIT 50"
        )?;
        let rows = stmt.query_map([], |row| Ok(PlayHistory {
            id: row.get(0)?,
            item_type: row.get(1)?,
            item_id: row.get(2)?,
            progress: row.get(3)?,
            last_played: row.get(4)?,
        }))?;
        rows.collect()
    }
}
```

- [ ] **Step 3: 创建 services/mod.rs**

```rust
pub mod storage;
pub mod parser;

pub use storage::Storage;
pub use parser::Parser;
```

- [ ] **Step 4: 创建订阅解析器 src-tauri/src/services/parser.rs**

```rust
use serde::{Deserialize, Serialize};
use crate::models::{LiveChannel, VodItem};

#[derive(Debug, Deserialize)]
pub struct SubscriptionJson {
    #[serde(rename = "lives")]
    pub lives: Option<Vec<LiveChannelJson>>,
    #[serde(rename = "vods")]
    pub vods: Option<Vec<VodItemJson>>,
}

#[derive(Debug, Deserialize)]
pub struct LiveChannelJson {
    pub name: String,
    pub logo: Option<String>,
    pub url: String,
    pub category: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct VodItemJson {
    pub name: String,
    #[serde(rename = "type")]
    pub vtype: Option<String>,
    pub poster: Option<String>,
    pub description: Option<String>,
    pub episodes: Option<Vec<EpisodeJson>>,
}

#[derive(Debug, Deserialize)]
pub struct EpisodeJson {
    pub name: String,
    pub url: String,
}

pub struct Parser;

impl Parser {
    pub fn parse_subscription(content: &str) -> Result<SubscriptionJson, String> {
        serde_json::from_str(content).map_err(|e| format!("JSON解析失败: {}", e))
    }

    pub fn parse_episodes(episodes: Option<Vec<EpisodeJson>>) -> String {
        match episodes {
            Some(eps) => serde_json::to_string(&eps).unwrap_or_else(|_| "[]".to_string()),
            None => "[]".to_string(),
        }
    }
}
```

- [ ] **Step 5: 更新 main.rs**

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod models;
mod services;

use services::Storage;
use std::sync::Mutex;
use tauri::Manager;

struct AppState {
    storage: Storage,
}

fn main() {
    env_logger::init();
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir().expect("无法获取应用数据目录");
            let storage = Storage::new(app_data_dir).expect("无法初始化数据库");
            app.manage(AppState { storage });
            log::info!("TVBox 应用启动成功");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::subscription::add_subscription,
            commands::subscription::get_subscriptions,
            commands::subscription::delete_subscription,
            commands::subscription::refresh_subscription,
            commands::subscription::toggle_subscription,
            commands::live::get_live_channels,
            commands::live::get_live_categories,
            commands::vod::get_vod_items,
            commands::vod::get_vod_detail,
            commands::vod::search_vod,
            commands::player::save_play_history,
            commands::player::get_play_history,
        ])
        .run(tauri::generate_context!())
        .expect("启动 Tauri 应用时出错");
}
```

- [ ] **Step 6: 创建 Tauri 命令模块**

首先创建 `src-tauri/src/commands/mod.rs`:

```rust
pub mod subscription;
pub mod live;
pub mod vod;
pub mod player;
```

- [ ] **Step 7: 提交代码**

```bash
git add -A
git commit -m "feat: add SQLite storage and data models"
```

---

### Task 3: 实现订阅管理命令

**Files:**
- Create: `src-tauri/src/commands/subscription.rs`

- [ ] **Step 1: 创建订阅命令 src-tauri/src/commands/subscription.rs**

```rust
use crate::models::{Subscription, NewSubscription};
use crate::AppState;
use tauri::State;
use std::sync::Mutex;

#[tauri::command]
pub async fn add_subscription(
    name: String,
    url: String,
    state: State<'_, AppState>,
) -> Result<Subscription, String> {
    let storage = state.storage.clone();
    tokio::task::spawn_blocking(move || {
        storage.add_subscription(&name, &url).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_subscriptions(
    state: State<'_, AppState>,
) -> Result<Vec<Subscription>, String> {
    let storage = state.storage.clone();
    tokio::task::spawn_blocking(move || {
        storage.get_subscriptions().map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn delete_subscription(
    id: i64,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let storage = state.storage.clone();
    tokio::task::spawn_blocking(move || {
        storage.delete_subscription(id).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn refresh_subscription(
    id: i64,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let storage = state.storage.clone();

    // 获取订阅信息
    let subscription = tokio::task::spawn_blocking({
        let storage = storage.clone();
        move || storage.get_subscription(id)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;

    // 从 URL 获取内容
    let response = reqwest::get(&subscription.url)
        .await
        .map_err(|e| format!("网络请求失败: {}", e))?
        .text()
        .await
        .map_err(|e| format!("读取响应失败: {}", e))?;

    // 解析 JSON
    let parsed = crate::services::Parser::parse_subscription(&response)
        .map_err(|e| e.to_string())?;

    // 更新数据库
    tokio::task::spawn_blocking(move || {
        let conn = storage.conn.lock().unwrap();
        // 清除旧数据
        conn.execute("DELETE FROM live_channels WHERE subscription_id = ?1", [id])?;
        conn.execute("DELETE FROM vod_items WHERE subscription_id = ?1", [id])?;

        // 插入新数据
        if let Some(lives) = parsed.lives {
            for live in lives {
                conn.execute(
                    "INSERT INTO live_channels (subscription_id, name, logo, url, category) VALUES (?1, ?2, ?3, ?4, ?5)",
                    [id.to_string(), live.name, live.logo.unwrap_or_default(), live.url, live.category.unwrap_or_default()],
                )?;
            }
        }

        if let Some(vods) = parsed.vods {
            for vod in vods {
                let episodes = crate::services::Parser::parse_episodes(vod.episodes);
                conn.execute(
                    "INSERT INTO vod_items (subscription_id, name, type, poster, description, episodes) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    [id.to_string(), vod.name, vod.vtype.unwrap_or_default(), vod.poster.unwrap_or_default(), vod.description.unwrap_or_default(), episodes],
                )?;
            }
        }

        // 更新订阅时间
        conn.execute(
            "UPDATE subscriptions SET updated_at = CURRENT_TIMESTAMP WHERE id = ?1",
            [id],
        )?;

        Ok::<(), rusqlite::Error>(())
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn toggle_subscription(
    id: i64,
    enabled: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let storage = state.storage.clone();
    tokio::task::spawn_blocking(move || {
        storage.toggle_subscription(id, enabled).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
```

- [ ] **Step 2: 修复 Storage 的 Mutex 问题**

Storage 中的 Mutex 字段需要实现 Clone:

```rust
pub struct Storage {
    conn: Mutex<Connection>,
}

impl Clone for Storage {
    fn clone(&self) -> Self {
        Storage {
            conn: self.conn.clone(),
        }
    }
}
```

- [ ] **Step 3: 提交代码**

```bash
git add -A
git commit -m "feat: implement subscription management commands"
```

---

### Task 4: 实现直播和点播命令

**Files:**
- Create: `src-tauri/src/commands/live.rs`
- Create: `src-tauri/src/commands/vod.rs`

- [ ] **Step 1: 创建直播命令 src-tauri/src/commands/live.rs**

```rust
use crate::models::LiveChannel;
use crate::AppState;
use tauri::State;

#[tauri::command]
pub async fn get_live_channels(
    category: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<LiveChannel>, String> {
    let storage = state.storage.clone();
    tokio::task::spawn_blocking(move || {
        storage.get_live_channels(category).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_live_categories(
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    let storage = state.storage.clone();
    tokio::task::spawn_blocking(move || {
        storage.get_live_categories().map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
```

- [ ] **Step 2: 创建点播命令 src-tauri/src/commands/vod.rs**

```rust
use crate::models::VodItem;
use crate::AppState;
use tauri::State;

#[tauri::command]
pub async fn get_vod_items(
    vtype: Option<String>,
    page: u32,
    state: State<'_, AppState>,
) -> Result<Vec<VodItem>, String> {
    let storage = state.storage.clone();
    tokio::task::spawn_blocking(move || {
        storage.get_vod_items(vtype, page).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_vod_detail(
    id: i64,
    state: State<'_, AppState>,
) -> Result<VodItem, String> {
    let storage = state.storage.clone();
    tokio::task::spawn_blocking(move || {
        storage.get_vod_detail(id).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn search_vod(
    keyword: String,
    state: State<'_, AppState>,
) -> Result<Vec<VodItem>, String> {
    let storage = state.storage.clone();
    tokio::task::spawn_blocking(move || {
        storage.search_vod(&keyword).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
```

- [ ] **Step 3: 创建播放命令 src-tauri/src/commands/player.rs**

```rust
use crate::models::PlayHistory;
use crate::AppState;
use tauri::State;

#[tauri::command]
pub async fn save_play_history(
    item_type: String,
    item_id: i64,
    progress: f64,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let storage = state.storage.clone();
    tokio::task::spawn_blocking(move || {
        storage.save_play_history(&item_type, item_id, progress).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_play_history(
    state: State<'_, AppState>,
) -> Result<Vec<PlayHistory>, String> {
    let storage = state.storage.clone();
    tokio::task::spawn_blocking(move || {
        storage.get_play_history().map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
```

- [ ] **Step 4: 提交代码**

```bash
git add -A
git commit -m "feat: implement live, VOD and player commands"
```

---

## Phase 2: 前端基础组件 (Tasks 5-6)

### Task 5: 创建前端类型定义和路由

**Files:**
- Create: `src/types/index.ts`
- Create: `src/router/index.ts`

- [ ] **Step 1: 创建类型定义 src/types/index.ts**

```typescript
export interface Subscription {
  id: number;
  name: string;
  url: string;
  enabled: boolean;
  created_at?: string;
  updated_at?: string;
}

export interface LiveChannel {
  id: number;
  subscription_id: number;
  name: string;
  logo?: string;
  url: string;
  category?: string;
}

export interface VodItem {
  id: number;
  subscription_id: number;
  name: string;
  type: 'movie' | 'tv' | 'variety' | 'anime';
  poster?: string;
  description?: string;
  episodes: Episode[];
}

export interface Episode {
  name: string;
  url: string;
}

export interface PlayHistory {
  id: number;
  item_type: 'live' | 'vod';
  item_id: number;
  progress: number;
  last_played: string;
}
```

- [ ] **Step 2: 创建路由 src/router/index.ts**

```typescript
import { createRouter, createWebHistory } from 'vue-router'
import Home from '@/views/Home.vue'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: '/',
      name: 'home',
      component: Home
    },
    {
      path: '/live',
      name: 'live',
      component: () => import('@/views/Live.vue')
    },
    {
      path: '/vod',
      name: 'vod',
      component: () => import('@/views/Vod.vue')
    },
    {
      path: '/player/:type/:id',
      name: 'player',
      component: () => import('@/views/PlayerPage.vue')
    },
    {
      path: '/subscriptions',
      name: 'subscriptions',
      component: () => import('@/views/Subscriptions.vue')
    },
    {
      path: '/vod/:id',
      name: 'vod-detail',
      component: () => import('@/views/VodDetail.vue')
    },
    {
      path: '/settings',
      name: 'settings',
      component: () => import('@/views/Settings.vue')
    }
  ]
})

export default router
```

- [ ] **Step 3: 提交代码**

```bash
git add -A
git commit -m "feat: add TypeScript types and Vue router"
```

---

### Task 6: 创建 Pinia Stores

**Files:**
- Create: `src/stores/subscription.ts`
- Create: `src/stores/live.ts`
- Create: `src/stores/vod.ts`
- Create: `src/stores/player.ts`

- [ ] **Step 1: 创建订阅 Store src/stores/subscription.ts**

```typescript
import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { Subscription } from '@/types'

export const useSubscriptionStore = defineStore('subscription', () => {
  const subscriptions = ref<Subscription[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function fetchSubscriptions() {
    loading.value = true
    error.value = null
    try {
      subscriptions.value = await invoke<Subscription[]>('get_subscriptions')
    } catch (e) {
      error.value = String(e)
    } finally {
      loading.value = false
    }
  }

  async function addSubscription(name: string, url: string) {
    try {
      const sub = await invoke<Subscription>('add_subscription', { name, url })
      subscriptions.value.push(sub)
      return sub
    } catch (e) {
      error.value = String(e)
      throw e
    }
  }

  async function deleteSubscription(id: number) {
    try {
      await invoke('delete_subscription', { id })
      subscriptions.value = subscriptions.value.filter(s => s.id !== id)
    } catch (e) {
      error.value = String(e)
      throw e
    }
  }

  async function refreshSubscription(id: number) {
    try {
      await invoke('refresh_subscription', { id })
    } catch (e) {
      error.value = String(e)
      throw e
    }
  }

  async function toggleSubscription(id: number, enabled: boolean) {
    try {
      await invoke('toggle_subscription', { id, enabled })
      const sub = subscriptions.value.find(s => s.id === id)
      if (sub) sub.enabled = enabled
    } catch (e) {
      error.value = String(e)
      throw e
    }
  }

  return {
    subscriptions,
    loading,
    error,
    fetchSubscriptions,
    addSubscription,
    deleteSubscription,
    refreshSubscription,
    toggleSubscription
  }
})
```

- [ ] **Step 2: 创建直播 Store src/stores/live.ts**

```typescript
import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { LiveChannel } from '@/types'

export const useLiveStore = defineStore('live', () => {
  const channels = ref<LiveChannel[]>([])
  const categories = ref<string[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function fetchChannels(category?: string) {
    loading.value = true
    error.value = null
    try {
      channels.value = await invoke<LiveChannel[]>('get_live_channels', { category: category || null })
    } catch (e) {
      error.value = String(e)
    } finally {
      loading.value = false
    }
  }

  async function fetchCategories() {
    try {
      categories.value = await invoke<string[]>('get_live_categories')
    } catch (e) {
      error.value = String(e)
    }
  }

  return {
    channels,
    categories,
    loading,
    error,
    fetchChannels,
    fetchCategories
  }
})
```

- [ ] **Step 3: 创建点播 Store src/stores/vod.ts**

```typescript
import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { VodItem } from '@/types'

export const useVodStore = defineStore('vod', () => {
  const items = ref<VodItem[]>([])
  const currentItem = ref<VodItem | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function fetchItems(type?: string, page = 0) {
    loading.value = true
    error.value = null
    try {
      items.value = await invoke<VodItem[]>('get_vod_items', { vtype: type || null, page })
    } catch (e) {
      error.value = String(e)
    } finally {
      loading.value = false
    }
  }

  async function fetchDetail(id: number) {
    loading.value = true
    error.value = null
    try {
      currentItem.value = await invoke<VodItem>('get_vod_detail', { id })
    } catch (e) {
      error.value = String(e)
    } finally {
      loading.value = false
    }
  }

  async function search(keyword: string) {
    loading.value = true
    error.value = null
    try {
      items.value = await invoke<VodItem[]>('search_vod', { keyword })
    } catch (e) {
      error.value = String(e)
    } finally {
      loading.value = false
    }
  }

  return {
    items,
    currentItem,
    loading,
    error,
    fetchItems,
    fetchDetail,
    search
  }
})
```

- [ ] **Step 4: 创建播放器 Store src/stores/player.ts**

```typescript
import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { PlayHistory } from '@/types'

export const usePlayerStore = defineStore('player', () => {
  const currentUrl = ref<string | null>(null)
  const history = ref<PlayHistory[]>([])
  const loading = ref(false)

  async function saveHistory(itemType: string, itemId: number, progress: number) {
    try {
      await invoke('save_play_history', { itemType, itemId, progress })
    } catch (e) {
      console.error('保存播放历史失败:', e)
    }
  }

  async function fetchHistory() {
    loading.value = true
    try {
      history.value = await invoke<PlayHistory[]>('get_play_history')
    } catch (e) {
      console.error('获取播放历史失败:', e)
    } finally {
      loading.value = false
    }
  }

  return {
    currentUrl,
    history,
    loading,
    saveHistory,
    fetchHistory
  }
})
```

- [ ] **Step 5: 提交代码**

```bash
git add -A
git commit -m "feat: add Pinia stores for state management"
```

---

## Phase 3: 前端页面组件 (Tasks 7-11)

### Task 7: 创建公共组件

**Files:**
- Create: `src/components/ChannelCard.vue`
- Create: `src/components/VodCard.vue`
- Create: `src/components/SearchBar.vue`
- Create: `src/components/LoadingSpinner.vue`

- [ ] **Step 1: 创建 ChannelCard.vue**

```vue
<script setup lang="ts">
import type { LiveChannel } from '@/types'

defineProps<{
  channel: LiveChannel
}>()

defineEmits<{
  play: [channel: LiveChannel]
}>()
</script>

<template>
  <div
    class="channel-card bg-gray-800 rounded-lg p-4 cursor-pointer hover:bg-gray-700 transition-colors"
    @click="$emit('play', channel)"
  >
    <img
      v-if="channel.logo"
      :src="channel.logo"
      :alt="channel.name"
      class="w-16 h-16 mx-auto mb-2 object-contain"
    />
    <div v-else class="w-16 h-16 mx-auto mb-2 bg-gray-600 rounded-full flex items-center justify-center text-2xl">
      📺
    </div>
    <div class="text-center text-sm truncate">{{ channel.name }}</div>
    <div v-if="channel.category" class="text-center text-xs text-gray-400 mt-1">
      {{ channel.category }}
    </div>
  </div>
</template>
```

- [ ] **Step 2: 创建 VodCard.vue**

```vue
<script setup lang="ts">
import type { VodItem } from '@/types'

defineProps<{
  item: VodItem
}>()

defineEmits<{
  click: [item: VodItem]
}>()
</script>

<template>
  <div
    class="vod-card bg-gray-800 rounded-lg overflow-hidden cursor-pointer hover:bg-gray-700 transition-colors"
    @click="$emit('click', item)"
  >
    <img
      v-if="item.poster"
      :src="item.poster"
      :alt="item.name"
      class="w-full aspect-[2/3] object-cover"
    />
    <div v-else class="w-full aspect-[2/3] bg-gray-600 flex items-center justify-center text-4xl">
      🎬
    </div>
    <div class="p-2">
      <div class="text-sm truncate">{{ item.name }}</div>
      <div class="text-xs text-gray-400 mt-1">{{ item.type }}</div>
    </div>
  </div>
</template>
```

- [ ] **Step 3: 创建 SearchBar.vue**

```vue
<script setup lang="ts">
import { ref } from 'vue'

const props = defineProps<{
  placeholder?: string
}>()

const emit = defineEmits<{
  search: [keyword: string]
}>()

const keyword = ref('')

function handleSearch() {
  emit('search', keyword.value)
}
</script>

<template>
  <div class="search-bar flex items-center bg-gray-700 rounded-lg px-3 py-2">
    <input
      v-model="keyword"
      type="text"
      :placeholder="placeholder || '搜索...'"
      class="bg-transparent flex-1 outline-none text-white"
      @keyup.enter="handleSearch"
    />
    <button @click="handleSearch" class="text-gray-400 hover:text-white">
      🔍
    </button>
  </div>
</template>
```

- [ ] **Step 4: 创建 LoadingSpinner.vue**

```vue
<script setup lang="ts">
defineProps<{
  size?: 'sm' | 'md' | 'lg'
}>()
</script>

<template>
  <div class="loading-spinner flex items-center justify-center">
    <div
      :class="[
        'animate-spin rounded-full border-2 border-gray-600 border-t-primary',
        { 'w-4 h-4': size === 'sm', 'w-8 h-8': size === 'md', 'w-12 h-12': size === 'lg' || !size }
      ]"
    ></div>
  </div>
</template>
```

- [ ] **Step 5: 提交代码**

```bash
git add -A
git commit -m "feat: add common UI components"
```

---

### Task 8: 创建 Home 页面

**Files:**
- Create: `src/views/Home.vue`

- [ ] **Step 1: 创建 Home.vue**

```vue
<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { RouterView } from 'vue-router'
import { useLiveStore } from '@/stores/live'
import { useVodStore } from '@/stores/vod'
import ChannelCard from '@/components/ChannelCard.vue'
import VodCard from '@/components/VodCard.vue'
import type { LiveChannel, VodItem } from '@/types'

const liveStore = useLiveStore()
const vodStore = useVodStore()

const activeTab = ref<'live' | 'vod'>('live')

onMounted(async () => {
  await Promise.all([
    liveStore.fetchChannels(),
    vodStore.fetchItems()
  ])
})

function handlePlayChannel(channel: LiveChannel) {
  window.location.href = `/player/live/${channel.id}`
}

function handleVodClick(item: VodItem) {
  window.location.href = `/vod/${item.id}`
}
</script>

<template>
  <div class="home min-h-screen bg-gray-900 text-white">
    <!-- Header -->
    <header class="bg-gray-800 p-4 flex items-center justify-between">
      <h1 class="text-2xl font-bold">📺 TVBox 影视仓</h1>
      <div class="flex gap-4">
        <RouterLink to="/subscriptions" class="px-4 py-2 bg-primary rounded hover:bg-blue-600 transition">
          订阅管理
        </RouterLink>
        <RouterLink to="/settings" class="px-4 py-2 bg-gray-700 rounded hover:bg-gray-600 transition">
          ⚙️
        </RouterLink>
      </div>
    </header>

    <!-- Tabs -->
    <div class="flex border-b border-gray-700">
      <button
        :class="['px-6 py-3 text-lg', activeTab === 'live' ? 'border-b-2 border-primary text-primary' : 'text-gray-400']"
        @click="activeTab = 'live'"
      >
        📺 直播
      </button>
      <button
        :class="['px-6 py-3 text-lg', activeTab === 'vod' ? 'border-b-2 border-primary text-primary' : 'text-gray-400']"
        @click="activeTab = 'vod'"
      >
        🎬 点播
      </button>
    </div>

    <!-- Content -->
    <main class="p-4">
      <!-- Live Tab -->
      <div v-if="activeTab === 'live'">
        <div v-if="liveStore.loading" class="flex justify-center py-8">
          <div class="animate-spin w-8 h-8 border-2 border-primary border-t-transparent rounded-full"></div>
        </div>
        <div v-else-if="liveStore.channels.length === 0" class="text-center py-8 text-gray-400">
          <p class="text-xl mb-4">暂无频道</p>
          <RouterLink to="/subscriptions" class="text-primary hover:underline">
            添加订阅源
          </RouterLink>
        </div>
        <div v-else class="grid grid-cols-4 md:grid-cols-6 lg:grid-cols-8 gap-4">
          <ChannelCard
            v-for="channel in liveStore.channels"
            :key="channel.id"
            :channel="channel"
            @play="handlePlayChannel"
          />
        </div>
      </div>

      <!-- VOD Tab -->
      <div v-if="activeTab === 'vod'">
        <div v-if="vodStore.loading" class="flex justify-center py-8">
          <div class="animate-spin w-8 h-8 border-2 border-primary border-t-transparent rounded-full"></div>
        </div>
        <div v-else-if="vodStore.items.length === 0" class="text-center py-8 text-gray-400">
          <p class="text-xl mb-4">暂无影视</p>
          <RouterLink to="/subscriptions" class="text-primary hover:underline">
            添加订阅源
          </RouterLink>
        </div>
        <div v-else class="grid grid-cols-3 md:grid-cols-4 lg:grid-cols-6 gap-4">
          <VodCard
            v-for="item in vodStore.items"
            :key="item.id"
            :item="item"
            @click="handleVodClick"
          />
        </div>
      </div>
    </main>
  </div>
</template>
```

- [ ] **Step 2: 提交代码**

```bash
git add -A
git commit -m "feat: add Home page with live and VOD tabs"
```

---

### Task 9: 创建 Live 和 Subscriptions 页面

**Files:**
- Create: `src/views/Live.vue`
- Create: `src/views/Subscriptions.vue`

- [ ] **Step 1: 创建 Live.vue**

```vue
<script setup lang="ts">
import { onMounted } from 'vue'
import { useLiveStore } from '@/stores/live'
import ChannelCard from '@/components/ChannelCard.vue'
import type { LiveChannel } from '@/types'

const liveStore = useLiveStore()

onMounted(async () => {
  await liveStore.fetchChannels()
  await liveStore.fetchCategories()
})

function handlePlayChannel(channel: LiveChannel) {
  window.location.href = `/player/live/${channel.id}`
}
</script>

<template>
  <div class="live-page min-h-screen bg-gray-900 text-white p-4">
    <header class="mb-6">
      <h1 class="text-2xl font-bold mb-4">📺 直播电视</h1>

      <!-- Categories -->
      <div class="flex gap-2 flex-wrap">
        <button
          class="px-3 py-1 bg-gray-700 rounded hover:bg-primary transition"
          @click="liveStore.fetchChannels()"
        >
          全部
        </button>
        <button
          v-for="cat in liveStore.categories"
          :key="cat"
          class="px-3 py-1 bg-gray-700 rounded hover:bg-primary transition"
          @click="liveStore.fetchChannels(cat)"
        >
          {{ cat }}
        </button>
      </div>
    </header>

    <div v-if="liveStore.loading" class="flex justify-center py-8">
      <div class="animate-spin w-8 h-8 border-2 border-primary border-t-transparent rounded-full"></div>
    </div>

    <div v-else-if="liveStore.channels.length === 0" class="text-center py-8 text-gray-400">
      暂无频道
    </div>

    <div v-else class="grid grid-cols-4 md:grid-cols-6 lg:grid-cols-8 gap-4">
      <ChannelCard
        v-for="channel in liveStore.channels"
        :key="channel.id"
        :channel="channel"
        @play="handlePlayChannel"
      />
    </div>
  </div>
</template>
```

- [ ] **Step 2: 创建 Subscriptions.vue**

```vue
<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useSubscriptionStore } from '@/stores/subscription'
import type { Subscription } from '@/types'

const subStore = useSubscriptionStore()

const showAddForm = ref(false)
const newName = ref('')
const newUrl = ref('')
const refreshing = ref<number | null>(null)

onMounted(() => {
  subStore.fetchSubscriptions()
})

async function handleAdd() {
  if (!newName.value || !newUrl.value) return
  try {
    await subStore.addSubscription(newName.value, newUrl.value)
    newName.value = ''
    newUrl.value = ''
    showAddForm.value = false
  } catch (e) {
    alert('添加失败: ' + e)
  }
}

async function handleRefresh(sub: Subscription) {
  refreshing.value = sub.id
  try {
    await subStore.refreshSubscription(sub.id)
    alert('刷新成功')
  } catch (e) {
    alert('刷新失败: ' + e)
  } finally {
    refreshing.value = null
  }
}

async function handleToggle(sub: Subscription) {
  try {
    await subStore.toggleSubscription(sub.id, !sub.enabled)
  } catch (e) {
    alert('切换失败: ' + e)
  }
}

async function handleDelete(sub: Subscription) {
  if (!confirm(`确定删除订阅 "${sub.name}" 吗？`)) return
  try {
    await subStore.deleteSubscription(sub.id)
  } catch (e) {
    alert('删除失败: ' + e)
  }
}
</script>

<template>
  <div class="subscriptions-page min-h-screen bg-gray-900 text-white p-4">
    <header class="mb-6 flex items-center justify-between">
      <h1 class="text-2xl font-bold">📡 订阅管理</h1>
      <button
        class="px-4 py-2 bg-primary rounded hover:bg-blue-600 transition"
        @click="showAddForm = !showAddForm"
      >
        {{ showAddForm ? '取消' : '+ 添加订阅' }}
      </button>
    </header>

    <!-- Add Form -->
    <div v-if="showAddForm" class="bg-gray-800 p-4 rounded-lg mb-6">
      <div class="mb-4">
        <label class="block text-sm text-gray-400 mb-1">名称</label>
        <input
          v-model="newName"
          type="text"
          class="w-full bg-gray-700 rounded px-3 py-2 outline-none focus:ring-2 focus:ring-primary"
          placeholder="例如: 我的收藏"
        />
      </div>
      <div class="mb-4">
        <label class="block text-sm text-gray-400 mb-1">订阅地址 (JSON)</label>
        <input
          v-model="newUrl"
          type="text"
          class="w-full bg-gray-700 rounded px-3 py-2 outline-none focus:ring-2 focus:ring-primary"
          placeholder="https://example.com/subscription.json"
        />
      </div>
      <button
        class="px-4 py-2 bg-primary rounded hover:bg-blue-600 transition"
        @click="handleAdd"
      >
        添加
      </button>
    </div>

    <!-- Subscription List -->
    <div v-if="subStore.loading" class="flex justify-center py-8">
      <div class="animate-spin w-8 h-8 border-2 border-primary border-t-transparent rounded-full"></div>
    </div>

    <div v-else-if="subStore.subscriptions.length === 0" class="text-center py-8 text-gray-400">
      暂无订阅
    </div>

    <div v-else class="space-y-3">
      <div
        v-for="sub in subStore.subscriptions"
        :key="sub.id"
        class="bg-gray-800 p-4 rounded-lg flex items-center justify-between"
      >
        <div class="flex items-center gap-3">
          <button
            :class="['w-12 h-6 rounded-full transition', sub.enabled ? 'bg-primary' : 'bg-gray-600']"
            @click="handleToggle(sub)"
          >
            <div :class="['w-5 h-5 bg-white rounded-full transition transform', sub.enabled ? 'translate-x-6' : 'translate-x-0.5']"></div>
          </button>
          <div>
            <div class="font-medium">{{ sub.name }}</div>
            <div class="text-sm text-gray-400 truncate max-w-md">{{ sub.url }}</div>
          </div>
        </div>
        <div class="flex gap-2">
          <button
            :disabled="refreshing === sub.id"
            class="px-3 py-1 bg-gray-700 rounded hover:bg-gray-600 transition disabled:opacity-50"
            @click="handleRefresh(sub)"
          >
            {{ refreshing === sub.id ? '刷新中...' : '🔄 刷新' }}
          </button>
          <button
            class="px-3 py-1 bg-red-600 rounded hover:bg-red-700 transition"
            @click="handleDelete(sub)"
          >
            🗑️
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
```

- [ ] **Step 3: 提交代码**

```bash
git add -A
git commit -m "feat: add Live and Subscriptions pages"
```

---

### Task 10: 创建 VOD 和详情页面

**Files:**
- Create: `src/views/Vod.vue`
- Create: `src/views/VodDetail.vue`

- [ ] **Step 1: 创建 Vod.vue**

```vue
<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useVodStore } from '@/stores/vod'
import VodCard from '@/components/VodCard.vue'
import SearchBar from '@/components/SearchBar.vue'
import type { VodItem } from '@/types'

const vodStore = useVodStore()
const selectedType = ref<string | null>(null)

const types = [
  { label: '全部', value: null },
  { label: '电影', value: 'movie' },
  { label: '电视剧', value: 'tv' },
  { label: '综艺', value: 'variety' },
  { label: '动漫', value: 'anime' }
]

onMounted(() => {
  vodStore.fetchItems()
})

function handleSearch(keyword: string) {
  if (keyword) {
    vodStore.search(keyword)
  } else {
    vodStore.fetchItems(selectedType.value || undefined)
  }
}

function handleTypeChange(type: string | null) {
  selectedType.value = type
  vodStore.fetchItems(type || undefined)
}

function handleVodClick(item: VodItem) {
  window.location.href = `/vod/${item.id}`
}
</script>

<template>
  <div class="vod-page min-h-screen bg-gray-900 text-white p-4">
    <header class="mb-6">
      <h1 class="text-2xl font-bold mb-4">🎬 影视点播</h1>
      <SearchBar placeholder="搜索影视..." @search="handleSearch" />

      <!-- Type Filter -->
      <div class="flex gap-2 mt-4 flex-wrap">
        <button
          v-for="t in types"
          :key="t.value ?? 'all'"
          :class="[
            'px-3 py-1 rounded transition',
            selectedType === t.value ? 'bg-primary' : 'bg-gray-700 hover:bg-gray-600'
          ]"
          @click="handleTypeChange(t.value)"
        >
          {{ t.label }}
        </button>
      </div>
    </header>

    <div v-if="vodStore.loading" class="flex justify-center py-8">
      <div class="animate-spin w-8 h-8 border-2 border-primary border-t-transparent rounded-full"></div>
    </div>

    <div v-else-if="vodStore.items.length === 0" class="text-center py-8 text-gray-400">
      暂无影视
    </div>

    <div v-else class="grid grid-cols-3 md:grid-cols-4 lg:grid-cols-6 gap-4">
      <VodCard
        v-for="item in vodStore.items"
        :key="item.id"
        :item="item"
        @click="handleVodClick"
      />
    </div>
  </div>
</template>
```

- [ ] **Step 2: 创建 VodDetail.vue**

```vue
<script setup lang="ts">
import { onMounted } from 'vue'
import { useVodStore } from '@/stores/vod'
import type { Episode } from '@/types'

const vodStore = useVodStore()

// Get id from URL
const pathParts = window.location.pathname.split('/')
const id = parseInt(pathParts[pathParts.length - 1])

onMounted(() => {
  vodStore.fetchDetail(id)
})

function handlePlay(episode: Episode) {
  window.location.href = `/player/vod/${id}?episode=${encodeURIComponent(episode.url)}`
}
</script>

<template>
  <div class="vod-detail-page min-h-screen bg-gray-900 text-white p-4">
    <header class="mb-6">
      <button
        class="px-4 py-2 bg-gray-700 rounded hover:bg-gray-600 transition mb-4"
        @click="window.history.back()"
      >
        ← 返回
      </button>
    </header>

    <div v-if="vodStore.loading" class="flex justify-center py-8">
      <div class="animate-spin w-8 h-8 border-2 border-primary border-t-transparent rounded-full"></div>
    </div>

    <div v-else-if="vodStore.currentItem" class="max-w-4xl mx-auto">
      <!-- Info -->
      <div class="flex gap-6 mb-6">
        <img
          v-if="vodStore.currentItem.poster"
          :src="vodStore.currentItem.poster"
          :alt="vodStore.currentItem.name"
          class="w-48 aspect-[2/3] object-cover rounded-lg"
        />
        <div v-else class="w-48 aspect-[2/3] bg-gray-700 rounded-lg flex items-center justify-center text-4xl">
          🎬
        </div>
        <div class="flex-1">
          <h1 class="text-2xl font-bold mb-2">{{ vodStore.currentItem.name }}</h1>
          <div class="text-gray-400 mb-2">类型: {{ vodStore.currentItem.type }}</div>
          <p v-if="vodStore.currentItem.description" class="text-gray-300">
            {{ vodStore.currentItem.description }}
          </p>
        </div>
      </div>

      <!-- Episodes -->
      <div v-if="vodStore.currentItem.episodes?.length" class="mt-6">
        <h2 class="text-xl font-bold mb-4">选集</h2>
        <div class="grid grid-cols-6 gap-2">
          <button
            v-for="(ep, idx) in vodStore.currentItem.episodes"
            :key="idx"
            class="px-3 py-2 bg-gray-800 rounded hover:bg-primary transition text-center"
            @click="handlePlay(ep)"
          >
            {{ ep.name }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
```

- [ ] **Step 3: 提交代码**

```bash
git add -A
git commit -m "feat: add VOD and VOD detail pages"
```

---

### Task 11: 创建播放器页面和设置页面

**Files:**
- Create: `src/views/PlayerPage.vue`
- Create: `src/views/Settings.vue`

- [ ] **Step 1: 创建 PlayerPage.vue**

```vue
<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { useLiveStore } from '@/stores/live'
import { useVodStore } from '@/stores/vod'
import { usePlayerStore } from '@/stores/player'

const liveStore = useLiveStore()
const vodStore = useVodStore()
const playerStore = usePlayerStore()

const videoRef = ref<HTMLVideoElement | null>(null)
const playing = ref(false)
const currentTime = ref(0)
const duration = ref(0)
const volume = ref(1)
const fullscreen = ref(false)

let progressUpdateInterval: number | null = null

// Parse URL params
const params = new URLSearchParams(window.location.search)
const episodeUrl = params.get('episode')

// Get type and id from path
const pathParts = window.location.pathname.split('/')
const type = pathParts[2] // 'live' or 'vod'
const id = parseInt(pathParts[3])

onMounted(async () => {
  if (type === 'live') {
    await liveStore.fetchChannels()
    const channel = liveStore.channels.find(c => c.id === id)
    if (channel) {
      playerStore.currentUrl = channel.url
    }
  } else if (type === 'vod' && episodeUrl) {
    playerStore.currentUrl = decodeURIComponent(episodeUrl)
  }

  if (videoRef.value && playerStore.currentUrl) {
    videoRef.value.src = playerStore.currentUrl
    videoRef.value.volume = volume.value
    videoRef.value.play().then(() => {
      playing.value = true
    }).catch(console.error)
  }

  progressUpdateInterval = window.setInterval(() => {
    if (videoRef.value) {
      currentTime.value = videoRef.value.currentTime
      duration.value = videoRef.value.duration || 0
    }
  }, 1000)
})

onUnmounted(() => {
  if (progressUpdateInterval) {
    clearInterval(progressUpdateInterval)
  }
  // Save play history
  if (type === 'vod' && duration.value > 0) {
    const progress = (currentTime.value / duration.value) * 100
    playerStore.saveHistory('vod', id, progress)
  }
})

function togglePlay() {
  if (!videoRef.value) return
  if (playing.value) {
    videoRef.value.pause()
  } else {
    videoRef.value.play()
  }
  playing.value = !playing.value
}

function seek(time: number) {
  if (!videoRef.value) return
  videoRef.value.currentTime = time
}

function handleVolumeChange(e: Event) {
  const target = e.target as HTMLInputElement
  volume.value = parseFloat(target.value)
  if (videoRef.value) {
    videoRef.value.volume = volume.value
  }
}

function toggleFullscreen() {
  if (!document.fullscreenElement) {
    document.documentElement.requestFullscreen()
    fullscreen.value = true
  } else {
    document.exitFullscreen()
    fullscreen.value = false
  }
}

function formatTime(seconds: number): string {
  const h = Math.floor(seconds / 3600)
  const m = Math.floor((seconds % 3600) / 60)
  const s = Math.floor(seconds % 60)
  if (h > 0) {
    return `${h}:${m.toString().padStart(2, '0')}:${s.toString().padStart(2, '0')}`
  }
  return `${m}:${s.toString().padStart(2, '0')}`
}
</script>

<template>
  <div class="player-page min-h-screen bg-black text-white">
    <!-- Video -->
    <div class="relative">
      <video
        ref="videoRef"
        class="w-full aspect-video bg-black"
        @click="togglePlay"
      ></video>

      <!-- Controls -->
      <div class="absolute bottom-0 left-0 right-0 bg-gradient-to-t from-black/80 to-transparent p-4">
        <!-- Progress -->
        <div class="flex items-center gap-2 mb-2">
          <span class="text-sm">{{ formatTime(currentTime) }}</span>
          <input
            type="range"
            :value="currentTime"
            :max="duration || 100"
            class="flex-1 h-1 bg-gray-600 rounded-lg appearance-none cursor-pointer"
            @input="seek(parseFloat(($event.target as HTMLInputElement).value))"
          />
          <span class="text-sm">{{ formatTime(duration) }}</span>
        </div>

        <!-- Buttons -->
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-4">
            <button
              class="px-4 py-2 bg-white/20 rounded hover:bg-white/30 transition"
              @click="togglePlay"
            >
              {{ playing ? '⏸️' : '▶️' }}
            </button>
            <div class="flex items-center gap-2">
              <span>🔊</span>
              <input
                type="range"
                :value="volume"
                min="0"
                max="1"
                step="0.1"
                class="w-20 h-1 bg-gray-600 rounded-lg appearance-none cursor-pointer"
                @input="handleVolumeChange"
              />
            </div>
          </div>
          <button
            class="px-4 py-2 bg-white/20 rounded hover:bg-white/30 transition"
            @click="toggleFullscreen"
          >
            {{ fullscreen ? '⛶' : '⛶' }}
          </button>
        </div>
      </div>
    </div>

    <!-- Info -->
    <div class="p-4">
      <button
        class="px-4 py-2 bg-gray-700 rounded hover:bg-gray-600 transition mb-4"
        @click="window.history.back()"
      >
        ← 返回
      </button>
    </div>
  </div>
</template>
```

- [ ] **Step 2: 创建 Settings.vue**

```vue
<script setup lang="ts">
// Settings page placeholder
</script>

<template>
  <div class="settings-page min-h-screen bg-gray-900 text-white p-4">
    <header class="mb-6">
      <button
        class="px-4 py-2 bg-gray-700 rounded hover:bg-gray-600 transition mb-4"
        @click="window.history.back()"
      >
        ← 返回
      </button>
      <h1 class="text-2xl font-bold">⚙️ 设置</h1>
    </header>

    <div class="max-w-2xl space-y-6">
      <!-- Playback Settings -->
      <div class="bg-gray-800 p-4 rounded-lg">
        <h2 class="text-lg font-bold mb-4">播放设置</h2>
        <div class="space-y-3">
          <div class="flex items-center justify-between">
            <span>默认播放画质</span>
            <select class="bg-gray-700 px-3 py-1 rounded">
              <option>自动</option>
              <option>1080P</option>
              <option>720P</option>
              <option>480P</option>
            </select>
          </div>
          <div class="flex items-center justify-between">
            <span>启用硬解</span>
            <input type="checkbox" class="w-5 h-5" checked />
          </div>
        </div>
      </div>

      <!-- Interface Settings -->
      <div class="bg-gray-800 p-4 rounded-lg">
        <h2 class="text-lg font-bold mb-4">界面设置</h2>
        <div class="space-y-3">
          <div class="flex items-center justify-between">
            <span>主题</span>
            <select class="bg-gray-700 px-3 py-1 rounded">
              <option>深色</option>
              <option>浅色</option>
              <option>自动</option>
            </select>
          </div>
        </div>
      </div>

      <!-- About -->
      <div class="bg-gray-800 p-4 rounded-lg">
        <h2 class="text-lg font-bold mb-4">关于</h2>
        <div class="text-gray-400">
          <p>TVBox 影视仓 v0.1.0</p>
          <p class="mt-2">基于 Rust + Tauri + Vue 构建</p>
        </div>
      </div>
    </div>
  </div>
</template>
```

- [ ] **Step 3: 创建 index.html**

```html
<!DOCTYPE html>
<html lang="zh-CN">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>TVBox 影视仓</title>
  </head>
  <body>
    <div id="app"></div>
    <script type="module" src="/src/main.ts"></script>
  </body>
</html>
```

- [ ] **Step 4: 提交代码**

```bash
git add -A
git commit -m "feat: add Player and Settings pages"
```

---

## Phase 4: 项目验证 (Task 12)

### Task 12: 项目构建验证

- [ ] **Step 1: 安装依赖并构建项目**

```bash
npm install
npm run build
```

- [ ] **Step 2: 检查构建输出**

确认 `dist` 目录生成成功

- [ ] **Step 3: 提交代码**

```bash
git add -A
git commit -m "feat: complete TVBox project implementation"
```

---

## 计划完成

此计划涵盖 TVBox 影视仓应用的完整实现：

**Phase 1: 项目搭建**
- Task 1: 初始化 Tauri + Vue 项目结构
- Task 2: SQLite 数据库和存储服务
- Task 3: 订阅管理命令
- Task 4: 直播/点播/播放命令

**Phase 2: 前端基础**
- Task 5: TypeScript 类型和 Vue Router
- Task 6: Pinia 状态管理

**Phase 3: 前端页面**
- Task 7: 公共组件
- Task 8: Home 页面
- Task 9: Live 和 Subscriptions 页面
- Task 10: VOD 和详情页面
- Task 11: 播放器和设置页面

**Phase 4: 验证**
- Task 12: 项目构建验证
