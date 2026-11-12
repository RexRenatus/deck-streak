//! The APNs fake (SPEC-343 R10): a recording server on a loopback port that speaks HTTP/2 without
//! TLS, by prior knowledge, and verifies each provider token with the test's public key. It answers
//! as each test scripts it, and 200 when nothing is scripted, as APNs answers a notification it
//! accepted.

use p256::ecdsa::VerifyingKey;
use serde_json::{Value, json};

use super::{Answer, Fake, Recorded, Verified, verify_jwt};

/// A recording APNs fake.
pub struct FakeApns {
    /// The recording server.
    pub fake: Fake,
    key: VerifyingKey,
}

impl FakeApns {
    /// A fake that verifies provider tokens with `key`.
    pub async fn start(key: VerifyingKey) -> Self {
        Self {
            fake: Fake::start(200).await,
            key,
        }
    }

    /// The provider token `request` carried, verified with the test's public key; `None` when it
    /// carried none, or one whose signature does not verify.
    pub fn token(&self, request: &Recorded) -> Option<Verified> {
        let bearer = request.header("authorization")?.strip_prefix("bearer ")?;
        verify_jwt(bearer, &self.key)
    }

    /// The raw provider token `request` carried.
    pub fn raw_token(request: &Recorded) -> String {
        request
            .header("authorization")
            .and_then(|value| value.strip_prefix("bearer "))
            .expect("a bearer token")
            .to_owned()
    }

    /// The JSON body `request` carried.
    pub fn json(request: &Recorded) -> Value {
        serde_json::from_slice(&request.body).expect("a JSON body")
    }
}

/// APNs's error answer: `status`, with `{"reason": reason}`.
pub fn refusal(status: u16, reason: &str) -> Answer {
    Answer::status(status).body(json!({ "reason": reason }).to_string())
}

/// APNs's answer for a device that is gone: 410, with its reason and Apple's timestamp in epoch
/// milliseconds.
pub fn gone(reason: &str, timestamp: i64) -> Answer {
    Answer::status(410).body(json!({ "reason": reason, "timestamp": timestamp }).to_string())
}
