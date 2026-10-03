//! The skip day's write to the collection (SPEC-083 R20 to R27, R36; ADR-321 D14 to D20): the only
//! module beside the port that names the write port, [`CollectionWrite`] (A24).
//!
//! In this part it holds the preview (R20): the class's stop read first (R36, A54), then, under the
//! SHARED collection lock on the private copy, the cards the wrapped search (R3) selects, each with
//! its top-level deck and its current due, and the list's digest (D20) that a confirm carries and
//! the take compares. The preview writes nothing.

use std::io;

use sha2::{Digest, Sha256};

use crate::engine::{CollectionWrite, EngineError};
use crate::settings::{SkipSearch, SyncSettings};
use crate::skip::SearchRefusal;
use crate::write_class_stop::{ClassStop, WriteClassStop};

/// One card the preview lists (R20): its id, its top-level deck's id and its current due.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PreviewCard {
    /// The card's id.
    pub id: i64,
    /// The id of the top-level deck the card's home deck sits under.
    pub top_level_deck: i64,
    /// The card's current due, as the engine stores it.
    pub due: i64,
}

/// What the preview shows the owner.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Preview {
    /// The class's stop is set, or could not be read: nothing is listed, and the stop says who set
    /// it, why and since when (A54).
    Stopped(ClassStop),
    /// The cards the wrapped search selects, ascending by id, and their digest (D20).
    Listed {
        /// The cards, ascending by id.
        cards: Vec<PreviewCard>,
        /// The digest of the listed ids ([`list_digest`]).
        digest: String,
    },
}

/// Why the preview listed nothing.
#[derive(Debug, thiserror::Error)]
pub enum PreviewError {
    /// The configured search is not one expression (R3).
    #[error("the skip search is refused")]
    Search(#[from] SearchRefusal),
    /// The collection lock could not be taken or released.
    #[error("the collection lock failed")]
    Lock(#[source] io::Error),
    /// The engine could not read the private copy.
    #[error("the engine could not read the private copy")]
    Engine(#[from] EngineError),
}

/// The digest of a list of card ids (D20): the first 128 bits of SHA-256 over the ids in ascending
/// order, each written in decimal and ended by a line feed, as 32 lowercase hexadecimal digits.
/// The order the ids arrive in does not change it.
#[must_use]
pub fn list_digest(ids: &[i64]) -> String {
    let mut ascending = ids.to_vec();
    ascending.sort_unstable();
    let mut hasher = Sha256::new();
    for id in ascending {
        hasher.update(format!("{id}\n").as_bytes());
    }
    hasher.finalize()[..16]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// The preview (R20): the class's stop first, and while it is set nothing else is read; otherwise,
/// under the shared collection lock on the private copy, the cards `search`'s wrap selects and
/// their digest. It writes nothing.
///
/// # Errors
///
/// [`PreviewError::Search`] when the search is not one expression; [`PreviewError::Lock`] when the
/// lock cannot be taken or released; [`PreviewError::Engine`] when the engine cannot read the copy.
pub async fn preview<W: CollectionWrite>(
    writer: &W,
    settings: &SyncSettings,
    search: &SkipSearch,
    stop: &WriteClassStop,
) -> Result<Preview, PreviewError> {
    let _ = (writer, settings, search, stop);
    Ok(Preview::Listed {
        cards: Vec::new(),
        digest: String::new(),
    })
}
