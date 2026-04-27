// src-tauri/src/services/provider/mod.rs
use thiserror::Error;

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
