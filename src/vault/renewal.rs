use super::{instance, models::Lease};
use once_cell::sync::Lazy;
use std::{
    collections::HashMap,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::{
    select,
    sync::{broadcast::Receiver, RwLock},
    time::{self, Duration},
};
use tracing::{debug, error, info, info_span};

pub static LEASES: Lazy<RwLock<HashMap<String, Vec<Lease>>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

/// Automatically renew the token at the specified interval
pub async fn token(interval: Duration, mut stop: Receiver<()>) {
    let mut interval = time::interval(interval);

    loop {
        select! {
            _ = interval.tick() => {
                let span = info_span!("token");
                let _ = span.enter();

                match instance().renew().await {
                    Ok(_) => { info!(parent: &span, "successfully renewed token"); },
                    Err(e) => { error!(parent: &span, error = %e, "failed to renew token"); },
                }
            }
            _ = stop.recv() => {
                info!("stopping vault token renewal");
                break
            }
        }
    }
}

/// Automatically renew the credential leases at the specified interval
pub async fn leases(interval: Duration, max_percent: f64, mut stop: Receiver<()>) {
    let mut interval = time::interval(interval);

    loop {
        select! {
            _ = interval.tick() => {
                let span = info_span!("leases");
                let _ = span.enter();

                let mut sets = LEASES.write().await;
                let mut count = 0;

                let vault = instance();
                for leases in sets.values_mut() {
                    for lease in leases {
                        let elapsed = (now() - lease.updated_at) as f64;
                        let refresh_at = max_percent * lease.ttl as f64;

                        if elapsed >= refresh_at {
                            match vault.renew_lease(&lease).await {
                                Ok(_) => {
                                    lease.updated_at = now();
                                    count += 1;
                                    info!(parent: &span, id = %lease.id, "successfully renewed lease");
                                },
                                Err(e) => {
                                    error!(parent: &span, id = %lease.id, error = %e, "failed to renew lease");
                                },
                            }
                        }
                        debug!(parent: &span, id = %lease.id, "checked lease for renewal");
                    }
                }

                if count > 0 {
                    info!(parent: &span, count = count, "successfully renewed leases");
                } else {
                    debug!(parent: &span, "no leases needed renewal");
                }
            }
            _ = stop.recv() => {
                info!("stopping credential lease renewal");
                break;
            }
        }
    }
}

/// Get the current unix timestamp
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}
