use std::{collections::VecDeque, future::Future, io, pin::Pin, process::ExitStatus, sync::Arc};

use tokio::{
    process::Child,
    sync::{Mutex, Notify},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

use crate::{
    history::{History, HistoryEntry},
    job::{Job, JobType, JobTypeString, StartedJob},
    meta::InspectMetadata,
    yt_dlp::TrackInfo,
};

struct QueueState {
    data: Mutex<QueueData>,
    notify: Notify,
    history: Mutex<History>,
    executor: Arc<dyn JobExecutor>,
}

type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

trait QueueChild: Send {
    fn wait(&mut self) -> BoxFuture<'_, io::Result<ExitStatus>>;
    fn kill(&mut self) -> BoxFuture<'_, io::Result<()>>;
}

trait JobExecutor: Send + Sync {
    fn execute(&self, job: Job) -> BoxFuture<'static, anyhow::Result<StartedQueueJob>>;
}

struct StartedQueueJob {
    child: Box<dyn QueueChild>,
    metadata: TrackInfo,
}

struct ProcessChild(Child);

impl QueueChild for ProcessChild {
    fn wait(&mut self) -> BoxFuture<'_, io::Result<ExitStatus>> {
        Box::pin(async move { self.0.wait().await })
    }

    fn kill(&mut self) -> BoxFuture<'_, io::Result<()>> {
        Box::pin(async move { self.0.kill().await })
    }
}

struct ProcessJobExecutor;

impl JobExecutor for ProcessJobExecutor {
    fn execute(&self, job: Job) -> BoxFuture<'static, anyhow::Result<StartedQueueJob>> {
        Box::pin(async move {
            let StartedJob { child, metadata } = job.execute().await?;
            Ok(StartedQueueJob {
                child: Box::new(ProcessChild(child)) as Box<dyn QueueChild>,
                metadata,
            })
        })
    }
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
        Self::new_with_executor(history, Arc::new(ProcessJobExecutor))
    }

    fn new_with_executor(history: History, executor: Arc<dyn JobExecutor>) -> Self {
        Self {
            state: Arc::new(QueueState {
                data: Mutex::new(QueueData {
                    pending: VecDeque::new(),
                    running: None,
                    next_job_id: 1,
                }),
                notify: Notify::new(),
                history: Mutex::new(history),
                executor,
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

            let job_id = job.id;

            let mut started = match state.executor.execute(job).await {
                Ok(started) => started,
                Err(e) => {
                    error!(job_id, "failed to start process: {e}");
                    Self::clear_running(&state, job_id).await;
                    continue;
                }
            };

            let metadata_clone = started.metadata.clone();
            Self::update_running_metadata(&state, job_id, started.metadata).await;

            tokio::select! {
                result = started.child.wait() => {
                    match result {
                        Ok(status) => info!(job_id, "task done: {status}"),
                        Err(e) => error!(job_id, "wait error: {e}"),
                    }
                }
                _ = cancel_token.cancelled() => {
                    info!(job_id, "cancel requested, killing child");
                    let _ = started.child.kill().await;
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

    async fn update_running_metadata(state: &QueueState, job_id: usize, metadata: TrackInfo) {
        let mut data = state.data.lock().await;
        if let Some(running) = data
            .running
            .as_mut()
            .filter(|running| running.job.id == job_id)
        {
            running.job.metadata = metadata;
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

    pub async fn submit_many(&self, jobs: Vec<(JobType, TrackInfo)>) -> Vec<usize> {
        if jobs.is_empty() {
            return Vec::new();
        }

        let ids = {
            let mut data = self.state.data.lock().await;
            let mut ids = Vec::with_capacity(jobs.len());

            for (job_type, metadata) in jobs {
                let id = data.next_job_id;
                data.next_job_id += 1;
                ids.push(id);
                data.pending.push_back(Job {
                    id,
                    metadata,
                    job_type,
                });
            }

            ids
        };

        self.state.notify.notify_one();
        ids
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

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, sync::Arc, time::Duration};

    use serde_json::json;
    use tempfile::TempDir;
    use tokio::{
        sync::{mpsc, oneshot},
        time::{sleep, timeout},
    };

    use super::*;
    use crate::{
        config::{Config, VlcRpcConfig},
        yt_dlp::TrackInfo,
    };

    struct ExecutedJob {
        job: Job,
        finish: oneshot::Sender<()>,
        killed: oneshot::Receiver<()>,
    }

    struct MockExecutor {
        tx: mpsc::UnboundedSender<ExecutedJob>,
    }

    struct PendingExecution {
        job: Job,
        allow_child: oneshot::Sender<()>,
    }

    struct PendingExecutor {
        pending_tx: mpsc::UnboundedSender<PendingExecution>,
        executed_tx: mpsc::UnboundedSender<ExecutedJob>,
    }

    impl JobExecutor for MockExecutor {
        fn execute(&self, job: Job) -> BoxFuture<'static, anyhow::Result<StartedQueueJob>> {
            let tx = self.tx.clone();
            Box::pin(async move {
                let metadata = job.metadata.clone();
                let (finish_tx, finish_rx) = oneshot::channel();
                let (killed_tx, killed_rx) = oneshot::channel();
                tx.send(ExecutedJob {
                    job,
                    finish: finish_tx,
                    killed: killed_rx,
                })?;

                Ok(StartedQueueJob {
                    child: Box::new(MockChild {
                        finish_rx: Some(finish_rx),
                        killed_tx: Some(killed_tx),
                    }) as Box<dyn QueueChild>,
                    metadata,
                })
            })
        }
    }

    impl JobExecutor for PendingExecutor {
        fn execute(&self, job: Job) -> BoxFuture<'static, anyhow::Result<StartedQueueJob>> {
            let pending_tx = self.pending_tx.clone();
            let executed_tx = self.executed_tx.clone();
            Box::pin(async move {
                let metadata = job.metadata.clone();
                let (allow_tx, allow_rx) = oneshot::channel();
                pending_tx.send(PendingExecution {
                    job: job.clone(),
                    allow_child: allow_tx,
                })?;
                allow_rx.await?;

                let (finish_tx, finish_rx) = oneshot::channel();
                let (killed_tx, killed_rx) = oneshot::channel();
                executed_tx.send(ExecutedJob {
                    job,
                    finish: finish_tx,
                    killed: killed_rx,
                })?;

                Ok(StartedQueueJob {
                    child: Box::new(MockChild {
                        finish_rx: Some(finish_rx),
                        killed_tx: Some(killed_tx),
                    }) as Box<dyn QueueChild>,
                    metadata,
                })
            })
        }
    }

    struct MockChild {
        finish_rx: Option<oneshot::Receiver<()>>,
        killed_tx: Option<oneshot::Sender<()>>,
    }

    impl QueueChild for MockChild {
        fn wait(&mut self) -> BoxFuture<'_, io::Result<ExitStatus>> {
            let finish_rx = self.finish_rx.take().expect("mock child waited once");
            Box::pin(async move {
                finish_rx
                    .await
                    .map_err(|_| io::Error::other("mock finish sender dropped"))?;
                Err(io::Error::other("mock child finished"))
            })
        }

        fn kill(&mut self) -> BoxFuture<'_, io::Result<()>> {
            let killed_tx = self.killed_tx.take();
            Box::pin(async move {
                if let Some(killed_tx) = killed_tx {
                    let _ = killed_tx.send(());
                }
                Ok(())
            })
        }
    }

    async fn mock_manager() -> (TempDir, QueueManager, mpsc::UnboundedReceiver<ExecutedJob>) {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let history = History::new(temp_dir.path().join("history.json"))
            .await
            .expect("create history");
        let (tx, rx) = mpsc::unbounded_channel();
        let manager = QueueManager::new_with_executor(history, Arc::new(MockExecutor { tx }));

        (temp_dir, manager, rx)
    }

    async fn pending_manager() -> (
        TempDir,
        QueueManager,
        mpsc::UnboundedReceiver<PendingExecution>,
        mpsc::UnboundedReceiver<ExecutedJob>,
    ) {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let history = History::new(temp_dir.path().join("history.json"))
            .await
            .expect("create history");
        let (pending_tx, pending_rx) = mpsc::unbounded_channel();
        let (executed_tx, executed_rx) = mpsc::unbounded_channel();
        let manager = QueueManager::new_with_executor(
            history,
            Arc::new(PendingExecutor {
                pending_tx,
                executed_tx,
            }),
        );

        (temp_dir, manager, pending_rx, executed_rx)
    }

    fn test_config() -> Arc<Config> {
        Arc::new(Config {
            vlc_path: "vlc".to_owned(),
            yt_dlp_path: "yt-dlp".to_owned(),
            vlc_rpc: VlcRpcConfig::default(),
            providers: HashMap::new(),
        })
    }

    fn job_type(index: usize) -> JobType {
        JobType::Queue {
            url: format!("https://example.com/{index}"),
            config: test_config(),
        }
    }

    fn track_info(index: usize) -> TrackInfo {
        serde_json::from_value(json!({
            "title": format!("title {index}"),
            "channel": "channel",
            "uploader_id": "uploader",
            "acodec": "aac",
            "vcodec": "h264",
            "height": 720,
            "width": 1280,
            "thumbnail": "https://example.com/thumb.jpg",
            "track_type": "merged",
            "format_id": "format",
            "duration": 60,
            "webpage_url": format!("https://example.com/watch/{index}"),
        }))
        .expect("valid track info")
    }

    fn job_ids(queue: &[InspectMetadata]) -> Vec<usize> {
        queue.iter().map(|job| job.job_id).collect()
    }

    async fn recv_executed(rx: &mut mpsc::UnboundedReceiver<ExecutedJob>) -> ExecutedJob {
        timeout(Duration::from_secs(1), rx.recv())
            .await
            .expect("worker executed job in time")
            .expect("mock executor channel open")
    }

    async fn recv_pending(rx: &mut mpsc::UnboundedReceiver<PendingExecution>) -> PendingExecution {
        timeout(Duration::from_secs(1), rx.recv())
            .await
            .expect("worker started execution in time")
            .expect("mock pending executor channel open")
    }

    async fn wait_for_history_len(manager: &QueueManager, len: usize) {
        timeout(Duration::from_secs(1), async {
            loop {
                if manager.get_history().await.len() == len {
                    break;
                }
                sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("history length reached in time");
    }

    async fn wait_for_no_current(manager: &QueueManager) {
        timeout(Duration::from_secs(1), async {
            loop {
                let (current, _) = manager.inspect().await;
                if current.is_none() {
                    break;
                }
                sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("current job cleared in time");
    }

    #[tokio::test]
    async fn submit_assigns_ids_and_inspects_pending_in_order() {
        let (_temp_dir, manager, _rx) = mock_manager().await;

        let first = manager.submit(job_type(1), track_info(1)).await;
        let second = manager.submit(job_type(2), track_info(2)).await;
        let third = manager.submit(job_type(3), track_info(3)).await;

        assert_eq!((first, second, third), (1, 2, 3));

        let (current, pending) = manager.inspect().await;
        assert!(current.is_none());
        assert_eq!(job_ids(&pending), vec![first, second, third]);
        assert_eq!(pending[0].track_info.title, "title 1");
        assert_eq!(
            pending[2].track_info.webpage_url,
            "https://example.com/watch/3"
        );
    }

    #[tokio::test]
    async fn submit_many_assigns_ids_and_appends_jobs_together() {
        let (_temp_dir, manager, _rx) = mock_manager().await;

        let first = manager.submit(job_type(1), track_info(1)).await;
        let batch = manager
            .submit_many(vec![
                (job_type(2), track_info(2)),
                (job_type(3), track_info(3)),
            ])
            .await;

        assert_eq!(first, 1);
        assert_eq!(batch, vec![2, 3]);

        let (current, pending) = manager.inspect().await;
        assert!(current.is_none());
        assert_eq!(job_ids(&pending), vec![1, 2, 3]);
    }

    #[tokio::test]
    async fn cancel_by_id_removes_pending_job() {
        let (_temp_dir, manager, _rx) = mock_manager().await;

        let first = manager.submit(job_type(1), track_info(1)).await;
        let second = manager.submit(job_type(2), track_info(2)).await;
        let third = manager.submit(job_type(3), track_info(3)).await;

        assert!(manager.cancel_by_id(second).await);
        assert!(!manager.cancel_by_id(999).await);

        let (current, pending) = manager.inspect().await;
        assert!(current.is_none());
        assert_eq!(job_ids(&pending), vec![first, third]);
    }

    #[tokio::test]
    async fn clear_cancels_running_job_and_removes_pending_jobs() {
        let (_temp_dir, manager, _rx) = mock_manager().await;

        let first = manager.submit(job_type(1), track_info(1)).await;
        manager.submit(job_type(2), track_info(2)).await;
        manager.submit(job_type(3), track_info(3)).await;

        let cancel_token = CancellationToken::new();
        {
            let mut data = manager.state.data.lock().await;
            let running_job = data.pending.pop_front().expect("pending job");
            assert_eq!(running_job.id, first);
            data.running = Some(RunningJob {
                job: running_job,
                cancel_token: cancel_token.clone(),
            });
        }

        manager.clear().await;

        assert!(cancel_token.is_cancelled());
        let (current, pending) = manager.inspect().await;
        assert!(current.is_none());
        assert!(pending.is_empty());
    }

    #[tokio::test]
    async fn cancel_cancels_running_job_only() {
        let (_temp_dir, manager, _rx) = mock_manager().await;

        let first = manager.submit(job_type(1), track_info(1)).await;
        let second = manager.submit(job_type(2), track_info(2)).await;

        let cancel_token = CancellationToken::new();
        {
            let mut data = manager.state.data.lock().await;
            let running_job = data.pending.pop_front().expect("pending job");
            assert_eq!(running_job.id, first);
            data.running = Some(RunningJob {
                job: running_job,
                cancel_token: cancel_token.clone(),
            });
        }

        assert!(manager.cancel().await);
        assert!(cancel_token.is_cancelled());
        assert!(!manager.cancel().await);

        let (current, pending) = manager.inspect().await;
        assert!(current.is_none());
        assert_eq!(job_ids(&pending), vec![second]);
    }

    #[tokio::test]
    async fn reorder_job_moves_pending_job_and_clamps_index() {
        let (_temp_dir, manager, _rx) = mock_manager().await;

        let first = manager.submit(job_type(1), track_info(1)).await;
        let second = manager.submit(job_type(2), track_info(2)).await;
        let third = manager.submit(job_type(3), track_info(3)).await;

        manager
            .reorder_job(third, 0)
            .await
            .expect("move third to front");
        let (_, pending) = manager.inspect().await;
        assert_eq!(job_ids(&pending), vec![third, first, second]);

        manager
            .reorder_job(first, 99)
            .await
            .expect("move first to clamped end");
        let (_, pending) = manager.inspect().await;
        assert_eq!(job_ids(&pending), vec![third, second, first]);

        assert!(manager.reorder_job(999, 0).await.is_err());
    }

    #[tokio::test]
    async fn swap_with_running_promotes_pending_job_and_cancels_running_job() {
        let (_temp_dir, manager, _rx) = mock_manager().await;

        let first = manager.submit(job_type(1), track_info(1)).await;
        let second = manager.submit(job_type(2), track_info(2)).await;
        let third = manager.submit(job_type(3), track_info(3)).await;

        let cancel_token = CancellationToken::new();
        {
            let mut data = manager.state.data.lock().await;
            let running_job = data.pending.pop_front().expect("pending job");
            assert_eq!(running_job.id, first);
            data.running = Some(RunningJob {
                job: running_job,
                cancel_token: cancel_token.clone(),
            });
        }

        manager
            .swap_with_running(third)
            .await
            .expect("swap pending with running");

        assert!(cancel_token.is_cancelled());
        let (current, pending) = manager.inspect().await;
        assert!(current.is_none());
        assert_eq!(job_ids(&pending), vec![third, second, first]);
    }

    #[tokio::test]
    async fn worker_executes_jobs_with_mock_executor_and_updates_history() {
        let (_temp_dir, manager, mut rx) = mock_manager().await;
        let handle = manager.start();

        let job_id = manager.submit(job_type(1), track_info(1)).await;
        let executed = recv_executed(&mut rx).await;
        assert_eq!(executed.job.id, job_id);

        let (current, pending) = manager.inspect().await;
        assert_eq!(current.expect("running job").job_id, job_id);
        assert!(pending.is_empty());

        executed.finish.send(()).expect("finish mock child");

        wait_for_history_len(&manager, 1).await;
        wait_for_no_current(&manager).await;
        handle.abort();
    }

    #[tokio::test]
    async fn cancel_by_id_kills_running_mock_child() {
        let (_temp_dir, manager, mut rx) = mock_manager().await;
        let handle = manager.start();

        let job_id = manager.submit(job_type(1), track_info(1)).await;
        let executed = recv_executed(&mut rx).await;
        assert_eq!(executed.job.id, job_id);

        assert!(manager.cancel_by_id(job_id).await);
        timeout(Duration::from_secs(1), executed.killed)
            .await
            .expect("mock child killed in time")
            .expect("mock kill sender used");

        wait_for_history_len(&manager, 1).await;
        wait_for_no_current(&manager).await;
        handle.abort();
    }

    #[tokio::test]
    async fn clear_multiple_videos_while_execute_is_pending_cancels_running_and_drains_queue() {
        let (_temp_dir, manager, mut pending_rx, mut executed_rx) = pending_manager().await;
        let handle = manager.start();

        let first = manager.submit(job_type(1), track_info(1)).await;
        let second = manager.submit(job_type(2), track_info(2)).await;
        let third = manager.submit(job_type(3), track_info(3)).await;

        let pending_first = recv_pending(&mut pending_rx).await;
        assert_eq!(pending_first.job.id, first);

        let (current, pending) = manager.inspect().await;
        assert_eq!(current.expect("running job").job_id, first);
        assert_eq!(job_ids(&pending), vec![second, third]);

        manager.clear().await;

        let (current, pending) = manager.inspect().await;
        assert!(current.is_none());
        assert!(pending.is_empty());

        pending_first
            .allow_child
            .send(())
            .expect("allow first child creation");
        let executed_first = recv_executed(&mut executed_rx).await;
        assert_eq!(executed_first.job.id, first);

        timeout(Duration::from_secs(1), executed_first.killed)
            .await
            .expect("mock child killed in time")
            .expect("mock kill sender used");

        wait_for_history_len(&manager, 1).await;
        assert!(
            timeout(Duration::from_millis(50), pending_rx.recv())
                .await
                .is_err(),
            "cleared pending jobs should not start after pending execution completes"
        );

        handle.abort();
    }

    #[tokio::test]
    async fn cancel_running_job_while_execute_is_pending_preserves_remaining_queue() {
        let (_temp_dir, manager, mut pending_rx, mut executed_rx) = pending_manager().await;
        let handle = manager.start();

        let first = manager.submit(job_type(1), track_info(1)).await;
        let second = manager.submit(job_type(2), track_info(2)).await;
        let third = manager.submit(job_type(3), track_info(3)).await;

        let pending_first = recv_pending(&mut pending_rx).await;
        assert_eq!(pending_first.job.id, first);

        assert!(manager.cancel_by_id(first).await);

        let (current, pending) = manager.inspect().await;
        assert!(current.is_none());
        assert_eq!(job_ids(&pending), vec![second, third]);

        pending_first
            .allow_child
            .send(())
            .expect("allow first child creation");
        let executed_first = recv_executed(&mut executed_rx).await;
        assert_eq!(executed_first.job.id, first);

        timeout(Duration::from_secs(1), executed_first.killed)
            .await
            .expect("mock child killed in time")
            .expect("mock kill sender used");

        let pending_second = recv_pending(&mut pending_rx).await;
        assert_eq!(pending_second.job.id, second);

        let (current, pending) = manager.inspect().await;
        assert_eq!(current.expect("running job").job_id, second);
        assert_eq!(job_ids(&pending), vec![third]);

        pending_second
            .allow_child
            .send(())
            .expect("allow second child creation");
        let executed_second = recv_executed(&mut executed_rx).await;
        executed_second
            .finish
            .send(())
            .expect("finish second mock child");

        wait_for_history_len(&manager, 2).await;
        handle.abort();
    }
}
