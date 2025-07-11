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
}

impl QueueManager {
    pub fn new() -> Self {
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

    pub async fn inspect(&self) -> Vec<InspectMetadata> {
        let mut result = vec![];

        if let Some((job, metadata)) = self.current.lock().await.clone() {
            result.push(InspectMetadata {
                job_id: job.id,
                current: true,
                track_info: metadata.clone(),
            })
        }
        let queue = self.queue.lock().await;
        for job in queue.iter() {
            result.push(InspectMetadata {
                job_id: job.id,
                current: false,
                track_info: job.metadata.clone(),
            });
        }

        result
    }

    pub async fn reorder_job(&self, job_id: usize, new_index: usize) -> anyhow::Result<()> {
        let mut q = self.queue.lock().await;

        if let Some(old_pos) = q.iter().position(|job| job.id == job_id) {
            if old_pos == new_index {
                return Ok(());
            }

            let job = q.remove(old_pos).unwrap();

            // After removing an element at old_pos:
            // - Elements before old_pos keep their indices
            // - Elements after old_pos shift left by 1

            let insert_pos = if new_index > old_pos {
                // Moving to higher index (toward back)
                // Account for the shift caused by removal
                (new_index - 1).min(q.len())
            } else {
                // Moving to lower index (toward front)
                // No adjustment needed
                new_index
            };

            q.insert(insert_pos, job);

            info!("reordered job {job_id} from position {old_pos} to position {new_index}");
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "job {job_id} not found in queue or already running"
            ))
        }
    }

    pub async fn swap_with_running(&self, job_id: usize) -> anyhow::Result<()> {
        // Lock queue
        let mut q = self.queue.lock().await;

        let target_index = q.iter().position(|job| job.id == job_id);
        if target_index.is_none() {
            return Err(anyhow::anyhow!("job {job_id} not found in queue"));
        }

        // Lock currently running job
        let running_lock = self.running.lock().await;
        if running_lock.is_none() {
            return Err(anyhow::anyhow!("no job is currently running"));
        }

        let (running_job, cancel_token) = running_lock.as_ref().unwrap().clone();

        if running_job.id == job_id {
            return Err(anyhow::anyhow!("cannot swap a job with itself"));
        }

        let target_index = target_index.unwrap();

        // First, replace the target job with the running job
        let swapped_job = std::mem::replace(&mut q[target_index], running_job.clone());

        // Then push the swapped job to the front
        q.push_front(swapped_job);

        // Trigger cancellation of the currently running job
        cancel_token.cancel();

        info!(
            "swapped running job {} with queued job {}",
            running_job.id, job_id
        );

        Ok(())
    }
}
