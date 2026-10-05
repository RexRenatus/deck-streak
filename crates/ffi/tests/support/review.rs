//! The review fixture (SPEC-348 R8): a collection whose deck `Review` holds a text card, an image
//! card, a sound card and a speech card, all new on the default preset, beside a media folder that
//! holds the image and the sound.
//!
//! One builder, written with the engine's own API before the adapter runs. The `review-fixture`
//! example includes this file alone to write the fixture where the Apple job uploads it, and the
//! adapter's tests include it to build the same collection in a scratch directory of their own.
//! The image and the sound are built here from their bytes, never committed as binaries. It returns
//! every failure as an error rather than panicking, because the example is not a test.

#![allow(
    dead_code,
    reason = "the example and each test that includes this file read the part they need"
)]

use std::error::Error;
use std::path::{Path, PathBuf};

/// The deck that holds the fixture's four cards.
pub const DECK: &str = "Review";
/// The image's file name in the media folder.
pub const IMAGE: &str = "dot.png";
/// The sound's file name in the media folder.
pub const SOUND: &str = "tone.wav";
/// The image's width and height, in pixels.
pub const SIDE: u32 = 64;
/// The image card's alternative text.
pub const ALT: &str = "a grey dot";

/// The text card's front: no media and no tag.
const TEXT_FRONT: &str = "a text card";
/// The image card's front.
const IMAGE_FRONT: &str = r#"<img src="dot.png" alt="a grey dot">"#;
/// The sound card's front.
const SOUND_FRONT: &str = "[sound:tone.wav]";
/// The speech card's front.
const SPEECH_FRONT: &str = "[anki:tts lang=en_US]a spoken card[/anki:tts]";
/// Every card's back. The stock Basic answer template opens with `{{FrontSide}}`.
const BACK: &str = "the back";

/// The fixture's four cards, by what each shows.
pub struct Cards {
    /// The text card.
    pub text: i64,
    /// The card whose front shows the image.
    pub image: i64,
    /// The card whose front plays the sound.
    pub sound: i64,
    /// The card whose front speaks through a TTS tag.
    pub speech: i64,
}

/// A review fixture built and closed, ready for the adapter to open.
pub struct Review {
    /// The directory that holds the collection and its media folder.
    pub dir: PathBuf,
    /// The collection file, `collection.anki2` in `dir`.
    pub collection: PathBuf,
    /// The four cards' ids.
    pub cards: Cards,
}

/// Builds the review fixture in `dir` (stubbed: it writes nothing).
///
/// # Errors
///
/// None while stubbed.
#[expect(
    clippy::unnecessary_wraps,
    reason = "the stub writes nothing, and the builder it stands for returns each step's error"
)]
pub fn build(dir: &Path) -> Result<Review, Box<dyn Error>> {
    Ok(Review {
        dir: dir.to_path_buf(),
        collection: dir.join("collection.anki2"),
        cards: Cards {
            text: 0,
            image: 0,
            sound: 0,
            speech: 0,
        },
    })
}
