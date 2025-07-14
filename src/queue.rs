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
    job::{Job, JobType},
    meta::InspectMetadata,
    yt_dlp::TrackInfo,
};

pub struct QueueManager {
    queue: Arc<Mutex<VecDeque<Job>>>,
    notify: Arc<Notify>,
    running: Arc<Mutex<Option<(Job, CancellationToken)>>>,
    current: Arc<Mutex<Option<(Job, TrackInfo)>>>,
    clear_requested: Arc<AtomicBool>,
    job_id: Arc<AtomicUsize>,
    history: Arc<Mutex<History>>,
}

impl QueueManager {
    pub fn new(history: History) -> Self {
        let notify = Arc::new(Notify::new());
        let notify_ref = notify.clone();

        let queue = Arc::new(Mutex::new(VecDeque::<Job>::new()));
        let queue_ref = queue.clone();

        let running = Arc::new(Mutex::new(None));
        let running_ref = running.clone();

        let clear_requested = Arc::new(AtomicBool::new(false));
        let clear_ref = clear_requested.clone();

        let current = Arc::new(Mutex::new(None));
        let current_ref = current.clone();

        let history = Arc::new(Mutex::new(history));
        let history_ref = history.clone();

        let job_id = Arc::new(AtomicUsize::new(1));

        tokio::spawn(async move {
            loop {
                let job = {
                    let mut q = queue_ref.lock().await;
                    match q.pop_front() {
                        Some(job) => job,
                        None => {
                            drop(q);
                            notify_ref.notified().await;
                            continue;
                        }
                    }
                };

                info!("starting job...");

                let cancel_token = CancellationToken::new();
                {
                    let mut lock = running_ref.lock().await;
                    *lock = Some((job.clone(), cancel_token.clone()));
                }
                {
                    let mut current_lock = current_ref.lock().await;
                    *current_lock = Some((job.clone(), job.metadata.clone()));
                }

                let metadata_clone = job.metadata.clone();

                let mut child = match job.execute().await {
                    Ok(child) => child,
                    Err(e) => {
                        error!("failed to start process: {e}");
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
                    let mut lock = history_ref.lock().await;
                    match lock.insert(metadata_clone).await {
                        Ok(()) => info!("history updated"),
                        Err(e) => error!("failed to update history: {e}"),
                    };
                }

                {
                    let mut lock = running_ref.lock().await;
                    *lock = None;
                }
                {
                    let mut current_lock = current_ref.lock().await;
                    *current_lock = None;
                }

                if clear_ref.load(Ordering::SeqCst) {
                    info!("clearing pending tasks...");
                    {
                        let mut q = queue_ref.lock().await;
                        q.clear();
                    }
                    clear_ref.store(false, Ordering::SeqCst);
                }
            }
        });

        QueueManager {
            queue,
            notify,
            running,
            current,
            clear_requested,
            job_id,
            history,
        }
    }

    pub async fn submit(&self, args: JobType, metadata: TrackInfo) -> usize {
        let id = self.job_id.fetch_add(1, Ordering::SeqCst);

        let job = Job {
            id,
            metadata,
            job_type: args,
        };
        {
            let mut q = self.queue.lock().await;
            q.push_back(job);
        }
        self.notify.notify_one();
        id
    }

    pub async fn cancel_by_id(&self, job_id: usize) -> bool {
        // First try to remove from queue
        {
            let mut q = self.queue.lock().await;
            let index = q.iter().position(|job| job.id == job_id);

            if let Some(i) = index {
                q.remove(i).unwrap();
                drop(q); // Release the lock early before running async cleanup

                info!("cancelled job {job_id} from queue");
                return true;
            }
        }

        // Then try to cancel running task
        {
            let lock = self.running.lock().await;
            if let Some((running_job, token)) = lock.as_ref() {
                if running_job.id == job_id {
                    token.cancel();
                    info!("cancelled currently running job {job_id}");
                    return true;
                }
            }
        }

        info!("job {job_id} not found");
        false
    }

    pub async fn cancel(&self) -> bool {
        let mut lock = self.running.lock().await;
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
            let mut lock = self.running.lock().await;
            if let Some((job, token)) = lock.take() {
                token.cancel();
                info!("clear: cancelling current job {}", job.id);
            } else {
                info!("nothing to clear");
            }
        }

        // Drain queue and run cleanup for each
        let mut q = self.queue.lock().await;
        let drained_jobs: Vec<_> = q.drain(..).collect();
        drop(q);

        for job in drained_jobs {
            info!("cancelled job {}", job.id);
        }

        self.clear_requested.store(true, Ordering::SeqCst);
    }

    pub async fn inspect(&self) -> (Option<InspectMetadata>, Vec<InspectMetadata>) {
        let current = self
            .current
            .lock()
            .await
            .clone()
            .map(|(job, metadata)| InspectMetadata {
                job_id: job.id,
                current: true,
                track_info: metadata.clone(),
            });

        let mut curr_queue = vec![];
        let queue = self.queue.lock().await;
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
        let mut q = self.queue.lock().await;

        // Find the job in the queue
        let old_pos = q
            .iter()
            .position(|job| job.id == job_id)
            .ok_or_else(|| anyhow::anyhow!("job {job_id} not found in queue or already running"))?;

        if old_pos == new_index {
            return Ok(());
        }

        // Convert to Vec for predictable behavior
        let mut items: Vec<Job> = q.drain(..).collect();

        // Remove the job from its current position
        let job = items.remove(old_pos);

        // Insert at the new position (clamped to valid range)
        let target_index = new_index.min(items.len());
        items.insert(target_index, job);

        // Convert back to VecDeque
        q.extend(items);

        info!("reordered job {job_id} from position {old_pos} to position {new_index}");
        Ok(())
    }

    pub async fn swap_with_running(&self, job_id: usize) -> anyhow::Result<()> {
        // Lock queue
        let mut q = self.queue.lock().await;

        let target_index = q
            .iter()
            .position(|job| job.id == job_id)
            .ok_or_else(|| anyhow::anyhow!("job {job_id} not found in queue"))?;

        // Lock currently running job
        let running_lock = self.running.lock().await;
        let (running_job, cancel_token) = running_lock
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("no job is currently running"))?
            .clone();

        if running_job.id == job_id {
            return Err(anyhow::anyhow!("cannot swap a job with itself"));
        }

        // Convert to Vec for manipulation
        let mut items: Vec<Job> = q.drain(..).collect();

        // Remove the target job and insert it at the front
        let swapped_job = items.remove(target_index);
        items.insert(0, swapped_job);

        // Now insert the running job where the target job was
        // Since we removed one item and added it at the front,
        // the original index is now at target_index
        items.insert(target_index + 1, running_job.clone());

        // Convert back to VecDeque
        q.extend(items);

        // Trigger cancellation of the currently running job
        cancel_token.cancel();

        info!(
            "swapped running job {} with queued job {}",
            running_job.id, job_id
        );

        Ok(())
    }

    pub async fn get_history(&self) -> Vec<HistoryEntry> {
        let lock = self.history.lock().await;
        lock.get_history()
    }
    pub async fn remove_history_entry(&self, webpage_url: &str) -> anyhow::Result<()> {
        let mut lock = self.history.lock().await;
        lock.remove(webpage_url).await?;
        Ok(())
    }
}
