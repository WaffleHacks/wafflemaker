use crate::config::Dns as DnsConfig;
use once_cell::sync::OnceCell;
use redis::{Client, RedisResult};
use std::sync::Arc;

mod records;
mod service;

pub use service::Dns;

static INSTANCE: OnceCell<Arc<Dns>> = OnceCell::new();

/// Create the DNS management service
pub async fn initialize(config: &DnsConfig) -> RedisResult<()> {
    let client = Client::open(config.redis.as_str())?;

    // Test the connection
    let mut conn = client.get_async_connection().await?;
    redis::cmd("PING")
        .query_async::<_, String>(&mut conn)
        .await?;

    let dns = Dns::new(client, &config.key_prefix, &config.zone);
    INSTANCE.get_or_init(|| Arc::from(dns));

    Ok(())
}

/// Retrieve an instance of the DNS management service
pub fn instance() -> Arc<Dns> {
    INSTANCE.get().unwrap().clone()
}
