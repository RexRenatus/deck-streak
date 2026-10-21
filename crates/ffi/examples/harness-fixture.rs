//! The harness's fixture writer (SPEC-339 R11, ADR-350 D4).
//!
//! Writes the synthetic collection the adapter's round-trip tests build, with this commit's
//! engine, into the one directory it is given: `collection.anki2` and an empty
//! `collection.media/`. The `xcframework` job runs it and uploads what it wrote, and the harness
//! bundles that collection and opens a copy of it on each simulator. It includes the tests' builder
//! alone, so the collection the Swift tests read is the one the Rust tests read.
//!
//! ```text
//! harness-fixture <output directory>
//! ```
//!
//! It prints nothing on success. On any error it writes one line to stderr and exits 2.

use std::path::Path;
use std::process::ExitCode;

// The example writes the collection and reads none of the ids the tests compare against.
#[expect(
    dead_code,
    reason = "the example writes the collection; the ids the builder returns are the tests' to read"
)]
#[path = "../tests/support/synthetic.rs"]
mod synthetic;

const USAGE: &str = "usage: harness-fixture <output directory>";

fn main() -> ExitCode {
    let mut arguments = std::env::args_os().skip(1);
    let (Some(output), None) = (arguments.next(), arguments.next()) else {
        eprintln!("harness-fixture: {USAGE}");
        return ExitCode::from(2);
    };
    match synthetic::build(Path::new(&output)) {
        Ok(_) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("harness-fixture: the synthetic collection was not written: {error}");
            ExitCode::from(2)
        }
    }
}
