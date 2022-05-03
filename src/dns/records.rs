use serde::Serialize;

const TTL: usize = 60;

#[derive(Serialize)]
pub(crate) struct Records<'r> {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    a: Vec<ARecord<'r>>,
}

impl<'r> Records<'r> {
    pub fn a_record(ip: &'r str) -> Self {
        Self {
            a: vec![ARecord { ip, ttl: TTL }],
        }
    }
}

#[derive(Serialize)]
pub(crate) struct ARecord<'r> {
    ip: &'r str,
    ttl: usize,
}
