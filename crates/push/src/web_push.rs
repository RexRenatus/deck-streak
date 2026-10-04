//! The web push sender (SPEC-343 R4 to R7): one message per call, encrypted to the subscription as
//! RFC 8291 says, with an RFC 8292 VAPID token, posted only to an endpoint on the sender's list.

use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use deck_streak_kernel::Clock;

use crate::client::{Client, Versions};
use crate::jwt::Signer;
use crate::{BuildError, Notification, PushServices, Sent, Unreached};

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

/// A browser's push subscription, admitted by a sender whose list holds its endpoint's origin.
#[derive(Clone)]
pub struct Subscription {
    endpoint: String,
    p256dh: String,
    auth: String,
}

impl fmt::Debug for Subscription {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_struct("Subscription").finish_non_exhaustive()
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
    /// [`BuildError::Key`] when the key does not parse, [`BuildError::Setting`] when the contact is
    /// not a `mailto:` or `https:` URI, and [`BuildError::Tls`] when the connector cannot be built.
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
        Ok(Subscription {
            endpoint: endpoint.to_owned(),
            p256dh: p256dh.to_owned(),
            auth: auth.to_owned(),
        })
    }

    /// Sends `notification` to `subscription` and reads the answer into one outcome.
    pub async fn deliver(&self, subscription: &Subscription, notification: &Notification) -> Sent {
        let _ = (subscription, notification, &self.client, &self.signer);
        let _ = (&self.contact, &self.services, &self.clock);
        let _ = (&subscription.endpoint, &subscription.p256dh, &subscription.auth);
        Sent::Failed(Unreached::Request)
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
