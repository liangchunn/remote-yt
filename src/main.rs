use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use tower_http::{
    compression::CompressionLayer,
    services::{ServeDir, ServeFile},
};
use tracing::{Level, error, info};

use crate::{
    format::MinHeight,
    history::{History, HistoryEntry},
    meta::InspectMetadata,
    playlist::{PlaylistEntry, PlaylistStore},
    queue::QueueManager,
    rpc::{Rpc, RpcCommand, RpcResponse},
    yt_dlp::Video,
};
use tokio::sync::Mutex;

pub mod config;
mod format;
mod history;
mod job;
mod meta;
mod playlist;
mod queue;
mod rpc;
mod vlc;
mod yt_dlp;

struct AppState {
    queue: Arc<QueueManager>,
    playlists: Mutex<PlaylistStore>,
    rpc: Arc<Rpc>,
    config: Arc<config::Config>,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    let history = History::new("history.json".into()).await?;
    let playlists = PlaylistStore::new("playlists.json".into()).await?;
    let config = config::parse_config("config.toml")?;

    let config = Arc::new(config);
    let queue = Arc::new(QueueManager::new(history));
    let _queue_worker = queue.start();

    let app_state = Arc::new(AppState {
        queue,
        playlists: Mutex::new(playlists),
        rpc: Arc::new(Rpc::new(
            config.vlc_rpc.host.clone(),
            config.vlc_rpc.port,
            config.vlc_rpc.password.clone(),
        )),
        config,
    });

    let serve_app =
        ServeDir::new("ui/dist").not_found_service(ServeFile::new("ui/dist/index.html"));

    let app = Router::new()
        .route("/api/queue", post(queue_handler))
        .route("/api/cancel", post(cancel_current_handler))
        .route("/api/cancel/{id}", post(cancel_id_handler))
        .route("/api/clear", post(clear_handler))
        .route("/api/inspect", get(inspect_handler))
        .route("/api/execute_command", post(player_commands))
        .route("/api/swap/{id}", post(swap))
        .route("/api/move/{id}/{new_pos}", post(move_to))
        .route("/api/playlists", get(get_playlists).post(add_playlist))
        .route("/api/queue_playlist", post(queue_playlist))
        .route("/api/refresh_playlist", post(refresh_playlist))
        .route("/api/remove_playlist", post(remove_playlist))
        .route("/api/history", get(get_history))
        .route("/api/remove_history", post(remove_history_entry))
        .layer(CompressionLayer::new())
        .with_state(app_state)
        .fallback_service(serve_app);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
    axum::serve(listener, app).await?;

    Ok(())
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
enum QueueType {
    Queue,
    QueueMerged,
    QueueSplit,
}

#[derive(Deserialize)]
struct QueuePayload {
    url: String,
    #[serde(rename = "type")]
    queue_type: QueueType,
    height: Option<u32>,
}

#[derive(Serialize)]
struct QueueResponse {
    job_id: usize,
}

#[derive(Serialize)]
struct PlaylistQueueResponse {
    playlist: PlaylistEntry,
    job_ids: Vec<usize>,
}

async fn queue_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<QueuePayload>,
) -> Result<Json<QueueResponse>, AppError> {
    let url = payload.url.clone();
    info!("queueing {url}...");

    match payload.queue_type {
        QueueType::Queue => {
            let config = &state.config;
            let track = Video::get_track(&url, config).await?;

            let job_id = state
                .queue
                .submit(
                    job::JobType::Queue {
                        url: payload.url,
                        config: config.clone(),
                    },
                    track.track_info().to_owned(),
                )
                .await;

            info!("queued {url} with job_id {job_id}");
            Ok(Json(QueueResponse { job_id }))
        }
        QueueType::QueueMerged => {
            let min_height = payload.height.map(MinHeight).unwrap_or_default();
            let merged_track =
                Video::get_merged_track(&payload.url, min_height, &state.config).await?;
            let format_id = merged_track.track_info.format_id.clone();
            let track_info = merged_track.track_info;

            let job_id = state
                .queue
                .submit(
                    job::JobType::QueueMerged {
                        url: payload.url,
                        height: payload.height,
                        format_id,
                        config: state.config.clone(),
                    },
                    track_info,
                )
                .await;

            info!("queued {url} with job_id {job_id}");
            Ok(Json(QueueResponse { job_id }))
        }
        QueueType::QueueSplit => {
            let min_height = payload.height.map(MinHeight).unwrap_or_default();
            let split_track =
                Video::get_split_track(&payload.url, min_height, &state.config).await?;
            let format_id = split_track.track_info.format_id.clone();
            let track_info = split_track.track_info;

            let job_id = state
                .queue
                .submit(
                    job::JobType::QueueSplit {
                        url: payload.url,
                        height: payload.height,
                        format_id,
                        config: state.config.clone(),
                    },
                    track_info,
                )
                .await;

            info!("queued {url} with job_id {job_id}");
            Ok(Json(QueueResponse { job_id }))
        }
    }
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
    now_playing: Option<InspectMetadata>,
    queue: Vec<InspectMetadata>,
    player: Option<RpcResponse>,
}

async fn inspect_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<InspectResponse>, AppError> {
    let ((now_playing, queue), player) =
        tokio::join!(state.queue.inspect(), state.rpc.get_status());
    let player = match player {
        Ok(v) => Some(v),
        Err(e) => {
            if now_playing.is_some() {
                error!("rpc error: {e}");
            }
            None
        }
    };

    Ok(Json(InspectResponse {
        now_playing,
        queue,
        player,
    }))
}

async fn player_commands(
    State(state): State<Arc<AppState>>,
    Json(command): Json<RpcCommand>,
) -> Result<Json<bool>, AppError> {
    state.rpc.execute_command(command).await?;
    Ok(Json(true))
}

async fn swap(
    State(state): State<Arc<AppState>>,
    Path(job_id): Path<usize>,
) -> Result<Json<bool>, AppError> {
    state.queue.swap_with_running(job_id).await?;

    Ok(Json(true))
}

async fn move_to(
    State(state): State<Arc<AppState>>,
    Path((job_id, new_index)): Path<(usize, usize)>,
) -> Result<Json<bool>, AppError> {
    state.queue.reorder_job(job_id, new_index).await?;

    Ok(Json(true))
}

async fn get_playlists(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<PlaylistEntry>>, AppError> {
    let mut playlists = state.playlists.lock().await.list();
    playlists.reverse();

    Ok(Json(playlists))
}

#[derive(Deserialize)]
struct PlaylistPayload {
    url: String,
}

async fn add_playlist(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<PlaylistPayload>,
) -> Result<Json<PlaylistQueueResponse>, AppError> {
    let playlist =
        PlaylistEntry::from(Video::get_youtube_playlist(&payload.url, &state.config).await?);
    let jobs = build_playlist_jobs(&state, &playlist);

    state
        .playlists
        .lock()
        .await
        .upsert(playlist.clone())
        .await?;
    let job_ids = state.queue.submit_many(jobs).await;

    Ok(Json(PlaylistQueueResponse { playlist, job_ids }))
}

#[derive(Deserialize)]
struct QueuePlaylistPayload {
    playlist_url: String,
}

async fn queue_playlist(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<QueuePlaylistPayload>,
) -> Result<Json<PlaylistQueueResponse>, AppError> {
    let playlist = state
        .playlists
        .lock()
        .await
        .get(&payload.playlist_url)
        .ok_or_else(|| anyhow::anyhow!("playlist not found"))?;
    let jobs = build_playlist_jobs(&state, &playlist);
    let job_ids = state.queue.submit_many(jobs).await;

    Ok(Json(PlaylistQueueResponse { playlist, job_ids }))
}

#[derive(Deserialize)]
struct RefreshPlaylistPayload {
    playlist_url: String,
}

async fn refresh_playlist(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RefreshPlaylistPayload>,
) -> Result<Json<PlaylistEntry>, AppError> {
    let playlist_url = state
        .playlists
        .lock()
        .await
        .get(&payload.playlist_url)
        .ok_or_else(|| anyhow::anyhow!("playlist not found"))?
        .webpage_url;

    let playlist =
        PlaylistEntry::from(Video::get_youtube_playlist(&playlist_url, &state.config).await?);
    state
        .playlists
        .lock()
        .await
        .upsert(playlist.clone())
        .await?;

    Ok(Json(playlist))
}

fn build_playlist_jobs(
    state: &AppState,
    playlist: &PlaylistEntry,
) -> Vec<(job::JobType, crate::yt_dlp::TrackInfo)> {
    let mut jobs = Vec::with_capacity(playlist.videos.len());

    for video in &playlist.videos {
        jobs.push((
            job::JobType::Queue {
                url: video.url.clone(),
                config: state.config.clone(),
            },
            video.track_info.clone(),
        ));
    }

    jobs
}

#[derive(Deserialize)]
struct RemovePlaylistPayload {
    playlist_url: String,
}

async fn remove_playlist(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RemovePlaylistPayload>,
) -> Result<(), AppError> {
    state
        .playlists
        .lock()
        .await
        .remove(&payload.playlist_url)
        .await?;

    Ok(())
}

async fn get_history(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<HistoryEntry>>, AppError> {
    let mut history_entries = state.queue.get_history().await;
    history_entries.reverse();

    Ok(Json(history_entries))
}

#[derive(Deserialize)]
struct RemoveHistoryPayload {
    webpage_url: String,
}

async fn remove_history_entry(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RemoveHistoryPayload>,
) -> Result<(), AppError> {
    state
        .queue
        .remove_history_entry(&payload.webpage_url)
        .await?;

    Ok(())
}

#[derive(Debug)]
struct AppError(anyhow::Error);

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        AppError(err)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        error!(error = ?self.0, "internal error");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": self.0.to_string()
            })),
        )
            .into_response()
    }
}
