use std::{ffi::OsString, path::PathBuf};

use glob::glob;
use serde::{Deserialize, Serialize};
use tempfile::NamedTempFile;
use tokio::process::Command;
use tracing::{error, info};
use url::Url;

use crate::{
    config::{Config, Provider},
    format::{Format, MinHeight},
};

pub struct Video;

impl Video {
    fn log_command(binary: &str, args: &[OsString]) {
        let args: Vec<_> = args.iter().map(|arg| arg.to_string_lossy()).collect();
        info!(binary = %binary, args = ?args, "running command");
    }

    async fn get_json(
        link: &str,
        format: Format,
        min_height: MinHeight,
        config: &Config,
    ) -> anyhow::Result<JsonDump> {
        let format = format.get_format_string(min_height);
        let args = vec![
            OsString::from("-f"),
            OsString::from(format),
            OsString::from("--skip-download"),
            OsString::from("--dump-json"),
            OsString::from(link),
        ];
        Self::log_command(&config.yt_dlp_path, &args);
        let output = Command::new(&config.yt_dlp_path)
            // .arg("--impersonate")
            // .arg("Chrome")
            .args(&args)
            .output()
            .await?;
        let stdout = output.stdout;
        let stderr = output.stderr;
        if stdout.len() == 0 {
            let err_msg = String::from_utf8(stderr)?;
            error!("yt-dlp error: {}", err_msg);
            return Err(anyhow::anyhow!("yt-dlp failed: {}", err_msg));
        }
        let json = String::from_utf8(stdout)?.trim().to_string();
        let dump = serde_json::from_str::<JsonDump>(&json)?;
        Ok(dump)
    }

    pub async fn get_track<'a>(url: &str, config: &Config) -> anyhow::Result<Track<'a>> {
        let parsed_url = Url::parse(url)?;
        let host = parsed_url
            .host_str()
            .ok_or_else(|| anyhow::anyhow!("invalid url"))?;

        let provider = Self::find_provider_for_host(host, config)
            .ok_or_else(|| anyhow::anyhow!("provider not found for host: {}", host))?;

        let mut args = provider.args.iter().map(OsString::from).collect::<Vec<_>>();
        args.extend([
            OsString::from("-f"),
            OsString::from(provider.format.as_str()),
            OsString::from("--skip-download"),
            OsString::from("--dump-json"),
            OsString::from(url),
        ]);
        Self::log_command(&config.yt_dlp_path, &args);

        let output = Command::new(&config.yt_dlp_path)
            .args(&args)
            .output()
            .await?;

        let stdout = output.stdout;
        let stderr = output.stderr;
        if stdout.len() == 0 {
            let err_msg = String::from_utf8(stderr)?;
            error!("yt-dlp error: {}", err_msg);
            return Err(anyhow::anyhow!("yt-dlp failed: {}", err_msg));
        }
        let json = String::from_utf8(stdout)?.trim().to_string();
        let dump = serde_json::from_str::<JsonDump>(&json)?;

        let track = match provider.r#type {
            TrackType::Merged => {
                let json: MergedTrack = dump.try_into()?;
                Track::Merged(json)
            }
            TrackType::Split => {
                let json: SplitTrack = dump.try_into()?;
                Track::Split(json)
            }
        };

        Ok(track)
    }

    fn find_provider_for_host<'a>(host: &str, config: &'a Config) -> Option<&'a Provider> {
        for key in config.providers.keys() {
            if host.contains(key) {
                return config.providers.get(key);
            }
        }
        None
    }

    pub async fn get_merged_track(
        link: &str,
        min_height: MinHeight,
        config: &Config,
    ) -> anyhow::Result<MergedTrack> {
        let json = Self::get_json(link, Format::Merged, min_height, config).await?;
        json.try_into()
    }

    pub async fn get_split_track(
        link: &str,
        min_height: MinHeight,
        config: &Config,
    ) -> anyhow::Result<SplitTrack> {
        let json = Self::get_json(link, Format::Split, min_height, config).await?;
        json.try_into()
    }

    pub async fn download_file(
        temp_file: &NamedTempFile,
        link: &str,
        min_height: MinHeight,
    ) -> anyhow::Result<()> {
        info!("starting download {link}");
        let yt_dlp_path = "/Users/liangchun/dev/ex/yt-dlp/yt-dlp.sh";
        let args = vec![
            OsString::from("-f"),
            OsString::from(Format::Split.get_format_string(min_height)),
            OsString::from("--retries"),
            OsString::from("0"),
            OsString::from("--fragment-retries"),
            OsString::from("0"),
            OsString::from("--abort-on-unavailable-fragments"),
            OsString::from("-o"),
            temp_file.as_ref().as_os_str().to_os_string(),
            OsString::from(link),
        ];
        Self::log_command(yt_dlp_path, &args);
        let exit_staus = Command::new(yt_dlp_path)
            .args(&args)
            .spawn()?
            .wait()
            .await?;

        if !exit_staus.success() {
            return Err(anyhow::anyhow!("failed to download {}", link));
        }

        info!("download success {link}");

        info!(
            "moving file to correct path -> {}",
            temp_file.as_ref().display()
        );

        let pattern = format!("{}.*", temp_file.as_ref().display());

        let paths = glob(&pattern)?;
        let mut path = None;
        for p in paths {
            match p {
                Ok(p) => path = Some(p),
                Err(e) => error!("glob error: {e}"),
            }
        }

        match std::fs::rename(path.unwrap(), temp_file.as_ref()) {
            Ok(_) => {}
            Err(e) => {
                error!("failed to rename file: {e}");
                return Err(anyhow::anyhow!("failed to rename file: {e}"));
            }
        };

        Ok(())
    }
}

#[derive(Debug)]
pub struct MergedTrack {
    pub merged_url: String,
    pub track_info: TrackInfo,
}

impl TryFrom<JsonDump> for MergedTrack {
    type Error = anyhow::Error;

    fn try_from(value: JsonDump) -> Result<Self, Self::Error> {
        match value.url {
            Some(merged_url) => {
                let track_info = TrackInfo {
                    title: value.title,
                    channel: value.channel.or(value.uploader).unwrap_or_default(),
                    uploader_id: value.uploader_id.unwrap_or_default(),
                    acodec: value.acodec.unwrap_or_default(),
                    vcodec: value.vcodec.unwrap_or_default(),
                    height: value.height,
                    width: value.width,
                    thumbnail: value.thumbnail,
                    track_type: TrackType::Merged,
                    format_id: value.format_id,
                    duration: value.duration.unwrap_or_default(),
                    webpage_url: value.webpage_url,
                };

                Ok(Self {
                    merged_url,
                    track_info,
                })
            }
            None => Err(anyhow::anyhow!(
                "expected url to be not empty, but was empty",
            )),
        }
    }
}

#[derive(Debug)]
pub struct SplitTrack {
    pub audio_url: String,
    pub video_url: String,
    pub track_info: TrackInfo,
}

impl TryFrom<JsonDump> for SplitTrack {
    type Error = anyhow::Error;

    fn try_from(value: JsonDump) -> Result<Self, Self::Error> {
        match value.requested_formats {
            Some(requested_formats) => {
                if requested_formats.len() != 2 {
                    return Err(anyhow::anyhow!(
                        "expected exactly 2 requested formats, got {}",
                        requested_formats.len()
                    ));
                }

                let mut audio_url = None;
                let mut video_url = None;
                let mut vcodec = None;
                let mut acodec = None;
                let mut height = None;
                let mut width = None;

                for format in requested_formats {
                    match format.fps {
                        Some(_) => {
                            if video_url.is_some() {
                                return Err(anyhow::anyhow!("multiple video formats found"));
                            }
                            video_url = Some(format.url);
                            vcodec = Some(format.vcodec);
                            height = format.height;
                            width = format.width
                        }
                        None => {
                            if audio_url.is_some() {
                                return Err(anyhow::anyhow!("multiple audio formats found"));
                            }
                            audio_url = Some(format.url);
                            acodec = Some(format.acodec);
                        }
                    }
                }

                let audio_url = audio_url.ok_or_else(|| anyhow::anyhow!("missing audio format"))?;
                let video_url = video_url.ok_or_else(|| anyhow::anyhow!("missing video format"))?;

                let track_info = TrackInfo {
                    title: value.title,
                    channel: value.channel.or(value.uploader).unwrap_or_default(),
                    uploader_id: value.uploader_id.unwrap_or_default(),
                    acodec: acodec.unwrap_or_default(),
                    vcodec: vcodec.unwrap_or_default(),
                    height,
                    width,
                    thumbnail: value.thumbnail,
                    track_type: TrackType::Split,
                    format_id: value.format_id,
                    duration: value.duration.unwrap_or_default(),
                    webpage_url: value.webpage_url,
                };

                Ok(SplitTrack {
                    audio_url,
                    video_url,
                    track_info,
                })
            }
            None => Err(anyhow::anyhow!(
                "expected requested_formats to have a value",
            )),
        }
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub enum TrackType {
    #[serde(rename = "merged")]
    Merged,
    #[serde(rename = "split")]
    Split,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct TrackInfo {
    pub title: String,
    channel: String,
    uploader_id: String,
    acodec: String,
    vcodec: String,
    height: Option<u32>,
    width: Option<u32>,
    thumbnail: String,
    track_type: TrackType,
    pub format_id: String,
    duration: u32,
    pub webpage_url: String,
}

pub enum Track<'a> {
    Merged(MergedTrack),
    Split(SplitTrack),
    File(&'a PathBuf),
}

impl<'a> Track<'a> {
    pub fn track_info(&self) -> &TrackInfo {
        match self {
            Track::Merged(track) => &track.track_info,
            Track::Split(track) => &track.track_info,
            Track::File(_) => panic!("file track has no track info"),
        }
    }
    pub fn get_title(&self) -> String {
        match self {
            Track::Merged(track) => track.track_info.title.clone(),
            Track::Split(track) => track.track_info.title.clone(),
            Track::File(path) => path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
        }
    }
}

#[derive(Deserialize)]
struct JsonDump {
    title: String,
    requested_formats: Option<Vec<RequestedFormat>>,
    url: Option<String>,
    channel: Option<String>,
    uploader: Option<String>,
    uploader_id: Option<String>,
    thumbnail: String,
    duration: Option<u32>,
    // used for merged format
    acodec: Option<String>,
    vcodec: Option<String>,
    height: Option<u32>,
    width: Option<u32>,
    // used for validation only
    format_id: String,
    // original link of video
    webpage_url: String,
}

#[derive(Deserialize)]
struct RequestedFormat {
    url: String,
    fps: Option<f32>,
    // used for split format
    acodec: String,
    vcodec: String,
    height: Option<u32>,
    width: Option<u32>,
}
