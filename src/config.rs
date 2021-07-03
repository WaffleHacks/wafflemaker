use anyhow::Result;
use cloudflare::framework::auth::Credentials as CloudflareCredentials;
use serde::Deserialize;
use std::{
    net::{Ipv4Addr, Ipv6Addr, SocketAddr},
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
    pub dns: Dns,
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
pub struct Webhooks {
    pub docker: String,
    pub github: String,
}

#[derive(Debug, Deserialize)]
pub struct Dns {
    pub zones: Vec<String>,
    pub credentials: Credentials,
    pub addresses: Addresses,
}

#[derive(Debug, Deserialize)]
pub struct Addresses {
    pub v4: Ipv4Addr,
    #[serde(default)]
    pub v6: Option<Ipv6Addr>,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum Credentials {
    UserKey { email: String, key: String },
    UserToken { token: String },
    Service { key: String },
}

impl Credentials {
    pub fn to_cloudflare(&self) -> CloudflareCredentials {
        match self {
            Self::Service { key } => CloudflareCredentials::Service {
                key: key.to_owned(),
            },
            Self::UserKey { email, key } => CloudflareCredentials::UserAuthKey {
                email: email.to_owned(),
                key: key.to_owned(),
            },
            Self::UserToken { token } => CloudflareCredentials::UserAuthToken {
                token: token.to_owned(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{parse, Connection, Credentials, DeploymentEngine};

    #[tokio::test]
    async fn parse_config() {
        let config = parse("./wafflemaker.example.toml")
            .await
            .expect("failed to parse configuration");

        assert_eq!("127.0.0.1:8000", &config.server.address.to_string());
        assert_eq!("info", &config.server.log);
        assert_eq!(2, config.server.workers);

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

        assert_eq!(vec!["wafflehacks.tech"], config.dns.zones);
        assert_eq!("127.0.0.1", config.dns.addresses.v4.to_string());
        assert_eq!(None, config.dns.addresses.v6);
        assert_eq!(
            Credentials::UserToken {
                token: "ABCd-eFGHijKlmNoPQrsTUVWxyz0123456789-ab".into()
            },
            config.dns.credentials
        );

        assert_eq!("./configuration", config.git.clone_to.to_str().unwrap());
        assert_eq!("WaffleHacks/waffles", &config.git.repository);

        assert_eq!("please-change:this-token", &config.webhooks.docker);
        assert_eq!("please-change-this-secret", &config.webhooks.github);
    }
}
