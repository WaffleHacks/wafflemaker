use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Docker {
    pub callback_url: String,
    pub push_data: PushData,
    pub repository: Repository,
}

#[derive(Debug, Deserialize)]
pub struct PushData {
    pub tag: String,
}

#[derive(Debug, Deserialize)]
pub struct Repository {
    pub repo_name: String,
}

#[cfg(test)]
mod tests {
    use super::Docker;
    use std::fs;

    #[test]
    fn parse_docker() {
        let content = fs::read_to_string("testdata/webhooks/docker.json")
            .expect("failed to read docker.json test data");

        let parsed: Docker = serde_json::from_str(&content).expect("invalid JSON format");

        assert_eq!("https://registry.hub.docker.com/u/svendowideit/testhook/hook/2141b5bi5i5b02bec211i4eeih0242eg11000a/", &parsed.callback_url);
        assert_eq!("latest", &parsed.push_data.tag);
        assert_eq!("svendowideit/testhook", &parsed.repository.repo_name);
    }
}
