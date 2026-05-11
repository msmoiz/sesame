use serde::{Deserialize, Serialize};
use sesame_model::Encoding;

/// A stored secret.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Secret {
    /// The value of the secret.
    pub value: Vec<u8>,
    /// The encoding of the secret.
    pub encoding: Encoding,
}
