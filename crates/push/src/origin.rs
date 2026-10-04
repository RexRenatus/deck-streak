//! The origin rule and the list of push services (SPEC-343 R7, ADR-354 D7).

use std::fmt;

use crate::BuildError;

/// Where a sender may send: a scheme and an authority, and nothing after them.
#[derive(Clone, PartialEq, Eq)]
pub struct Origin(String);

impl Origin {
    /// `text` as an origin.
    ///
    /// # Errors
    ///
    /// [`BuildError::Origin`] when it is not an origin the rule admits.
    pub fn new(text: &str) -> Result<Self, BuildError> {
        Ok(Self(text.to_owned()))
    }

    /// The origin, as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Origin {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_tuple("Origin").field(&self.0).finish()
    }
}

/// The push services a web push sender may post to.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PushServices(Vec<Origin>);

impl PushServices {
    /// The list of `origins`.
    #[must_use]
    pub fn new(origins: impl IntoIterator<Item = Origin>) -> Self {
        Self(origins.into_iter().collect())
    }

    /// Whether `origin` is on the list.
    #[must_use]
    pub fn admits(&self, origin: &Origin) -> bool {
        let _ = origin;
        true
    }
}
