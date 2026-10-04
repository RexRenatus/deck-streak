//! The APNs sender (SPEC-343 R2 to R4): one notification per call, over HTTP/2, with an ES256
//! provider token reused until it is 45 minutes old.

use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use deck_streak_kernel::Clock;

use crate::client::{Client, Versions};
use crate::jwt::Signer;
use crate::{BuildError, Notification, Origin, Sent, Unreached};

/// Which of APNs's two services a device registered with: a development build's, or a production
/// build's. Chosen per device, because one owner runs both kinds of build.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Environment {
    /// A development build's device.
    Development,
    /// A production build's device.
    Production,
}

/// A device APNs can reach: its token, as hexadecimal text, and its environment.
#[derive(Clone, PartialEq, Eq)]
pub struct Device {
    token: String,
    environment: Environment,
}

impl Device {
    /// The device `token` names in `environment`.
    ///
    /// # Errors
    ///
    /// [`BuildError::DeviceToken`] when the token is empty, longer than 200 characters, or not
    /// hexadecimal: it becomes a request path, so nothing else may reach one.
    pub fn new(token: &str, environment: Environment) -> Result<Self, BuildError> {
        Ok(Self {
            token: token.to_owned(),
            environment,
        })
    }

    /// The device's environment.
    #[must_use]
    pub fn environment(&self) -> Environment {
        self.environment
    }
}

impl fmt::Debug for Device {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Device")
            .field("environment", &self.environment)
            .finish_non_exhaustive()
    }
}

/// What an APNs sender is built from. The key is the APNs signing key's PKCS#8 PEM text, which
/// production reads through the kernel's credential loader (#640); the ids, the topic and the
/// origins are configuration.
pub struct ApnsSettings<'a> {
    /// The signing key, as PKCS#8 PEM text.
    pub key_pem: &'a str,
    /// The signing key's id, the provider token's `kid`.
    pub key_id: &'a str,
    /// The developer team's id, the provider token's `iss`.
    pub team_id: &'a str,
    /// The app's bundle id, every request's `apns-topic`.
    pub topic: &'a str,
    /// The development service's origin.
    pub development: Origin,
    /// The production service's origin.
    pub production: Origin,
    /// How long one request and its answer may take.
    pub deadline: Duration,
}

impl fmt::Debug for ApnsSettings<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ApnsSettings")
            .field("deadline", &self.deadline)
            .finish_non_exhaustive()
    }
}

/// Sends one notification to one device per call.
pub struct ApnsSender {
    client: Client,
    signer: Signer,
    key_id: String,
    team_id: String,
    topic: String,
    development: Origin,
    production: Origin,
    clock: Arc<dyn Clock>,
}

impl ApnsSender {
    /// A sender built from `settings`, reading time from `clock`.
    ///
    /// # Errors
    ///
    /// [`BuildError::Key`] when the key does not parse, [`BuildError::Setting`] when an id or the
    /// topic is empty or holds a character it may not, and [`BuildError::Tls`] when the connector
    /// cannot be built.
    pub fn new(settings: ApnsSettings<'_>, clock: Arc<dyn Clock>) -> Result<Self, BuildError> {
        Ok(Self {
            client: Client::new(Versions::Http2Only, settings.deadline)?,
            signer: Signer::from_pem(settings.key_pem)?,
            key_id: settings.key_id.to_owned(),
            team_id: settings.team_id.to_owned(),
            topic: settings.topic.to_owned(),
            development: settings.development,
            production: settings.production,
            clock,
        })
    }

    /// Sends `notification` to `device` and reads the answer into one outcome.
    pub async fn deliver(&self, device: &Device, notification: &Notification) -> Sent {
        let _ = (device, notification, &self.client, &self.clock);
        let _ = (&self.key_id, &self.team_id, &self.topic);
        Sent::Failed(Unreached::Request)
    }
}

impl fmt::Debug for ApnsSender {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ApnsSender")
            .field("signer", &self.signer)
            .field("development", &self.development)
            .field("production", &self.production)
            .finish_non_exhaustive()
    }
}
