//! The ES256 signer both tokens use: APNs's provider token and the VAPID token (SPEC-343 R3, R5).
//! One P-256 key per sender, parsed once from its PKCS#8 PEM text and held in memory.

use std::fmt;

use base64ct::{Base64UrlUnpadded, Encoding};
use p256::ecdsa::signature::Signer as _;
use p256::ecdsa::{Signature, SigningKey};
use p256::pkcs8::DecodePrivateKey;
use serde_json::Value;

use crate::BuildError;

/// A sender's signing key.
pub(crate) struct Signer {
    key: SigningKey,
}

impl Signer {
    /// The key `pem` holds, as PKCS#8 PEM text.
    pub(crate) fn from_pem(pem: &str) -> Result<Self, BuildError> {
        SigningKey::from_pkcs8_pem(pem)
            .map(|key| Self { key })
            .map_err(|_| BuildError::Key)
    }

    /// The public key, uncompressed (SEC1), in base64url: VAPID's `k`.
    pub(crate) fn public_key(&self) -> String {
        let point = self.key.verifying_key().to_encoded_point(false);
        Base64UrlUnpadded::encode_string(point.as_bytes())
    }

    /// A compact JWS of `header` and `claims`, signed with ES256: the base64url of each, joined by
    /// dots, and the fixed-size `r||s` signature over the first two. `None` when signing fails.
    pub(crate) fn token(&self, header: &Value, claims: &Value) -> Option<String> {
        let signing_input = format!(
            "{}.{}",
            Base64UrlUnpadded::encode_string(header.to_string().as_bytes()),
            Base64UrlUnpadded::encode_string(claims.to_string().as_bytes()),
        );
        let signature: Signature = self.key.try_sign(signing_input.as_bytes()).ok()?;
        Some(format!(
            "{signing_input}.{}",
            Base64UrlUnpadded::encode_string(&signature.to_bytes())
        ))
    }
}

impl fmt::Debug for Signer {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_struct("Signer").finish_non_exhaustive()
    }
}
