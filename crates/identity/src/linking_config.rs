//! The public origin that turns linking on, and the relying party it names (SPEC-359 R1; ADR-370).
//!
//! The setting [`PUBLIC_ORIGIN`] is the web client's `https` origin: a scheme, a host and an
//! optional port, nothing else. Unset or blank, linking is off and every route of SPEC-359 answers
//! 404 `linking_off`. A value of any other shape refuses start by the setting's name, never by its
//! value. The relying party id is the origin's host, and the expected origin is that origin
//! exactly: one entry, built once at start, never the request's `Origin` header.

use std::fmt;
use std::sync::Arc;

use deck_streak_kernel::{Environment, Setting, SettingsError};
use webauthn_rs::prelude::Url;
use webauthn_rs::{Webauthn, WebauthnBuilder};

use crate::Refusal;

/// The web client's `https` origin; unset, linking is off.
pub const PUBLIC_ORIGIN: &str = "DECKSTREAK_PUBLIC_ORIGIN";
/// The shape the setting must have, named in a refusal in place of the value.
const SHAPE: &str = <OriginText as Setting>::SHAPE;
/// The relying party's name, as the browser shows it: fixed text, no personal data.
const RP_NAME: &str = "DeckStreak";

/// The setting's text, read whole so this module decides what a blank value means.
struct OriginText(String);

impl Setting for OriginText {
    const SHAPE: &'static str =
        "an https origin: the scheme, a host and an optional port, nothing else";

    fn parse(text: &str) -> Option<Self> {
        Some(Self(text.to_owned()))
    }
}

/// The relying party of every passkey ceremony: R1's origin and its host, built once.
#[derive(Clone)]
pub struct RelyingParty {
    webauthn: Arc<Webauthn>,
}

impl RelyingParty {
    /// The ceremonies' verifier, bound to the one origin.
    #[must_use]
    pub fn webauthn(&self) -> &Webauthn {
        &self.webauthn
    }
}

impl fmt::Debug for RelyingParty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("RelyingParty { .. }")
    }
}

/// Whether linking is on, and its relying party when it is.
#[derive(Clone, Debug, Default)]
pub struct LinkingConfig {
    relying_party: Option<RelyingParty>,
}

impl LinkingConfig {
    /// Linking off: every route of SPEC-359 answers `linking_off`.
    #[must_use]
    pub fn off() -> Self {
        Self::default()
    }

    /// The configuration [`PUBLIC_ORIGIN`] sets.
    ///
    /// # Errors
    ///
    /// [`SettingsError::Malformed`] naming [`PUBLIC_ORIGIN`] when it is set and is not an `https`
    /// origin.
    pub fn from_env(env: &Environment) -> Result<Self, SettingsError> {
        let text = env.optional::<OriginText>(PUBLIC_ORIGIN)?;
        Self::from_setting(text.as_ref().map(|text| text.0.as_str()))
    }

    /// The configuration the setting's text `value` gives, `None` being unset.
    ///
    /// # Errors
    ///
    /// [`SettingsError::Malformed`] naming [`PUBLIC_ORIGIN`] when `value` is not blank and is not
    /// an `https` origin.
    pub fn from_setting(value: Option<&str>) -> Result<Self, SettingsError> {
        let malformed = || SettingsError::Malformed {
            setting: PUBLIC_ORIGIN,
            expected: SHAPE,
        };
        let Some(text) = value.map(str::trim).filter(|text| !text.is_empty()) else {
            return Ok(Self::off());
        };
        let origin = Url::parse(text).map_err(|_| malformed())?;
        if origin.scheme() != "https" {
            return Err(malformed());
        }
        // An origin alone: a path, a query, a fragment or a user name makes the text more.
        if origin.origin().ascii_serialization() != text {
            return Err(malformed());
        }
        let host = origin.host_str().ok_or_else(malformed)?.to_owned();
        let webauthn = WebauthnBuilder::new(&host, &origin)
            .and_then(|builder| builder.rp_name(RP_NAME).build())
            .map_err(|_| malformed())?;
        Ok(Self {
            relying_party: Some(RelyingParty {
                webauthn: Arc::new(webauthn),
            }),
        })
    }

    /// The relying party.
    ///
    /// # Errors
    ///
    /// [`Refusal::LinkingOff`] when no public origin is configured.
    pub fn relying_party(&self) -> Result<&RelyingParty, Refusal> {
        self.relying_party.as_ref().ok_or(Refusal::LinkingOff)
    }
}
