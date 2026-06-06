use std::{
    io,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

use crate::{job::JobTypeString, yt_dlp::TrackInfo};

pub struct History {
    history_file: PathBuf,
    contents: Vec<HistoryEntry>,
}

#[derive(Serialize, Deserialize, Clone)]
struct ExtraInfo {
    inserted_at: u64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct HistoryEntry {
    #[serde(flatten)]
    track_info: TrackInfo,
    #[serde(flatten)]
    extra_info: ExtraInfo,
    job_type: JobTypeString,
}

const MAX_HISTORY_LEN: usize = 100;

impl History {
    pub async fn new(history_file: PathBuf) -> anyhow::Result<Self> {
        let contents = match tokio::fs::read_to_string(&history_file).await {
            Ok(str) => serde_json::from_str::<Vec<HistoryEntry>>(&str)?,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                let default_value: Vec<HistoryEntry> = Default::default();
                tokio::fs::write(&history_file, serde_json::to_string(&default_value)?).await?;
                default_value
            }
            Err(e) => return Err(e.into()),
        };
        Ok(Self {
            history_file,
            contents,
        })
    }

    pub fn get_history(&self) -> Vec<HistoryEntry> {
        self.contents.clone()
    }

    async fn flush(&self) -> anyhow::Result<()> {
        tokio::fs::write(&self.history_file, serde_json::to_string(&self.contents)?).await?;
        Ok(())
    }

    pub async fn insert(
        &mut self,
        track_info: TrackInfo,
        job_type: JobTypeString,
    ) -> anyhow::Result<()> {
        let inserted_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let extra_info = ExtraInfo { inserted_at };
        let has_entry = self
            .contents
            .iter()
            .position(|content| content.track_info.webpage_url == track_info.webpage_url);

        if let Some(index) = has_entry {
            self.contents.remove(index);
        }

        self.contents.push(HistoryEntry {
            track_info,
            extra_info,
            job_type,
        });

        if self.contents.len() > MAX_HISTORY_LEN {
            self.contents = self
                .contents
                .split_off(self.contents.len().saturating_sub(MAX_HISTORY_LEN));
        }

        self.flush().await?;
        Ok(())
    }

    pub async fn remove(&mut self, webpage_url: &str) -> anyhow::Result<()> {
        let index = self
            .contents
            .iter()
            .position(|content| content.track_info.webpage_url == webpage_url)
            .ok_or_else(|| anyhow::anyhow!("entry with webpage_url '{webpage_url}' not found"))?;
        self.contents.remove(index);
        self.flush().await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::yt_dlp::TrackInfo;

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

    #[tokio::test]
    async fn new_creates_missing_history_file() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let history_file = temp_dir.path().join("history.json");

        let history = History::new(history_file.clone())
            .await
            .expect("create history");

        assert!(history_file.exists());
        assert!(history.get_history().is_empty());
        assert_eq!(
            tokio::fs::read_to_string(history_file)
                .await
                .expect("read history file"),
            "[]"
        );
    }

    #[tokio::test]
    async fn insert_persists_and_new_loads_existing_history() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let history_file = temp_dir.path().join("history.json");
        let mut history = History::new(history_file.clone())
            .await
            .expect("create history");

        history
            .insert(track_info(1), JobTypeString::Queue)
            .await
            .expect("insert history entry");

        let loaded = History::new(history_file).await.expect("load history");
        let entries = loaded.get_history();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].track_info.title, "title 1");
        assert_eq!(
            entries[0].track_info.webpage_url,
            "https://example.com/watch/1"
        );
    }

    #[tokio::test]
    async fn insert_replaces_duplicate_webpage_url_and_moves_it_to_end() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let history_file = temp_dir.path().join("history.json");
        let mut history = History::new(history_file).await.expect("create history");
        let mut duplicate = track_info(1);
        duplicate.title = "updated title".to_owned();

        history
            .insert(track_info(1), JobTypeString::Queue)
            .await
            .expect("insert first entry");
        history
            .insert(track_info(2), JobTypeString::QueueSplit)
            .await
            .expect("insert second entry");
        history
            .insert(duplicate, JobTypeString::QueueMerged)
            .await
            .expect("replace duplicate entry");

        let entries = history.get_history();
        assert_eq!(entries.len(), 2);
        assert_eq!(
            entries[0].track_info.webpage_url,
            "https://example.com/watch/2"
        );
        assert_eq!(
            entries[1].track_info.webpage_url,
            "https://example.com/watch/1"
        );
        assert_eq!(entries[1].track_info.title, "updated title");
    }

    #[tokio::test]
    async fn insert_keeps_only_most_recent_entries() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let history_file = temp_dir.path().join("history.json");
        let mut history = History::new(history_file).await.expect("create history");

        for index in 0..105 {
            history
                .insert(track_info(index), JobTypeString::Queue)
                .await
                .expect("insert history entry");
        }

        let entries = history.get_history();
        assert_eq!(entries.len(), 100);
        assert_eq!(
            entries[0].track_info.webpage_url,
            "https://example.com/watch/5"
        );
        assert_eq!(
            entries[99].track_info.webpage_url,
            "https://example.com/watch/104"
        );
    }

    #[tokio::test]
    async fn remove_deletes_entry_and_persists_change() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let history_file = temp_dir.path().join("history.json");
        let mut history = History::new(history_file.clone())
            .await
            .expect("create history");

        history
            .insert(track_info(1), JobTypeString::Queue)
            .await
            .expect("insert first entry");
        history
            .insert(track_info(2), JobTypeString::Queue)
            .await
            .expect("insert second entry");

        history
            .remove("https://example.com/watch/1")
            .await
            .expect("remove history entry");
        assert!(history.remove("missing").await.is_err());

        let loaded = History::new(history_file).await.expect("reload history");
        let entries = loaded.get_history();
        assert_eq!(entries.len(), 1);
        assert_eq!(
            entries[0].track_info.webpage_url,
            "https://example.com/watch/2"
        );
    }
}
