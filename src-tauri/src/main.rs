#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod models;
mod services;

use commands::subscription::{
    add_subscription, delete_subscription, get_subscriptions, refresh_subscription,
    toggle_subscription,
};
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
