use glob::glob;
use log::{error, info};
use serde::Deserialize;
use tempfile::NamedTempFile;
use tokio::process::Command;

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

    pub async fn get_merged_url(link: &str, min_height: MinHeight) -> anyhow::Result<MergedTrack> {
        let json = Self::get_json(link, Format::Merged, min_height).await?;
        json.try_into()
    }

    pub async fn get_split_urls(link: &str, min_height: MinHeight) -> anyhow::Result<SplitTrack> {
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

        let mv_output = Command::new("mv")
            .arg(path.unwrap()) // TODO: this might be wrong
            .arg(temp_file.as_ref())
            .output()
            .await?;
        info!("mv status: {}", mv_output.status);

        Ok(())
    }
}

#[derive(Debug)]
pub struct MergedTrack {
    pub title: String,
    pub merged_url: String,
}

impl TryFrom<JsonDump> for MergedTrack {
    type Error = anyhow::Error;

    fn try_from(value: JsonDump) -> Result<Self, Self::Error> {
        match value.url {
            Some(merged_url) => {
                let title = value.title;

                Ok(Self { title, merged_url })
            }
            None => Err(anyhow::anyhow!(
                "expected url to be not empty, but was empty",
            )),
        }
    }
}

#[derive(Debug)]
pub struct SplitTrack {
    pub title: String,
    pub audio_url: String,
    pub video_url: String,
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

                for format in requested_formats {
                    match format.fps {
                        Some(_) => {
                            if video_url.is_some() {
                                return Err(anyhow::anyhow!("multiple video formats found"));
                            }
                            video_url = Some(format.url);
                        }
                        None => {
                            if audio_url.is_some() {
                                return Err(anyhow::anyhow!("multiple audio formats found"));
                            }
                            audio_url = Some(format.url);
                        }
                    }
                }

                let audio_url = audio_url.ok_or_else(|| anyhow::anyhow!("missing audio format"))?;
                let video_url = video_url.ok_or_else(|| anyhow::anyhow!("missing video format"))?;

                Ok(SplitTrack {
                    title: value.title,
                    audio_url,
                    video_url,
                })
            }
            None => Err(anyhow::anyhow!(
                "expected requested_formats to have a value",
            )),
        }
    }
}

pub enum Track<'a> {
    MergedTrack(MergedTrack),
    SplitTrack(SplitTrack),
    FileTrack(
        &'a NamedTempFile,
        String, /* TODO: this is a bit messy here */
    ),
}

#[derive(Deserialize)]
struct JsonDump {
    title: String,
    requested_formats: Option<Vec<RequestedFormat>>,
    url: Option<String>,
}

#[derive(Deserialize)]
struct RequestedFormat {
    url: String,
    fps: Option<f32>,
}

//yt-dlp -f "ba+bv[height<=720]" --skip-download --dump-json "https://www.youtube.com/watch?v=GNXNwT65ymg" | jq > out.json
