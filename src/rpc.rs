use std::collections::HashMap;

use reqwest::Client;
use serde::{Deserialize, Serialize};

pub struct Rpc {
    url: String,
    password: String,
    client: Client,
}

#[derive(Deserialize, Serialize)]
pub enum State {
    #[serde(rename = "playing")]
    Playing,
    #[serde(rename = "paused")]
    Paused,
}

#[derive(Deserialize, Serialize)]
pub struct RpcResponse {
    state: State,
    time: u32,
    length: u32,
    volume: u16,
}

#[derive(Deserialize)]
pub enum RpcCommand {
    SeekForward,
    SeekRewind,
    SeekTo(u32),
    TogglePause,
    Mute,
    FullVolume,
    SetVolume(u8),
}

impl RpcCommand {
    fn to_query_string(&self) -> anyhow::Result<String> {
        let mut map: HashMap<&'static str, String> = HashMap::new();
        match self {
            RpcCommand::SeekForward => {
                map.insert("command", "seek".into());
                map.insert("val", "+10".into());
            }
            RpcCommand::SeekRewind => {
                map.insert("command", "seek".into());
                map.insert("val", "-10".into());
            }
            RpcCommand::SeekTo(ts) => {
                map.insert("command", "seek".into());
                map.insert("val", ts.to_string());
            }
            RpcCommand::TogglePause => {
                map.insert("command", "pl_pause".into());
            }
            RpcCommand::Mute => {
                map.insert("command", "volume".into());
                map.insert("val", "0".to_string());
            }
            RpcCommand::FullVolume => {
                map.insert("command", "volume".into());
                map.insert("val", "255".to_string());
            }
            RpcCommand::SetVolume(percent) => {
                let percent = (*percent).min(100) as u16;
                let vlc_volume = (percent * 255 + 50) / 100;
                map.insert("command", "volume".into());
                map.insert("val", vlc_volume.to_string());
            }
        };

        Ok(serde_urlencoded::to_string(map)?)
    }
}

// https://github.com/videolan/vlc/tree/master/share/lua/http/requests
impl Rpc {
    pub fn new(host: impl Into<String>, port: u16, password: impl Into<String>) -> Self {
        let host = host.into();
        let url = format!("http://{host}:{port}/requests/status.json");
        Self {
            url,
            password: password.into(),
            client: Client::new(),
        }
    }

    pub async fn get_status(&self) -> anyhow::Result<RpcResponse> {
        let response = self
            .client
            .get(&self.url)
            .basic_auth("", Some(&self.password))
            .send()
            .await?;
        let json = response.json::<RpcResponse>().await?;
        Ok(json)
    }

    pub async fn execute_command(&self, command: RpcCommand) -> anyhow::Result<()> {
        self.client
            .get(format!("{}?{}", self.url, command.to_query_string()?))
            .basic_auth("", Some(&self.password))
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn query(command: RpcCommand) -> HashMap<String, String> {
        serde_urlencoded::from_str(&command.to_query_string().expect("query string"))
            .expect("decode query string")
    }

    #[test]
    fn seek_commands_encode_expected_queries() {
        let forward = query(RpcCommand::SeekForward);
        assert_eq!(forward.get("command").map(String::as_str), Some("seek"));
        assert_eq!(forward.get("val").map(String::as_str), Some("+10"));

        let rewind = query(RpcCommand::SeekRewind);
        assert_eq!(rewind.get("command").map(String::as_str), Some("seek"));
        assert_eq!(rewind.get("val").map(String::as_str), Some("-10"));

        let seek_to = query(RpcCommand::SeekTo(42));
        assert_eq!(seek_to.get("command").map(String::as_str), Some("seek"));
        assert_eq!(seek_to.get("val").map(String::as_str), Some("42"));
    }

    #[test]
    fn playback_and_fixed_volume_commands_encode_expected_queries() {
        let pause = query(RpcCommand::TogglePause);
        assert_eq!(pause.get("command").map(String::as_str), Some("pl_pause"));
        assert!(!pause.contains_key("val"));

        let mute = query(RpcCommand::Mute);
        assert_eq!(mute.get("command").map(String::as_str), Some("volume"));
        assert_eq!(mute.get("val").map(String::as_str), Some("0"));

        let full_volume = query(RpcCommand::FullVolume);
        assert_eq!(
            full_volume.get("command").map(String::as_str),
            Some("volume")
        );
        assert_eq!(full_volume.get("val").map(String::as_str), Some("255"));
    }

    #[test]
    fn set_volume_clamps_percent_and_converts_to_vlc_scale() {
        let zero = query(RpcCommand::SetVolume(0));
        assert_eq!(zero.get("val").map(String::as_str), Some("0"));

        let half = query(RpcCommand::SetVolume(50));
        assert_eq!(half.get("command").map(String::as_str), Some("volume"));
        assert_eq!(half.get("val").map(String::as_str), Some("128"));

        let full = query(RpcCommand::SetVolume(100));
        assert_eq!(full.get("val").map(String::as_str), Some("255"));

        let clamped = query(RpcCommand::SetVolume(255));
        assert_eq!(clamped.get("val").map(String::as_str), Some("255"));
    }
}
