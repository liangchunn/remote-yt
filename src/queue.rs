use std::{
    collections::VecDeque,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
};

use futures::future::BoxFuture;
use tokio::{
    process::Child,
    sync::{Mutex, Notify},
};
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

use crate::{meta::InspectMetadata, yt_dlp::TrackInfo};

type BoxedCommand = Box<dyn FnOnce() -> BoxFuture<'static, Result<Child, anyhow::Error>> + Send>;
type CleanupFn = Box<dyn FnOnce() -> BoxFuture<'static, ()> + Send>;

struct Job {
    pub id: usize,
    pub metadata: TrackInfo,
    pub task: BoxedCommand,
    pub cleanup: CleanupFn,
}

pub struct QueueManager {
    queue: Arc<Mutex<VecDeque<Job>>>,
    notify: Arc<Notify>,
    running: Arc<Mutex<Option<(usize, CancellationToken)>>>,
    current: Arc<Mutex<Option<(usize, TrackInfo)>>>,
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
                    *lock = Some((job.id, cancel_token.clone()));
                }

                {
                    let mut current_lock = current_ref.lock().await;
                    *current_lock = Some((job.id, job.metadata));
                }

                let mut child = match (job.task)().await {
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

                info!("running cleanup for job {}", job.id);
                (job.cleanup)().await;

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

    pub async fn submit<F, Fut, C, CFut>(&self, f: F, metadata: TrackInfo, cleanup: C) -> usize
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = Result<Child, anyhow::Error>> + Send + 'static,
        C: FnOnce() -> CFut + Send + 'static,
        CFut: Future<Output = ()> + Send + 'static,
    {
        let task: BoxedCommand = Box::new(|| Box::pin(f()));
        let cleanup: CleanupFn = Box::new(|| Box::pin(cleanup()));
        let id = self.job_id.fetch_add(1, Ordering::SeqCst);

        let job = Job {
            id,
            metadata,
            task,
            cleanup,
        };
        let job_id = job.id;
        {
            let mut q = self.queue.lock().await;
            q.push_back(job);
        }
        self.notify.notify_one();
        job_id
    }

    pub async fn cancel_by_id(&self, job_id: usize) -> bool {
        // First try to remove from queue
        {
            let mut q = self.queue.lock().await;
            let index = q.iter().position(|job| job.id == job_id);

            if let Some(i) = index {
                let job = q.remove(i).unwrap();
                drop(q); // Release the lock early before running async cleanup

                info!("running cleanup for cancelled job {}", job.id);
                (job.cleanup)().await;

                info!("cancelled job {job_id} from queue");
                return true;
            }
        }

        // Then try to cancel running task
        {
            let lock = self.running.lock().await;
            if let Some((running_id, token)) = lock.as_ref() {
                if *running_id == job_id {
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
        if let Some((job_id, token)) = lock.take() {
            token.cancel();
            info!("cancelling current job {job_id}");
            true
        } else {
            info!("nothing to cancel");
            false
        }
    }

    pub async fn clear(&self) {
        {
            let mut lock = self.running.lock().await;
            if let Some((job_id, token)) = lock.take() {
                token.cancel();
                info!("clear: cancelling current job {job_id}");
            } else {
                info!("nothing to clear");
            }
        }

        // Drain queue and run cleanup for each
        let mut q = self.queue.lock().await;
        let drained_jobs: Vec<_> = q.drain(..).collect();
        drop(q);

        for job in drained_jobs {
            info!("running cleanup for cancelled job {}", job.id);
            (job.cleanup)().await;
        }

        self.clear_requested.store(true, Ordering::SeqCst);
    }

    pub async fn inspect(&self) -> Vec<InspectMetadata> {
        let mut result = vec![];

        if let Some((job_id, metadata)) = self.current.lock().await.clone() {
            result.push(InspectMetadata {
                job_id,
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
}
