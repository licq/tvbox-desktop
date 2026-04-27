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
pub mod registry;

pub use traits::VideoProvider;
pub use cms_provider::CmsProvider;
pub use spider_provider::SpiderProvider;
pub use registry::ProviderRegistry;
