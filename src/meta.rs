use serde::Serialize;

use crate::yt_dlp::TrackInfo;

#[derive(Serialize)]
pub struct InspectMetadata {
    pub job_id: usize,
    pub current: bool,
    pub track_info: TrackInfo,
}
