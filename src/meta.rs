use serde::Serialize;
use uuid::Uuid;

use crate::yt_dlp::TrackInfo;

#[derive(Serialize)]
pub struct InspectMetadata {
    pub job_id: Uuid,
    pub current: bool,
    pub track_info: TrackInfo,
}
