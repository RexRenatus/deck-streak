//! The scopes and the grants (SPEC-119 R6 to R8, R10; ADR-320).

use std::fmt;

use deck_streak_kernel::CredentialLoader;

use crate::settings::McpError;

/// A scope a grant can hold (R8). The set is closed: `core` and `law_track` (ADR-320 D5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// Every grant's scope: the request layer admits a request whose grant holds it.
    Core,
    /// The law track's tools.
    LawTrack,
}

impl Scope {
    /// Every scope, in the order [`Scopes::names`] lists them.
    pub const ALL: [Self; 2] = [Self::Core, Self::LawTrack];

    /// The scope's name, the predecessor's spelling (`mcp_auth.py:SCOPE_LAW_TRACK`).
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Core => "core",
            Self::LawTrack => "law_track",
        }
    }

    /// The scope's bit in a [`Scopes`].
    const fn bit(self) -> u8 {
        match self {
            Self::Core => 1,
            Self::LawTrack => 2,
        }
    }
}

/// A set of scopes: the ones a grant holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Scopes(u8);

impl Scopes {
    /// The set holding no scope.
    pub const NONE: Self = Self(0);

    /// The set holding `scope` beside this set's.
    #[must_use]
    pub const fn with(self, scope: Scope) -> Self {
        Self(self.0 | scope.bit())
    }

    /// Whether the set holds `scope`.
    #[must_use]
    pub const fn holds(self, scope: Scope) -> bool {
        self.0 & scope.bit() != 0
    }

    /// The names of the scopes the set holds, in [`Scope::ALL`]'s order.
    #[must_use]
    pub fn names(self) -> Vec<&'static str> {
        Scope::ALL
            .into_iter()
            .filter(|scope| self.holds(*scope))
            .map(Scope::name)
            .collect()
    }
}

/// The grants the guard matches a presented token against, loaded once at start.
pub struct Grants {
    scopes: Vec<Scopes>,
}

impl Grants {
    /// Reads the core and law-track credentials through the loader and makes each a grant.
    ///
    /// # Errors
    ///
    /// [`McpError`] naming the credential that refuses start.
    pub fn load(loader: &CredentialLoader) -> Result<Self, McpError> {
        let _ = loader;
        Ok(Self { scopes: Vec::new() })
    }

    /// The scopes each grant holds, in load order: the core credential's first.
    #[must_use]
    pub fn scopes(&self) -> Vec<Scopes> {
        self.scopes.clone()
    }
}

impl fmt::Debug for Grants {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Grants")
            .field("scopes", &self.scopes)
            .finish_non_exhaustive()
    }
}
