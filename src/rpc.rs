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
}

impl RpcCommand {
    fn to_query_string(&self) -> String {
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
        };

        return serde_urlencoded::to_string(map).unwrap();
    }
}

impl Rpc {
    pub fn new(host: String, port: u16, password: String) -> Self {
        let url = format!("http://{host}:{port}/requests/status.json");
        Self {
            url,
            password,
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

    pub async fn execute_command(&self, command: RpcCommand) -> anyhow::Result<RpcResponse> {
        let response = self
            .client
            .get(format!("{}?{}", self.url, command.to_query_string()))
            .basic_auth("", Some(&self.password))
            .send()
            .await?;
        let json = response.json::<RpcResponse>().await?;
        Ok(json)
    }
}
