use anyhow::Result;
use serde::Deserialize;
use std::{
    collections::HashMap,
    net::SocketAddr,
    num::ParseIntError,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tokio::fs;

/// Parse the configuration from a given file
pub async fn parse<P: AsRef<Path>>(path: P) -> Result<Arc<Config>> {
    let raw = fs::read(path).await?;
    let config = toml::from_slice(&raw)?;
    Ok(Arc::new(config))
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub agent: Agent,
    pub dependencies: HashMap<String, Dependency>,
    pub deployment: Deployment,
    pub dns: Dns,
    pub git: Git,
    pub http: Http,
    pub notifiers: Vec<Notifier>,
    pub secrets: Secrets,
}

#[derive(Debug, Deserialize)]
pub struct Agent {
    pub log: String,
    pub sentry: Option<String>,
    #[serde(rename = "tokio-console", default)]
    pub tokio_console: bool,
    pub workers: u32,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum Dependency {
    Static {
        value: String,
        default_env: String,
    },
    Postgres {
        connection_template: String,
        default_env: String,
    },
}

#[derive(Debug, Deserialize)]
pub struct Deployment {
    #[serde(flatten)]
    pub connection: Connection,
    pub endpoint: String,
    pub timeout: u64,
    pub network: String,
    pub state: PathBuf,
}

impl Default for Deployment {
    fn default() -> Deployment {
        Deployment {
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
pub struct Http {
    pub address: SocketAddr,
    #[serde(rename = "management-token")]
    pub management_token: String,
    pub webhooks: Webhooks,
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
    use super::{parse, Connection, Dependency, Notifier};
    use std::time::Duration;

    #[tokio::test]
    async fn parse_config() {
        let config = parse("../wafflemaker.example.toml")
            .await
            .expect("failed to parse configuration");

        assert_eq!("info", &config.agent.log);
        assert_eq!(false, config.agent.tokio_console);
        assert_eq!(2, config.agent.workers);

        assert_eq!(config.dependencies.len(), 2);
        {
            let dependency = config.dependencies.get("postgres").unwrap();
            assert!(matches!(
                dependency,
                Dependency::Postgres { connection_template, default_env }
                if connection_template == "postgres://{{username}}:{{password}}@127.0.0.1:5432/{{database}}" && default_env == "POSTGRES_URL"
            ));
        }
        {
            let dependency = config.dependencies.get("redis").unwrap();
            assert!(matches!(
                dependency,
                Dependency::Static { value, default_env } if value == "redis://127.0.0.1:6379" && default_env == "REDIS_URL"
            ));
        }

        assert_eq!(Connection::Local, config.deployment.connection);
        assert_eq!("unix:///var/run/docker.sock", config.deployment.endpoint);
        assert_eq!(120, config.deployment.timeout);
        assert_eq!("traefik", config.deployment.network);
        assert_eq!("./state", config.deployment.state.to_str().unwrap());

        assert_eq!("dns:", &config.dns.key_prefix);
        assert_eq!("redis://127.0.0.1:6379", &config.dns.redis);
        assert_eq!("127.0.0.1:1053", &config.dns.server);
        assert_eq!("wafflemaker.internal", &config.dns.zone);

        assert_eq!("master", &config.git.branch);
        assert_eq!("./configuration", config.git.clone_to.to_str().unwrap());
        assert_eq!("WaffleHacks/waffles", &config.git.repository);

        assert_eq!("127.0.0.1:8000", &config.http.address.to_string());
        assert_eq!("please-change-me", &config.http.management_token);
        assert_eq!("please-change:this-token", &config.http.webhooks.docker);
        assert_eq!("please-change-this-secret", &config.http.webhooks.github);

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
    }
}