//! The iOS face reader over the collection store's media store (SPEC-601 R6; ADR-621 D7), built
//! switched off: [`StoreMedia`] answers the core's [`Reader`] through the media store's
//! [`CasReader`](deck_streak_collection::CasReader), as [`MediaFolder`](crate::face::MediaFolder)
//! answers it from the engine's media folder. `MediaFolder` stays the live reader, and `CardFace`
//! is unchanged, until the iOS client opens its store; `MediaFolder` leaves in that change.

use deck_streak_collection::MediaStore;
use deck_streak_engine_core::media::Reader;

/// The iOS face reader over one user's media store.
pub struct StoreMedia {
    media: MediaStore,
}

impl StoreMedia {
    /// The reader over `media`.
    #[must_use]
    pub const fn new(media: MediaStore) -> Self {
        Self { media }
    }

    /// The media store it reads.
    #[must_use]
    pub const fn media(&self) -> &MediaStore {
        &self.media
    }
}

impl Reader for StoreMedia {
    /// At most `limit` bytes of the file `name` is bound to, read through the store's
    /// [`CasReader`](deck_streak_collection::CasReader), which checks each file's length.
    fn read(&self, name: &str, limit: u64) -> Option<Vec<u8>> {
        self.media.reader().read(name, limit)
    }

    /// A face over the store inlines fonts, as `MediaFolder` does.
    fn inlines_fonts(&self) -> bool {
        true
    }

    /// A face over the store shows a video, as `MediaFolder` does.
    fn shows_video(&self) -> bool {
        true
    }
}
