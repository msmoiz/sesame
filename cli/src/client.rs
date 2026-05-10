use anyhow::{Context, bail};
use reqwest::blocking::Client as HttpClient;
use reqwest::header::{CONTENT_TYPE, HeaderMap, HeaderValue};
use sesame_model::{
    ErrorInfo, GetSecretInput, GetSecretOutput, ListSecretsInput, ListSecretsOutput,
    PASSWORD_HEADER, PublishSecretInput, PublishSecretOutput,
};

/// The API client.
pub struct Client {
    /// The base URL for the server.
    base_url: String,
    /// The password for the server.
    password: String,
    /// The inner HTTP client.
    http: HttpClient,
}

impl Client {
    /// Creates a new client.
    pub fn new(base_url: String, password: String) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_owned(),
            password,
            http: HttpClient::new(),
        }
    }

    /// Publishes a secret to the store.
    pub fn publish_secret(&self, name: &str, value: &str) -> anyhow::Result<PublishSecretOutput> {
        let response = self
            .http
            .post(format!("{}/publish-secret", self.base_url))
            .headers(self.headers()?)
            .json(&PublishSecretInput {
                name: name.to_owned(),
                value: value.to_owned(),
            })
            .send()
            .context("failed to publish secret")?;

        decode_response(response)
    }

    /// Lists secrets in the store.
    pub fn list_secrets(&self) -> anyhow::Result<ListSecretsOutput> {
        let response = self
            .http
            .post(format!("{}/list-secrets", self.base_url))
            .headers(self.headers()?)
            .json(&ListSecretsInput {})
            .send()
            .context("failed to list secrets")?;

        decode_response(response)
    }

    /// Gets a secret from the store.
    pub fn get_secret(&self, name: &str) -> anyhow::Result<GetSecretOutput> {
        let response = self
            .http
            .post(format!("{}/get-secret", self.base_url))
            .headers(self.headers()?)
            .json(&GetSecretInput {
                name: name.to_owned(),
            })
            .send()
            .with_context(|| format!("failed to fetch secret {name}"))?;

        decode_response(response)
    }

    /// Returns the headers for a request.
    fn headers(&self) -> anyhow::Result<HeaderMap> {
        let password = HeaderValue::from_str(&self.password)
            .context("password contains invalid characters")?;

        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(PASSWORD_HEADER, password);

        Ok(headers)
    }
}

/// Decodes an API response.
fn decode_response<T>(response: reqwest::blocking::Response) -> anyhow::Result<T>
where
    T: serde::de::DeserializeOwned,
{
    let status = response.status();

    if status.is_success() {
        return response
            .json::<T>()
            .context("failed to decode successful response");
    }

    let error = response.json::<ErrorInfo>().unwrap_or_else(|_| ErrorInfo {
        code: String::from("unknown_error"),
        message: format!("request failed with status {status}"),
    });

    bail!("{} ({})", error.message, error.code)
}
