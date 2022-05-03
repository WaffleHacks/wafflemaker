use super::records::Records;
use redis::{AsyncCommands, Client, RedisResult};

#[derive(Debug)]
pub struct Dns {
    client: Client,
    key: String,
}

impl Dns {
    pub(crate) fn new(client: Client, key_prefix: &str, zone: &str) -> Self {
        let key = [key_prefix, zone, "."].concat();
        Self { client, key }
    }

    /// Register service's DNS record
    pub async fn register(&self, service: &str, ip: &str) -> RedisResult<()> {
        // Build the records
        let records = Records::a_record(ip);
        let value = serde_json::to_string(&records).unwrap();

        // Insert into the hashmap
        let mut conn = self.client.get_tokio_connection_manager().await?;
        conn.hset(&self.key, service, value).await?;

        Ok(())
    }

    /// Unregister a service's DNS record
    pub async fn unregister(&self, service: &str) -> RedisResult<()> {
        let mut conn = self.client.get_tokio_connection_manager().await?;
        conn.hdel(&self.key, service).await?;

        Ok(())
    }
}
