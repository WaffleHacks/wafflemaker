use redis::Client;

#[derive(Debug)]
pub struct Dns {
    client: Client,
    key_prefix: String,
    zone_suffix: String,
}

impl Dns {
    pub(crate) fn new(client: Client, key_prefix: String, zone_suffix: String) -> Self {
        Self {
            client,
            key_prefix,
            zone_suffix,
        }
    }
}
