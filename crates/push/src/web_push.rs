//! The web push sender (SPEC-343 R4 to R7): one message per call, encrypted to the subscription as
//! RFC 8291 says, with an RFC 8292 VAPID token, posted only to an endpoint on the sender's list.

use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use base64ct::{Base64UrlUnpadded, Encoding};
use deck_streak_kernel::{Clock, UtcMillis};
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::header::{AUTHORIZATION, CONTENT_ENCODING, CONTENT_TYPE};
use hyper::{Request, Uri};
use p256::PublicKey;
use serde_json::json;
use web_push_native::Auth;

use crate::client::{Answer, Client, Versions};
use crate::jwt::Signer;
use crate::{BuildError, Notification, Origin, PushServices, Sent, Unreached};

/// How long a VAPID token is valid for: RFC 8292 allows at most 24 hours, and 12 leaves a push
/// service's clock room to differ (R5).
const VAPID_LIFETIME: Duration = Duration::from_mins(12 * 60);

/// What a web push sender is built from. The key is the VAPID signing key's PKCS#8 PEM text, which
/// production reads through the kernel's credential loader (#640); the contact and the list are
/// configuration.
pub struct WebPushSettings<'a> {
    /// The VAPID signing key, as PKCS#8 PEM text.
    pub key_pem: &'a str,
    /// The contact a push service may reach the sender at, a `mailto:` or `https:` URI: the VAPID
    /// token's `sub`.
    pub contact: &'a str,
    /// The push services the sender may post to.
    pub services: PushServices,
    /// How long one request and its answer may take.
    pub deadline: Duration,
}

impl fmt::Debug for WebPushSettings<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("WebPushSettings")
            .field("deadline", &self.deadline)
            .finish_non_exhaustive()
    }
}

/// A browser's push subscription, admitted by a sender whose list holds its endpoint's origin: its
/// endpoint, that endpoint's origin, and the browser's P-256 public key and authentication secret,
/// parsed once.
#[derive(Clone)]
pub struct Subscription {
    endpoint: String,
    origin: Origin,
    key: PublicKey,
    auth: Auth,
}

impl fmt::Debug for Subscription {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Subscription")
            .finish_non_exhaustive()
    }
}

/// Sends one message to one subscription per call.
pub struct WebPushSender {
    client: Client,
    signer: Signer,
    contact: String,
    services: PushServices,
    clock: Arc<dyn Clock>,
}

impl WebPushSender {
    /// A sender built from `settings`, reading time from `clock`.
    ///
    /// # Errors
    ///
    /// [`BuildError::Key`] when the key does not parse, and [`BuildError::Tls`] when the connector
    /// cannot be built.
    pub fn new(settings: WebPushSettings<'_>, clock: Arc<dyn Clock>) -> Result<Self, BuildError> {
        Ok(Self {
            client: Client::new(Versions::Any, settings.deadline)?,
            signer: Signer::from_pem(settings.key_pem)?,
            contact: settings.contact.to_owned(),
            services: settings.services,
            clock,
        })
    }

    /// The subscription a browser gave: its endpoint, and its P-256 public key and authentication
    /// secret, each in base64url as the browser's `PushSubscription` gives them.
    ///
    /// # Errors
    ///
    /// [`BuildError::OffTheList`] when the endpoint's origin is not on the sender's list, and
    /// [`BuildError::Subscription`] when the endpoint or a key does not parse.
    pub fn subscription(
        &self,
        endpoint: &str,
        p256dh: &str,
        auth: &str,
    ) -> Result<Subscription, BuildError> {
        let uri: Uri = endpoint.parse().map_err(|_| BuildError::Subscription)?;
        let (Some(scheme), Some(authority)) = (uri.scheme_str(), uri.authority()) else {
            return Err(BuildError::Subscription);
        };
        let origin = Origin::new(&format!("{scheme}://{authority}"))?;
        if !self.services.admits(&origin) {
            return Err(BuildError::OffTheList);
        }
        let key = Base64UrlUnpadded::decode_vec(p256dh)
            .ok()
            .and_then(|point| PublicKey::from_sec1_bytes(&point).ok())
            .ok_or(BuildError::Subscription)?;
        let auth: [u8; 16] = Base64UrlUnpadded::decode_vec(auth)
            .ok()
            .and_then(|secret| secret.try_into().ok())
            .ok_or(BuildError::Subscription)?;
        Ok(Subscription {
            endpoint: endpoint.to_owned(),
            origin,
            key,
            auth: Auth::from(auth),
        })
    }

    /// Sends `notification` to `subscription` and reads the answer into one outcome.
    pub async fn deliver(&self, subscription: &Subscription, notification: &Notification) -> Sent {
        self.attempt(subscription, notification).await
    }

    /// One call: the body encrypted to the subscription, one request, and its answer read.
    async fn attempt(&self, subscription: &Subscription, notification: &Notification) -> Sent {
        let plaintext = json!({"title": notification.title(), "body": notification.body()})
            .to_string()
            .into_bytes();
        let Ok(body) = web_push_native::encrypt(plaintext, &subscription.key, &subscription.auth)
        else {
            return Sent::Failed(Unreached::Request);
        };
        let Some(request) = self.request(subscription, notification, body) else {
            return Sent::Failed(Unreached::Request);
        };
        match self.client.post(request).await {
            Ok(answer) => read(&answer),
            Err(unreached) => Sent::Failed(unreached),
        }
    }

    /// The request carrying `body`, the encrypted message, to `subscription`'s endpoint, with
    /// its VAPID token; `None` when the token cannot be signed or a header cannot carry a value.
    fn request(
        &self,
        subscription: &Subscription,
        notification: &Notification,
        body: Vec<u8>,
    ) -> Option<Request<Full<Bytes>>> {
        let token = self.vapid_token(subscription.origin.as_str(), self.clock.now())?;
        let vapid = format!("vapid t={token}, k={}", self.signer.public_key());
        let mut request = Request::post(subscription.endpoint.as_str())
            .header(AUTHORIZATION, vapid)
            .header(CONTENT_ENCODING, "aes128gcm")
            .header(CONTENT_TYPE, "application/octet-stream")
            .header("urgency", "normal");
        // RFC 8030: `TTL` is required, so it is sent even when it is zero.
        request = request.header("ttl", notification.time_to_live().as_secs());
        if let Some(key) = notification.collapse_key() {
            request = request.header("topic", key.as_str());
        }
        request.body(Full::new(Bytes::from(body))).ok()
    }
}

impl WebPushSender {
    /// RFC 8292's token for `audience`, the endpoint's origin, minted at `now`: ES256 over
    /// `{"typ","alg"}` and `{"aud","exp","sub"}`, valid for [`VAPID_LIFETIME`] (R5).
    fn vapid_token(&self, audience: &str, now: UtcMillis) -> Option<String> {
        let lifetime = i64::try_from(VAPID_LIFETIME.as_secs()).ok()?;
        let expiry = now.epoch_millis().div_euclid(1000).saturating_add(lifetime);
        let header = json!({"typ": "JWT", "alg": "ES256"});
        let claims = json!({"aud": audience, "exp": expiry, "sub": self.contact});
        self.signer.token(&header, &claims)
    }
}

/// The outcome a push service's answer stands for (R4).
fn read(answer: &Answer) -> Sent {
    match answer.status.as_u16() {
        200..=299 => Sent::Delivered,
        404 | 410 => Sent::Gone { since: None },
        _ => Sent::Failed(Unreached::Unexpected),
    }
}

impl fmt::Debug for WebPushSender {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("WebPushSender")
            .field("signer", &self.signer)
            .field("services", &self.services)
            .finish_non_exhaustive()
    }
}
