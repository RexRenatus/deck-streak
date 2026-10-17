//! The track: language or law, a first-class dimension of XP, streaks and the daily rollup
//! (SPEC-020 R8, docs/LEXICON.md).

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// Language or law: the two tracks every study event belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Track {
    /// The language track.
    Language,
    /// The law track.
    Law,
}

impl Track {
    /// Both tracks, in their serialised order.
    pub const ALL: [Self; 2] = [Self::Language, Self::Law];

    /// The track's name as it is serialised, stored and shown: `language` or `law`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Language => "language",
            Self::Law => "law",
        }
    }
}

impl fmt::Display for Track {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A name that is neither `language` nor `law`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("a track is `language` or `law`")]
pub struct UnknownTrack;

impl FromStr for Track {
    type Err = UnknownTrack;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|track| track.as_str() == text)
            .ok_or(UnknownTrack)
    }
}

#[cfg(test)]
mod tests {
    use super::Track;

    #[test]
    fn a_track_is_serialised_as_language_and_law() {
        assert_eq!(
            serde_json::to_string(&Track::ALL).ok().as_deref(),
            Some(r#"["language","law"]"#)
        );
        assert_eq!(
            serde_json::from_str::<Track>(r#""law""#).ok(),
            Some(Track::Law)
        );
        assert_eq!("language".parse::<Track>(), Ok(Track::Language));
        assert_eq!(Track::Law.to_string(), "law");
    }

    #[test]
    fn a_name_that_is_not_a_track_is_refused() {
        assert_eq!(
            "Law"
                .parse::<Track>()
                .map_err(|refusal| refusal.to_string()),
            Err("a track is `language` or `law`".to_owned())
        );
    }
}
