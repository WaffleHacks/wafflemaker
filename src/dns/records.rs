use serde::Serialize;

const TTL: usize = 60;

#[derive(Serialize)]
pub(crate) struct Records<'r> {
    #[serde(skip_serializing_if = "Option::is_none")]
    a: Option<ARecord<'r>>,
}

impl<'r> Records<'r> {
    pub fn a_record(ip: &'r str) -> Self {
        Self {
            a: Some(ARecord { ip4: ip, ttl: TTL }),
        }
    }
}

#[derive(Serialize)]
pub(crate) struct ARecord<'r> {
    ip4: &'r str,
    ttl: usize,
}
