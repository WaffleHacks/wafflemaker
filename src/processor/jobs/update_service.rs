use super::Job;
use crate::{
    deployer::{self, CreateOpts},
    fail,
    service::Service,
};
use async_trait::async_trait;
use tracing::{info, instrument};

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
            let domain = match &service.web.base {
                Some(base) => format!("{}.{}", subdomain, base),
                // TODO: substitute default domain
                None => format!("{}.wafflehacks.tech", subdomain),
            };

            options = options.domain(domain);
        }

        for (k, v) in service.environment.iter() {
            options = options.environment(k.to_uppercase(), v);
        }

        // TODO: retrieve secrets

        // Create and start the container
        let id = fail!(deployer::instance().create(options.build()).await);
        fail!(deployer::instance().start(self.name.clone()).await);
        info!("deployed with id \"{}\"", id);
    }

    fn name<'a>(&self) -> &'a str {
        "update_service"
    }
}
