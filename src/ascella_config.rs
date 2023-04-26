use std::collections::HashMap;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::{screenshots::ScreenshotType, utils::ascella_dir};

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct AscellaConfig {
    #[serde(default)]
    pub api_url: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(alias = "RequestURL")]
    pub request_url: String,
    #[serde(alias = "Headers")]
    pub headers: HashMap<String, String>,
    pub debug: bool,
    pub s_type: ScreenshotType,
}

impl AscellaConfig {
    pub async fn save(&self) -> Result<()> {
        let file = ascella_dir().join("ascella.toml");
        tokio::fs::write(file, toml_edit::ser::to_string_pretty(self)?).await?;

        Ok(())
    }
}
