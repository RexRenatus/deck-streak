//! The review fixture's writer (SPEC-348 R8).
//!
//! Writes the review fixture, with this commit's engine, into the one directory it is given:
//! `collection.anki2`, whose deck `Review` holds a text card, an image card, a sound card and a
//! speech card, and `collection.media/` with the image and the sound. Beside them it writes a second
//! collection in `occlusion/`, whose deck `Occlusion` holds one image occlusion card, with its own
//! `collection.media/` holding the card's image (SPEC-380 R10). The `xcframework` job runs it and
//! uploads what it wrote beside the harness's fixture. It includes the tests' two builders alone, so
//! the collections the Swift tests read are the ones the Rust tests read.
//!
//! ```text
//! review-fixture <output directory>
//! ```
//!
//! It prints nothing on success. On any error it writes one line to stderr and exits 2.

use std::path::Path;
use std::process::ExitCode;

// The example writes the fixture and reads none of the ids the tests compare against; the
// builder's own module allows what its includers leave unread.
#[path = "../tests/support/occlusion.rs"]
mod occlusion;
#[path = "../tests/support/review.rs"]
mod review;

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
    match written {
        Ok(_) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("review-fixture: the review fixture was not written: {error}");
            ExitCode::from(2)
        }
    }
}
