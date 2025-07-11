use std::path::PathBuf;

use tokio::process::Child;
use tracing::{info, warn};

use crate::{
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
    },
    QueueSplit {
        url: String,
        height: Option<u32>,
        format_id: String,
    },
    QueueFile {
        title: String,
        file: PathBuf,
    },
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
            } => {
                // the first run is just to get the title, we're running it again in case the URLs expire
                let track = Video::get_merged_track(&url, MinHeight(height.unwrap_or(480))).await?;

                let curr_format_id = track.track_info.format_id.clone();
                if curr_format_id != format_id {
                    warn!(
                        "track_info desync: queued format {}, but playing {} format",
                        format_id, curr_format_id
                    );
                }

                let title = track.track_info.title.clone();
                info!("starting {title}");

                VlcClient::default()
                    .oneshot(Track::Merged(track), &title)
                    .await
            }
            JobType::QueueSplit {
                url,
                height,
                format_id,
            } => {
                // the first run is just to get the title, we're running it again in case the URLs expire
                let track = Video::get_split_track(&url, MinHeight(height.unwrap_or(480))).await?;

                let curr_format_id = track.track_info.format_id.clone();
                if curr_format_id != format_id {
                    warn!(
                        "track_info desync: queued format {}, but playing {} format",
                        format_id, curr_format_id
                    );
                }

                let title = track.track_info.title.clone();
                info!("starting {title}");

                VlcClient::default()
                    .oneshot(Track::Split(track), &title)
                    .await
            }
            JobType::QueueFile { title, file } => {
                info!("starting {title}");
                VlcClient::default()
                    .oneshot(Track::File(&file), &title)
                    .await
            }
        }
    }
}
