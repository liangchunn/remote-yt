// use std::{
//     process::Stdio,
//     sync::{
//         Arc,
//         atomic::{AtomicBool, Ordering},
//     },
// };

// use log::{error, info};
// use tokio::{
//     process::Command,
//     sync::{Mutex, mpsc},
// };
// use tokio_util::sync::CancellationToken;

// #[derive(Debug)]
// enum QueueMessage {
//     Command(String),
// }

// pub struct QueueManager {
//     sender: mpsc::Sender<QueueMessage>,
//     running: Arc<Mutex<Option<CancellationToken>>>,
//     clear_requested: Arc<AtomicBool>,
// }

// impl QueueManager {
//     pub fn new() -> Self {
//         let (tx, mut rx) = mpsc::channel::<QueueMessage>(100);
//         let running = Arc::new(Mutex::new(None));
//         let running_ref = running.clone();
//         let clear_requested = Arc::new(AtomicBool::new(false));
//         let clear_ref = clear_requested.clone();

//         tokio::spawn(async move {
//             while let Some(message) = rx.recv().await {
//                 match message {
//                     QueueMessage::Command(command) => {
//                         info!("running command: {}", command);

//                         let cancel_token = CancellationToken::new();
//                         {
//                             let mut lock = running_ref.lock().await;
//                             *lock = Some(cancel_token.clone())
//                         }

//                         let child_fut = async move {
//                             let mut child = match Command::new("sleep")
//                                 .arg(command)
//                                 .stdout(Stdio::inherit())
//                                 .stderr(Stdio::inherit())
//                                 .spawn()
//                             {
//                                 Ok(c) => c,
//                                 Err(e) => {
//                                     error!("failed to start process: {}", e);
//                                     return;
//                                 }
//                             };

//                             tokio::select! {
//                                 result = child.wait() => {
//                                     match result {
//                                         Ok(status) => info!("task done: {}", status),
//                                         Err(e) => error!("[Queue] Wait error: {}", e),
//                                     }
//                                 }
//                                 _ = cancel_token.cancelled() => {
//                                     info!("cancel requested, killing child...");
//                                     let _ = child.kill().await;
//                                 }
//                             }
//                         };

//                         child_fut.await;

//                         let mut lock = running_ref.lock().await;
//                         *lock = None;

//                         if clear_ref.load(Ordering::SeqCst) {
//                             info!("clearing pending tasks...");
//                             while let Ok(_) = rx.try_recv() {
//                                 continue;
//                             }
//                             clear_ref.store(false, Ordering::SeqCst)
//                         }
//                     }
//                 }
//             }
//         });

//         QueueManager {
//             sender: tx,
//             running,
//             clear_requested,
//         }
//     }

//     pub async fn submit(&self, command: String) {
//         if let Err(e) = self.sender.send(QueueMessage::Command(command)).await {
//             error!("failed to submit command: {}", e)
//         }
//     }

//     pub async fn cancel(&self) -> bool {
//         let mut lock = self.running.lock().await;
//         if let Some(token) = lock.take() {
//             token.cancel();
//             info!("cancelling current task");
//             true
//         } else {
//             info!("nothing to cancel");
//             false
//         }
//     }

//     pub async fn clear(&self) {
//         {
//             let mut lock = self.running.lock().await;
//             if let Some(token) = lock.take() {
//                 token.cancel();
//                 info!("clear: cancelling current task")
//             } else {
//                 info!("nothing to clear")
//             }
//         }
//         self.clear_requested.store(true, Ordering::SeqCst)
//     }
// }

use std::{
    collections::VecDeque,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use futures::future::BoxFuture;
use log::{error, info};
use tokio::{
    process::Child,
    sync::{Mutex, Notify},
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::meta::{InspectMetadata, Metadata};

type BoxedCommand = Box<dyn FnOnce() -> BoxFuture<'static, Result<Child, anyhow::Error>> + Send>;
type CleanupFn = Box<dyn FnOnce() -> BoxFuture<'static, ()> + Send>;

struct Job {
    pub id: Uuid,
    pub metadata: Metadata,
    pub task: BoxedCommand,
    pub cleanup: CleanupFn,
}

pub struct QueueManager {
    queue: Arc<Mutex<VecDeque<Job>>>,
    notify: Arc<Notify>,
    running: Arc<Mutex<Option<(Uuid, CancellationToken)>>>,
    current: Arc<Mutex<Option<(Uuid, Metadata)>>>,
    clear_requested: Arc<AtomicBool>,
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

                info!("starting job {}", job.id);

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
        }
    }

    pub async fn submit<F, Fut, C, CFut>(&self, f: F, metadata: Metadata, cleanup: C) -> Uuid
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = Result<Child, anyhow::Error>> + Send + 'static,
        C: FnOnce() -> CFut + Send + 'static,
        CFut: Future<Output = ()> + Send + 'static,
    {
        let task: BoxedCommand = Box::new(|| Box::pin(f()));
        let cleanup: CleanupFn = Box::new(|| Box::pin(cleanup()));

        let job = Job {
            id: Uuid::new_v4(),
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

    pub async fn cancel_by_id(&self, job_id: Uuid) -> bool {
        // First try to remove from queue
        {
            let mut q = self.queue.lock().await;
            let mut index = None;
            for (i, job) in q.iter().enumerate() {
                if job.id == job_id {
                    index = Some(i);
                    break;
                }
            }

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
                title: metadata.title.clone(),
                url: metadata.url.clone(),
                channel: metadata.channel.clone(),
                uploader_id: metadata.uploader_id.clone(),
            })
        }
        let queue = self.queue.lock().await;
        for job in queue.iter() {
            result.push(InspectMetadata {
                job_id: job.id,
                current: false,
                title: job.metadata.title.clone(),
                url: job.metadata.url.clone(),
                channel: job.metadata.channel.clone(),
                uploader_id: job.metadata.uploader_id.clone(),
            });
        }

        result
    }
}
