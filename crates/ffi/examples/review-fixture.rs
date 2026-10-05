//! The review fixture's writer (SPEC-348 R8).
//!
//! Writes the review fixture, with this commit's engine, into the one directory it is given:
//! `collection.anki2`, whose deck `Review` holds a text card, an image card, a sound card and a
//! speech card, and `collection.media/` with the image and the sound. The `xcframework` job runs it
//! and uploads what it wrote beside the harness's fixture. It includes the tests' builder alone, so
//! the collection the Swift tests read is the one the Rust tests read.
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
#[path = "../tests/support/review.rs"]
mod review;

const USAGE: &str = "usage: review-fixture <output directory>";

fn main() -> ExitCode {
    let mut arguments = std::env::args_os().skip(1);
    let (Some(output), None) = (arguments.next(), arguments.next()) else {
        eprintln!("review-fixture: {USAGE}");
        return ExitCode::from(2);
    };
    match review::build(Path::new(&output)) {
        Ok(_) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("review-fixture: the review fixture was not written: {error}");
            ExitCode::from(2)
        }
    }
}
