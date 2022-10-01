use super::{events, options::CreateOpts, Result};
use crate::config::{Connection, Deployment};
use bollard::{
    container::{
        Config as CreateContainerConfig, CreateContainerOptions, NetworkingConfig,
        RemoveContainerOptions,
    },
    image::CreateImageOptions,
    models::{EndpointSettings, HostConfig},
    Docker as Bollard, API_DEFAULT_VERSION,
};
use futures::stream::StreamExt;
use rand::{distributions::Alphanumeric, Rng, SeedableRng};
use rand_chacha::ChaCha20Rng;
use sled::{Config, Db, Mode};
use std::{collections::HashMap, fs};
use tokio::sync::broadcast::Receiver;
use tracing::{debug, error, info, instrument};

/// Create a new connection to the Docker API from the configuration
pub(crate) fn connect(config: &Deployment) -> Result<Bollard> {
    let docker = match &config.connection {
        Connection::Local => {
            Bollard::connect_with_local(&config.endpoint, config.timeout, API_DEFAULT_VERSION)?
        }
        Connection::Http => {
            Bollard::connect_with_http(&config.endpoint, config.timeout, API_DEFAULT_VERSION)?
        }
        Connection::Ssl {
            ca,
            certificate,
            key,
        } => Bollard::connect_with_ssl(
            &config.endpoint,
            key,
            certificate,
            ca,
            config.timeout,
            API_DEFAULT_VERSION,
        )?,
    };
    Ok(docker)
}

#[derive(Debug)]
pub struct Deployer {
    instance: Bollard,
    state: Db,
    network: String,
    network_config: NetworkingConfig<String>,
    dns: String,
}

impl Deployer {
    /// Connect to a new docker instance
    #[instrument(
        name = "docker",
        skip(config, stop),
        fields(
            connection = config.connection.kind(),
            endpoint = config.endpoint,
            state = config.state.to_str().unwrap(),
            network = config.network,
        )
    )]
    pub async fn new(config: &Deployment, dns_server: &str, stop: Receiver<()>) -> Result<Self> {
        // Create the connection
        let instance = connect(config)?;
        debug!("created docker connection");

        // Create the database folder if not exists
        if !config.state.exists() {
            fs::create_dir_all(&config.state)?;
        }

        // Open the state database with a 32MB cache and save to disk every 1s
        let state = Config::new()
            .path(&config.state)
            .cache_capacity(32 * 1024 * 1024)
            .flush_every_ms(Some(1000))
            .mode(Mode::HighThroughput)
            .use_compression(true)
            .open()?;

        // Fetch the network information
        let network = instance
            .inspect_network::<&str>(&config.network, None)
            .await?;
        let network_name = network.name.unwrap();
        let network_config = NetworkingConfig {
            endpoints_config: {
                let mut h = HashMap::new();
                h.insert(
                    network_name.clone(),
                    EndpointSettings {
                        network_id: network.id,
                        ..Default::default()
                    },
                );
                h
            },
        };

        tokio::task::spawn({
            // This is safe to unwrap since we already created a connection earlier
            let instance = connect(config).unwrap();

            events::watch(instance, stop)
        });

        Ok(Self {
            instance,
            state,
            network: network_name,
            network_config,
            dns: dns_server.to_owned(),
        })
    }

    /// Convert a deployment name to a docker id
    fn id_from_name<S: AsRef<str>>(&self, name: S) -> Result<String> {
        let tree = self.state.open_tree(name.as_ref())?;
        get_string(&tree, "id").transpose().unwrap()
    }

    #[instrument(skip(self))]
    pub async fn test(&self) -> Result<()> {
        let info = self.instance.info().await?;
        let id = info.id.unwrap_or_default();
        info!(id = %id, "connected to docker");

        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn list(&self) -> Result<HashMap<String, String>> {
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

    #[instrument(skip(self))]
    pub async fn service_id(&self, name: &str) -> Result<Option<String>> {
        let tree = self.state.open_tree(name)?;
        get_string(&tree, "id")
    }

    #[instrument(
        skip(self, options),
        fields(
            name = %options.name,
            web = %options.routing.is_some(),
            routing = ?options.routing,
            image = %options.image),
        )
    ]
    pub async fn create(&self, options: CreateOpts) -> Result<String> {
        let tree = self.state.open_tree(&options.name)?;

        tree.insert("image", options.image.as_str())?;

        // Generate a (hopefully) collision-resistant name
        let suffix = ChaCha20Rng::from_rng(rand::thread_rng())
            .unwrap()
            .sample_iter(&Alphanumeric)
            .take(8)
            .map(char::from)
            .collect::<String>();
        let name = format!("{}-{}", &options.name, suffix);

        let environment = options
            .environment
            .into_iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();

        let mut labels = HashMap::new();

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

        if let Some(routing) = &options.routing {
            // Add routing label
            let rule = match &routing.path {
                Some(p) => format!("Host(`{}`) && PathPrefix(`{}`)", routing.domain, p),
                None => format!("Host(`{}`)", routing.domain),
            };
            labels.insert(format!("traefik.http.routers.{}.rule", &name), rule);

            // Add path prefix middleware if necessary
            if let Some(path) = &routing.path {
                let middleware_name = format!("{}-strip", &name);

                labels.insert(
                    format!("traefik.http.routers.{}.middlewares", &name),
                    format!("{}@docker", middleware_name),
                );
                labels.insert(
                    format!(
                        "traefik.http.middlewares.{}.stripprefix.prefixes",
                        middleware_name
                    ),
                    path.to_string(),
                );
            }

            // Enable HTTPS
            labels.insert(
                format!("traefik.http.routers.{}.tls.certresolver", &name),
                "le".to_string(),
            );

            debug!("added routing labels");

            // Determine the service port
            info!("attempting to determine service port for load balancing...");
            let image = self
                .instance
                .inspect_image(&format!("{}:{}", &options.image, &options.tag))
                .await?;
            if let Some(image_config) = image.config {
                if let Some(ports) = image_config.exposed_ports {
                    if !ports.is_empty() {
                        // The port specification is in the format <port>/<tcp|udp|sctp>, but we
                        // only care about the port itself, the protocol is assumed to be TCP
                        let mut port = ports.keys().take(1).next().cloned().unwrap();
                        let proto_idx = port.find('/').unwrap();
                        port.truncate(proto_idx);

                        info!("found port {} for service", port);

                        labels.insert(
                            format!("traefik.http.services.{}.loadbalancer.server.port", &name),
                            port,
                        );
                    }
                }
            }
        }

        // Enable traefik if a domain is added
        labels.insert(
            "traefik.enable".to_string(),
            options.routing.is_some().to_string(),
        );

        let config = CreateContainerConfig {
            image: Some(format!("{}:{}", &options.image, &options.tag)),
            env: Some(environment),
            attach_stderr: Some(true),
            attach_stdout: Some(true),
            labels: Some(labels),
            networking_config: Some(self.network_config.clone()),
            host_config: Some(HostConfig {
                dns: Some(vec![self.dns.clone()]),
                ..Default::default()
            }),
            ..Default::default()
        };

        let result = self
            .instance
            .create_container(Some(CreateContainerOptions { name }), config)
            .await?;

        tree.insert("id", result.id.as_str())?;

        Ok(result.id)
    }

    #[instrument(skip(self))]
    pub async fn start(&self, id: &str) -> Result<()> {
        self.instance.start_container::<&str>(id, None).await?;
        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn ip(&self, id: &str) -> Result<String> {
        let info = self.instance.inspect_container(id, None).await?;
        let networks = info.network_settings.unwrap().networks.unwrap();
        let network = networks.get(&self.network.clone()).unwrap();

        Ok(network.ip_address.clone().unwrap())
    }

    #[instrument(skip(self))]
    pub async fn stop(&self, id: &str) -> Result<()> {
        self.instance.stop_container(id, None).await?;
        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn delete(&self, id: &str) -> Result<()> {
        self.instance
            .remove_container(
                id,
                Some(RemoveContainerOptions {
                    v: true,
                    link: false,
                    force: false,
                }),
            )
            .await?;

        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn delete_by_name(&self, name: &str) -> Result<()> {
        let id = self.id_from_name(&name)?;
        self.delete(&id).await?;

        // Remove the state for the deployment
        self.state.drop_tree(name)?;

        Ok(())
    }
}

impl Drop for Deployer {
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
