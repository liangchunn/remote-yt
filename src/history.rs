use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use tokio::fs::{read_to_string, write};

use crate::yt_dlp::TrackInfo;

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
}

const MAX_HISTORY_LEN: usize = 20;

impl History {
    pub async fn new(history_file: PathBuf) -> anyhow::Result<Self> {
        let contents = match read_to_string(&history_file).await {
            Ok(str) => serde_json::from_str::<Vec<HistoryEntry>>(&str)?,
            Err(_) => {
                let default_value: Vec<HistoryEntry> = Default::default();
                write(&history_file, serde_json::to_string(&default_value)?).await?;
                default_value
            }
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
        write(&self.history_file, serde_json::to_string(&self.contents)?).await?;
        Ok(())
    }
    pub async fn insert(&mut self, track_info: TrackInfo) -> anyhow::Result<()> {
        let inserted_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let extra_info = ExtraInfo { inserted_at };
        let has_entry = self
            .contents
            .iter()
            .position(|content| content.track_info.webpage_url == track_info.webpage_url);

        // if there is a duplicate entry, we want to remove it
        // so that it gets pushed to the end
        if let Some(index) = has_entry {
            self.contents.remove(index);
        }

        self.contents.push(HistoryEntry {
            track_info,
            extra_info,
        });

        // truncate 20 items
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
            .ok_or_else(|| anyhow::anyhow!("entry with webpage_url '{webpage_url}' not found "))?;
        self.contents.remove(index);
        self.flush().await?;

        Ok(())
    }
}
