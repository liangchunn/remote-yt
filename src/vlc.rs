use std::path::PathBuf;

use tokio::process::{Child, Command};

use crate::yt_dlp::Track;

pub struct VlcClient {
    binary_path: PathBuf,
}

impl Default for VlcClient {
    fn default() -> Self {
        let binary_path = if cfg!(target_os = "macos") {
            "/Applications/VLC.app/Contents/MacOS/VLC".into()
        } else if cfg!(target_os = "linux") {
            "vlc".into()
        } else {
            unimplemented!()
        };
        Self { binary_path }
    }
}

impl VlcClient {
    // pub fn with_binary_path(binary_path: PathBuf) -> Self {
    //     Self { binary_path }
    // }
    pub async fn oneshot<'a>(&self, track: Track<'a>, title: &str) -> anyhow::Result<Child> {
        let binary_path = self.binary_path.clone();
        let mut child = Command::new(binary_path);
        child.arg("--play-and-exit").arg("--fullscreen");

        match track {
            Track::MergedTrack(merged_track) => child
                .arg("--meta-title")
                .arg(title)
                .arg(merged_track.merged_url),
            Track::SplitTrack(split_track) => child
                .arg("--meta-title")
                .arg(title)
                .arg("--input-slave")
                .arg(split_track.audio_url)
                .arg(split_track.video_url),
            Track::FileTrack(file) => child.arg("--meta-title").arg(title).arg(file.as_ref()),
        };

        Ok(child.spawn()?)
    }
    // pub async fn launch_persistent_with_http_api(&self) -> anyhow::Result<()> {
    //     let binary_path = self.binary_path.clone();
    //     tokio::spawn(async move {
    //         let mut cmd = Command::new(binary_path);
    //         let cmd = cmd
    //             .arg("--extraintf=http")
    //             .arg("--http-password=abc")
    //             .arg("--http-host=0.0.0.0")
    //             .arg("--http-port=8081");

    //         cmd.stdout(Stdio::piped());
    //         cmd.stderr(Stdio::piped());

    //         let mut child = cmd.spawn().expect("failed to spawn vlc");

    //         let stdout = child
    //             .stdout
    //             .take()
    //             .expect("child did not have a handle to stdout");
    //         let stderr = child
    //             .stderr
    //             .take()
    //             .expect("child did not have a handle to stderr");

    //         tokio::spawn(async move {
    //             let mut lines = BufReader::new(stdout).lines();
    //             while let Ok(Some(line)) = lines.next_line().await {
    //                 info!("[vlc::stdout] {line}")
    //             }
    //         });
    //         tokio::spawn(async move {
    //             let mut lines = BufReader::new(stderr).lines();
    //             while let Ok(Some(line)) = lines.next_line().await {
    //                 info!("[vlc::stderr] {line}")
    //             }
    //         });
    //     });
    //     Ok(())
    // }
}
