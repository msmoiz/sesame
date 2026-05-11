use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const PASSWORD_HEADER: &str = "x-sesame-password";

/// Information about an error that occurred during an API operation.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct ErrorInfo {
    /// The error code.
    pub code: String,
    /// The error message.
    pub message: String,
}

/// An error that can occur during an API operation.
#[derive(Debug, Error)]
pub enum ApiError {
    #[error("password is missing")]
    PasswordMissing,
    #[error("password is invalid")]
    PasswordInvalid,
    #[error("secret name is invalid")]
    InvalidSecretName,
    #[error("secret already exists")]
    SecretAlreadyExists,
    #[error("secret not found")]
    SecretNotFound,
    #[error("internal error")]
    InternalError,
}

impl ApiError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::PasswordMissing => "password_missing",
            Self::PasswordInvalid => "password_invalid",
            Self::InvalidSecretName => "invalid_secret_name",
            Self::SecretAlreadyExists => "secret_already_exists",
            Self::SecretNotFound => "secret_not_found",
            Self::InternalError => "internal_error",
        }
    }
}

impl From<ApiError> for ErrorInfo {
    fn from(value: ApiError) -> Self {
        Self {
            code: value.code().to_owned(),
            message: value.to_string(),
        }
    }
}

/// The encoding of a secret.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Encoding {
    /// A text string in UTF-8.
    Text,
    /// A binary blob.
    Binary,
}

/// The input for the publish secret operation.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct PublishSecretInput {
    /// The name of the secret.
    pub name: String,
    /// The value of the secret.
    pub value: String,
    /// The encoding of the secret.
    pub encoding: Encoding,
}

/// The input for the health operation.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct HealthInput {}

/// The output for the health operation.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct HealthOutput {}

/// The output for the publish secret operation.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct PublishSecretOutput {}

/// The input for the list secrets operation.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct ListSecretsInput {}

/// The output for the list secrets operation.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct ListSecretsOutput {
    /// The list of secret names.
    pub secrets: Vec<String>,
}

/// The input for the get secret operation.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct GetSecretInput {
    /// The name of the secret.
    pub name: String,
}

/// The output of the get secret operation.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct GetSecretOutput {
    /// The name of the secret.
    pub name: String,
    /// The value of the secret.
    pub value: String,
    /// The encoding of the secret.
    pub encoding: Encoding,
}

/// The input for the delete secret operation.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct DeleteSecretInput {
    /// The name of the secret.
    pub name: String,
}

/// The output of the delete secret operation.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct DeleteSecretOutput {}
