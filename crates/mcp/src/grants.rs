//! The scopes and the grants (SPEC-119 R6 to R8, R10; ADR-320).

use std::fmt;
use std::iter;

use deck_streak_kernel::{CredentialError, CredentialLoader};
use sha2::{Digest, Sha256};
use subtle::{Choice, ConditionallySelectable, ConstantTimeEq};

use crate::settings::{CORE_CREDENTIAL, LAW_TRACK_CREDENTIAL, MIN_CREDENTIAL_CHARS, McpError};

/// A scope a grant can hold (R8). The set is closed: `core` and `law_track` (ADR-320 D5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// Every grant's scope: the request layer admits a request whose grant holds it.
    Core,
    /// The law track's tools.
    LawTrack,
    /// The tools that change data: the write grant's alone (SPEC-369 R1, R6, R7).
    Write,
}

impl Scope {
    /// Every scope, in the order [`Scopes::names`] lists them.
    pub const ALL: [Self; 3] = [Self::Core, Self::LawTrack, Self::Write];

    /// The scope's name, the predecessor's spelling (`mcp_auth.py:SCOPE_LAW_TRACK`).
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Core => "core",
            Self::LawTrack => "law_track",
            Self::Write => "write",
        }
    }

    /// The scope's bit in a [`Scopes`].
    const fn bit(self) -> u8 {
        match self {
            Self::Core => 1,
            Self::LawTrack => 2,
            Self::Write => 4,
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

    /// This set with `other`'s scopes added when `choice` is set, chosen in constant time: the
    /// guard's match folds every grant's scopes and digest comparison through it, with no branch
    /// on a digest (R10).
    pub(crate) fn or_if(self, (other, choice): (Self, Choice)) -> Self {
        Self(self.0 | u8::conditional_select(&0, &other.0, choice))
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

/// One grant: its token's SHA-256 digest, computed once at load, and the scopes it holds (R8).
/// The token itself is never kept.
pub(crate) struct Grant {
    /// The SHA-256 digest of the grant's token.
    pub(crate) digest: [u8; 32],
    /// The scopes the grant holds.
    pub(crate) scopes: Scopes,
}

impl Grant {
    /// The grant of the credential `id` holding `token`, or why it refuses start (R7 as amended by
    /// T15): a byte outside 0x21 to 0x7E first, which no request can present, then fewer than
    /// [`MIN_CREDENTIAL_CHARS`] characters.
    fn of(id: &'static str, token: &str, scopes: Scopes) -> Result<Self, McpError> {
        if !token.bytes().all(|byte| byte.is_ascii_graphic()) {
            return Err(McpError::UnpresentableCredential { id });
        }
        if token.chars().count() < MIN_CREDENTIAL_CHARS {
            return Err(McpError::WeakCredential { id });
        }
        Ok(Self {
            digest: Sha256::digest(token.as_bytes()).into(),
            scopes,
        })
    }
}

impl fmt::Debug for Grant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Grant")
            .field("scopes", &self.scopes)
            .finish_non_exhaustive()
    }
}

/// The grants the guard matches a presented token against, loaded once at start.
pub struct Grants {
    grants: Vec<Grant>,
}

impl Grants {
    /// Reads the core and law-track credentials through the loader and makes each a grant.
    ///
    /// The core credential is required, so every loader error refuses start by its id. A missing
    /// law-track credential grants nothing, and any other loader error refuses start by its id
    /// (R6). Two credentials holding one value refuse start (R7).
    ///
    /// # Errors
    ///
    /// [`McpError`] naming the credential that refuses start.
    pub fn load(loader: &CredentialLoader) -> Result<Self, McpError> {
        let core = Grant::of(
            CORE_CREDENTIAL,
            loader.load(CORE_CREDENTIAL)?.expose(),
            Scopes::NONE.with(Scope::Core),
        )?;
        let law_track = match loader.load(LAW_TRACK_CREDENTIAL) {
            Ok(secret) => Some(Grant::of(
                LAW_TRACK_CREDENTIAL,
                secret.expose(),
                Scopes::NONE.with(Scope::Core).with(Scope::LawTrack),
            )?),
            Err(CredentialError::Missing { .. }) => None,
            Err(error) => return Err(error.into()),
        };
        if let Some(law_track) = &law_track
            && bool::from(law_track.digest.ct_eq(&core.digest))
        {
            return Err(McpError::SharedCredential {
                first: CORE_CREDENTIAL,
                second: LAW_TRACK_CREDENTIAL,
            });
        }
        Ok(Self {
            grants: iter::once(core).chain(law_track).collect(),
        })
    }

    /// The scopes each grant holds, in load order: the core credential's first.
    #[must_use]
    pub fn scopes(&self) -> Vec<Scopes> {
        self.grants.iter().map(|grant| grant.scopes).collect()
    }

    /// Every grant, for the guard's match.
    pub(crate) fn grants(&self) -> &[Grant] {
        &self.grants
    }
}

impl fmt::Debug for Grants {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Grants")
            .field("scopes", &self.scopes())
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn or_if_keeps_a_scope_already_held() {
        let held = Scopes::NONE.with(Scope::Core);
        let kept = held.or_if((held, Choice::from(1)));
        assert!(kept.holds(Scope::Core), "{kept:?}");
        assert_eq!(kept, held);
    }

    #[test]
    fn a_grant_debug_names_its_type_and_never_its_token() {
        let token = "t".repeat(MIN_CREDENTIAL_CHARS) + &std::process::id().to_string();
        let grant = Grant::of("core", &token, Scopes::NONE.with(Scope::Core)).expect("a grant");
        let shown = format!("{grant:?}");
        let hex = grant
            .digest
            .iter()
            .fold(String::new(), |hex, byte| hex + &format!("{byte:02x}"));
        assert!(shown.contains("Grant"), "{shown}");
        assert!(shown.contains("scopes"), "{shown}");
        assert!(!shown.contains(&token), "{shown}");
        assert!(!shown.contains(&hex), "{shown}");
    }
}
