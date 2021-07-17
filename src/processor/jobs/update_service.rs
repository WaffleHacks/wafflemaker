use super::Job;
use crate::{
    config,
    deployer::{self, CreateOpts},
    fail,
    service::{AWSPart, Format, Secret, Service},
    vault::{self, AWS},
};
use async_trait::async_trait;
use rand::{distributions::Alphanumeric, Rng, RngCore, SeedableRng};
use rand_chacha::ChaCha20Rng;
use tracing::{info, instrument, warn};

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
        let service = &self.config;

        // Create the base container creation args
        let mut options = CreateOpts::builder()
            .name(&self.name)
            .image(&service.docker.image, &service.docker.tag);

        if service.web.enabled {
            let subdomain = self.name.replace("-", ".");
            let base = match &service.web.base {
                Some(base) => base,
                None => &config::instance().deployment.domain,
            };

            options = options.domain(format!("{}.{}", subdomain, base));
        }

        for (k, v) in service.environment.iter() {
            options = options.environment(k.to_uppercase(), v);
        }

        // Load secrets into the environment
        let mut static_secrets = match fail!(vault::instance().fetch_static(&self.name).await) {
            Some(s) => s,
            None => Default::default(),
        };
        let mut aws_creds: Option<AWS> = None;
        for (k, secret) in service.secrets.iter() {
            let value = match secret {
                Secret::AWS { role, part } => {
                    // Retrieve the initial set of credentials if they haven't been already
                    if aws_creds.is_none() {
                        aws_creds = Some(fail!(vault::instance().aws_credentials(role).await));
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
        }

        if let Some(name) = service.dependencies.postgres() {
            // Create the role if it doesn't exist
            let roles = fail!(vault::instance().list_database_roles().await);
            if !roles.contains(&self.name) {
                fail!(vault::instance().create_database_role(&self.name).await);
            } else {
                fail!(
                    vault::instance()
                        .rotate_database_credentials(&self.name)
                        .await
                );
            }

            let credentials = fail!(vault::instance().get_database_credentials(&self.name).await);
            // TODO: parameterize connection url template
            let connection_url = "postgres://{{username}}:{{password}}@127.0.0.1:5432/{{username}}"
                .replace("{{username}}", &credentials.username)
                .replace("{{password}}", &credentials.password);

            options = options.environment(name.to_uppercase(), connection_url);
        }
        if let Some(name) = service.dependencies.redis() {
            // TODO: parameterize redis value
            options = options.environment(name.to_uppercase(), "redis://127.0.0.1:6379");
        }

        // Create and start the container
        let id = fail!(deployer::instance().create(options.build()).await);
        fail!(deployer::instance().start(self.name.clone()).await);
        info!("deployed with id \"{}\"", id);

        // Save any modifications to the static secrets
        fail!(
            vault::instance()
                .put_static(&self.name, static_secrets)
                .await
        );
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
