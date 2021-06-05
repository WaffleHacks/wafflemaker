use anyhow::Result;
use serde::Deserialize;
use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::fs;

/// Parse the configuration from a given file
pub async fn parse<P: AsRef<Path>>(path: P) -> Result<SharedConfig> {
    let raw = fs::read(path).await?;
    let data = toml::from_slice(&raw)?;
    Ok(Arc::new(data))
}

pub type SharedConfig = Arc<Config>;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub server: Server,
    pub docker: Docker,
    pub github: Github,
}

#[derive(Debug, Deserialize)]
pub struct Server {
    pub address: SocketAddr,
    pub log: String,
    pub workers: u32,
}

#[derive(Debug, Deserialize)]
pub struct Docker {
    pub connection: Connection,
    pub token: String,
}

#[derive(Debug, Deserialize)]
pub struct Connection {
    #[serde(flatten)]
    pub connection_type: ConnectionType,
    pub endpoint: String,
    pub timeout: u64,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ConnectionType {
    Local,
    Http,
    Ssl {
        ca: PathBuf,
        certificate: PathBuf,
        key: PathBuf,
    },
}

#[derive(Debug, Deserialize)]
pub struct Github {
    pub clone_to: PathBuf,
    pub repository: String,
    pub secret: String,
}

#[cfg(test)]
mod tests {
    use super::parse;
    use crate::config::ConnectionType;

    #[tokio::test]
    async fn parse_config() {
        let config = parse("./wafflemaker.example.toml")
            .await
            .expect("failed to parse configuration");

        assert_eq!("127.0.0.1:8000", &config.server.address.to_string());
        assert_eq!("info", &config.server.log);
        assert_eq!(2, config.server.workers);
        assert_eq!(
            "unix:///var/run/docker.sock",
            &config.docker.connection.endpoint
        );
        assert_eq!(120, config.docker.connection.timeout);
        assert_eq!(
            ConnectionType::Local,
            config.docker.connection.connection_type
        );
        assert_eq!("please-change:this-token", &config.docker.token);
        assert_eq!("./configuration", config.github.clone_to.to_str().unwrap());
        assert_eq!("WaffleHacks/waffles", &config.github.repository);
        assert_eq!("please-change-this-secret", &config.github.secret);
    }
}
