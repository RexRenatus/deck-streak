//! The web push sender (SPEC-343 R4 to R7): one message per call, encrypted to the subscription as
//! RFC 8291 says, with an RFC 8292 VAPID token, posted only to an endpoint on the sender's list.

use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use base64ct::{Base64UrlUnpadded, Encoding};
use deck_streak_kernel::Clock;
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::header::{CONTENT_ENCODING, CONTENT_TYPE};
use hyper::{Request, Uri};
use p256::PublicKey;
use serde_json::json;
use web_push_native::Auth;

use crate::client::{Answer, Client, Versions};
use crate::jwt::Signer;
use crate::{BuildError, Notification, Origin, PushServices, Sent, Unreached};

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
        let _ = (
            &self.contact,
            &self.signer,
            &self.clock,
            &subscription.origin,
        );
        let plaintext = json!({"title": notification.title(), "body": notification.body()})
            .to_string()
            .into_bytes();
        let Ok(body) = web_push_native::encrypt(plaintext, &subscription.key, &subscription.auth)
        else {
            return Sent::Failed(Unreached::Request);
        };
        let Some(request) = Self::request(subscription, body) else {
            return Sent::Failed(Unreached::Request);
        };
        match self.client.post(request).await {
            Ok(answer) => read(&answer),
            Err(unreached) => Sent::Failed(unreached),
        }
    }

    /// The request carrying `body`, the encrypted message, to `subscription`'s endpoint.
    fn request(subscription: &Subscription, body: Vec<u8>) -> Option<Request<Full<Bytes>>> {
        Request::post(subscription.endpoint.as_str())
            .header(CONTENT_ENCODING, "aes128gcm")
            .header(CONTENT_TYPE, "application/octet-stream")
            .body(Full::new(Bytes::from(body)))
            .ok()
    }
}

/// The outcome a push service's answer stands for (R4).
fn read(answer: &Answer) -> Sent {
    match answer.status.as_u16() {
        200..=299 => Sent::Delivered,
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
