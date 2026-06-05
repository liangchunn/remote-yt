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
                std::fs::write(&history_file, serde_json::to_string(&default_value)?)?;
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

    fn flush(&self) -> anyhow::Result<()> {
        std::fs::write(&self.history_file, serde_json::to_string(&self.contents)?)?;
        Ok(())
    }

    pub fn insert(&mut self, track_info: TrackInfo, job_type: JobTypeString) -> anyhow::Result<()> {
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

        self.flush()?;
        Ok(())
    }

    pub fn remove(&mut self, webpage_url: &str) -> anyhow::Result<()> {
        let index = self
            .contents
            .iter()
            .position(|content| content.track_info.webpage_url == webpage_url)
            .ok_or_else(|| anyhow::anyhow!("entry with webpage_url '{webpage_url}' not found"))?;
        self.contents.remove(index);
        self.flush()?;

        Ok(())
    }
}
