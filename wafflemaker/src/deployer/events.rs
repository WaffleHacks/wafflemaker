use super::instance;
use bollard::{models::EventMessage, system::EventsOptions, Docker};
use std::collections::HashMap;
use tokio::{select, sync::broadcast::Receiver};
use tokio_stream::StreamExt;
use tracing::{debug, error, info, info_span, instrument, warn};

/// Watch the docker events for any unexpected changes in container state
#[instrument(skip_all)]
pub async fn watch(client: Docker, mut stop: Receiver<()>) {
    let mut events = client.events(Some(EventsOptions {
        filters: {
            let mut filters = HashMap::new();
            filters.insert("scope", vec!["local"]);
            filters.insert("type", vec!["container"]);
            filters
        },
        ..Default::default()
    }));

    let mut last_events = HashMap::<String, Action>::new();

    select! {
        _ = async {
            while let Some(event) = events.next().await {
                let event = Event::new(event?);
                if matches!(event.action, Action::OutOfScope) {
                    continue;
                }

                let span = info_span!("event", name = %event.name());
                let _ = span.enter();

                debug!(parent: &span, "container state changed");

                let previous = last_events.get(&event.id);

                // Restart the service if it exited with a non-zero exit code.
                // Any service that was killed will not be restarted. If there is no previous event
                // and the service exited, it is assumed to be unintentional.
                let exited_non_zero = matches!(event.action, Action::Exit { code } if code != 0);
                if exited_non_zero && (previous.is_none() || matches!(previous, Some(event) if event != &Action::Kill)) {
                    warn!(parent: &span, id = %event.id, "container exited unexpectedly");
                    match instance().start(&event.id).await {
                        Ok(_) => { info!(parent: &span, id = %event.id, "restarted container"); },
                        Err(e) => { error!(parent: &span, error = %e, "failed to restart container"); },
                    }
                }

                last_events.insert(event.id, event.action);
            }

            Ok::<_, bollard::errors::Error>(())
        } => {}
        _ = stop.recv() => {
            info!("stopped deployer event listener");
        }
    }
}

#[derive(Debug, PartialEq)]
enum Action {
    Create,
    Destroy,
    Exit { code: u8 },
    Kill,
    Start,
    Stop,
    OutOfScope,
}

impl Action {
    fn new(name: String, attributes: HashMap<String, String>) -> Self {
        match name.as_str() {
            "create" => Self::Create,
            "destroy" => Self::Destroy,
            "die" => Self::Exit {
                code: attributes.get("exitCode").unwrap().parse().unwrap(),
            },
            "kill" => Self::Kill,
            "start" => Self::Start,
            "stop" => Self::Stop,
            _ => Self::OutOfScope, // Any events we don't care about
        }
    }

    fn name(&self) -> &str {
        match self {
            Self::Create => "create",
            Self::Destroy => "destroy",
            Self::Exit { .. } => "exit",
            Self::Kill => "kill",
            Self::Start => "start",
            Self::Stop => "stop",
            _ => unreachable!(),
        }
    }
}

#[derive(Debug)]
struct Event {
    action: Action,
    id: String,
}

impl Event {
    fn new(source: EventMessage) -> Event {
        let actor = source.actor.unwrap();
        let action = Action::new(source.action.unwrap(), actor.attributes.unwrap());

        Event {
            action,
            id: actor.id.unwrap(),
        }
    }

    fn name(&self) -> &str {
        self.action.name()
    }
}
