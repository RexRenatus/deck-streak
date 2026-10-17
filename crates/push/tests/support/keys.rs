//! Keys made in memory when a test starts, and the synthetic values every test sends (SPEC-343
//! R10). Nothing here is real or copied from any vendor's documentation, and nothing is written to
//! disk: a key's PEM text lives in a `String` for the life of the test.

use base64ct::{Base64UrlUnpadded, Encoding};
use p256::ecdsa::{SigningKey, VerifyingKey};
use p256::elliptic_curve::rand_core::{OsRng, RngCore};
use p256::pkcs8::{EncodePrivateKey, LineEnding};
use p256::{PublicKey, SecretKey};

/// A synthetic device token: one byte repeated, so it can never be a device's.
pub fn device_token() -> String {
    "ab".repeat(32)
}

/// A synthetic APNs key id: it spells TEST.
pub const KEY_ID: &str = "TESTKEYID0";
/// A synthetic developer team id: it spells TEST.
pub const TEAM_ID: &str = "TESTTEAM00";
/// A synthetic bundle id, under the reserved `.invalid` name.
pub const TOPIC: &str = "deckstreak.synthetic.invalid";
/// A synthetic VAPID contact, under the reserved `.invalid` name.
pub const CONTACT: &str = "mailto:push@synthetic.invalid";
/// The path of every synthetic subscription's endpoint.
pub const ENDPOINT_PATH: &str = "/push/synthetic-subscription";

/// A signing key made for one test, with its PKCS#8 PEM text.
pub struct TestKey {
    /// The key.
    pub signing: SigningKey,
    /// Its PKCS#8 PEM text, as a sender is given it.
    pub pem: String,
}

impl TestKey {
    /// A new P-256 key.
    pub fn generate() -> Self {
        let secret = SecretKey::random(&mut OsRng);
        let pem = secret
            .to_pkcs8_pem(LineEnding::LF)
            .expect("the key encodes")
            .to_string();
        Self {
            signing: SigningKey::from(secret),
            pem,
        }
    }

    /// The public key a fake verifies with.
    pub fn verifying(&self) -> VerifyingKey {
        *self.signing.verifying_key()
    }

    /// The PEM's base64 lines, without the armour lines around them: what no log line or `Debug`
    /// may hold.
    pub fn pem_body(&self) -> Vec<String> {
        self.pem
            .lines()
            .filter(|line| !line.starts_with("-----") && !line.is_empty())
            .map(str::to_owned)
            .collect()
    }

    /// The private scalar's bytes as `Debug` would print them.
    pub fn scalar_debug(&self) -> String {
        format!("{:?}", self.signing.to_bytes())
    }
}

/// A browser's side of a synthetic subscription: its P-256 key pair and its authentication secret.
pub struct TestSubscriber {
    /// The browser's private key, which the push service fake decrypts with.
    pub secret: SecretKey,
    /// The authentication secret.
    pub auth: [u8; 16],
}

impl TestSubscriber {
    /// A new key pair and secret.
    pub fn generate() -> Self {
        let mut auth = [0u8; 16];
        OsRng.fill_bytes(&mut auth);
        Self {
            secret: SecretKey::random(&mut OsRng),
            auth,
        }
    }

    /// The public key, uncompressed, in base64url, as `PushSubscription.getKey('p256dh')` gives it.
    pub fn p256dh(&self) -> String {
        let public: PublicKey = self.secret.public_key();
        Base64UrlUnpadded::encode_string(public.to_sec1_bytes().as_ref())
    }

    /// The authentication secret in base64url.
    pub fn auth(&self) -> String {
        Base64UrlUnpadded::encode_string(&self.auth)
    }
}
