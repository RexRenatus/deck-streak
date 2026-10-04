//! The APNs sender (SPEC-343 R2 to R4): one notification per call, over HTTP/2, with an ES256
//! provider token reused until it is 45 minutes old.

use std::fmt;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use deck_streak_kernel::{Clock, UtcMillis};
use http_body_util::Full;
use hyper::Request;
use hyper::body::Bytes;
use hyper::header::AUTHORIZATION;
use serde_json::{Value, json};

use crate::client::{Answer, Client, Versions};
use crate::jwt::Signer;
use crate::{BuildError, Notification, Origin, Refusal, Sent, Unreached};

/// How old a provider token may grow before the sender mints the next. APNs refuses a token
/// older than an hour, and asks for one no more often than every 20 minutes (R3).
const REFRESH_AGE: Duration = Duration::from_mins(45);

/// The largest payload APNs accepts for a notification, in bytes (R2).
const PAYLOAD_MAX: usize = 4096;

/// The longest a device token may be, in hexadecimal characters.
const DEVICE_TOKEN_MAX: usize = 200;

/// The youngest a refused provider token may be for the sender to mint the next: APNs asks for a
/// new token no more often than every 20 minutes, so a younger refusal is the key's, not the
/// token's age (R3).
const REMINT_FLOOR: Duration = Duration::from_mins(20);

/// How many times one call resends after APNs refuses its provider token as expired (R3, R4).
const RESENDS: u32 = 1;

/// APNs's reason for a provider token it no longer accepts.
const EXPIRED_TOKEN: &str = "ExpiredProviderToken";

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
        let admitted = !token.is_empty()
            && token.len() <= DEVICE_TOKEN_MAX
            && token.bytes().all(|byte| byte.is_ascii_hexdigit());
        if admitted {
            Ok(Self {
                token: token.to_owned(),
                environment,
            })
        } else {
            Err(BuildError::DeviceToken)
        }
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
    token: Mutex<Option<ProviderToken>>,
}

/// A provider token and the instant it was minted. It is never printed: neither it nor the sender
/// that holds it writes the token into a `Debug` (R8).
#[derive(Clone)]
struct ProviderToken {
    text: String,
    minted: UtcMillis,
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
            token: Mutex::new(None),
        })
    }

    /// Sends `notification` to `device` and reads the answer into one outcome.
    pub async fn deliver(&self, device: &Device, notification: &Notification) -> Sent {
        self.attempt(device, notification).await
    }

    /// One call: the request, and R3's one resend when APNs refuses the token as expired.
    async fn attempt(&self, device: &Device, notification: &Notification) -> Sent {
        let body = json!({
            "aps": {"alert": {"title": notification.title(), "body": notification.body()}}
        })
        .to_string();
        if body.len() > PAYLOAD_MAX {
            return Sent::Rejected(Refusal::TooLarge);
        }
        let Some(mut token) = self.current_token(self.clock.now()) else {
            return Sent::Failed(Unreached::Request);
        };
        let mut resent = 0;
        loop {
            let now = self.clock.now();
            let Some(request) = self.request(device, notification, body.clone(), &token.text, now)
            else {
                return Sent::Failed(Unreached::Request);
            };
            let answer = match self.client.post(request).await {
                Ok(answer) => answer,
                Err(unreached) => return Sent::Failed(unreached),
            };
            if !expired(&answer) {
                return read(&answer);
            }
            if resent == RESENDS {
                return Sent::Rejected(Refusal::ProviderToken);
            }
            token = match self.remint(&token) {
                Ok(fresh) => fresh,
                Err(refused) => return refused,
            };
            resent += 1;
        }
    }

    /// The token to send at `now`: the one held while it is younger than [`REFRESH_AGE`], else a
    /// new one, minted and held under the one lock with no await inside it (R3).
    fn current_token(&self, now: UtcMillis) -> Option<ProviderToken> {
        let mut held = self.token.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(token) = held
            .as_ref()
            .filter(|token| age(now, token.minted) < REFRESH_AGE)
        {
            return Some(token.clone());
        }
        let fresh = self.mint(now)?;
        *held = Some(fresh.clone());
        Some(fresh)
    }

    /// The token to resend with after APNs refused `refused` as expired, under the one lock with no
    /// await inside it (R3). A refused token younger than [`REMINT_FLOOR`] is the refusal itself;
    /// a held token other than the refused one was minted by a call refused alongside, and is
    /// reused, so calls refused together mint one token between them.
    fn remint(&self, refused: &ProviderToken) -> Result<ProviderToken, Sent> {
        let now = self.clock.now();
        let mut held = self.token.lock().unwrap_or_else(PoisonError::into_inner);
        if age(now, refused.minted) < REMINT_FLOOR {
            return Err(Sent::Rejected(Refusal::ProviderToken));
        }
        if let Some(current) = held.as_ref().filter(|held| held.text != refused.text) {
            return Ok(current.clone());
        }
        let fresh = self.mint(now).ok_or(Sent::Failed(Unreached::Request))?;
        *held = Some(fresh.clone());
        Ok(fresh)
    }

    /// A provider token minted at `now`: ES256 over `{"alg","kid"}` and `{"iss","iat"}` (R3).
    fn mint(&self, now: UtcMillis) -> Option<ProviderToken> {
        let header = json!({"alg": "ES256", "kid": self.key_id});
        let claims = json!({"iss": self.team_id, "iat": now.epoch_millis().div_euclid(1000)});
        let text = self.signer.token(&header, &claims)?;
        Some(ProviderToken { text, minted: now })
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
        let origin = match device.environment {
            Environment::Development => &self.development,
            Environment::Production => &self.production,
        };
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

/// How long `minted` was before `now`; none when the clock has gone back.
fn age(now: UtcMillis, minted: UtcMillis) -> Duration {
    let millis = now.epoch_millis().saturating_sub(minted.epoch_millis());
    Duration::from_millis(u64::try_from(millis).unwrap_or(0))
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

/// APNs's error body as JSON, when it was read whole inside its bound and parses.
fn error_json(answer: &Answer) -> Option<Value> {
    serde_json::from_slice(answer.error_body.as_ref()?).ok()
}

/// The `reason` of APNs's error body, when it gave one.
fn reason(answer: &Answer) -> Option<String> {
    error_json(answer)?
        .get("reason")?
        .as_str()
        .map(str::to_owned)
}

/// Apple's `timestamp` on a 410: when it last knew the device's token as valid, in epoch
/// milliseconds.
fn timestamp(answer: &Answer) -> Option<UtcMillis> {
    error_json(answer)?
        .get("timestamp")?
        .as_i64()
        .map(UtcMillis::from_epoch_millis)
}

/// Whether a 400's reason says the device token is wrong, or not for this app's topic: the same
/// request would be refused again, so it is never retried (R4).
fn refuses_the_device(answer: &Answer) -> bool {
    matches!(
        reason(answer).as_deref(),
        Some("BadDeviceToken" | "DeviceTokenNotForTopic")
    )
}

/// Whether APNs refused the provider token as expired, which earns R3's one resend.
fn expired(answer: &Answer) -> bool {
    answer.status.as_u16() == 403 && reason(answer).as_deref() == Some(EXPIRED_TOKEN)
}

/// The outcome APNs's answer stands for (R4).
fn read(answer: &Answer) -> Sent {
    match answer.status.as_u16() {
        200..=299 => Sent::Delivered,
        410 => Sent::Gone {
            since: timestamp(answer),
        },
        400 if refuses_the_device(answer) => Sent::Rejected(Refusal::Token),
        403 => Sent::Rejected(Refusal::ProviderToken),
        413 => Sent::Rejected(Refusal::TooLarge),
        429 | 500..=599 => Sent::RetryLater {
            after: answer.retry_after,
        },
        400..=499 => Sent::Rejected(Refusal::Request),
        300..=399 => Sent::Failed(Unreached::Redirect),
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
