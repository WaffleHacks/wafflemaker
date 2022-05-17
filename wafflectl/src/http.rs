use eyre::Result;
use reqwest::{
    blocking::Client as HTTPClient,
    header::{HeaderMap, HeaderValue, AUTHORIZATION},
};
use serde::{de::DeserializeOwned, Serialize};
use url::Url;

/// A customized HTTP client
pub struct Client {
    inner: HTTPClient,
    base: Url,
}

impl Client {
    /// Create a new HTTP client
    pub fn new(base: Url, token: &str) -> Result<Self> {
        let headers = {
            let mut map = HeaderMap::new();
            map.insert(
                AUTHORIZATION,
                HeaderValue::from_str(&format!("Bearer {}", token))?,
            );
            map
        };
        let inner = HTTPClient::builder()
            .default_headers(headers)
            .user_agent(concat!(
                env!("CARGO_PKG_NAME"),
                "/",
                env!("CARGO_PKG_VERSION")
            ))
            .build()?;

        Ok(Self { inner, base })
    }

    fn full_url<I>(&mut self, segments: I)
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        let mut paths = self.base.path_segments_mut().unwrap();
        for segment in segments {
            paths.push(segment.as_ref());
        }
    }

    /// Send a GET request
    pub fn get<I, R>(mut self, path: I) -> Result<R>
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
        R: DeserializeOwned,
    {
        self.full_url(path);

        let response = self
            .inner
            .get(self.base)
            .send()?
            .error_for_status()?
            .json()?;
        Ok(response)
    }

    /// Send a PUT request with an optional body
    pub fn put<B, I>(mut self, path: I, body: Option<B>) -> Result<()>
    where
        B: Serialize,
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        self.full_url(path);

        let mut request = self.inner.put(self.base);
        if let Some(body) = body {
            request = request.json(&body)
        }

        request.send()?.error_for_status()?;
        Ok(())
    }

    /// Send a DELETE request with optional query parameters
    pub fn delete<I, Q>(mut self, path: I, query: Option<Q>) -> Result<()>
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
        Q: Serialize,
    {
        self.full_url(path);

        let mut request = self.inner.delete(self.base);
        if let Some(query) = query {
            request = request.query(&query);
        }

        request.send()?.error_for_status()?;
        Ok(())
    }
}

/// Ensure service names with slashes `/` in the name are not url encoded
pub fn service_path<'s>(base: &'s str, service: &'s str) -> Vec<&'s str> {
    let mut path = vec![base];
    for part in service.split('/') {
        path.push(part)
    }

    path
}
