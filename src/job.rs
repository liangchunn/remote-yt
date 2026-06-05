use std::sync::Arc;

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
        config: Arc<Config>,
    },
    QueueSplit {
        url: String,
        height: Option<u32>,
        format_id: String,
        config: Arc<Config>,
    },
    Queue {
        url: String,
        config: Arc<Config>,
    },
}

#[derive(Serialize, Deserialize, Copy, Clone)]
pub enum JobTypeString {
    QueueMerged,
    QueueSplit,
    Queue,
}

impl From<&JobType> for JobTypeString {
    fn from(job_type: &JobType) -> Self {
        match job_type {
            JobType::QueueMerged { .. } => JobTypeString::QueueMerged,
            JobType::QueueSplit { .. } => JobTypeString::QueueSplit,
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
                let min_height = height.map(MinHeight).unwrap_or_default();
                let track = Video::get_merged_track(&url, min_height, &config).await?;

                let curr_format_id = track.track_info.format_id.clone();
                if curr_format_id != format_id {
                    warn!(
                        "track_info desync: queued format {}, but playing {} format",
                        format_id, curr_format_id
                    );
                }

                let title = track.track_info.title.clone();
                info!("starting {title}");

                VlcClient::new(config.vlc_path.as_str())
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
                let min_height = height.map(MinHeight).unwrap_or_default();
                let track = Video::get_split_track(&url, min_height, &config).await?;

                let curr_format_id = track.track_info.format_id.clone();
                if curr_format_id != format_id {
                    warn!(
                        "track_info desync: queued format {}, but playing {} format",
                        format_id, curr_format_id
                    );
                }

                let title = track.track_info.title.clone();
                info!("starting {title}");

                VlcClient::new(config.vlc_path.as_str())
                    .oneshot(Track::Split(track), &title)
                    .await
            }
            JobType::Queue { url, config } => {
                // the first run is just to get the title, we're running it again in case the URLs expire
                let track = Video::get_track(&url, &config).await?;

                let title = track.title();
                info!("starting {title}");

                VlcClient::new(config.vlc_path.as_str())
                    .oneshot(track, &title)
                    .await
            }
        }
    }
}
