use anyhow::Result;
use serde::Deserialize;
use std::{
    net::SocketAddr,
    num::ParseIntError,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
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
    pub agent: Agent,
    pub deployment: Deployment,
    pub git: Git,
    pub secrets: Secrets,
    pub webhooks: Webhooks,
}

#[derive(Debug, Deserialize)]
pub struct Agent {
    pub address: SocketAddr,
    pub log: String,
    pub workers: u32,
}

#[derive(Debug, Deserialize)]
pub struct Deployment {
    pub domain: String,
    #[serde(flatten)]
    pub engine: DeploymentEngine,
}

impl Default for Deployment {
    fn default() -> Deployment {
        Deployment {
            domain: "wafflehacks.tech".into(),
            engine: Default::default(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum DeploymentEngine {
    Docker {
        #[serde(flatten)]
        connection: Connection,
        endpoint: String,
        timeout: u64,
        state: PathBuf,
    },
}

impl Default for DeploymentEngine {
    fn default() -> DeploymentEngine {
        DeploymentEngine::Docker {
            connection: Default::default(),
            endpoint: "unix:///var/run/docker.sock".into(),
            timeout: 10,
            state: "./state".into(),
        }
    }
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

impl Default for Connection {
    fn default() -> Connection {
        Connection::Local
    }
}

impl Connection {
    /// A friendly name for the connection type
    pub fn kind<'a>(&self) -> &'a str {
        match self {
            Self::Local => "local",
            Self::Http => "http",
            Self::Ssl { .. } => "ssl",
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct Git {
    pub clone_to: PathBuf,
    pub repository: String,
}

#[derive(Debug, Deserialize)]
pub struct Secrets {
    pub address: String,
    period: String,
    pub token: String,
}

impl Secrets {
    /// Get the period in which the token should be renewed
    pub fn period(&self) -> Result<Duration, ParseIntError> {
        let raw = self.period.to_lowercase();
        let seconds = if let Some(time) = raw.strip_suffix("h") {
            time.parse::<u64>()? * 60 * 60
        } else if let Some(time) = raw.strip_suffix("m") {
            time.parse::<u64>()? * 60
        } else if let Some(time) = raw.strip_suffix("s") {
            time.parse::<u64>()?
        } else {
            raw.parse::<u64>()?
        };

        Ok(Duration::new(seconds, 0))
    }
}

#[derive(Debug, Deserialize)]
pub struct Webhooks {
    pub docker: String,
    pub github: String,
}

#[cfg(test)]
mod tests {
    use super::{parse, Connection, DeploymentEngine};
    use std::time::Duration;

    #[tokio::test]
    async fn parse_config() {
        let config = parse("./wafflemaker.example.toml")
            .await
            .expect("failed to parse configuration");

        assert_eq!("127.0.0.1:8000", &config.agent.address.to_string());
        assert_eq!("info", &config.agent.log);
        assert_eq!(2, config.agent.workers);

        assert_eq!("wafflehacks.tech", &config.deployment.domain);
        assert!(matches!(
            config.deployment.engine,
            DeploymentEngine::Docker { .. }
        ));
        let DeploymentEngine::Docker {
            connection,
            endpoint,
            timeout,
            state,
        } = &config.deployment.engine;
        assert_eq!(&Connection::Local, connection);
        assert_eq!("unix:///var/run/docker.sock", endpoint.as_str());
        assert_eq!(&120, timeout);
        assert_eq!("./state", state.to_str().unwrap());

        assert_eq!("./configuration", config.git.clone_to.to_str().unwrap());
        assert_eq!("WaffleHacks/waffles", &config.git.repository);

        assert_eq!("http://127.0.0.1:8200", config.secrets.address);
        assert_eq!("s.some-token", config.secrets.token);
        assert_eq!(
            Duration::new(60 * 60 * 24, 0),
            config.secrets.period().unwrap()
        );

        assert_eq!("please-change:this-token", &config.webhooks.docker);
        assert_eq!("please-change-this-secret", &config.webhooks.github);
    }
}
