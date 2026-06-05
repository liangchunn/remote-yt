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
