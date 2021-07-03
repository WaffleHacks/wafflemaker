use crate::config::Dns as DnsConfig;
use arc_swap::ArcSwap;
use cloudflare::{
    endpoints::{
        dns::{
            CreateDnsRecord, CreateDnsRecordParams, DeleteDnsRecord, DnsContent, ListDnsRecords,
        },
        zone::ListZones,
    },
    framework::{
        async_api::{ApiClient, Client},
        auth::Credentials,
        Environment, HttpApiClientConfig,
    },
};
use once_cell::sync::Lazy;
use std::{
    collections::HashMap,
    net::{Ipv4Addr, Ipv6Addr},
    sync::Arc,
};
use tracing::{debug, info, instrument};

mod error;

use error::{Error, Result};

static STATIC_INSTANCE: Lazy<ArcSwap<Dns>> = Lazy::new(|| ArcSwap::from_pointee(Dns::default()));

/// Retrieve an instance of the dns client
pub fn instance() -> Arc<Dns> {
    STATIC_INSTANCE.load().clone()
}

/// Initialize & configure a new instance of the dns client
pub async fn initialize(config: &DnsConfig) -> Result<()> {
    let client = Dns::new(
        &config.zones,
        config.credentials.to_cloudflare(),
        config.addresses.v4,
        config.addresses.v6,
    )
    .await?;
    STATIC_INSTANCE.swap(Arc::from(client));
    Ok(())
}

/// An interface to the Cloudflare API
pub struct Dns {
    client: Option<Client>,
    public_v4: Ipv4Addr,
    public_v6: Option<Ipv6Addr>,
    // A mapping from zone name to zone information
    zones: HashMap<String, String>,
}

impl Default for Dns {
    fn default() -> Dns {
        Self {
            client: None,
            public_v4: Ipv4Addr::new(0, 0, 0, 0),
            public_v6: None,
            zones: HashMap::new(),
        }
    }
}

impl Dns {
    /// Create and verify a new dns client instance
    #[instrument(skip(credentials))]
    async fn new(
        zone_names: &[String],
        credentials: Credentials,
        public_v4: Ipv4Addr,
        public_v6: Option<Ipv6Addr>,
    ) -> Result<Self> {
        let client = Client::new(
            credentials,
            HttpApiClientConfig::default(),
            Environment::Production,
        )?;

        let registered_zones = client
            .request(&ListZones {
                params: Default::default(),
            })
            .await?
            .result;
        debug!(
            zones = ?registered_zones.iter().map(|z| &z.name).collect::<Vec<_>>(),
            "got all authorized registered zones"
        );

        // Create a mapping from zone name to id
        let mut zones = HashMap::new();
        for zone in registered_zones {
            if zone_names.contains(&zone.name) {
                zones.insert(zone.name.clone(), zone.id.clone());
            }
        }

        if zones.len() != zone_names.len() {
            return Err(Error::MissingZones(zone_names.len() - zones.len()));
        }

        info!(zones = ?zones.keys(), "connected to cloudflare for DNS");

        Ok(Self {
            client: Some(client),
            public_v4,
            public_v6,
            zones,
        })
    }

    /// Get the id of the zone the domain is under
    fn zone_id(&self, domain: &str) -> Option<&str> {
        for (zone, id) in self.zones.iter() {
            if domain.strip_suffix(zone).is_some() {
                return Some(&id);
            }
        }

        None
    }

    /// Generate the create record parameters for a given domain
    fn record_params(name: &str, content: DnsContent) -> CreateDnsRecordParams {
        CreateDnsRecordParams {
            ttl: None,
            priority: None,
            proxied: Some(true),
            name,
            content,
        }
    }

    /// Create a new DNS record
    pub async fn create<S: AsRef<str>>(&self, domain: S) -> Result<()> {
        let client = self.client.as_ref().ok_or(Error::Uninitialized)?;
        let domain = domain.as_ref();

        let zone = self
            .zone_id(domain)
            .ok_or(Error::NonExistentZone(domain.to_owned()))?;

        // Create the A record
        let a_params = Self::record_params(
            domain,
            DnsContent::A {
                content: self.public_v4,
            },
        );
        client
            .request(&CreateDnsRecord {
                params: a_params,
                zone_identifier: zone,
            })
            .await?;

        // Create the AAAA record if exists
        if let Some(v6) = self.public_v6 {
            let aaaa_params = Self::record_params(domain, DnsContent::AAAA { content: v6 });
            client
                .request(&CreateDnsRecord {
                    params: aaaa_params,
                    zone_identifier: zone,
                })
                .await?;
        }

        Ok(())
    }

    /// Delete an existing DNS record, will not fail if the record does not exist.
    /// It will try to delete the subdomain across ALL enabled zones.
    pub async fn delete<S: AsRef<str>>(&self, name: S) -> Result<()> {
        let name = name.as_ref();
        let client = self.client.as_ref().ok_or(Error::Uninitialized)?;

        for (base, id) in self.zones.iter() {
            let domain = format!("{}.{}", name, base);

            let records = client
                .request(&ListDnsRecords {
                    zone_identifier: id,
                    params: Default::default(),
                })
                .await?
                .result;

            // Delete any matching A or AAAA records
            for record in records {
                if record.name == domain
                    && matches!(
                        record.content,
                        DnsContent::A { .. } | DnsContent::AAAA { .. }
                    )
                {
                    client
                        .request(&DeleteDnsRecord {
                            zone_identifier: id,
                            identifier: &record.id,
                        })
                        .await?;
                }
            }
        }

        Ok(())
    }
}
