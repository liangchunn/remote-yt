use serde::Deserialize;
use std::collections::HashMap;

use crate::yt_dlp::TrackType;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub vlc_path: String,
    pub yt_dlp_path: String,

    #[serde(flatten)]
    pub providers: HashMap<String, Provider>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Provider {
    pub args: Vec<String>,
    pub format: String,
    pub r#type: TrackType,
}

pub fn parse_config(file_path: &str) -> anyhow::Result<Config> {
    let config_content = std::fs::read_to_string(file_path)?;
    let config: Config = toml::from_str(&config_content)?;
    Ok(config)
}
