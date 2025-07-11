use std::path::PathBuf;

use glob::glob;
use serde::{Deserialize, Serialize};
use tempfile::NamedTempFile;
use tokio::process::Command;
use tracing::{error, info};

use crate::format::{Format, MinHeight};

pub struct Video;

impl Video {
    async fn get_json(
        link: &str,
        format: Format,
        min_height: MinHeight,
    ) -> anyhow::Result<JsonDump> {
        let output = Command::new("yt-dlp")
            .arg("-f")
            .arg(format.get_format_string(min_height))
            .arg("--skip-download")
            .arg("--dump-json")
            .arg(link)
            .output()
            .await?
            .stdout;
        let json = String::from_utf8(output)?.trim().to_string();
        let dump = serde_json::from_str::<JsonDump>(&json)?;
        Ok(dump)
    }

    pub async fn get_merged_track(
        link: &str,
        min_height: MinHeight,
    ) -> anyhow::Result<MergedTrack> {
        let json = Self::get_json(link, Format::Merged, min_height).await?;
        json.try_into()
    }

    pub async fn get_split_track(link: &str, min_height: MinHeight) -> anyhow::Result<SplitTrack> {
        let json = Self::get_json(link, Format::Split, min_height).await?;
        json.try_into()
    }

    pub async fn download_file(
        temp_file: &NamedTempFile,
        link: &str,
        min_height: MinHeight,
    ) -> anyhow::Result<()> {
        info!("starting download {link}");
        let exit_staus = Command::new("yt-dlp")
            .arg("-f")
            .arg(Format::Split.get_format_string(min_height))
            .arg("--retries")
            .arg("0")
            .arg("--fragment-retries")
            .arg("0")
            .arg("--abort-on-unavailable-fragments")
            .arg("-o")
            .arg(temp_file.as_ref())
            .arg(link)
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
                    channel: value.channel,
                    uploader_id: value.uploader_id,
                    acodec: value.acodec,
                    vcodec: value.vcodec,
                    height: value.height,
                    width: value.width,
                    thumbnail: value.thumbnail,
                    track_type: TrackType::Merged,
                    format_id: value.format_id,
                    duration: value.duration,
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
                    channel: value.channel,
                    uploader_id: value.uploader_id,
                    acodec: acodec.unwrap_or_default(),
                    vcodec: vcodec.unwrap_or_default(),
                    height,
                    width,
                    thumbnail: value.thumbnail,
                    track_type: TrackType::Split,
                    format_id: value.format_id,
                    duration: value.duration,
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

#[derive(Serialize, Clone, Debug)]
enum TrackType {
    #[serde(rename = "merged")]
    Merged,
    #[serde(rename = "split")]
    Split,
}

#[derive(Serialize, Clone, Debug)]
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
}

pub enum Track<'a> {
    Merged(MergedTrack),
    Split(SplitTrack),
    File(&'a PathBuf),
}

#[derive(Deserialize)]
struct JsonDump {
    title: String,
    requested_formats: Option<Vec<RequestedFormat>>,
    url: Option<String>,
    channel: String,
    uploader_id: String,
    thumbnail: String,
    duration: u32,
    // used for merged format
    acodec: String,
    vcodec: String,
    height: Option<u32>,
    width: Option<u32>,
    // used for validation only
    format_id: String,
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

//yt-dlp -f "ba+bv[height<=720]" --skip-download --dump-json "https://www.youtube.com/watch?v=GNXNwT65ymg" | jq > out.json
