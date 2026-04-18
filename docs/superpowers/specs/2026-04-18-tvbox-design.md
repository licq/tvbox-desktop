# TVBox 影视仓 - 设计文档

## 1. 项目概述

**项目名称**：TVBox（影视仓）
**项目类型**：跨平台桌面应用
**技术栈**：Rust + Tauri 2.x + Vue 3 + TypeScript + SQLite
**目标平台**：Windows、macOS、Linux 桌面端

### 核心功能
- 直播电视：观看电视频道直播
- 影视点播：电影、电视剧、综艺、动漫等点播内容
- 订阅管理：支持 JSON 格式的订阅源配置

### 目标用户
- 拥有电脑且需要观看网络电视/影视的用户
- 习惯使用遥控器或键盘操作的用户

---

## 2. 架构设计

### 2.1 整体架构

```
┌──────────────────────────────────────────────┐
│                  Frontend                    │
│  Vue 3 + TypeScript + TailwindCSS           │
│  ├── 主页（直播/点播Tab切换）                 │
│  ├── 直播频道列表 + 播放                     │
│  ├── 影视分类/详情/播放                      │
│  ├── 订阅管理                                │
│  └── 设置                                    │
└────────────────────┬────────────────────────┘
                     │ Tauri IPC (invoke)
┌────────────────────▼────────────────────────┐
│                Rust Backend                  │
│  ├── subscribe_parser  │ 订阅解析（JSON）   │
│  ├── channel_manager   │ 频道管理            │
│  ├── media_player      │ 播放器控制          │
│  ├── storage           │ SQLite CRUD         │
│  └── http_client       │ 网络请求            │
└──────────────────────────────────────────────┘
```

### 2.2 技术选型

| 层级 | 技术 | 说明 |
|------|------|------|
| 前端框架 | Vue 3 + Vite | 响应式、组件化 |
| UI 样式 | TailwindCSS | 原子化 CSS，快速开发 |
| 状态管理 | Pinia | Vue 3 官方推荐 |
| 桌面框架 | Tauri 2.x | 轻量、安全 |
| 后端语言 | Rust | 性能、安全 |
| 数据库 | SQLite (rusqlite) | 轻量级，无需配置 |
| 播放器 | 内置播放器 (gstreamer) | 完全自包含 |

### 2.3 目录结构

```
tvbox/
├── src/                      # Vue 前端源码
│   ├── assets/               # 静态资源
│   ├── components/           # 公共组件
│   │   ├── ChannelCard.vue   # 频道卡片
│   │   ├── VodCard.vue       # 影视卡片
│   │   ├── Player.vue         # 播放器组件
│   │   └── ...
│   ├── views/                # 页面
│   │   ├── Home.vue           # 主页
│   │   ├── Live.vue           # 直播页
│   │   ├── Vod.vue            # 点播页
│   │   ├── Player.vue         # 播放页
│   │   ├── Subscriptions.vue  # 订阅管理
│   │   ├── VodDetail.vue      # 影视详情
│   │   └── Settings.vue       # 设置页
│   ├── stores/               # Pinia 状态
│   │   ├── subscription.ts    # 订阅状态
│   │   ├── live.ts           # 直播状态
│   │   ├── vod.ts            # 点播状态
│   │   └── player.ts         # 播放状态
│   ├── App.vue
│   └── main.ts
├── src-tauri/                 # Rust 后端源码
│   ├── src/
│   │   ├── commands/         # Tauri 命令
│   │   │   ├── mod.rs
│   │   │   ├── subscription.rs
│   │   │   ├── live.rs
│   │   │   ├── vod.rs
│   │   │   └── player.rs
│   │   ├── models/           # 数据模型
│   │   │   └── mod.rs
│   │   ├── services/         # 业务逻辑
│   │   │   ├── mod.rs
│   │   │   ├── parser.rs     # JSON 解析
│   │   │   └── storage.rs    # SQLite 操作
│   │   └── main.rs
│   ├── Cargo.toml
│   └── tauri.conf.json
├── docs/                     # 文档
├── SPEC.md
├── README.md
└── package.json
```

---

## 3. 数据模型

### 3.1 SQLite 表结构

```sql
-- 订阅源表
CREATE TABLE subscriptions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    url TEXT NOT NULL UNIQUE,
    enabled INTEGER DEFAULT 1,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 直播频道表
CREATE TABLE live_channels (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    subscription_id INTEGER,
    name TEXT NOT NULL,
    logo TEXT,
    url TEXT NOT NULL,
    category TEXT,
    FOREIGN KEY (subscription_id) REFERENCES subscriptions(id)
);

-- 点播影视表
CREATE TABLE vod_items (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    subscription_id INTEGER,
    name TEXT NOT NULL,
    type TEXT,
    poster TEXT,
    description TEXT,
    episodes TEXT,
    FOREIGN KEY (subscription_id) REFERENCES subscriptions(id)
);

-- 播放历史
CREATE TABLE play_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    item_type TEXT,
    item_id INTEGER,
    progress REAL,
    last_played TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
```

### 3.2 前端类型定义

```typescript
interface Subscription {
  id: number;
  name: string;
  url: string;
  enabled: boolean;
  created_at?: string;
  updated_at?: string;
}

interface LiveChannel {
  id: number;
  subscription_id: number;
  name: string;
  logo?: string;
  url: string;
  category?: string;
}

interface VodItem {
  id: number;
  subscription_id: number;
  name: string;
  type: 'movie' | 'tv' | 'variety' | 'anime';
  poster?: string;
  description?: string;
  episodes: Episode[];
}

interface Episode {
  name: string;
  url: string;
}

interface PlayHistory {
  id: number;
  item_type: 'live' | 'vod';
  item_id: number;
  progress: number;
  last_played: string;
}
```

---

## 4. Tauri 命令接口

### 4.1 订阅命令

```rust
#[tauri::command]
async fn add_subscription(name: String, url: String) -> Result<Subscription, String>;

#[tauri::command]
async fn get_subscriptions() -> Result<Vec<Subscription>, String>;

#[tauri::command]
async fn delete_subscription(id: i64) -> Result<(), String>;

#[tauri::command]
async fn refresh_subscription(id: i64) -> Result<(), String>;

#[tauri::command]
async fn toggle_subscription(id: i64, enabled: bool) -> Result<(), String>;
```

### 4.2 直播命令

```rust
#[tauri::command]
async fn get_live_channels(category: Option<String>) -> Result<Vec<LiveChannel>, String>;

#[tauri::command]
async fn get_live_categories() -> Result<Vec<String>, String>;
```

### 4.3 点播命令

```rust
#[tauri::command]
async fn get_vod_items(vtype: Option<String>, page: u32) -> Result<Vec<VodItem>, String>;

#[tauri::command]
async fn search_vod(keyword: String) -> Result<Vec<VodItem>, String>;

#[tauri::command]
async fn get_vod_detail(id: i64) -> Result<VodItem, String>;
```

### 4.4 播放命令

```rust
#[tauri::command]
async fn get_play_url(channel_id: i64) -> Result<String, String>;

#[tauri::command]
async fn save_play_history(item_type: String, item_id: i64, progress: f64) -> Result<(), String>;

#[tauri::command]
async fn get_play_history() -> Result<Vec<PlayHistory>, String>;
```

---

## 5. UI 设计

### 5.1 页面结构

```
App
├── 主页（Home）
│   ├── 顶部导航栏（Logo + 搜索 + 设置）
│   ├── Tab切换（直播 / 点播）
│   └── 内容区域
│       ├── 直播：分类频道列表
│       └── 点播：推荐/分类/热门
├── 播放页（Player）
│   ├── 播放器
│   ├── 线路选择
│   └── 播放控制条
├── 订阅管理页（Subscriptions）
│   ├── 订阅列表
│   ├── 添加订阅表单
│   └── 刷新/删除操作
├── 详情页（VodDetail）
│   ├── 海报/封面
│   ├── 简介
│   └── 选集列表
└── 设置页（Settings）
    ├── 播放设置
    ├── 界面设置
    └── 关于
```

### 5.2 布局特点

- **大屏幕优先**：大字体、大按钮，适合遥控器/鼠标操作
- **网格布局**：频道/影片使用网格展示
- **侧边导航**：可折叠的左侧导航栏
- **悬浮控制**：播放时底部控制条

### 5.3 遥控器支持

| 按键 | 功能 |
|------|------|
| 上/下 | 频道/选集切换 |
| 左/右 | 列表翻页 |
| 确定 | 播放/进入详情 |
| 返回 | 退出/返回上级 |
| 菜单 | 显示更多选项 |

---

## 6. 错误处理

### 6.1 网络错误
- 订阅URL无法访问：提示"订阅源不可用，请检查网络"
- 播放URL失效：提示"播放源已失效，尝试切换线路"
- 自动重试：失败后最多重试3次

### 6.2 解析错误
- JSON格式错误：提示"订阅格式错误，无法解析"
- 数据不完整：使用默认值填充，标记为"数据异常"

### 6.3 播放错误
- 解码失败：提示"该格式不支持"，可选择调用外部播放器
- 卡顿/缓冲：显示加载指示，提供画质切换选项

### 6.4 存储错误
- 数据库写入失败：提示"保存失败"，保留内存缓存
- 数据损坏：提供"重置数据库"选项

---

## 7. 实现计划

### Phase 1: 项目搭建
- [ ] 初始化 Tauri + Vue 项目
- [ ] 配置 TailwindCSS
- [ ] 设置 SQLite 依赖
- [ ] 配置 gstreamer 依赖

### Phase 2: 核心功能
- [ ] 实现订阅管理（CRUD）
- [ ] 实现订阅解析（JSON）
- [ ] 实现直播频道列表
- [ ] 实现点播列表

### Phase 3: 播放器
- [ ] 集成播放器
- [ ] 实现播放控制
- [ ] 实现线路切换

### Phase 4: 完善功能
- [ ] 搜索功能
- [ ] 播放历史
- [ ] UI优化
