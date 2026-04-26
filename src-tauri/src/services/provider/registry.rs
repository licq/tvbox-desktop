// src-tauri/src/services/provider/registry.rs
// Registry module - to be implemented in a future task
// This module will manage VideoProvider instances

use super::{VideoProvider, ProviderError};

/// A registry for managing VideoProvider instances
pub struct ProviderRegistry;

impl ProviderRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self
    }

    /// Register a provider (placeholder - to be implemented)
    pub fn register(&mut self, _provider: Box<dyn VideoProvider>) -> Result<(), ProviderError> {
        Ok(())
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}
