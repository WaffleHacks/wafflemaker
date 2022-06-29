use super::*;
use reqwest::Client;
use serde::Serialize;
use serde_json::json;
use std::borrow::Cow;
use tracing::instrument;

/// Dispatch an event to Discord
#[instrument(skip(client, event), fields(event = %event))]
pub async fn dispatch(
    client: &Client,
    url: &str,
    default_repo: &str,
    event: &Event<'_, '_>,
) -> Result<()> {
    let embed = Embed::from(event, default_repo);
    client
        .post(url)
        .json(&json!({ "content": null, "embeds": [ embed ] }))
        .send()
        .await?
        .error_for_status()?;

    Ok(())
}

/// Content that will be inserted into the embed
#[derive(Serialize)]
struct Embed<'key, 'value> {
    title: String,
    color: Color,
    fields: Vec<Field<'key, 'value>>,
}

impl<'n, 'v> Embed<'n, 'v> {
    fn from(event: &Event<'v, 'v>, default_repo: &str) -> Embed<'n, 'v> {
        let mut fields = Vec::new();

        // Add any event-specific fields
        let state = match event {
            Event::Deployment { commit, state } => {
                let short_commit = &commit[..8];
                let commit_url = format!(
                    "[`{short_commit}`](https://github.com/{repo}/commit/{commit})",
                    repo = default_repo,
                    short_commit = short_commit,
                    commit = commit,
                );
                fields.push(Field::new("Version", commit_url, true));
                state
            }
            Event::ServiceUpdate { name, state } | Event::ServiceDelete { name, state } => {
                fields.push(Field::new("Service", *name, true));
                state
            }
        };

        // Add state information
        fields.push(state.into());
        if let Some(error) = state.error() {
            fields.push(Field::new("Error", error.to_string(), false));
        }

        Embed {
            title: uppercase(event.as_str()),
            color: state.into(),
            fields,
        }
    }
}

/// A field within an embed
#[derive(Serialize)]
struct Field<'name, 'value> {
    name: &'name str,
    value: Cow<'value, str>,
    inline: bool,
}

impl<'n, 'v> Field<'n, 'v> {
    fn new<V>(name: &'n str, value: V, inline: bool) -> Field<'n, 'v>
    where
        V: Into<Cow<'v, str>>,
    {
        Field {
            name,
            value: value.into(),
            inline,
        }
    }
}

impl<'k, 'v> From<&State> for Field<'k, 'v> {
    fn from(state: &State) -> Field<'k, 'v> {
        Field {
            inline: true,
            name: "Status",
            value: state.as_str().into(),
        }
    }
}

/// The color of an embed
type Color = u32;

impl From<&State> for Color {
    fn from(state: &State) -> Color {
        match state {
            State::InProgress => 10976011, // equivalent to #a77b0b
            State::Success => 5488140,     // equivalent to #53be0c
            State::Failure(_) => 10819351, // equivalent to #a51717
        }
    }
}

/// Convert the first character of a string to uppercase
fn uppercase(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}
