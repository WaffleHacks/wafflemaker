use super::{CreateOpts, Deployer, Result};
use crate::config::Connection;
use async_trait::async_trait;
use bollard::{
    container::{Config as CreateContainerConfig, RemoveContainerOptions},
    image::CreateImageOptions,
    Docker as Bollard, API_DEFAULT_VERSION,
};
use futures::stream::StreamExt;
use sled::{Config, Db, Mode};
use std::{collections::HashMap, fs, path::Path};
use tracing::{debug, error, info, instrument};

#[derive(Debug)]
pub struct Docker {
    instance: Bollard,
    domain: String,
    state: Db,
}

impl Docker {
    /// Connect to a new docker instance
    #[instrument(
        name = "docker",
        skip(connection, endpoint, domain, path),
        fields(
            connection = connection.kind(),
            endpoint = endpoint.as_ref(),
            state = path.as_ref().to_str().unwrap()
        )
    )]
    pub fn new<S: AsRef<str>, P: AsRef<Path>>(
        connection: &Connection,
        endpoint: S,
        timeout: &u64,
        domain: String,
        path: P,
    ) -> Result<Self> {
        let endpoint = endpoint.as_ref();
        let path = path.as_ref();

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

        // Create the database folder if not exists
        if !path.exists() {
            fs::create_dir_all(path)?;
        }

        // Open the state database with a 32MB cache and save to disk every 1s
        let state = Config::new()
            .path(path)
            .cache_capacity(32 * 1024 * 1024)
            .flush_every_ms(Some(1000))
            .mode(Mode::HighThroughput)
            .use_compression(true)
            .open()?;

        Ok(Self {
            instance,
            domain,
            state,
        })
    }

    /// Convert a deployment name to a docker id
    fn id_from_name<S: AsRef<str>>(&self, name: S) -> Result<String> {
        let tree = self.state.open_tree(name.as_ref())?;
        get_string(&tree, "id").transpose().unwrap()
    }
}

#[async_trait]
impl Deployer for Docker {
    #[instrument(skip(self))]
    async fn test(&self) -> Result<()> {
        let info = self.instance.info().await?;
        let id = info.id.unwrap_or_default();
        info!(id = %id, "connected to docker");

        Ok(())
    }

    #[instrument(skip(self))]
    async fn list(&self) -> Result<HashMap<String, String>> {
        let mut mapping = HashMap::new();
        for tree_name in self.state.tree_names() {
            let id = get_string(&self.state.open_tree(&tree_name)?, "id")?;
            if let Some(id) = id {
                let name = String::from_utf8(tree_name.as_ref().to_vec()).unwrap();
                mapping.insert(name, id);
            } else {
                continue;
            }
        }

        Ok(mapping)
    }

    #[instrument(
        skip(self, options),
        fields(
            name = %options.name,
            web = %options.domain.is_some(),
            domain = ?options.domain,
            image = %options.image),
        )
    ]
    async fn create(&self, options: CreateOpts) -> Result<String> {
        let tree = self.state.open_tree(&options.name)?;

        tree.insert("image", options.image.as_str())?;

        let environment = options
            .environment
            .into_iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();

        let mut labels = HashMap::new();

        if let Some(domain) = &options.domain {
            // Add routing labels
            let router_name = domain.replace(".", "-");
            labels.insert(
                format!("traefik.http.routers.{}.rule", router_name),
                format!("Host(`{}`)", domain),
            );
            labels.insert(
                format!("traefik.http.routers.{}.tls.certresolver", router_name),
                "letsencrypt".to_string(),
            );
        }

        // Enable traefik if a domain is added
        labels.insert(
            "traefik.enable".to_string(),
            options.domain.is_some().to_string(),
        );

        // Pull the image
        info!(
            "pulling image \"{}:{}\" from Docker Hub",
            &options.image, &options.tag
        );
        let mut stream = self.instance.create_image(
            Some(CreateImageOptions {
                from_image: options.image.clone(),
                tag: options.tag.clone(),
                ..Default::default()
            }),
            None,
            None,
        );
        while let Some(info) = stream.next().await {
            let info = info?;
            if let Some(message) = info.status {
                debug!("{}", message);
            }
        }

        let config = CreateContainerConfig {
            image: Some(format!("{}:{}", &options.image, &options.tag)),
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

        tree.insert("id", result.id.as_str())?;

        Ok(result.id)
    }

    #[instrument(skip(self))]
    async fn start(&self, name: &str) -> Result<()> {
        let id = self.id_from_name(name)?;
        self.start_by_id(&id).await?;
        Ok(())
    }

    #[instrument(skip(self))]
    async fn start_by_id(&self, id: &str) -> Result<()> {
        self.instance.start_container::<&str>(&id, None).await?;
        Ok(())
    }

    #[instrument(skip(self))]
    async fn stop(&self, name: &str) -> Result<()> {
        let id = self.id_from_name(name)?;
        self.stop_by_id(&id).await?;
        Ok(())
    }

    #[instrument(skip(self))]
    async fn stop_by_id(&self, id: &str) -> Result<()> {
        self.instance.stop_container(&id, None).await?;
        Ok(())
    }

    #[instrument(skip(self))]
    async fn delete(&self, name: &str) -> Result<()> {
        let id = self.id_from_name(&name)?;
        self.delete_by_id(&id).await?;

        // Remove the state for the deployment
        self.state.drop_tree(name)?;

        Ok(())
    }

    #[instrument(skip(self))]
    async fn delete_by_id(&self, id: &str) -> Result<()> {
        self.instance
            .remove_container(
                &id,
                Some(RemoveContainerOptions {
                    v: true,
                    link: false,
                    force: false,
                }),
            )
            .await?;

        Ok(())
    }
}

impl Drop for Docker {
    fn drop(&mut self) {
        if let Err(e) = self.state.flush() {
            error!("failed to flush state database: {}", e)
        }
    }
}

/// Retrieve a string from a given key
fn get_string<K: AsRef<[u8]>>(tree: &sled::Tree, key: K) -> Result<Option<String>> {
    Ok(tree
        .get(key)?
        .map(|v| v.to_vec())
        .map(String::from_utf8)
        .transpose()?)
}
