use std::fmt;

/// Possible events that can be emitted
#[derive(Debug)]
pub enum Event<'commit, 'name> {
    Deployment { commit: &'commit str, state: State },
    ServiceUpdate { name: &'name str, state: State },
    ServiceDelete { name: &'name str, state: State },
}

impl<'commit, 'name> Event<'commit, 'name> {
    /// Create a new deployment event
    pub fn deployment<S>(commit: &'commit S, state: State) -> Self
    where
        S: AsRef<str> + ?Sized,
    {
        Self::Deployment {
            commit: commit.as_ref(),
            state,
        }
    }

    /// Create a new service update event
    pub fn service_update<S>(name: &'name S, state: State) -> Self
    where
        S: AsRef<str> + ?Sized,
    {
        Self::ServiceUpdate {
            name: name.as_ref(),
            state,
        }
    }

    /// Create a new service update event
    pub fn service_delete<S>(name: &'name S, state: State) -> Self
    where
        S: AsRef<str> + ?Sized,
    {
        Self::ServiceDelete {
            name: name.as_ref(),
            state,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Event::Deployment { .. } => "deployment",
            Event::ServiceUpdate { .. } => "service update",
            Event::ServiceDelete { .. } => "service delete",
        }
    }
}

impl<'commit, 'name> fmt::Display for Event<'commit, 'name> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// The state of an event
#[derive(Debug)]
pub enum State {
    InProgress,
    Success,
    Failure(String),
}

impl State {
    pub fn as_str(&self) -> &'static str {
        match self {
            State::InProgress => "in progress",
            State::Success => "success",
            State::Failure(_) => "failure",
        }
    }

    pub fn error(&self) -> Option<&str> {
        match self {
            State::InProgress | State::Success => None,
            State::Failure(e) => Some(e),
        }
    }
}

impl AsRef<str> for State {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
