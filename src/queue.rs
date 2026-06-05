use std::{collections::VecDeque, sync::Arc};

use tokio::{
    sync::{Mutex, Notify},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

use crate::{
    history::{History, HistoryEntry},
    job::{Job, JobType, JobTypeString},
    meta::InspectMetadata,
    yt_dlp::TrackInfo,
};

struct QueueState {
    data: Mutex<QueueData>,
    notify: Notify,
    history: Mutex<History>,
}

struct QueueData {
    pending: VecDeque<Job>,
    running: Option<RunningJob>,
    next_job_id: usize,
}

#[derive(Clone)]
struct RunningJob {
    job: Job,
    cancel_token: CancellationToken,
}

pub struct QueueManager {
    state: Arc<QueueState>,
}

impl QueueManager {
    pub fn new(history: History) -> Self {
        Self {
            state: Arc::new(QueueState {
                data: Mutex::new(QueueData {
                    pending: VecDeque::new(),
                    running: None,
                    next_job_id: 1,
                }),
                notify: Notify::new(),
                history: Mutex::new(history),
            }),
        }
    }

    pub fn start(&self) -> JoinHandle<()> {
        let state = self.state.clone();
        tokio::spawn(async move {
            Self::worker_loop(state).await;
        })
    }

    async fn worker_loop(state: Arc<QueueState>) {
        loop {
            let (job, cancel_token) = loop {
                let notified = state.notify.notified();
                let job = {
                    let mut data = state.data.lock().await;
                    data.pending.pop_front().map(|job| {
                        let cancel_token = CancellationToken::new();
                        data.running = Some(RunningJob {
                            job: job.clone(),
                            cancel_token: cancel_token.clone(),
                        });
                        (job, cancel_token)
                    })
                };

                if let Some(job) = job {
                    break job;
                }

                notified.await;
            };

            let job_type: JobTypeString = (&job.job_type).into();

            info!(job_id = job.id, "starting job");

            let metadata_clone = job.metadata.clone();
            let job_id = job.id;

            let mut child = match job.execute().await {
                Ok(child) => child,
                Err(e) => {
                    error!(job_id, "failed to start process: {e}");
                    Self::clear_running(&state, job_id).await;
                    continue;
                }
            };

            tokio::select! {
                result = child.wait() => {
                    match result {
                        Ok(status) => info!(job_id, "task done: {status}"),
                        Err(e) => error!(job_id, "wait error: {e}"),
                    }
                }
                _ = cancel_token.cancelled() => {
                    info!(job_id, "cancel requested, killing child");
                    let _ = child.kill().await;
                }
            }

            {
                let mut lock = state.history.lock().await;
                match lock.insert(metadata_clone, job_type).await {
                    Ok(()) => info!(job_id, "history updated"),
                    Err(e) => error!(job_id, "failed to update history: {e}"),
                };
            }

            Self::clear_running(&state, job_id).await;
        }
    }

    async fn clear_running(state: &QueueState, job_id: usize) {
        let mut data = state.data.lock().await;
        if data
            .running
            .as_ref()
            .is_some_and(|running| running.job.id == job_id)
        {
            data.running = None;
        }
    }

    pub async fn submit(&self, args: JobType, metadata: TrackInfo) -> usize {
        let id = {
            let mut data = self.state.data.lock().await;
            let id = data.next_job_id;
            data.next_job_id += 1;

            data.pending.push_back(Job {
                id,
                metadata,
                job_type: args,
            });
            id
        };

        self.state.notify.notify_one();
        id
    }

    pub async fn cancel_by_id(&self, job_id: usize) -> bool {
        let mut data = self.state.data.lock().await;
        let index = data.pending.iter().position(|job| job.id == job_id);

        if let Some(i) = index {
            data.pending
                .remove(i)
                .expect("index from VecDeque::position is valid");
            info!(job_id, "cancelled job from queue");
            return true;
        }

        if data
            .running
            .as_ref()
            .is_some_and(|running| running.job.id == job_id)
        {
            let running = data
                .running
                .take()
                .expect("running job checked as Some above");
            running.cancel_token.cancel();
            info!(job_id, "cancelled currently running job");
            return true;
        }

        info!(job_id, "job not found");
        false
    }

    pub async fn cancel(&self) -> bool {
        let mut data = self.state.data.lock().await;
        if let Some(running) = data.running.take() {
            running.cancel_token.cancel();
            info!(job_id = running.job.id, "cancelling current job");
            true
        } else {
            info!("nothing to cancel");
            false
        }
    }

    pub async fn clear(&self) {
        let mut data = self.state.data.lock().await;
        if let Some(running) = data.running.take() {
            running.cancel_token.cancel();
            info!(job_id = running.job.id, "clear: cancelling current job");
        } else {
            info!("nothing to clear");
        }

        for job in data.pending.drain(..) {
            info!(job_id = job.id, "cancelled job");
        }
    }

    pub async fn inspect(&self) -> (Option<InspectMetadata>, Vec<InspectMetadata>) {
        let data = self.state.data.lock().await;
        let current = data.running.as_ref().map(|running| InspectMetadata {
            job_id: running.job.id,
            current: true,
            track_info: running.job.metadata.clone(),
        });

        let curr_queue = data
            .pending
            .iter()
            .map(|job| InspectMetadata {
                job_id: job.id,
                current: false,
                track_info: job.metadata.clone(),
            })
            .collect();

        (current, curr_queue)
    }

    pub async fn reorder_job(&self, job_id: usize, new_index: usize) -> anyhow::Result<()> {
        let mut data = self.state.data.lock().await;

        let old_pos = data
            .pending
            .iter()
            .position(|job| job.id == job_id)
            .ok_or_else(|| anyhow::anyhow!("job {job_id} not found in queue or already running"))?;

        if old_pos == new_index {
            return Ok(());
        }

        let mut items: Vec<Job> = data.pending.drain(..).collect();
        let job = items.remove(old_pos);
        let target_index = new_index.min(items.len());
        items.insert(target_index, job);
        data.pending.extend(items);

        info!(job_id, old_pos, new_index, "reordered job");
        Ok(())
    }

    pub async fn swap_with_running(&self, job_id: usize) -> anyhow::Result<()> {
        let mut data = self.state.data.lock().await;

        let target_index = data
            .pending
            .iter()
            .position(|job| job.id == job_id)
            .ok_or_else(|| anyhow::anyhow!("job {job_id} not found in queue"))?;

        let running = data
            .running
            .take()
            .ok_or_else(|| anyhow::anyhow!("no job is currently running"))?;

        if running.job.id == job_id {
            data.running = Some(running);
            return Err(anyhow::anyhow!("cannot swap a job with itself"));
        }

        let swapped_job = data
            .pending
            .remove(target_index)
            .expect("index from VecDeque::position is valid");
        data.pending.push_front(swapped_job);
        data.pending.insert(target_index + 1, running.job.clone());

        running.cancel_token.cancel();

        info!(
            running_job_id = running.job.id,
            queued_job_id = job_id,
            "swapped running job with queued job"
        );

        Ok(())
    }

    pub async fn get_history(&self) -> Vec<HistoryEntry> {
        let lock = self.state.history.lock().await;
        lock.get_history()
    }

    pub async fn remove_history_entry(&self, webpage_url: &str) -> anyhow::Result<()> {
        let mut lock = self.state.history.lock().await;
        lock.remove(webpage_url).await?;
        Ok(())
    }
}
