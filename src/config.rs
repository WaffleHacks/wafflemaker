use anyhow::Result;
use once_cell::sync::OnceCell;
use serde::Deserialize;
use std::{
    net::SocketAddr,
    num::ParseIntError,
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::fs;

static CONFIG: OnceCell<Config> = OnceCell::new();

/// Parse the configuration from a given file
pub async fn parse<P: AsRef<Path>>(path: P) -> Result<()> {
    let raw = fs::read(path).await?;
    let data = toml::from_slice(&raw)?;
    CONFIG.set(data).unwrap();
    Ok(())
}

/// Retrieve the configuration
pub fn instance() -> &'static Config {
    CONFIG.get().unwrap()
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub agent: Agent,
    pub dependencies: Dependencies,
    pub deployment: Deployment,
    pub dns: Dns,
    pub git: Git,
    pub management: Management,
    pub notifiers: Vec<Notifier>,
    pub secrets: Secrets,
    pub webhooks: Webhooks,
}

#[derive(Debug, Deserialize)]
pub struct Agent {
    pub address: SocketAddr,
    pub log: String,
    pub sentry: Option<String>,
    pub workers: u32,
}

#[derive(Debug, Deserialize)]
pub struct Dependencies {
    pub postgres: String,
    pub redis: String,
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
        network: String,
        state: PathBuf,
    },
}

impl Default for DeploymentEngine {
    fn default() -> DeploymentEngine {
        DeploymentEngine::Docker {
            connection: Default::default(),
            endpoint: "unix:///var/run/docker.sock".into(),
            timeout: 10,
            network: "traefik".into(),
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
pub struct Dns {
    pub server: String,
    pub redis: String,
    pub key_prefix: String,
    pub zone: String,
}

#[derive(Debug, Deserialize)]
pub struct Git {
    pub branch: String,
    pub clone_to: PathBuf,
    pub repository: String,
}

#[derive(Debug, Deserialize)]
pub struct Management {
    pub enabled: bool,
    pub address: SocketAddr,
    pub token: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase", tag = "type")]
pub enum Notifier {
    Discord {
        webhook: String,
    },
    GitHub {
        app_id: String,
        installation_id: String,
        key: PathBuf,
        repository: Option<String>,
    },
}

#[derive(Debug, Deserialize)]
pub struct Secrets {
    pub address: String,
    lease_interval: String,
    pub lease_percent: f64,
    pub token: String,
    token_interval: String,
}

impl Secrets {
    fn parse_duration(raw: &str) -> Result<Duration, ParseIntError> {
        let raw = raw.to_lowercase();
        let seconds = if let Some(time) = raw.strip_suffix('h') {
            time.parse::<u64>()? * 60 * 60
        } else if let Some(time) = raw.strip_suffix('m') {
            time.parse::<u64>()? * 60
        } else if let Some(time) = raw.strip_suffix('s') {
            time.parse::<u64>()?
        } else {
            raw.parse::<u64>()?
        };

        Ok(Duration::new(seconds, 0))
    }

    /// How often the leases should be checked for renewal
    pub fn lease_interval(&self) -> Result<Duration, ParseIntError> {
        Self::parse_duration(&self.lease_interval)
    }

    /// How often the token should be renewed
    pub fn token_interval(&self) -> Result<Duration, ParseIntError> {
        Self::parse_duration(&self.token_interval)
    }
}

#[derive(Debug, Deserialize)]
pub struct Webhooks {
    pub docker: String,
    pub github: String,
}

#[cfg(test)]
mod tests {
    use super::{instance, parse, Connection, DeploymentEngine, Notifier};
    use std::time::Duration;

    #[tokio::test]
    async fn parse_config() {
        parse("./wafflemaker.example.toml")
            .await
            .expect("failed to parse configuration");
        let config = instance();

        assert_eq!("127.0.0.1:8000", &config.agent.address.to_string());
        assert_eq!("info", &config.agent.log);
        assert_eq!(2, config.agent.workers);

        assert_eq!(
            "postgres://{{username}}:{{password}}@127.0.0.1:5432/{{database}}",
            config.dependencies.postgres
        );
        assert_eq!("redis://127.0.0.1:6379", config.dependencies.redis);

        assert_eq!("wafflehacks.tech", &config.deployment.domain);
        assert!(matches!(
            config.deployment.engine,
            DeploymentEngine::Docker { .. }
        ));
        let DeploymentEngine::Docker {
            connection,
            endpoint,
            network,
            timeout,
            state,
        } = &config.deployment.engine;
        assert_eq!(&Connection::Local, connection);
        assert_eq!("unix:///var/run/docker.sock", endpoint.as_str());
        assert_eq!(&120, timeout);
        assert_eq!("traefik", network);
        assert_eq!("./state", state.to_str().unwrap());

        assert_eq!("dns:", &config.dns.key_prefix);
        assert_eq!("redis://127.0.0.1:6379", &config.dns.redis);
        assert_eq!("127.0.0.1:1053", &config.dns.server);
        assert_eq!("wafflemaker.internal", &config.dns.zone);

        assert_eq!("master", &config.git.branch);
        assert_eq!("./configuration", config.git.clone_to.to_str().unwrap());
        assert_eq!("WaffleHacks/waffles", &config.git.repository);

        assert!(config.management.enabled);
        assert_eq!("127.0.0.1:8001", &config.management.address.to_string());
        assert_eq!("please-change-me", config.management.token);

        assert_eq!(2, config.notifiers.len());
        assert!(matches!(
            &config.notifiers[0],
            Notifier::Discord { webhook }
                if webhook == "https://discord.com/api/webhooks/<id>/<key>"
        ));
        assert!(matches!(
            &config.notifiers[1],
            Notifier::GitHub {
                app_id,
                installation_id,
                key,
                repository,
            } if repository.is_none() && app_id == "123456"
                && installation_id == "12345678"
                && key.display().to_string() == "./github-app.private-key.pem"
        ));

        assert_eq!("http://127.0.0.1:8200", config.secrets.address);
        assert_eq!(
            Duration::new(60, 0),
            config.secrets.lease_interval().unwrap()
        );
        assert_eq!(0.75, config.secrets.lease_percent);
        assert_eq!("s.some-token", config.secrets.token);
        assert_eq!(
            Duration::new(60 * 60 * 24, 0),
            config.secrets.token_interval().unwrap()
        );

        assert_eq!("please-change:this-token", &config.webhooks.docker);
        assert_eq!("please-change-this-secret", &config.webhooks.github);
    }
}
