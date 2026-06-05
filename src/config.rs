use std::path::Path;

use serde::Deserialize;
use std::collections::HashMap;

use crate::yt_dlp::TrackType;

const DEFAULT_VLC_RPC_HOST: &str = "127.0.0.1";
const DEFAULT_VLC_RPC_PORT: u16 = 8081;
const DEFAULT_VLC_RPC_PASSWORD: &str = "abc";

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub vlc_path: String,
    pub yt_dlp_path: String,
    #[serde(default)]
    pub vlc_rpc: VlcRpcConfig,

    #[serde(flatten)]
    pub providers: HashMap<String, Provider>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct VlcRpcConfig {
    #[serde(default = "default_vlc_rpc_host")]
    pub host: String,
    #[serde(default = "default_vlc_rpc_port")]
    pub port: u16,
    #[serde(default = "default_vlc_rpc_password")]
    pub password: String,
}

impl Default for VlcRpcConfig {
    fn default() -> Self {
        Self {
            host: default_vlc_rpc_host(),
            port: default_vlc_rpc_port(),
            password: default_vlc_rpc_password(),
        }
    }
}

impl VlcRpcConfig {
    fn normalize(&mut self) {
        if self.host.is_empty() {
            self.host = default_vlc_rpc_host();
        }

        if self.port == 0 {
            self.port = default_vlc_rpc_port();
        }

        if self.password.is_empty() {
            self.password = default_vlc_rpc_password();
        }
    }
}

fn default_vlc_rpc_host() -> String {
    DEFAULT_VLC_RPC_HOST.to_owned()
}

fn default_vlc_rpc_port() -> u16 {
    DEFAULT_VLC_RPC_PORT
}

fn default_vlc_rpc_password() -> String {
    DEFAULT_VLC_RPC_PASSWORD.to_owned()
}

#[derive(Debug, Deserialize, Clone)]
pub struct Provider {
    pub args: Vec<String>,
    pub format: String,
    pub r#type: TrackType,
}

pub fn parse_config(file_path: impl AsRef<Path>) -> anyhow::Result<Config> {
    let config_content = std::fs::read_to_string(file_path)?;
    let mut config: Config = toml::from_str(&config_content)?;
    config.vlc_rpc.normalize();
    Ok(config)
}
