use std::{
    collections::VecDeque,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
};

use tokio::sync::{Mutex, Notify};
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

use crate::{
    history::{History, HistoryEntry},
    job::{Job, JobType, JobTypeString},
    meta::InspectMetadata,
    yt_dlp::TrackInfo,
};

struct QueueState {
    queue: Mutex<VecDeque<Job>>,
    notify: Notify,
    running: Mutex<Option<(Job, CancellationToken)>>,
    clear_requested: AtomicBool,
    job_id: AtomicUsize,
    history: Mutex<History>,
}

pub struct QueueManager {
    state: Arc<QueueState>,
}

impl QueueManager {
    pub fn new(history: History) -> Self {
        let state = Arc::new(QueueState {
            queue: Mutex::new(VecDeque::new()),
            notify: Notify::new(),
            running: Mutex::new(None),
            clear_requested: AtomicBool::new(false),
            job_id: AtomicUsize::new(1),
            history: Mutex::new(history),
        });

        let state_ref = state.clone();

        tokio::spawn(async move {
            loop {
                let job = {
                    let mut q = state_ref.queue.lock().await;
                    match q.pop_front() {
                        Some(job) => job,
                        None => {
                            drop(q);
                            state_ref.notify.notified().await;
                            continue;
                        }
                    }
                };

                let job_type: JobTypeString = (&job.job_type).into();

                info!("starting job...");

                let cancel_token = CancellationToken::new();
                {
                    let mut lock = state_ref.running.lock().await;
                    *lock = Some((job.clone(), cancel_token.clone()));
                }

                let metadata_clone = job.metadata.clone();

                let mut child = match job.execute().await {
                    Ok(child) => child,
                    Err(e) => {
                        error!("failed to start process: {e}");
                        {
                            let mut lock = state_ref.running.lock().await;
                            *lock = None;
                        }
                        continue;
                    }
                };

                tokio::select! {
                    result = child.wait() => {
                        match result {
                            Ok(status) => info!("task done: {status}"),
                            Err(e) => error!("wait error: {e}"),
                        }
                    }
                    _ = cancel_token.cancelled() => {
                        info!("cancel requested, killing child...");
                        let _ = child.kill().await;
                    }
                }

                {
                    let mut lock = state_ref.history.lock().await;
                    match lock.insert(metadata_clone, job_type) {
                        Ok(()) => info!("history updated"),
                        Err(e) => error!("failed to update history: {e}"),
                    };
                }

                {
                    let mut lock = state_ref.running.lock().await;
                    *lock = None;
                }

                if state_ref.clear_requested.load(Ordering::SeqCst) {
                    info!("clearing pending tasks...");
                    {
                        let mut q = state_ref.queue.lock().await;
                        q.clear();
                    }
                    state_ref.clear_requested.store(false, Ordering::SeqCst);
                }
            }
        });

        QueueManager { state }
    }

    pub async fn submit(&self, args: JobType, metadata: TrackInfo) -> usize {
        let id = self.state.job_id.fetch_add(1, Ordering::SeqCst);

        let job = Job {
            id,
            metadata,
            job_type: args,
        };
        {
            let mut q = self.state.queue.lock().await;
            q.push_back(job);
        }
        self.state.notify.notify_one();
        id
    }

    pub async fn cancel_by_id(&self, job_id: usize) -> bool {
        {
            let mut q = self.state.queue.lock().await;
            let index = q.iter().position(|job| job.id == job_id);

            if let Some(i) = index {
                q.remove(i).expect("index from VecDeque::position is valid");
                drop(q);

                info!("cancelled job {job_id} from queue");
                return true;
            }
        }

        {
            let lock = self.state.running.lock().await;
            if let Some((running_job, token)) = lock.as_ref()
                && running_job.id == job_id
            {
                token.cancel();
                info!("cancelled currently running job {job_id}");
                return true;
            }
        }

        info!("job {job_id} not found");
        false
    }

    pub async fn cancel(&self) -> bool {
        let mut lock = self.state.running.lock().await;
        if let Some((job, token)) = lock.take() {
            token.cancel();
            info!("cancelling current job {}", job.id);
            true
        } else {
            info!("nothing to cancel");
            false
        }
    }

    pub async fn clear(&self) {
        {
            let mut lock = self.state.running.lock().await;
            if let Some((job, token)) = lock.take() {
                token.cancel();
                info!("clear: cancelling current job {}", job.id);
            } else {
                info!("nothing to clear");
            }
        }

        let mut q = self.state.queue.lock().await;
        let drained_jobs: Vec<_> = q.drain(..).collect();
        drop(q);

        for job in drained_jobs {
            info!("cancelled job {}", job.id);
        }

        self.state.clear_requested.store(true, Ordering::SeqCst);
    }

    pub async fn inspect(&self) -> (Option<InspectMetadata>, Vec<InspectMetadata>) {
        let current = self
            .state
            .running
            .lock()
            .await
            .clone()
            .map(|(job, _)| InspectMetadata {
                job_id: job.id,
                current: true,
                track_info: job.metadata.clone(),
            });

        let mut curr_queue = vec![];
        let queue = self.state.queue.lock().await;
        for job in queue.iter() {
            curr_queue.push(InspectMetadata {
                job_id: job.id,
                current: false,
                track_info: job.metadata.clone(),
            });
        }

        (current, curr_queue)
    }

    pub async fn reorder_job(&self, job_id: usize, new_index: usize) -> anyhow::Result<()> {
        let mut q = self.state.queue.lock().await;

        let old_pos = q
            .iter()
            .position(|job| job.id == job_id)
            .ok_or_else(|| anyhow::anyhow!("job {job_id} not found in queue or already running"))?;

        if old_pos == new_index {
            return Ok(());
        }

        let mut items: Vec<Job> = q.drain(..).collect();
        let job = items.remove(old_pos);
        let target_index = new_index.min(items.len());
        items.insert(target_index, job);
        q.extend(items);

        info!("reordered job {job_id} from position {old_pos} to position {new_index}");
        Ok(())
    }

    pub async fn swap_with_running(&self, job_id: usize) -> anyhow::Result<()> {
        let mut q = self.state.queue.lock().await;

        let target_index = q
            .iter()
            .position(|job| job.id == job_id)
            .ok_or_else(|| anyhow::anyhow!("job {job_id} not found in queue"))?;

        let running_lock = self.state.running.lock().await;
        let (running_job, cancel_token) = running_lock
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("no job is currently running"))?
            .clone();

        if running_job.id == job_id {
            return Err(anyhow::anyhow!("cannot swap a job with itself"));
        }

        let mut items: Vec<Job> = q.drain(..).collect();
        let swapped_job = items.remove(target_index);
        items.insert(0, swapped_job);
        items.insert(target_index + 1, running_job.clone());
        q.extend(items);

        cancel_token.cancel();

        info!(
            "swapped running job {} with queued job {}",
            running_job.id, job_id
        );

        Ok(())
    }

    pub async fn get_history(&self) -> Vec<HistoryEntry> {
        let lock = self.state.history.lock().await;
        lock.get_history()
    }
    pub async fn remove_history_entry(&self, webpage_url: &str) -> anyhow::Result<()> {
        let mut lock = self.state.history.lock().await;
        lock.remove(webpage_url)?;
        Ok(())
    }
}
