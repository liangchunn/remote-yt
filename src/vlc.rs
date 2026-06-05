use std::path::PathBuf;

use tokio::process::{Child, Command};

use crate::{config::VlcRpcConfig, yt_dlp::Track};

pub struct VlcClient {
    binary_path: PathBuf,
    rpc_config: VlcRpcConfig,
}

impl VlcClient {
    pub fn new(binary_path: impl Into<PathBuf>, rpc_config: VlcRpcConfig) -> Self {
        Self {
            binary_path: binary_path.into(),
            rpc_config,
        }
    }
    pub async fn oneshot(&self, track: Track, title: &str) -> anyhow::Result<Child> {
        let binary_path = self.binary_path.clone();
        let mut child = Command::new(binary_path);
        child
            .arg("--play-and-exit")
            .arg("--fullscreen")
            .arg("--extraintf=http")
            .arg(format!("--http-password={}", self.rpc_config.password))
            .arg(format!("--http-host={}", self.rpc_config.host))
            .arg(format!("--http-port={}", self.rpc_config.port));

        match track {
            Track::Merged(merged_track) => child
                .arg("--meta-title")
                .arg(title)
                .arg(merged_track.merged_url),
            Track::Split(split_track) => child
                .arg("--meta-title")
                .arg(title)
                .arg("--input-slave")
                .arg(split_track.audio_url)
                .arg(split_track.video_url),
        };

        Ok(child.spawn()?)
    }
}
