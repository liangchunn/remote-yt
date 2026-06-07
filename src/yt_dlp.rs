use std::ffi::OsString;

use serde::{Deserialize, Serialize, de::DeserializeOwned};
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

    async fn run_yt_dlp<T: DeserializeOwned>(
        config: &Config,
        args: &[OsString],
    ) -> anyhow::Result<T> {
        Self::log_command(&config.yt_dlp_path, args);
        let output = Command::new(&config.yt_dlp_path)
            .args(args)
            .output()
            .await?;

        if !output.status.success() {
            let err_msg = String::from_utf8_lossy(&output.stderr);
            error!("yt-dlp error: {}", err_msg);
            return Err(anyhow::anyhow!(
                "yt-dlp failed with exit status {}: {}",
                output.status,
                err_msg,
            ));
        }

        let stdout = output.stdout;
        if stdout.is_empty() {
            return Err(anyhow::anyhow!("yt-dlp produced no output"));
        }
        let json = String::from_utf8(stdout)?.trim().to_string();
        let dump = serde_json::from_str::<T>(&json)?;
        Ok(dump)
    }

    async fn get_json(
        link: &str,
        format: Format,
        min_height: MinHeight,
        config: &Config,
    ) -> anyhow::Result<JsonDump> {
        let format_str = format.format_string(min_height);
        let args = [
            OsString::from("-f"),
            OsString::from(format_str),
            OsString::from("--skip-download"),
            OsString::from("--dump-json"),
            OsString::from(link),
        ];
        Self::run_yt_dlp(config, &args).await
    }

    pub async fn get_track(url: &str, config: &Config) -> anyhow::Result<Track> {
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

        let dump: JsonDump = Self::run_yt_dlp(config, &args).await?;

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
        config
            .providers
            .iter()
            .find_map(|(key, provider)| host.contains(key).then_some(provider))
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

    pub async fn get_youtube_playlist(
        url: &str,
        config: &Config,
    ) -> anyhow::Result<YouTubePlaylist> {
        let parsed_url = Url::parse(url)?;
        let host = parsed_url
            .host_str()
            .ok_or_else(|| anyhow::anyhow!("invalid url"))?;

        if !is_youtube_host(host) {
            return Err(anyhow::anyhow!(
                "playlist support is limited to YouTube URLs"
            ));
        }

        let has_playlist_id = parsed_url
            .query_pairs()
            .any(|(key, value)| key == "list" && !value.trim().is_empty());
        if !has_playlist_id {
            return Err(anyhow::anyhow!(
                "YouTube playlist URL must include a list parameter"
            ));
        }

        let provider = Self::find_provider_for_host(host, config)
            .ok_or_else(|| anyhow::anyhow!("provider not found for host: {}", host))?;
        let mut args = provider.args.iter().map(OsString::from).collect::<Vec<_>>();
        args.extend([
            OsString::from("--flat-playlist"),
            OsString::from("--dump-single-json"),
            OsString::from("--skip-download"),
            OsString::from(url),
        ]);

        let dump: PlaylistJsonDump = Self::run_yt_dlp(config, &args).await?;
        YouTubePlaylist::from_dump(dump, provider.r#type.clone())
    }
}

fn is_youtube_host(host: &str) -> bool {
    host == "youtu.be" || host == "youtube.com" || host.ends_with(".youtube.com")
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

pub enum Track {
    Merged(MergedTrack),
    Split(SplitTrack),
}

impl Track {
    pub fn track_info(&self) -> &TrackInfo {
        match self {
            Track::Merged(track) => &track.track_info,
            Track::Split(track) => &track.track_info,
        }
    }
    pub fn title(&self) -> &str {
        match self {
            Track::Merged(track) => &track.track_info.title,
            Track::Split(track) => &track.track_info.title,
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

#[derive(Clone, Debug)]
pub struct YouTubePlaylist {
    pub id: String,
    pub title: String,
    pub webpage_url: String,
    pub thumbnail: String,
    pub total_duration: u32,
    pub videos: Vec<PlaylistVideo>,
}

impl YouTubePlaylist {
    fn from_dump(value: PlaylistJsonDump, track_type: TrackType) -> anyhow::Result<Self> {
        let mut videos = Vec::with_capacity(value.entries.len());
        let mut total_duration = 0;
        let playlist_thumbnail = pick_thumbnail(&value.thumbnails).unwrap_or_default();
        let channel = value.channel.or(value.uploader).unwrap_or_default();
        let uploader_id = value.uploader_id.or(value.channel_id).unwrap_or_default();

        for entry in value.entries {
            let duration = entry.duration.unwrap_or_default() as u32;
            total_duration += duration;
            let thumbnail =
                pick_thumbnail(&entry.thumbnails).unwrap_or_else(|| playlist_thumbnail.clone());

            videos.push(PlaylistVideo {
                url: entry.url.clone(),
                track_info: TrackInfo {
                    title: entry.title,
                    channel: channel.clone(),
                    uploader_id: uploader_id.clone(),
                    acodec: String::new(),
                    vcodec: String::new(),
                    height: None,
                    width: None,
                    thumbnail,
                    track_type: track_type.clone(),
                    format_id: String::new(),
                    duration,
                    webpage_url: entry.url,
                },
            });
        }

        if videos.is_empty() {
            return Err(anyhow::anyhow!("playlist contains no videos"));
        }

        Ok(Self {
            id: value.id,
            title: value.title,
            webpage_url: value.webpage_url,
            thumbnail: playlist_thumbnail,
            total_duration,
            videos,
        })
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct PlaylistVideo {
    pub url: String,
    pub track_info: TrackInfo,
}

fn pick_thumbnail(thumbnails: &[Thumbnail]) -> Option<String> {
    thumbnails
        .iter()
        .max_by_key(|thumbnail| thumbnail.width.unwrap_or_default())
        .map(|thumbnail| thumbnail.url.clone())
}

#[derive(Deserialize)]
struct PlaylistJsonDump {
    id: String,
    title: String,
    channel: Option<String>,
    channel_id: Option<String>,
    uploader: Option<String>,
    uploader_id: Option<String>,
    #[serde(default)]
    thumbnails: Vec<Thumbnail>,
    entries: Vec<PlaylistEntryJson>,
    webpage_url: String,
}

#[derive(Deserialize)]
struct PlaylistEntryJson {
    title: String,
    url: String,
    #[serde(default)]
    thumbnails: Vec<Thumbnail>,
    duration: Option<f64>,
}

#[derive(Deserialize)]
struct Thumbnail {
    url: String,
    width: Option<u32>,
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::config::{Config, Provider, VlcRpcConfig};

    fn json_dump() -> JsonDump {
        JsonDump {
            title: "video title".to_owned(),
            requested_formats: None,
            url: Some("https://media.example.com/video.mp4".to_owned()),
            channel: Some("channel".to_owned()),
            uploader: Some("uploader".to_owned()),
            uploader_id: Some("uploader-id".to_owned()),
            thumbnail: "https://example.com/thumb.jpg".to_owned(),
            duration: Some(123),
            acodec: Some("aac".to_owned()),
            vcodec: Some("h264".to_owned()),
            height: Some(720),
            width: Some(1280),
            format_id: "18".to_owned(),
            webpage_url: "https://example.com/watch/1".to_owned(),
        }
    }

    fn video_format(url: &str) -> RequestedFormat {
        RequestedFormat {
            url: url.to_owned(),
            fps: Some(30.0),
            acodec: "none".to_owned(),
            vcodec: "h264".to_owned(),
            height: Some(720),
            width: Some(1280),
        }
    }

    fn audio_format(url: &str) -> RequestedFormat {
        RequestedFormat {
            url: url.to_owned(),
            fps: None,
            acodec: "aac".to_owned(),
            vcodec: "none".to_owned(),
            height: None,
            width: None,
        }
    }

    fn thumbnail(url: &str, width: u32) -> Thumbnail {
        Thumbnail {
            url: url.to_owned(),
            width: Some(width),
        }
    }

    fn config_with_providers(providers: HashMap<String, Provider>) -> Config {
        Config {
            vlc_path: "vlc".to_owned(),
            yt_dlp_path: "yt-dlp".to_owned(),
            vlc_rpc: VlcRpcConfig::default(),
            providers,
        }
    }

    #[test]
    fn merged_json_converts_to_track_info() {
        let track: MergedTrack = json_dump().try_into().expect("convert merged track");

        assert_eq!(track.merged_url, "https://media.example.com/video.mp4");
        assert_eq!(track.track_info.title, "video title");
        assert_eq!(track.track_info.channel, "channel");
        assert_eq!(track.track_info.uploader_id, "uploader-id");
        assert_eq!(track.track_info.acodec, "aac");
        assert_eq!(track.track_info.vcodec, "h264");
        assert_eq!(track.track_info.height, Some(720));
        assert_eq!(track.track_info.width, Some(1280));
        assert!(matches!(track.track_info.track_type, TrackType::Merged));
        assert_eq!(track.track_info.format_id, "18");
        assert_eq!(track.track_info.duration, 123);
        assert_eq!(track.track_info.webpage_url, "https://example.com/watch/1");
    }

    #[test]
    fn merged_json_requires_media_url() {
        let mut dump = json_dump();
        dump.url = None;

        let result = MergedTrack::try_from(dump);

        assert!(result.is_err());
    }

    #[test]
    fn split_json_extracts_audio_and_video_formats() {
        let mut dump = json_dump();
        dump.url = None;
        dump.requested_formats = Some(vec![
            audio_format("https://media.example.com/audio.m4a"),
            video_format("https://media.example.com/video.mp4"),
        ]);

        let track: SplitTrack = dump.try_into().expect("convert split track");

        assert_eq!(track.audio_url, "https://media.example.com/audio.m4a");
        assert_eq!(track.video_url, "https://media.example.com/video.mp4");
        assert_eq!(track.track_info.acodec, "aac");
        assert_eq!(track.track_info.vcodec, "h264");
        assert_eq!(track.track_info.height, Some(720));
        assert_eq!(track.track_info.width, Some(1280));
        assert!(matches!(track.track_info.track_type, TrackType::Split));
    }

    #[test]
    fn split_json_rejects_missing_or_malformed_requested_formats() {
        let mut no_formats = json_dump();
        no_formats.requested_formats = None;
        assert!(SplitTrack::try_from(no_formats).is_err());

        let mut too_many = json_dump();
        too_many.requested_formats = Some(vec![
            audio_format("https://media.example.com/audio.m4a"),
            video_format("https://media.example.com/video.mp4"),
            video_format("https://media.example.com/other.mp4"),
        ]);
        assert!(SplitTrack::try_from(too_many).is_err());

        let mut missing_audio = json_dump();
        missing_audio.requested_formats = Some(vec![
            video_format("https://media.example.com/video-1.mp4"),
            video_format("https://media.example.com/video-2.mp4"),
        ]);
        assert!(SplitTrack::try_from(missing_audio).is_err());
    }

    #[test]
    fn provider_matching_uses_first_key_contained_in_host() {
        let mut providers = HashMap::new();
        providers.insert(
            "youtube".to_owned(),
            Provider {
                args: vec!["--cookies".to_owned(), "cookies.txt".to_owned()],
                format: "best".to_owned(),
                r#type: TrackType::Merged,
            },
        );
        providers.insert(
            "vimeo".to_owned(),
            Provider {
                args: vec![],
                format: "bestvideo+bestaudio".to_owned(),
                r#type: TrackType::Split,
            },
        );
        let config = config_with_providers(providers);

        let provider =
            Video::find_provider_for_host("www.youtube.com", &config).expect("provider found");

        assert_eq!(provider.format, "best");
        assert_eq!(provider.args, vec!["--cookies", "cookies.txt"]);
        assert!(matches!(provider.r#type, TrackType::Merged));
        assert!(Video::find_provider_for_host("example.com", &config).is_none());
    }

    #[test]
    fn youtube_playlist_json_converts_to_playlist() {
        let dump = PlaylistJsonDump {
            id: "PL123".to_owned(),
            title: "Playlist".to_owned(),
            channel: Some("Playlist Channel".to_owned()),
            channel_id: Some("channel-id".to_owned()),
            uploader: None,
            uploader_id: None,
            thumbnails: vec![
                thumbnail("https://example.com/small.jpg", 120),
                thumbnail("https://example.com/large.jpg", 360),
            ],
            entries: vec![
                PlaylistEntryJson {
                    title: "First".to_owned(),
                    url: "https://www.youtube.com/watch?v=one".to_owned(),
                    thumbnails: vec![thumbnail("https://example.com/first.jpg", 480)],
                    duration: Some(60.0),
                },
                PlaylistEntryJson {
                    title: "Second".to_owned(),
                    url: "https://www.youtube.com/watch?v=two".to_owned(),
                    thumbnails: vec![],
                    duration: Some(75.0),
                },
            ],
            webpage_url: "https://www.youtube.com/playlist?list=PL123".to_owned(),
        };

        let playlist =
            YouTubePlaylist::from_dump(dump, TrackType::Split).expect("convert playlist");

        assert_eq!(playlist.id, "PL123");
        assert_eq!(playlist.title, "Playlist");
        assert_eq!(playlist.thumbnail, "https://example.com/large.jpg");
        assert_eq!(playlist.total_duration, 135);
        assert_eq!(playlist.videos.len(), 2);
        assert_eq!(
            playlist.videos[0].url,
            "https://www.youtube.com/watch?v=one"
        );
        assert_eq!(playlist.videos[0].track_info.title, "First");
        assert_eq!(playlist.videos[0].track_info.channel, "Playlist Channel");
        assert_eq!(
            playlist.videos[0].track_info.thumbnail,
            "https://example.com/first.jpg"
        );
        assert_eq!(
            playlist.videos[1].track_info.thumbnail,
            "https://example.com/large.jpg"
        );
        assert!(matches!(
            playlist.videos[0].track_info.track_type,
            TrackType::Split
        ));
    }

    #[test]
    fn youtube_playlist_json_rejects_empty_playlists() {
        let dump = PlaylistJsonDump {
            id: "PL123".to_owned(),
            title: "Playlist".to_owned(),
            channel: None,
            channel_id: None,
            uploader: None,
            uploader_id: None,
            thumbnails: vec![],
            entries: vec![],
            webpage_url: "https://www.youtube.com/playlist?list=PL123".to_owned(),
        };

        assert!(YouTubePlaylist::from_dump(dump, TrackType::Merged).is_err());
    }

    #[tokio::test]
    async fn get_track_errors_before_spawning_when_url_or_provider_is_invalid() {
        let config = config_with_providers(HashMap::new());

        assert!(Video::get_track("not a url", &config).await.is_err());
        assert!(
            Video::get_track("https://unsupported.example.com/watch/1", &config)
                .await
                .is_err()
        );
    }
}
