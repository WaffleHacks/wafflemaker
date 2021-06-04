use super::{Job, SharedJobQueue};
use crate::git::Repository;
use async_trait::async_trait;

#[derive(Debug)]
pub struct UpdateService {}

#[async_trait]
impl Job for UpdateService {
    async fn run(&self, _queue: SharedJobQueue, _repo: &Repository) {}
}
