//! The review fixture's writer (SPEC-348 R8).
//!
//! Writes the review fixture, with this commit's engine, into the one directory it is given:
//! `collection.anki2`, whose deck `Review` holds a text card, an image card, a sound card and a
//! speech card, and `collection.media/` with the image and the sound; and beside them `parity/`, the
//! parity collection (SPEC-393 R15), whose deck `Parity` holds one note's two cards over a font and
//! a video, `sound-video/`, the sound video collection (SPEC-410 R8), whose deck `Sound video`
//! holds one card whose sound tags name three videos and a sound, and `health/`, the card health
//! collection (SPEC-507 R17), whose deck `Health` holds one card with a review record. The
//! `xcframework` job runs it and uploads what it wrote beside the harness's fixture. It includes the
//! tests' five builders alone, so the collections the Swift tests read are the ones the Rust tests
//! read.
//! Beside them it writes a second collection in `occlusion/`, whose deck `Occlusion` holds one
//! image occlusion card, with its own `collection.media/` holding the card's image (SPEC-380 R10).
//!
//! ```text
//! review-fixture <output directory>
//! ```
//!
//! It prints nothing on success. On any error it writes one line to stderr and exits 2.
//!
//! After it writes the review collection, it sets that collection's rollover hour half a day ahead
//! of its own reading, through the day pin it includes beside the four builders (SPEC-615 R1).

use std::path::Path;
use std::process::ExitCode;

// The example writes the fixture and reads none of the ids the tests compare against; the
// builder's own module allows what its includers leave unread.
#[path = "../tests/support/occlusion.rs"]
mod occlusion;
#[path = "../tests/support/review.rs"]
mod review;

#[path = "../tests/support/parity.rs"]
mod parity;

#[path = "../tests/support/sound_video.rs"]
mod sound_video;

#[path = "../tests/support/health.rs"]
mod health;

#[path = "../tests/support/day_cutoff.rs"]
mod day_cutoff;

const USAGE: &str = "usage: review-fixture <output directory>";

fn main() -> ExitCode {
    let mut arguments = std::env::args_os().skip(1);
    let (Some(output), None) = (arguments.next(), arguments.next()) else {
        eprintln!("review-fixture: {USAGE}");
        return ExitCode::from(2);
    };
    let output = Path::new(&output);
    let written =
        review::build(output).and_then(|_| occlusion::build(&output.join(occlusion::DIRECTORY)));
    if let Err(error) = written {
        eprintln!("review-fixture: the review fixture was not written: {error}");
        return ExitCode::from(2);
    }
    if let Err(error) = day_cutoff::pin(&output.join("collection.anki2")) {
        eprintln!("review-fixture: the review fixture day was not pinned: {error}");
        return ExitCode::from(2);
    }
    if let Err(error) = parity::build(&output.join("parity")) {
        eprintln!("review-fixture: the parity collection was not written: {error}");
        return ExitCode::from(2);
    }
    if let Err(error) = sound_video::build(&output.join("sound-video")) {
        eprintln!("review-fixture: the sound video collection was not written: {error}");
        return ExitCode::from(2);
    }
    match health::build(&output.join("health")) {
        Ok(_) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("review-fixture: the card health collection was not written: {error}");
            ExitCode::from(2)
        }
    }
}
