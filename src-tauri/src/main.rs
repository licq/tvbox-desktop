#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod models;
mod services;

use commands::subscription::{
    add_subscription, delete_subscription, get_subscriptions, refresh_subscription,
    toggle_subscription,
};
use commands::live::{get_live_channels, get_live_categories};
use commands::vod::{get_vod_items, get_vod_detail, search_vod};
use commands::player::{save_play_history, get_play_history};
use services::Storage;
use tauri::Manager;

struct AppState {
    storage: Storage,
}

fn main() {
    env_logger::init();
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            add_subscription,
            get_subscriptions,
            delete_subscription,
            refresh_subscription,
            toggle_subscription,
            get_live_channels,
            get_live_categories,
            get_vod_items,
            get_vod_detail,
            search_vod,
            save_play_history,
            get_play_history,
        ])
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir().expect("无法获取应用数据目录");
            let storage = Storage::new(app_data_dir).expect("无法初始化数据库");
            app.manage(AppState { storage });
            log::info!("TVBox 应用启动成功");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("启动 Tauri 应用时出错");
}
