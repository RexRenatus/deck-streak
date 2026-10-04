//! The push service fake (SPEC-343 R10): a recording server on a loopback port, over HTTP/1.1,
//! that decrypts each body with the test subscription's private key and secret through its own RFC
//! 8291 decryption, and verifies each VAPID token with the `k` the request carried. It answers as
//! each test scripts it, and 201 when nothing is scripted, as a push service answers a message it
//! accepted.

use base64ct::{Base64UrlUnpadded, Encoding};
use p256::ecdsa::VerifyingKey;
use serde_json::Value;

use super::keys::TestSubscriber;
use super::rfc8291::{Undecrypted, decrypt};
use super::{Fake, Recorded, Verified, verify_jwt};

/// A recording push service fake.
pub struct FakePushService {
    /// The recording server.
    pub fake: Fake,
}

/// A VAPID `Authorization` header, read apart: the JWT and the `k` it names.
#[derive(Clone, Debug)]
pub struct Vapid {
    /// The token, `t`.
    pub token: String,
    /// The public key, `k`, in base64url.
    pub key: String,
}

impl FakePushService {
    /// A fake push service.
    pub async fn start() -> Self {
        Self {
            fake: Fake::start(201).await,
        }
    }

    /// The endpoint of a synthetic subscription on this fake.
    pub fn endpoint(&self) -> String {
        format!("{}{}", self.fake.origin(), super::keys::ENDPOINT_PATH)
    }

    /// `request`'s plaintext, decrypted as the browser `subscriber` would.
    pub fn plaintext(request: &Recorded, subscriber: &TestSubscriber) -> Result<Value, Undecrypted> {
        let plaintext = decrypt(&request.body, &subscriber.secret, &subscriber.auth)?;
        Ok(serde_json::from_slice(&plaintext).expect("JSON plaintext"))
    }

    /// The `Authorization` header read as `vapid t=<token>, k=<key>`, or `None` when it is not one.
    pub fn vapid(request: &Recorded) -> Option<Vapid> {
        let rest = request.header("authorization")?.strip_prefix("vapid ")?;
        let (token, key) = rest.split_once(", ")?;
        Some(Vapid {
            token: token.strip_prefix("t=")?.to_owned(),
            key: key.strip_prefix("k=")?.to_owned(),
        })
    }

    /// The VAPID token `request` carried, verified with the `k` it carried.
    pub fn token(request: &Recorded) -> Option<Verified> {
        let vapid = Self::vapid(request)?;
        let point = Base64UrlUnpadded::decode_vec(&vapid.key).ok()?;
        let key = VerifyingKey::from_sec1_bytes(&point).ok()?;
        verify_jwt(&vapid.token, &key)
    }
}
