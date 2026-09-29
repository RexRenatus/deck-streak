//! The AI route port (SPEC-043 R15, R16; ADR-054): whether this host has a way to run the agent.
//!
//! No AI is the default and a first-class path: an unset route is [`AiRoute::Absent`], a duty then
//! runs nothing and says coaching is unavailable, and only a mistyped value refuses start.

use deck_streak_kernel::{Environment, Setting, SettingsError};

/// The setting that selects the route. Unset or blank means [`AiRoute::Absent`].
pub const AI_ROUTE: &str = "DECKSTREAK_AI_ROUTE";

/// How a duty reaches a model.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AiRoute {
    /// No route: every duty runs its deterministic form.
    #[default]
    Absent,
    /// The subscription proxy, through this context's shell runner (R1 to R5).
    Proxy,
}

impl Setting for AiRoute {
    const SHAPE: &'static str = "proxy, or unset for no AI route";

    fn parse(text: &str) -> Option<Self> {
        let _ = text;
        None
    }
}

impl AiRoute {
    /// The configured route: `Absent` when unset, and an error naming the setting when unknown.
    ///
    /// # Errors
    ///
    /// [`SettingsError::Malformed`] when the value is set and is not a known route.
    pub fn from_env(env: &Environment) -> Result<Self, SettingsError> {
        Ok(env.optional::<Self>(AI_ROUTE)?.unwrap_or_default())
    }

    /// Whether a runner may be launched: the check every duty makes before its caps (R16).
    #[must_use]
    pub const fn is_configured(self) -> bool {
        matches!(self, Self::Proxy)
    }
}
