#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::Manager;

fn main() {
    env_logger::init();

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            tvbox_lib::commands::subscription::add_subscription,
            tvbox_lib::commands::subscription::get_subscriptions,
            tvbox_lib::commands::subscription::get_source_health_summaries,
            tvbox_lib::commands::subscription::delete_subscription,
            tvbox_lib::commands::subscription::refresh_subscription,
            tvbox_lib::commands::subscription::toggle_subscription,
            tvbox_lib::commands::live::get_live_channels,
            tvbox_lib::commands::live::get_live_categories,
            tvbox_lib::commands::live::get_live_channel_groups,
            tvbox_lib::commands::vod::get_vod_items,
            tvbox_lib::commands::vod::get_vod_detail,
            tvbox_lib::commands::vod::search_vod,
            tvbox_lib::commands::vod::get_library_home,
            tvbox_lib::commands::vod::get_catalog_items,
            tvbox_lib::commands::vod::get_catalog_detail,
            tvbox_lib::commands::player::save_play_history,
            tvbox_lib::commands::player::get_play_history,
            tvbox_lib::commands::player::resolve_playback,
            tvbox_lib::commands::player::fetch_hls_manifest,
            tvbox_lib::commands::player::fetch_hls_segment,
            tvbox_lib::commands::douban::get_douban_hot,
            tvbox_lib::commands::douban::get_douban_hot_by_id,
            tvbox_lib::commands::douban::fetch_douban_hot,
            tvbox_lib::commands::douban::get_matched_hot_list,
            tvbox_lib::commands::douban::fetch_all_douban_hot,
            tvbox_lib::commands::douban::fetch_douban_hot_by_type,
            tvbox_lib::commands::douban::search_vod_sources,
            tvbox_lib::commands::douban::get_douban_hot_by_type,
            tvbox_lib::commands::douban::proxy_image,
            tvbox_lib::commands::douban::fetch_douban_subject_metadata,
            tvbox_lib::commands::douban::fetch_douban_metadata_by_id,
            tvbox_lib::commands::douban::search_douban_subject_by_keyword,
            tvbox_lib::commands::search::search_all_sources,
            tvbox_lib::commands::search::provider_detail,
            tvbox_lib::commands::search::provider_play,
            tvbox_lib::commands::cache::clear_source_search_cache,
            tvbox_lib::commands::cache::clear_douban_search_cache,
        ])
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir().expect("无法获取应用数据目录");
            let storage = tvbox_lib::Storage::new(app_data_dir).expect("无法初始化数据库");
            let mut registry = tvbox_lib::services::provider::ProviderRegistry::new();
            registry.register_working_sources();
            let provider_registry = tokio::sync::RwLock::new(registry);
            let storage_for_prune = storage.clone();
            app.manage(tvbox_lib::AppState { storage, provider_registry });
            // Prune expired search caches on startup (synchronous, runs on a background thread)
            std::thread::spawn(move || {
                if let Err(e) = storage_for_prune.prune_expired_search_caches() {
                    log::warn!("[startup] Failed to prune expired search caches: {}", e);
                }
            });
            log::info!("TVBox 应用启动成功");
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("启动 Tauri 应用时出错");

    app.run(|_app_handle, _event| {});
}
