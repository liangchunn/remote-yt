use std::{io, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::yt_dlp::{PlaylistVideo, YouTubePlaylist};

pub struct PlaylistStore {
    playlist_file: PathBuf,
    contents: Vec<PlaylistEntry>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PlaylistEntry {
    pub id: String,
    pub title: String,
    pub webpage_url: String,
    pub thumbnail: String,
    pub video_count: usize,
    pub total_duration: u32,
    pub videos: Vec<PlaylistVideo>,
}

impl From<YouTubePlaylist> for PlaylistEntry {
    fn from(value: YouTubePlaylist) -> Self {
        Self {
            id: value.id,
            title: value.title,
            webpage_url: value.webpage_url,
            thumbnail: value.thumbnail,
            video_count: value.videos.len(),
            total_duration: value.total_duration,
            videos: value.videos,
        }
    }
}

impl PlaylistStore {
    pub async fn new(playlist_file: PathBuf) -> anyhow::Result<Self> {
        let contents = match tokio::fs::read_to_string(&playlist_file).await {
            Ok(str) => serde_json::from_str::<Vec<PlaylistEntry>>(&str)?,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                let default_value: Vec<PlaylistEntry> = Default::default();
                tokio::fs::write(&playlist_file, serde_json::to_string(&default_value)?).await?;
                default_value
            }
            Err(e) => return Err(e.into()),
        };

        Ok(Self {
            playlist_file,
            contents,
        })
    }

    pub fn list(&self) -> Vec<PlaylistEntry> {
        self.contents.clone()
    }

    pub fn get(&self, webpage_url: &str) -> Option<PlaylistEntry> {
        self.contents
            .iter()
            .find(|entry| entry.webpage_url == webpage_url)
            .cloned()
    }

    async fn flush(&self) -> anyhow::Result<()> {
        tokio::fs::write(&self.playlist_file, serde_json::to_string(&self.contents)?).await?;
        Ok(())
    }

    pub async fn upsert(&mut self, entry: PlaylistEntry) -> anyhow::Result<()> {
        if let Some(index) = self
            .contents
            .iter()
            .position(|content| content.id == entry.id)
        {
            self.contents.remove(index);
        }

        self.contents.push(entry);
        self.flush().await?;
        Ok(())
    }

    pub async fn remove(&mut self, webpage_url: &str) -> anyhow::Result<()> {
        let index = self
            .contents
            .iter()
            .position(|content| content.webpage_url == webpage_url)
            .ok_or_else(|| anyhow::anyhow!("playlist with url '{webpage_url}' not found"))?;

        self.contents.remove(index);
        self.flush().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn playlist_entry(index: usize) -> PlaylistEntry {
        let videos = vec![playlist_video(index, 1, 60), playlist_video(index, 2, 120)];

        PlaylistEntry {
            id: format!("playlist-{index}"),
            title: format!("Playlist {index}"),
            webpage_url: format!("https://www.youtube.com/playlist?list={index}"),
            thumbnail: "https://example.com/thumb.jpg".to_owned(),
            video_count: videos.len(),
            total_duration: 180,
            videos,
        }
    }

    fn playlist_video(playlist_index: usize, video_index: usize, duration: u32) -> PlaylistVideo {
        let url = format!("https://www.youtube.com/watch?v={playlist_index}-{video_index}");
        PlaylistVideo {
            url: url.clone(),
            track_info: serde_json::from_value(serde_json::json!({
                "title": format!("title {playlist_index}-{video_index}"),
                "channel": "channel",
                "uploader_id": "uploader",
                "acodec": "",
                "vcodec": "",
                "height": null,
                "width": null,
                "thumbnail": "https://example.com/thumb.jpg",
                "track_type": "split",
                "format_id": "",
                "duration": duration,
                "webpage_url": url,
            }))
            .expect("valid track info"),
        }
    }

    #[tokio::test]
    async fn new_creates_missing_playlist_file() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let playlist_file = temp_dir.path().join("playlists.json");

        let store = PlaylistStore::new(playlist_file.clone())
            .await
            .expect("create playlist store");

        assert!(playlist_file.exists());
        assert!(store.list().is_empty());
        assert_eq!(
            tokio::fs::read_to_string(playlist_file)
                .await
                .expect("read playlist file"),
            "[]"
        );
    }

    #[tokio::test]
    async fn upsert_persists_and_replaces_duplicate_id() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let playlist_file = temp_dir.path().join("playlists.json");
        let mut store = PlaylistStore::new(playlist_file.clone())
            .await
            .expect("create playlist store");

        store
            .upsert(playlist_entry(1))
            .await
            .expect("insert playlist");
        let mut updated = playlist_entry(1);
        updated.title = "Updated".to_owned();
        store.upsert(updated).await.expect("replace playlist");

        let loaded = PlaylistStore::new(playlist_file)
            .await
            .expect("load playlist store");
        let entries = loaded.list();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].title, "Updated");
    }

    #[tokio::test]
    async fn remove_deletes_entry_and_persists_change() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let playlist_file = temp_dir.path().join("playlists.json");
        let mut store = PlaylistStore::new(playlist_file.clone())
            .await
            .expect("create playlist store");

        let first = playlist_entry(1);
        store.upsert(first.clone()).await.expect("insert first");
        store
            .upsert(playlist_entry(2))
            .await
            .expect("insert second");

        store
            .remove(&first.webpage_url)
            .await
            .expect("remove playlist");
        assert!(store.remove("missing").await.is_err());

        let loaded = PlaylistStore::new(playlist_file)
            .await
            .expect("load playlist store");
        let entries = loaded.list();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, "playlist-2");
    }
}
