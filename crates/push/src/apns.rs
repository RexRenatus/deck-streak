//! The APNs sender (SPEC-343 R2 to R4): one notification per call, over HTTP/2, with an ES256
//! provider token reused until it is 45 minutes old.

use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use deck_streak_kernel::{Clock, UtcMillis};
use http_body_util::Full;
use hyper::Request;
use hyper::body::Bytes;
use hyper::header::AUTHORIZATION;
use serde_json::json;

use crate::client::{Answer, Client, Versions};
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
    /// [`BuildError::Key`] when the key does not parse, and [`BuildError::Tls`] when the connector
    /// cannot be built. A topic no header can carry answers `Failed` on the first call.
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
        let body = json!({
            "aps": {"alert": {"title": notification.title(), "body": notification.body()}}
        })
        .to_string();
        let now = self.clock.now();
        let Some(token) = self.mint(now) else {
            return Sent::Failed(Unreached::Request);
        };
        let Some(request) = self.request(device, notification, body, &token, now) else {
            return Sent::Failed(Unreached::Request);
        };
        match self.client.post(request).await {
            Ok(answer) => read(&answer),
            Err(unreached) => Sent::Failed(unreached),
        }
    }

    /// A provider token minted at `now`: ES256 over `{"alg","kid"}` and `{"iss","iat"}` (R3).
    fn mint(&self, now: UtcMillis) -> Option<String> {
        let header = json!({"alg": "ES256", "kid": self.key_id});
        let claims = json!({"iss": self.team_id, "iat": now.epoch_millis().div_euclid(1000)});
        self.signer.token(&header, &claims)
    }

    /// The request for `notification` to `device`, carrying `token`; `None` when a header cannot
    /// carry a configured value.
    fn request(
        &self,
        device: &Device,
        notification: &Notification,
        body: String,
        token: &str,
        now: UtcMillis,
    ) -> Option<Request<Full<Bytes>>> {
        let _ = &self.production;
        let origin = &self.development;
        let mut request = Request::post(format!("{}/3/device/{}", origin.as_str(), device.token))
            .header(AUTHORIZATION, format!("bearer {token}"))
            .header("apns-push-type", "alert")
            .header("apns-priority", "10")
            .header("apns-topic", self.topic.as_str())
            .header(
                "apns-expiration",
                expiration(now, notification.time_to_live()),
            );
        if let Some(key) = notification.collapse_key() {
            request = request.header("apns-collapse-id", key.as_str());
        }
        request.body(Full::new(Bytes::from(body))).ok()
    }
}

/// `apns-expiration`: the epoch second after which APNs stops trying, the clock's second plus the
/// time to live, or `0` for a notification that lives no time at all (R2).
fn expiration(now: UtcMillis, time_to_live: Duration) -> String {
    if time_to_live.is_zero() {
        return "0".to_owned();
    }
    let seconds = i64::try_from(time_to_live.as_secs()).unwrap_or(i64::MAX);
    now.epoch_millis()
        .div_euclid(1000)
        .saturating_add(seconds)
        .to_string()
}

/// The outcome APNs's answer stands for (R4).
fn read(answer: &Answer) -> Sent {
    match answer.status.as_u16() {
        200..=299 => Sent::Delivered,
        _ => Sent::Failed(Unreached::Unexpected),
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
