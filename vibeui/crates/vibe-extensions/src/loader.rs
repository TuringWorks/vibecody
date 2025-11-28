//! Extension loader

use anyhow::Result;

pub struct ExtensionLoader;

impl ExtensionLoader {
    pub fn new() -> Self {
        Self
    }

    pub fn load_extensions(&self) -> Result<()> {
        // TODO: Implement extension loading
        Ok(())
    }
}

impl Default for ExtensionLoader {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ExtensionAPI;
