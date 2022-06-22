use itertools::Itertools;
use shrinkwraprs::Shrinkwrap;
use std::fmt::{Debug, Display, Formatter};

/// The different ways of referring to a service. Can be used as a standard string via [shrinkwraprs::Shrinkwrap]
#[derive(Shrinkwrap)]
pub struct ServiceName {
    /// The proper name of the service by which most things should refer to it
    #[shrinkwrap(main_field)]
    pub proper: String,
    /// The domain/subdomain name of the service
    pub domain: String,
    /// A sanitized version of the name containing only a-z, A-Z, 0-9, -, .
    pub sanitized: String,
}

impl ServiceName {
    pub(super) fn new<S: Into<String>>(name: S) -> ServiceName {
        let proper = name.into();
        let domain = proper.split('/').rev().join(".");
        let sanitized = proper.replace('/', "_");

        ServiceName {
            proper,
            domain,
            sanitized,
        }
    }
}

impl AsRef<str> for ServiceName {
    fn as_ref(&self) -> &str {
        self.proper.as_ref()
    }
}

impl Debug for ServiceName {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.proper, f)
    }
}

impl Display for ServiceName {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.proper, f)
    }
}

impl From<&str> for ServiceName {
    fn from(name: &str) -> Self {
        ServiceName::new(name)
    }
}

impl From<String> for ServiceName {
    fn from(name: String) -> Self {
        ServiceName::new(name)
    }
}

impl From<&String> for ServiceName {
    fn from(name: &String) -> Self {
        ServiceName::new(name)
    }
}
