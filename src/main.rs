// #![allow(dead_code, unused_imports)]

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use log::{error, info};
use serde::{Deserialize, Serialize};
use tempfile::NamedTempFile;
use tower_http::{
    compression::CompressionLayer,
    services::{ServeDir, ServeFile},
};
use uuid::Uuid;

use crate::{
    format::MinHeight,
    meta::InspectMetadata,
    queue::QueueManager,
    vlc::VlcClient,
    yt_dlp::{Track, Video},
};

mod format;
mod meta;
mod queue;
mod vlc;
mod yt_dlp;

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // VlcClient::default().launch().await?;
    let queue = Arc::new(QueueManager::new());

    let serve_app =
        ServeDir::new("ui/dist").not_found_service(ServeFile::new("ui/dist/index.html"));

    let app = Router::new()
        .route("/api/queue_merged", post(queue_merged_handler))
        .route("/api/queue_split", post(queue_split_handler))
        .route("/api/queue_file", post(queue_file_handler))
        .route("/api/cancel", post(cancel_current_handler))
        .route("/api/cancel/{id}", post(cancel_id_handler))
        .route("/api/clear", post(clear_handler))
        .route("/api/inspect", get(inspect_handler))
        .layer(CompressionLayer::new())
        .with_state(queue)
        .fallback_service(serve_app);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
    axum::serve(listener, app).await?;

    Ok(())
}

#[derive(Deserialize)]
struct QueuePayload {
    url: String,
    height: Option<u32>,
}

#[derive(Serialize)]
struct QueueResponse {
    job_id: Uuid,
}

async fn queue_merged_handler(
    State(queue): State<Arc<QueueManager>>,
    Json(payload): Json<QueuePayload>,
) -> Result<Json<QueueResponse>, AppError> {
    let url = payload.url.clone();
    info!("queued {url}");

    let merged_track =
        Video::get_merged_track(&payload.url, MinHeight(payload.height.unwrap_or(480))).await?;

    let track_info = merged_track.track_info;

    let uuid = queue
        .submit(
            async move || {
                // the first run is just to get the title, we're running it again in case the URLs expire
                let track =
                    Video::get_merged_track(&payload.url, MinHeight(payload.height.unwrap_or(480)))
                        .await?;
                let title = track.track_info.title.clone();
                info!("starting {title}");
                VlcClient::default()
                    .oneshot(Track::MergedTrack(track), &title)
                    .await
            },
            track_info,
            async move || {},
        )
        .await;

    Ok(Json(QueueResponse { job_id: uuid }))
}

async fn queue_split_handler(
    State(queue): State<Arc<QueueManager>>,
    Json(payload): Json<QueuePayload>,
) -> Result<Json<QueueResponse>, AppError> {
    let url = payload.url.clone();
    info!("queued {url}");

    let split_track =
        Video::get_split_track(&payload.url, MinHeight(payload.height.unwrap_or(480))).await?;

    let track_info = split_track.track_info;

    let uuid = queue
        .submit(
            async move || {
                // the first run is just to get the title, we're running it again in case the URLs expire
                let track =
                    Video::get_split_track(&payload.url, MinHeight(payload.height.unwrap_or(480)))
                        .await?;
                let title = track.track_info.title.clone();
                info!("starting {title}");
                VlcClient::default()
                    .oneshot(Track::SplitTrack(track), &title)
                    .await
            },
            track_info,
            async move || {},
        )
        .await;

    Ok(Json(QueueResponse { job_id: uuid }))
}

async fn queue_file_handler(
    State(queue): State<Arc<QueueManager>>,
    Json(payload): Json<QueuePayload>,
) -> Result<Json<QueueResponse>, AppError> {
    let url = payload.url.clone();
    info!("queued {url}");

    let min_height = payload.height.unwrap_or(480);
    let merged_track = Video::get_merged_track(&payload.url, MinHeight(min_height)).await?;

    let track_info = merged_track.track_info;
    let title = track_info.title.clone();

    let mut temp_file = NamedTempFile::new().map_err(|e| anyhow::anyhow!(e))?;
    temp_file.disable_cleanup(true);
    let temp_file_clone = temp_file.as_ref().to_owned();
    Video::download_file(&temp_file, &payload.url, MinHeight(min_height)).await?;

    let uuid = queue
        .submit(
            async move || {
                info!("starting {title}");
                VlcClient::default()
                    .oneshot(Track::FileTrack(&temp_file), &title)
                    .await
            },
            track_info,
            async move || {
                match std::fs::remove_file(temp_file_clone.clone()) {
                    Ok(()) => info!("deleted file {}", temp_file_clone.display()),
                    Err(e) => error!("failed to delete file {}: {}", temp_file_clone.display(), e),
                };
            },
        )
        .await;

    Ok(Json(QueueResponse { job_id: uuid }))
}

async fn cancel_current_handler(State(queue): State<Arc<QueueManager>>) -> &'static str {
    if queue.cancel().await {
        "task cancelled"
    } else {
        "nothing to cancel"
    }
}

async fn cancel_id_handler(
    State(queue): State<Arc<QueueManager>>,
    Path(job_id): Path<Uuid>,
) -> &'static str {
    if queue.cancel_by_id(job_id).await {
        "task cancelled"
    } else {
        "not found"
    }
}

async fn clear_handler(State(queue): State<Arc<QueueManager>>) -> &'static str {
    queue.clear().await;
    "queue cleared"
}

async fn inspect_handler(State(queue): State<Arc<QueueManager>>) -> Json<Vec<InspectMetadata>> {
    let items = queue.inspect().await;

    Json(items)
}

// Wrapper type for anyhow::Error
#[derive(Debug)]
struct AppError(anyhow::Error);

// Implement From<anyhow::Error> to allow easy conversion
impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        AppError(err)
    }
}

// Implement IntoResponse so Axum can convert your error into an HTTP response
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        // Customize this to return different status codes if needed
        eprintln!("Internal error: {:?}", self.0); // Logging
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Internal Server Error: {}", self.0),
        )
            .into_response()
    }
}

// let url = Video::get_merged_url(
//         "https://www.youtube.com/watch?v=GNXNwT65ymg",
//         MinHeight::default(),
//     )
//     .await?;

//     println!("{}", url.merged_url);

//     let url = Video::get_split_urls(
//         "https://www.youtube.com/watch?v=GNXNwT65ymg",
//         MinHeight::default(),
//     )
//     .await?;

//     println!("{}", url.video_url);
//     println!("{}", url.audio_url);

// VlcClient::default().launch().await?;

// sleep(Duration::from_secs(100)).await;
