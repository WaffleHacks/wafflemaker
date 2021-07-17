use super::Service;
use crate::config;
use anyhow::Result;
use once_cell::sync::Lazy;
use std::{collections::HashMap, ffi::OsStr};
use tokio::{fs, sync::RwLock};
use tokio_stream::{wrappers::ReadDirStream, StreamExt};
use tracing::{debug, info, instrument};

pub static REGISTRY: Lazy<RwLock<HashMap<String, Service>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

/// Load the initial services from the filesystem
#[instrument]
pub async fn init() -> Result<()> {
    let mut reg = REGISTRY.write().await;

    let entries = fs::read_dir(&config::instance().git.clone_to).await?;
    let mut stream = ReadDirStream::new(entries);

    while let Some(entry) = stream.next().await {
        let entry = entry?;
        if !entry.file_type().await?.is_file() {
            continue;
        }

        if entry.path().extension().map(OsStr::to_str).flatten() != Some("toml") {
            continue;
        }

        let name = Service::name(&entry.path());
        let service = Service::parse(&entry.path()).await?;

        debug!("loaded service {}", &name);
        reg.insert(name, service);
    }

    info!("loaded {} services", reg.len());
    Ok(())
}
