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

#[cfg(test)]
mod tests {
    use super::*;

    fn write_config(contents: &str) -> (tempfile::TempDir, std::path::PathBuf) {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let path = temp_dir.path().join("config.toml");
        std::fs::write(&path, contents).expect("write config");
        (temp_dir, path)
    }

    #[test]
    fn parse_config_uses_default_vlc_rpc_when_missing() {
        let (_temp_dir, path) = write_config(
            r#"
vlc_path = "vlc"
yt_dlp_path = "yt-dlp"
"#,
        );

        let config = parse_config(path).expect("parse config");

        assert_eq!(config.vlc_path, "vlc");
        assert_eq!(config.yt_dlp_path, "yt-dlp");
        assert_eq!(config.vlc_rpc.host, "127.0.0.1");
        assert_eq!(config.vlc_rpc.port, 8081);
        assert_eq!(config.vlc_rpc.password, "abc");
    }

    #[test]
    fn parse_config_normalizes_empty_vlc_rpc_values() {
        let (_temp_dir, path) = write_config(
            r#"
vlc_path = "vlc"
yt_dlp_path = "yt-dlp"

[vlc_rpc]
host = ""
port = 0
password = ""
"#,
        );

        let config = parse_config(path).expect("parse config");

        assert_eq!(config.vlc_rpc.host, "127.0.0.1");
        assert_eq!(config.vlc_rpc.port, 8081);
        assert_eq!(config.vlc_rpc.password, "abc");
    }

    #[test]
    fn parse_config_preserves_explicit_vlc_rpc_values() {
        let (_temp_dir, path) = write_config(
            r#"
vlc_path = "vlc"
yt_dlp_path = "yt-dlp"

[vlc_rpc]
host = "0.0.0.0"
port = 9090
password = "secret"
"#,
        );

        let config = parse_config(path).expect("parse config");

        assert_eq!(config.vlc_rpc.host, "0.0.0.0");
        assert_eq!(config.vlc_rpc.port, 9090);
        assert_eq!(config.vlc_rpc.password, "secret");
    }

    #[test]
    fn parse_config_loads_flattened_provider_tables() {
        let (_temp_dir, path) = write_config(
            r#"
vlc_path = "vlc"
yt_dlp_path = "yt-dlp"

[youtube]
args = ["--cookies", "cookies.txt"]
format = "best"
type = "merged"

[vimeo]
args = []
format = "bestvideo+bestaudio"
type = "split"
"#,
        );

        let config = parse_config(path).expect("parse config");

        let youtube = config.providers.get("youtube").expect("youtube provider");
        assert_eq!(youtube.args, vec!["--cookies", "cookies.txt"]);
        assert_eq!(youtube.format, "best");
        assert!(matches!(youtube.r#type, TrackType::Merged));

        let vimeo = config.providers.get("vimeo").expect("vimeo provider");
        assert!(vimeo.args.is_empty());
        assert_eq!(vimeo.format, "bestvideo+bestaudio");
        assert!(matches!(vimeo.r#type, TrackType::Split));
    }

    #[test]
    fn parse_config_errors_for_invalid_toml() {
        let (_temp_dir, path) = write_config("not valid toml = [");

        assert!(parse_config(path).is_err());
    }
}
