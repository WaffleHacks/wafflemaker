use super::{Job, SharedJobQueue};
use crate::git::Repository;
use async_trait::async_trait;

#[derive(Debug)]
pub struct DeleteService {}

#[async_trait]
impl Job for DeleteService {
    async fn run(&self, _queue: SharedJobQueue, _repo: &Repository) {}
}
