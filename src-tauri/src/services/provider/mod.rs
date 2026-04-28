// src-tauri/src/services/provider/mod.rs
use thiserror::Error;

/// Normalize a raw type name (Chinese or English) from upstream sites into one of
/// the four canonical types consumed by the frontend: `movie`, `series`, `variety`, `anime`.
pub fn normalize_item_type(type_name: &str) -> String {
    let t = type_name.trim().to_lowercase();

    // Variety / 综艺 (check first — some variety names also contain “剧”)
    if t.contains("综艺")
        || t.contains("真人秀")
        || t.contains("脱口秀")
        || t.contains("选秀")
        || t.contains("晚会")
        || t.contains("颁奖")
        || t.contains("访谈")
        || t.contains("variety")
        || t == "show"
    {
        return "variety".to_string();
    }

    // Anime / 动漫 (check before generic “剧” because of 剧场版)
    if t.contains("动漫")
        || t.contains("动画")
        || t.contains("番剧")
        || t.contains("剧场版")
        || t.contains("ova")
        || t.contains("oad")
        || t.contains("特摄")
        || t.contains("anime")
        || t.contains("animation")
        || t.contains("cartoon")
        || t.contains("donghua")
        || t.contains("国漫")
        || t.contains("日漫")
        || t.contains("美漫")
    {
        return "anime".to_string();
    }

    // Series / 电视剧
    if t.contains("电视剧")
        || t.contains("网剧")
        || t.contains("短剧")
        || t.contains("连续剧")
        || t.contains("连载")
        || t.contains("tv series")
        || t.contains("tv drama")
        || t.contains("soap")
        || (t.ends_with('剧') && !t.contains("电影"))
    {
        return "series".to_string();
    }

    // Default to movie
    "movie".to_string()
}

#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("JS execution error: {0}")]
    JsRuntime(String),

    #[error("Unsupported source type: {0}")]
    UnsupportedType(String),

    #[error("Spider script unavailable: {0}")]
    SpiderUnavailable(String),

    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}

impl From<rquickjs::Error> for ProviderError {
    fn from(e: rquickjs::Error) -> Self {
        ProviderError::JsRuntime(e.to_string())
    }
}

pub mod traits;
pub mod cms_provider;
pub mod spider_provider;
pub mod native;
pub mod xb6v_scraper;
pub mod auete_scraper;
pub mod zxzj_scraper;
pub mod jianpian_scraper;
pub mod wencai_scraper;
pub mod libvio_scraper;
pub mod registry;
pub mod ygp_scraper;
pub mod kkss_scraper;
pub mod uuss_scraper;
pub mod ycyz_scraper;
pub mod lite_apple_scraper;
pub mod nuomi_scraper;
pub mod baibai_scraper;
pub mod changzhang_scraper;
pub mod yicai_scraper;
pub mod bite_scraper;
pub mod ddrk_scraper;
pub mod mengmi_scraper;
pub mod xiongdi_scraper;
pub mod rebo_scraper;
pub mod huanshi_scraper;
pub mod dm84_scraper;
pub mod ysj_scraper;
pub mod anime1_scraper;
pub mod ypanso_scraper;
pub mod xzso_scraper;
pub mod miso_scraper;
pub mod kuasou_scraper;
pub mod aliso_scraper;
pub mod yiso_scraper;
pub mod bili_scraper;
pub mod biliych_scraper;
pub mod fan_scraper;
pub mod cc_scraper;

pub use traits::VideoProvider;
pub use cms_provider::CmsProvider;
pub use spider_provider::SpiderProvider;
pub use native::NativeScraper;
pub use xb6v_scraper::Xb6vScraper;
pub use auete_scraper::AueteScraper;
pub use zxzj_scraper::ZxzjScraper;
pub use jianpian_scraper::JianpianScraper;
pub use wencai_scraper::WencaiScraper;
pub use registry::ProviderRegistry;
pub use libvio_scraper::LibvioScraper;
pub use ygp_scraper::YgpScraper;
pub use kkss_scraper::KkssScraper;
pub use uuss_scraper::UussScraper;
pub use ycyz_scraper::YcyzScraper;
pub use lite_apple_scraper::LiteAppleScraper;
pub use nuomi_scraper::NuomiScraper;
pub use baibai_scraper::BaibaiScraper;
pub use changzhang_scraper::ChangzhangScraper;
pub use yicai_scraper::YicaiScraper;
pub use bite_scraper::BiteScraper;
pub use ddrk_scraper::DdrkScraper;
pub use mengmi_scraper::MengmiScraper;
pub use xiongdi_scraper::XiongdiScraper;
pub use rebo_scraper::ReboScraper;
pub use huanshi_scraper::HuanshiScraper;
pub use dm84_scraper::Dm84Scraper;
pub use ysj_scraper::YsjScraper;
pub use anime1_scraper::Anime1Scraper;
pub use ypanso_scraper::YpansoScraper;
pub use xzso_scraper::XzsoScraper;
pub use miso_scraper::MisoScraper;
pub use kuasou_scraper::KuasouScraper;
pub use aliso_scraper::AlisoScraper;
pub use yiso_scraper::YisoScraper;
pub use bili_scraper::BiliScraper;
pub use biliych_scraper::BiliychScraper;
pub use fan_scraper::FanScraper;
pub use cc_scraper::CcScraper;

pub mod scraper_tests;
pub mod diagnostic_check;

#[cfg(test)]
mod tests {
    use super::normalize_item_type;

    #[test]
    fn normalizes_chinese_movie_types() {
        assert_eq!(normalize_item_type("电影"), "movie");
        assert_eq!(normalize_item_type("剧情片"), "movie");
        assert_eq!(normalize_item_type("动作片"), "movie");
        assert_eq!(normalize_item_type("喜剧片"), "movie");
        assert_eq!(normalize_item_type("科幻片"), "movie");
    }

    #[test]
    fn normalizes_chinese_series_types() {
        assert_eq!(normalize_item_type("电视剧"), "series");
        assert_eq!(normalize_item_type("国产剧"), "series");
        assert_eq!(normalize_item_type("美剧"), "series");
        assert_eq!(normalize_item_type("韩剧"), "series");
        assert_eq!(normalize_item_type("日剧"), "series");
        assert_eq!(normalize_item_type("港剧"), "series");
        assert_eq!(normalize_item_type("网剧"), "series");
        assert_eq!(normalize_item_type("短剧"), "series");
    }

    #[test]
    fn normalizes_chinese_variety_types() {
        assert_eq!(normalize_item_type("综艺"), "variety");
        assert_eq!(normalize_item_type("大陆综艺"), "variety");
        assert_eq!(normalize_item_type("真人秀"), "variety");
        assert_eq!(normalize_item_type("脱口秀"), "variety");
        assert_eq!(normalize_item_type("选秀"), "variety");
        assert_eq!(normalize_item_type("晚会"), "variety");
    }

    #[test]
    fn normalizes_chinese_anime_types() {
        assert_eq!(normalize_item_type("动漫"), "anime");
        assert_eq!(normalize_item_type("动画"), "anime");
        assert_eq!(normalize_item_type("番剧"), "anime");
        assert_eq!(normalize_item_type("剧场版"), "anime");
        assert_eq!(normalize_item_type("国漫"), "anime");
        assert_eq!(normalize_item_type("日漫"), "anime");
    }

    #[test]
    fn normalizes_english_types() {
        assert_eq!(normalize_item_type("Movie"), "movie");
        assert_eq!(normalize_item_type("TV Series"), "series");
        assert_eq!(normalize_item_type("Variety"), "variety");
        assert_eq!(normalize_item_type("Anime"), "anime");
        assert_eq!(normalize_item_type("Animation"), "anime");
    }

    #[test]
    fn variety_checked_before_series_due_to_shared_ju_character() {
        // 综艺 names often end with "秀" not "剧", but variety should take precedence
        assert_eq!(normalize_item_type("选秀"), "variety");
    }

    #[test]
    fn anime_checked_before_series_for_theater_version() {
        // 剧场版 contains "剧" but should map to anime
        assert_eq!(normalize_item_type("剧场版"), "anime");
    }

    #[test]
    fn defaults_unknown_types_to_movie() {
        assert_eq!(normalize_item_type(""), "movie");
        assert_eq!(normalize_item_type("unknown"), "movie");
        assert_eq!(normalize_item_type("纪录片"), "movie");
    }
}
