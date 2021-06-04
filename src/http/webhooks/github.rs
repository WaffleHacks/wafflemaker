use serde::Deserialize;

/// The overarching GitHub webhook types
#[derive(Debug, Deserialize)]
#[serde(untagged, rename_all = "lowercase")]
pub enum Github {
    Ping {
        zen: String,
        hook_id: i64,
    },
    Push {
        after: String,
        before: String,
        #[serde(rename = "ref")]
        reference: String,
        repository: Repository,
    },
}

impl Github {
    /// Get the name of the webhook being executed
    pub fn name<'a>(&self) -> &'a str {
        match self {
            Self::Ping { .. } => "ping",
            Self::Push { .. } => "push",
        }
    }
}

/// The repository information
#[derive(Clone, Debug, Deserialize)]
pub struct Repository {
    #[serde(rename = "full_name")]
    pub name: String,
    pub clone_url: String,
}

#[cfg(test)]
mod tests {
    use super::Github;
    use std::fs;

    #[test]
    fn parse_github_ping() {
        let content = fs::read_to_string("testdata/webhooks/github-ping.json")
            .expect("failed to read github-ping.json test data");

        let parsed: Github = serde_json::from_str(&content).expect("invalid JSON format");

        assert_eq!("ping", parsed.name());
        if let Github::Ping { zen, hook_id } = parsed {
            assert_eq!("Non-blocking is better than blocking.", &zen);
            assert_eq!(30, hook_id);
        }
    }

    #[test]
    fn parse_github_push() {
        let content = fs::read_to_string("testdata/webhooks/github-push.json")
            .expect("failed to read github-push.json test data");

        let parsed: Github = serde_json::from_str(&content).expect("invalid JSON format");

        assert_eq!("push", parsed.name());
        if let Github::Push {
            after,
            before,
            reference,
            repository,
        } = parsed
        {
            assert_eq!("0000000000000000000000000000000000000000", &after);
            assert_eq!("4544205a385319fd846d5df4ed2e3b8173529d78", &before);
            assert_eq!("refs/tags/simple-tag", &reference);
            assert_eq!("Codertocat/Hello-World", &repository.name);
            assert_eq!(
                "https://octocoders.github.io/Codertocat/Hello-World.git",
                &repository.clone_url
            );
        }
    }
}
