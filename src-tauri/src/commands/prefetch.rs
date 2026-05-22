use crate::AppState;
use crate::services::segment_cache::CacheStats;
use tauri::State;

/// Command to prefetch upcoming segments from an HLS playlist.
/// This triggers background prefetching to hide CDN latency for subsequent playback.
///
/// Parameters:
/// - playlist_url: The media playlist URL (not master playlist)
/// - count: Number of segments to prefetch (default: 5)
#[tauri::command]
pub async fn prefetch_segments(
    playlist_url: String,
    count: Option<usize>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    if let Some(worker) = state.segment_prefetch_worker.as_ref() {
        let count = count.unwrap_or(5);
        worker.prefetch_playlist_segments(&playlist_url, None, count).await;
    }
    Ok(())
}

/// Command to clear the segment cache.
/// Useful when switching to a different video or when cache grows too large.
#[tauri::command]
pub async fn clear_segment_cache(state: State<'_, AppState>) -> Result<(), String> {
    state.segment_cache.clear().await;
    Ok(())
}

/// Command to get segment cache statistics.
#[tauri::command]
pub async fn get_segment_cache_stats(
    state: State<'_, AppState>,
) -> Result<CacheStats, String> {
    Ok(state.segment_cache.stats().await)
}