use serde::Serialize;
use uuid::Uuid;

#[derive(Clone, Serialize)]
pub struct Metadata {
    pub url: String,
    pub title: String,
    pub channel: String,
    pub uploader_id: String,
}

#[derive(Serialize)]
pub struct InspectMetadata {
    pub job_id: Uuid,
    pub current: bool,
    pub title: String,
    pub url: String,
    pub channel: String,
    pub uploader_id: String,
}
