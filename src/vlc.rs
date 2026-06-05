use std::path::PathBuf;

use tokio::process::{Child, Command};

use crate::yt_dlp::Track;

pub struct VlcClient {
    binary_path: PathBuf,
}

impl VlcClient {
    pub fn new(binary_path: impl Into<PathBuf>) -> Self {
        Self {
            binary_path: binary_path.into(),
        }
    }
    pub async fn oneshot(&self, track: Track, title: &str) -> anyhow::Result<Child> {
        let binary_path = self.binary_path.clone();
        let mut child = Command::new(binary_path);
        child
            .arg("--play-and-exit")
            .arg("--fullscreen")
            .arg("--extraintf=http")
            .arg("--http-password=abc")
            .arg("--http-host=127.0.0.1")
            .arg("--http-port=8081");

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
