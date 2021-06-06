use super::{CreateOpts, Deployer, Result, ServiceInfo, Status};
use crate::config::Connection;
use async_trait::async_trait;
use bollard::{
    container::{Config as CreateContainerConfig, RemoveContainerOptions},
    models::ContainerSummaryInner,
    Docker as Bollard, API_DEFAULT_VERSION,
};
use std::collections::HashMap;
use tracing::{debug, info, instrument};

#[derive(Debug)]
pub struct Docker {
    instance: Bollard,
}

impl Docker {
    /// Connect to a new docker instance
    #[instrument(
        skip(connection, endpoint),
        fields(connection = connection.kind(), endpoint = endpoint.as_ref())
    )]
    pub async fn new<S: AsRef<str>>(
        connection: &Connection,
        endpoint: S,
        timeout: &u64,
    ) -> Result<Self> {
        let endpoint = endpoint.as_ref();

        // Create the connection
        let instance = match connection {
            Connection::Local => {
                Bollard::connect_with_local(endpoint, *timeout, API_DEFAULT_VERSION)?
            }
            Connection::Http => {
                Bollard::connect_with_http(endpoint, *timeout, API_DEFAULT_VERSION)?
            }
            Connection::Ssl {
                ca,
                certificate,
                key,
            } => Bollard::connect_with_ssl(
                endpoint,
                key,
                certificate,
                ca,
                *timeout,
                API_DEFAULT_VERSION,
            )?,
        };
        debug!("created docker connection");

        // Test connection
        let response = instance.ping().await?;
        info!(ping = %response, "connected to docker");

        Ok(Self { instance })
    }
}

#[async_trait]
impl Deployer for Docker {
    async fn list(&self) -> Result<Vec<ServiceInfo>> {
        Ok(self
            .instance
            .list_containers::<&str>(None)
            .await?
            .into_iter()
            .map(From::from)
            .collect())
    }

    async fn create(&self, options: CreateOpts) -> Result<String> {
        let environment = options
            .environment
            .into_iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();

        // Define the labels
        let router_name = options.subdomain.replace(".", "-");
        let mut labels = HashMap::new();
        labels.insert(
            format!("traefik.http.routers.{}.rule", router_name),
            format!("Host(`{}`)", options.subdomain), // TODO: add base domain
        );
        labels.insert(
            format!("traefik.http.routers.{}.tls.certresolver", router_name),
            "letsencrypt".to_string(),
        );
        labels.insert("traefik.enable".to_string(), "true".to_string());

        let config = CreateContainerConfig {
            image: Some(options.image),
            env: Some(environment),
            attach_stderr: Some(true),
            attach_stdout: Some(true),
            labels: Some(labels),
            ..Default::default()
        };

        let result = self
            .instance
            .create_container::<&str, _>(None, config)
            .await?;
        Ok(result.id)
    }

    async fn start(&self, id: String) -> Result<()> {
        self.instance.start_container::<&str>(&id, None).await?;
        Ok(())
    }

    async fn stop(&self, id: String) -> Result<()> {
        self.instance.stop_container(&id, None).await?;
        Ok(())
    }

    async fn delete(&self, id: String) -> Result<()> {
        self.instance
            .remove_container(
                &id,
                Some(RemoveContainerOptions {
                    v: true,
                    link: true,
                    force: false,
                }),
            )
            .await?;
        Ok(())
    }
}

impl From<ContainerSummaryInner> for ServiceInfo {
    fn from(summary: ContainerSummaryInner) -> ServiceInfo {
        // Extract the subdomain from the router label
        let mut subdomain = String::new();
        for (label, _) in summary.labels.unwrap_or_default().into_iter() {
            if let Some(s) = label
                .strip_prefix("traefik.http.routers.")
                .map(|l| l.strip_suffix(".rule"))
                .flatten()
                .map(|l| l.replace("-", "."))
            {
                subdomain = s;
                break;
            }
        }

        ServiceInfo {
            id: summary.id.unwrap_or_default(),
            image: summary.image.unwrap_or_default(),
            status: summary.status.unwrap_or_default().into(),
            subdomain,
        }
    }
}

impl From<String> for Status {
    fn from(s: String) -> Status {
        match s.as_str() {
            "created" => Status::Created,
            "running" => Status::Running,
            "paused" | "exited" | "removing" => Status::Stopped,
            "restarting" => Status::Restarting,
            "dead" => Status::Killed,
            _ => unreachable!(),
        }
    }
}
