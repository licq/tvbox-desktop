// src-tauri/src/services/provider/traits.rs
use async_trait::async_trait;
use crate::services::xb6v::ScrapedCatalogItem;
use crate::services::playback_types::PlaybackTarget;
use super::ProviderError;

#[derive(Debug, Clone)]
pub struct CatalogCategory {
    pub type_id: String,
    pub type_name: String,
}

#[async_trait]
pub trait VideoProvider: Send + Sync {
    fn source_key(&self) -> &str;
    fn source_name(&self) -> &str;

    /// 获取首页推荐
    async fn home(&self) -> Result<Vec<ScrapedCatalogItem>, ProviderError>;

    /// 获取分类列表
    async fn home_vod(&self) -> Result<Vec<CatalogCategory>, ProviderError>;

    /// 按分类和分页获取内容
    async fn category(&self, type_id: &str, page: u32) -> Result<Vec<ScrapedCatalogItem>, ProviderError>;

    /// 搜索
    async fn search(&self, keyword: &str) -> Result<Vec<ScrapedCatalogItem>, ProviderError>;

    /// 获取详情（含剧集列表）
    async fn detail(&self, ids: &str) -> Result<Option<ScrapedCatalogItem>, ProviderError>;

    /// 解析播放地址
    async fn play(&self, flag: &str, play_url: &str) -> Result<Vec<PlaybackTarget>, ProviderError>;
}
