pub mod commands;
pub mod models;
pub mod services;

pub use services::{Parser, Storage};

pub struct AppState {
    pub storage: Storage,
    pub provider_registry: tokio::sync::RwLock<services::provider::ProviderRegistry>,
    pub segment_cache: std::sync::Arc<services::segment_cache::SegmentCache>,
    pub segment_prefetch_worker: Option<std::sync::Arc<services::segment_cache::PrefetchWorker>>,
}
