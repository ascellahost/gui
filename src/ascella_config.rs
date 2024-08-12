use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::{screenshots::ScreenshotType, utils::ascella_dir};

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct AscellaConfig {
    pub s3_url: String,
    pub s3_access_key: String,
    pub s3_secret_key: String,
    pub s3_bucket: String,
    pub public_url: String,

    pub debug: bool,
    pub s_type: ScreenshotType,
    pub webserver: bool,
    pub theme: u8,
    pub optimize_png: bool,
    pub optimize_timeout: u64,
    pub console_logging: bool,
    pub notifications_enabled: bool,
}

impl AscellaConfig {
    pub async fn save(&self) -> Result<()> {
        let file = ascella_dir().join("ascella.toml");
        tokio::fs::write(file, toml_edit::ser::to_string_pretty(self)?).await?;

        Ok(())
    }
}
