use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tokio::process::Child;
use tracing::{info, warn};

use crate::{
    config::Config,
    format::MinHeight,
    vlc::VlcClient,
    yt_dlp::{Track, TrackInfo, Video},
};

#[allow(clippy::enum_variant_names)]
#[derive(Clone, Debug)]
pub enum JobType {
    QueueMerged {
        url: String,
        height: Option<u32>,
        format_id: String,
        config: Config,
    },
    QueueSplit {
        url: String,
        height: Option<u32>,
        format_id: String,
        config: Config,
    },
    QueueFile {
        title: String,
        file: PathBuf,
        config: Config,
    },
    Queue {
        url: String,
        config: Config,
    },
}

#[derive(Serialize, Deserialize, Copy, Clone)]
pub enum JobTypeString {
    QueueMerged,
    QueueSplit,
    QueueFile,
    Queue,
}

impl From<&JobType> for JobTypeString {
    fn from(job_type: &JobType) -> Self {
        match job_type {
            JobType::QueueMerged { .. } => JobTypeString::QueueMerged,
            JobType::QueueSplit { .. } => JobTypeString::QueueSplit,
            JobType::QueueFile { .. } => JobTypeString::QueueFile,
            JobType::Queue { .. } => JobTypeString::Queue,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Job {
    pub id: usize,
    pub metadata: TrackInfo,
    pub job_type: JobType,
}

impl Job {
    pub async fn execute(self) -> anyhow::Result<Child> {
        match self.job_type {
            JobType::QueueMerged {
                url,
                height,
                format_id,
                config,
            } => {
                // the first run is just to get the title, we're running it again in case the URLs expire
                let track =
                    Video::get_merged_track(&url, MinHeight(height.unwrap_or(480)), &config)
                        .await?;

                let curr_format_id = track.track_info.format_id.clone();
                if curr_format_id != format_id {
                    warn!(
                        "track_info desync: queued format {}, but playing {} format",
                        format_id, curr_format_id
                    );
                }

                let title = track.track_info.title.clone();
                info!("starting {title}");

                VlcClient::new(config.vlc_path.into())
                    .oneshot(Track::Merged(track), &title)
                    .await
            }
            JobType::QueueSplit {
                url,
                height,
                format_id,
                config,
            } => {
                // the first run is just to get the title, we're running it again in case the URLs expire
                let track =
                    Video::get_split_track(&url, MinHeight(height.unwrap_or(480)), &config).await?;

                let curr_format_id = track.track_info.format_id.clone();
                if curr_format_id != format_id {
                    warn!(
                        "track_info desync: queued format {}, but playing {} format",
                        format_id, curr_format_id
                    );
                }

                let title = track.track_info.title.clone();
                info!("starting {title}");

                VlcClient::new(config.vlc_path.into())
                    .oneshot(Track::Split(track), &title)
                    .await
            }
            JobType::QueueFile {
                title,
                file,
                config,
            } => {
                info!("starting {title}");
                VlcClient::new(config.vlc_path.into())
                    .oneshot(Track::File(&file), &title)
                    .await
            }
            JobType::Queue { url, config } => {
                // the first run is just to get the title, we're running it again in case the URLs expire
                let track = Video::get_track(&url, &config).await?;

                let title = track.get_title();
                info!("starting {title}");

                VlcClient::new(config.vlc_path.into())
                    .oneshot(track, &title)
                    .await
            }
        }
    }
}
