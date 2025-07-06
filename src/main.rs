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
use serde::Deserialize;
use tempfile::NamedTempFile;
use uuid::Uuid;

use crate::{
    format::MinHeight,
    queue::QueueManager,
    vlc::VlcClient,
    yt_dlp::{Track, Video},
};

mod format;
mod queue;
mod vlc;
mod yt_dlp;

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // VlcClient::default().launch().await?;
    let queue = Arc::new(QueueManager::<String>::new());

    let app = Router::new()
        .route("/api/queue_merged", post(queue_merged_handler))
        .route("/api/queue_split", post(queue_split_handler))
        .route("/api/queue_file", post(queue_file_handler))
        .route("/api/cancel", post(cancel_current_handler))
        .route("/api/cancel/{id}", post(cancel_id_handler))
        .route("/api/clear", post(clear_handler))
        .route("/api/inspect", get(inspect_handler))
        .with_state(queue);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
    axum::serve(listener, app).await?;

    Ok(())
}

#[derive(Deserialize)]
struct QueuePayload {
    url: String,
    height: Option<u32>,
}

async fn queue_merged_handler(
    State(queue): State<Arc<QueueManager<String>>>,
    Json(payload): Json<QueuePayload>,
) -> Result<String, AppError> {
    let url = payload.url.clone();
    info!("queued {url}");

    let uuid = queue
        .submit(
            async move || {
                // the first run is just to get the title, we're running it again in case the URLs expire
                let track =
                    Video::get_merged_url(&payload.url, MinHeight(payload.height.unwrap_or(480)))
                        .await?;
                info!("starting {}", track.title);
                VlcClient::default()
                    .oneshot(Track::MergedTrack(track))
                    .await
            },
            url,
            async move || {},
        )
        .await;

    Ok(uuid.to_string())
}

async fn queue_split_handler(
    State(queue): State<Arc<QueueManager<String>>>,
    Json(payload): Json<QueuePayload>,
) -> Result<String, AppError> {
    let url = payload.url.clone();
    info!("queued {url}");

    let uuid = queue
        .submit(
            async move || {
                // the first run is just to get the title, we're running it again in case the URLs expire
                let track =
                    Video::get_split_urls(&payload.url, MinHeight(payload.height.unwrap_or(480)))
                        .await?;
                info!("starting {}", track.title);
                VlcClient::default().oneshot(Track::SplitTrack(track)).await
            },
            url,
            async move || {},
        )
        .await;

    Ok(uuid.to_string())
}

async fn queue_file_handler(
    State(queue): State<Arc<QueueManager<String>>>,
    Json(payload): Json<QueuePayload>,
) -> Result<String, AppError> {
    let min_height = payload.height.unwrap_or(480);
    let title = Video::get_merged_url(&payload.url, MinHeight(min_height))
        .await?
        .title;
    info!("queued {title}");

    let mut temp_file = NamedTempFile::new().map_err(|e| anyhow::anyhow!(e))?;
    temp_file.disable_cleanup(true);
    let temp_file_clone = temp_file.as_ref().to_owned();
    Video::download_file(&temp_file, &payload.url, MinHeight(min_height)).await?;
    let title_clone = title.clone();

    let uuid = queue
        .submit(
            async move || {
                info!("starting {title_clone}");
                VlcClient::default()
                    .oneshot(Track::FileTrack(&temp_file, title_clone))
                    .await
            },
            title,
            async move || {
                match std::fs::remove_file(temp_file_clone.clone()) {
                    Ok(()) => info!("deleted file {}", temp_file_clone.display()),
                    Err(e) => error!("failed to delete file {}: {}", temp_file_clone.display(), e),
                };
            },
        )
        .await;

    Ok(uuid.to_string())
}

async fn cancel_current_handler(State(queue): State<Arc<QueueManager<String>>>) -> &'static str {
    if queue.cancel().await {
        "task cancelled"
    } else {
        "nothing to cancel"
    }
}

async fn cancel_id_handler(
    State(queue): State<Arc<QueueManager<String>>>,
    Path(job_id): Path<Uuid>,
) -> &'static str {
    if queue.cancel_by_id(job_id).await {
        "task cancelled"
    } else {
        "not found"
    }
}

async fn clear_handler(State(queue): State<Arc<QueueManager<String>>>) -> &'static str {
    queue.clear().await;
    "queue cleared"
}

async fn inspect_handler(State(queue): State<Arc<QueueManager<String>>>) -> String {
    let items = queue.inspect().await;

    serde_json::to_string(&items).unwrap()
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
