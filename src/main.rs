// #![allow(dead_code, unused_imports)]

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use tempfile::NamedTempFile;
use tower_http::{
    compression::CompressionLayer,
    services::{ServeDir, ServeFile},
};
use tracing::{Level, error, info, warn};

use crate::{
    format::MinHeight,
    meta::InspectMetadata,
    queue::QueueManager,
    rpc::{Rpc, RpcCommand, RpcResponse},
    vlc::VlcClient,
    yt_dlp::{Track, Video},
};

mod format;
mod meta;
mod queue;
mod rpc;
mod vlc;
mod yt_dlp;

struct AppState {
    queue: Arc<QueueManager>,
    rpc: Arc<Rpc>,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    let app_state = Arc::new(AppState {
        queue: Arc::new(QueueManager::new()),
        rpc: Arc::new(Rpc::new("0.0.0.0".into(), 8081, "abc".into())),
    });

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
        .route("/api/execute_command", post(player_commands))
        .layer(CompressionLayer::new())
        .with_state(app_state)
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
    job_id: usize,
}

async fn queue_merged_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<QueuePayload>,
) -> Result<Json<QueueResponse>, AppError> {
    let url = payload.url.clone();
    info!("queueing {url}...");

    let merged_track =
        Video::get_merged_track(&payload.url, MinHeight(payload.height.unwrap_or(480))).await?;

    let pre_format_id = merged_track.track_info.format_id.clone();
    let track_info = merged_track.track_info;

    let job_id = state
        .queue
        .submit(
            async move || {
                {
                    // the first run is just to get the title, we're running it again in case the URLs expire
                    let track = Video::get_merged_track(
                        &payload.url,
                        MinHeight(payload.height.unwrap_or(480)),
                    )
                    .await?;

                    let post_format_id = track.track_info.format_id.clone();
                    if pre_format_id != post_format_id {
                        warn!(
                            "track_info desync: queued format {}, but playing {} format",
                            pre_format_id, post_format_id
                        );
                    }

                    let title = track.track_info.title.clone();
                    info!("starting {title}");

                    VlcClient::default()
                        .oneshot(Track::MergedTrack(track), &title)
                        .await
                }
            },
            track_info,
            async move || {},
        )
        .await;

    info!("queued {url} with job_id {job_id}");

    Ok(Json(QueueResponse { job_id }))
}

async fn queue_split_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<QueuePayload>,
) -> Result<Json<QueueResponse>, AppError> {
    let url = payload.url.clone();
    info!("queueing {url}...");

    let split_track =
        Video::get_split_track(&payload.url, MinHeight(payload.height.unwrap_or(480))).await?;

    let pre_format_id = split_track.track_info.format_id.clone();
    let track_info = split_track.track_info;

    let job_id = state
        .queue
        .submit(
            async move || {
                // the first run is just to get the title, we're running it again in case the URLs expire
                let track =
                    Video::get_split_track(&payload.url, MinHeight(payload.height.unwrap_or(480)))
                        .await?;

                let post_format_id = track.track_info.format_id.clone();
                if pre_format_id != post_format_id {
                    warn!(
                        "track_info desync: queued format {}, but playing {} format",
                        pre_format_id, post_format_id
                    );
                }

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

    info!("queued {url} with job_id {job_id}");

    Ok(Json(QueueResponse { job_id }))
}

async fn queue_file_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<QueuePayload>,
) -> Result<Json<QueueResponse>, AppError> {
    let url = payload.url.clone();
    info!("queueing {url}...");

    let min_height = payload.height.unwrap_or(480);
    let merged_track = Video::get_merged_track(&payload.url, MinHeight(min_height)).await?;

    let track_info = merged_track.track_info;
    let title = track_info.title.clone();

    let mut temp_file = NamedTempFile::new().map_err(|e| anyhow::anyhow!(e))?;
    temp_file.disable_cleanup(true);
    let temp_file_clone = temp_file.as_ref().to_owned();
    Video::download_file(&temp_file, &payload.url, MinHeight(min_height)).await?;

    let job_id = state
        .queue
        .submit(
            async move || {
                info!("starting {title}");
                VlcClient::default()
                    .oneshot(Track::FileTrack(&temp_file), &title)
                    .await
            },
            track_info,
            async move || {
                match tokio::fs::remove_file(temp_file_clone.clone()).await {
                    Ok(()) => info!("deleted file {}", temp_file_clone.display()),
                    Err(e) => {
                        error!("failed to delete file {}: {}", temp_file_clone.display(), e)
                    }
                };
            },
        )
        .await;

    info!("queued {url} with job_id {job_id}");

    Ok(Json(QueueResponse { job_id }))
}

async fn cancel_current_handler(State(state): State<Arc<AppState>>) -> &'static str {
    if state.queue.cancel().await {
        "task cancelled"
    } else {
        "nothing to cancel"
    }
}

async fn cancel_id_handler(
    State(state): State<Arc<AppState>>,
    Path(job_id): Path<usize>,
) -> &'static str {
    if state.queue.cancel_by_id(job_id).await {
        "task cancelled"
    } else {
        "not found"
    }
}

async fn clear_handler(State(state): State<Arc<AppState>>) -> &'static str {
    state.queue.clear().await;
    "queue cleared"
}

#[derive(Serialize)]
struct InspectResponse {
    queue: Vec<InspectMetadata>,
    player: Option<RpcResponse>,
}

async fn inspect_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<InspectResponse>, AppError> {
    let (queue, player) = tokio::join!(state.queue.inspect(), state.rpc.get_status());
    let player = match player {
        Ok(v) => Some(v),
        Err(e) => {
            error!("{e:#?}");
            None
        }
    };

    Ok(Json(InspectResponse { queue, player }))
}

async fn player_commands(
    State(state): State<Arc<AppState>>,
    Json(command): Json<RpcCommand>,
) -> Result<Json<bool>, AppError> {
    state.rpc.execute_command(command).await?;
    Ok(Json(true))
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
