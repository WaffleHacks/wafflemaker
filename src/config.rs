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
    pub deployment: Deployment,
    pub git: Git,
    pub server: Server,
    pub webhooks: Webhooks,
}

#[derive(Debug, Deserialize)]
pub struct Server {
    pub address: SocketAddr,
    pub log: String,
    pub workers: u32,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Deployment {
    Docker {
        #[serde(flatten)]
        connection: Connection,
        endpoint: String,
        timeout: u64,
    },
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(tag = "connection", rename_all = "lowercase")]
pub enum Connection {
    Local,
    Http,
    Ssl {
        ca: PathBuf,
        certificate: PathBuf,
        key: PathBuf,
    },
}

#[derive(Debug, Deserialize)]
pub struct Git {
    pub clone_to: PathBuf,
    pub repository: String,
}

#[derive(Debug, Deserialize)]
pub struct Webhooks {
    pub docker: String,
    pub github: String,
}

#[cfg(test)]
mod tests {
    use super::{parse, Connection, Deployment};

    #[tokio::test]
    async fn parse_config() {
        let config = parse("./wafflemaker.example.toml")
            .await
            .expect("failed to parse configuration");

        assert_eq!("127.0.0.1:8000", &config.server.address.to_string());
        assert_eq!("info", &config.server.log);
        assert_eq!(2, config.server.workers);

        assert!(matches!(config.deployment, Deployment::Docker { .. }));
        let Deployment::Docker {
            connection,
            endpoint,
            timeout,
        } = &config.deployment;
        assert_eq!(&Connection::Local, connection);
        assert_eq!("unix:///var/run/docker.sock", endpoint.as_str());
        assert_eq!(&120, timeout);

        assert_eq!("./configuration", config.git.clone_to.to_str().unwrap());
        assert_eq!("WaffleHacks/waffles", &config.git.repository);

        assert_eq!("please-change:this-token", &config.webhooks.docker);
        assert_eq!("please-change-this-secret", &config.webhooks.github);
    }
}
