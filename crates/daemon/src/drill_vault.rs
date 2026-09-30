//! The law drills' vault, opened for the two surfaces that answer them (SPEC-110 R13, R15): the
//! bot's `/drills` and the API's drill routes read and answer the same notes through one
//! coordination use case, so both roles open it the same way.
//!
//! A role without a vault configured serves without drills: the setting names are an owner choice
//! (ADR-011), so an unset one is not a start refusal, and the surface says the drills cannot be
//! read. A setting that is set and wrong, or a root that cannot be opened, is logged by its rule
//! and never by its value.

use std::sync::Arc;

use deck_streak_coordination::drills::{DrillNotes, Rails, RealFs, VaultSettings};
use deck_streak_kernel::{Environment, SettingsError};

/// The drill notes of the configured vault, or `None` when none is configured or it cannot be
/// opened.
#[must_use]
pub fn open(env: &Environment) -> Option<Arc<DrillNotes<RealFs>>> {
    let settings = match VaultSettings::from_env(env) {
        Ok(settings) => settings,
        Err(SettingsError::Missing { .. }) => {
            tracing::info!("no vault is configured, so the drills are not served");
            return None;
        }
        Err(error) => {
            tracing::warn!(%error, "the vault settings are refused, so the drills are not served");
            return None;
        }
    };
    let rails = match Rails::vendored() {
        Ok(rails) => rails,
        Err(error) => {
            tracing::warn!(%error, "the vault rails could not be read, so the drills are not served");
            return None;
        }
    };
    match DrillNotes::open(&settings, RealFs, rails) {
        Ok(notes) => Some(Arc::new(notes)),
        Err(error) => {
            tracing::warn!(%error, "the drill notes could not be opened, so the drills are not served");
            None
        }
    }
}
