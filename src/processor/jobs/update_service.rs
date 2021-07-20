use super::Job;
use crate::{
    config,
    deployer::{self, CreateOpts},
    fail,
    service::{registry::REGISTRY, AWSPart, Format, Secret, Service},
    vault::{self, AWS},
};
use async_trait::async_trait;
use rand::{distributions::Alphanumeric, Rng, RngCore, SeedableRng};
use rand_chacha::ChaCha20Rng;
use tracing::{debug, error, info, instrument, warn};

#[derive(Debug)]
pub struct UpdateService {
    config: Service,
    name: String,
}

impl UpdateService {
    /// Create a new update service job
    pub fn new(config: Service, name: String) -> Self {
        Self { config, name }
    }
}

#[async_trait]
impl Job for UpdateService {
    #[instrument(skip(self), fields(name = %self.name))]
    async fn run(&self) {
        let config = config::instance();
        let service = &self.config;

        // TODO: create in-progress deployment notification

        // Update the service in the registry
        let mut reg = REGISTRY.write().await;
        reg.insert(self.name.clone(), service.clone());

        // Create the base container creation args
        let mut options = CreateOpts::builder()
            .name(&self.name)
            .image(&service.docker.image, &service.docker.tag);

        if service.web.enabled {
            let subdomain = self.name.replace("-", ".");
            let base = match &service.web.base {
                Some(base) => base,
                None => &config.deployment.domain,
            };

            options = options.domain(format!("{}.{}", subdomain, base));
        }

        for (k, v) in service.environment.iter() {
            options = options.environment(k.to_uppercase(), v);
            debug!(name = %k, "added static environment variable");
        }
        info!("loaded static environment variables");

        // Get existing secrets
        let mut static_secrets = match fail!(vault::instance().fetch_static(&self.name).await) {
            Some(s) => s,
            None => Default::default(),
        };

        // Load secrets into the environment
        let mut leases = Vec::new();
        let mut aws_creds: Option<AWS> = None;
        for (k, secret) in service.secrets.iter() {
            let value = match secret {
                Secret::AWS { role, part } => {
                    // Retrieve the initial set of credentials if they haven't been already
                    if aws_creds.is_none() {
                        let (creds, lease) = fail!(vault::instance().aws_credentials(role).await);
                        aws_creds = Some(creds);
                        leases.push(lease);
                    }

                    match part {
                        AWSPart::Access => aws_creds.as_ref().unwrap().access_key.clone(),
                        AWSPart::Secret => aws_creds.as_ref().unwrap().secret_key.clone(),
                    }
                }
                Secret::Generate {
                    format,
                    length,
                    regenerate,
                } => {
                    let value = if *regenerate {
                        generate_value(format, *length)
                    } else {
                        static_secrets
                            .get(k)
                            .cloned()
                            .unwrap_or_else(|| generate_value(format, *length))
                    };

                    static_secrets.insert(k.clone(), value.clone());
                    value
                }
                Secret::Load => static_secrets.get(k).cloned().unwrap_or_else(|| {
                    warn!(key = %k, "failed to load secret from Vault");
                    String::new()
                }),
            };

            options = options.environment(k.to_uppercase(), value);
            debug!(name = %k, r#type = %secret.name(), "added secret from vault");
        }
        info!("loaded secrets from vault into environment");

        if let Some(name) = service.dependencies.postgres() {
            // Create the role if it doesn't exist
            let roles = fail!(vault::instance().list_database_roles().await);
            if !roles.contains(&self.name) {
                fail!(vault::instance().create_database_role(&self.name).await);
            }

            let (credentials, lease) =
                fail!(vault::instance().get_database_credentials(&self.name).await);
            leases.push(lease);
            let connection_url = &config
                .dependencies
                .postgres
                .replace("{{username}}", &credentials.username)
                .replace("{{password}}", &credentials.password)
                .replace("{{database}}", &self.name);

            options = options.environment(name.to_uppercase(), connection_url);
            debug!(name = %name, "added postgres database url");
        }
        if let Some(name) = service.dependencies.redis() {
            options = options.environment(name.to_uppercase(), &config.dependencies.redis);
            debug!(name = %name, "added redis url");
        }
        info!("loaded service dependencies into environment");

        let known_services = fail!(deployer::instance().list().await);
        let previous_id = known_services.get(&self.name);

        // Perform a rolling update of the service (if a previous version existed)
        // Flow (assuming previous version existed):
        //   - create new version
        //   - stop old version
        //   - start new version
        //   - on:
        //     - failure:
        //       - start old version
        //       - delete new version
        //     - success:
        //       - delete old version
        // Flow (new service):
        //   - create new version
        //   - start new version
        let new_id = fail!(deployer::instance().create(options.build()).await);
        if let Some(id) = previous_id {
            fail!(deployer::instance().stop(id).await);
        }
        let deployed = deployer::instance().start(&new_id).await;
        match (previous_id, deployed) {
            // existing deployment succeeded, cleanup old version
            (Some(id), Ok(_)) => {
                fail!(deployer::instance().delete(&id).await);
                fail!(vault::instance().revoke_leases(&id).await);
            }
            // existing deployment failed, start old version and cleanup
            (Some(id), Err(e)) => {
                error!(error = %e, "failed to deploy new service, restarting old version");
                fail!(deployer::instance().start(id).await);
                fail!(deployer::instance().delete(&new_id).await);
                fail!(vault::instance().revoke_leases(&new_id).await);
                return;
            }
            // previously non-existent deployment failed, nothing to do
            (None, Err(e)) => {
                error!(error = %e, "failed to deploy new service");
                return;
            }
            // previously non-existent deployment succeeded, nothing to do
            (None, Ok(_)) => {}
        }

        // Save the credential leases for renewal
        vault::instance().register_leases(&new_id, leases).await;

        info!("deployed with id \"{}\"", new_id);

        // Save any modifications to the static secrets
        fail!(
            vault::instance()
                .put_static(&self.name, static_secrets)
                .await
        );

        // TODO: mark deployment as suceeded
    }

    fn name<'a>(&self) -> &'a str {
        "update_service"
    }
}

/// Generate a given value with the parameters
fn generate_value(format: &Format, length: u32) -> String {
    let length = length as usize;

    #[cfg(not(test))]
    let mut rng = ChaCha20Rng::from_rng(rand::thread_rng()).unwrap();
    #[cfg(test)]
    let mut rng = ChaCha20Rng::seed_from_u64(654136);

    match format {
        Format::Alphanumeric => std::iter::repeat(())
            .map(|()| rng.sample(Alphanumeric))
            .map(char::from)
            .take(length)
            .collect::<String>(),
        Format::Base64 => {
            let mut data = vec![0; length];
            rng.fill_bytes(&mut data);

            let mut result = base64::encode(&data);
            result.truncate(length);
            result
        }
        Format::Hex => {
            let mut data = vec![0; length / 2];
            rng.fill_bytes(&mut data);
            hex::encode(&data)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::generate_value;
    use crate::service::Format;

    #[test]
    fn alphanumeric() {
        let alpha = generate_value(&Format::Alphanumeric, 32);
        assert_eq!("8xt8sBpJdgzzs5ribEp4cJhpPWaBAnYj", alpha);
        assert_eq!(32, alpha.len());
    }

    #[test]
    fn base64() {
        let b64 = generate_value(&Format::Base64, 32);
        assert_eq!("JVtD8vx6qsSaFRi2n7A68xBJ7rKmSnAG", b64);
        assert_eq!(32, b64.len());
    }

    #[test]
    fn hex() {
        let hex = generate_value(&Format::Hex, 32);
        assert_eq!("255b43f2fc7aaac49a1518b69fb03af3", hex);
        assert_eq!(32, hex.len());
    }
}
