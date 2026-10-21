//! Prints the vault adapter's rails verdict on each file it is given, one JSON line per file:
//! `{"file": <the file's name>, "rows": [<each rail row that refuses it>]}`.
//!
//! SPEC-042's A3 runs it over the planted fixtures and compares each verdict with the vault-duties
//! pack's own probe, so the adapter's port of the rails and the pack cannot drift apart unseen.
//!
//! ```text
//! cargo run -p deck-streak-vault --example rails_verdicts -- FILE...
//! ```

use std::collections::BTreeSet;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use deck_streak_vault::Rails;

fn main() -> ExitCode {
    let rails = match Rails::vendored() {
        Ok(rails) => rails,
        Err(error) => {
            let _ = writeln!(io::stderr(), "rails_verdicts: {error}");
            return ExitCode::from(2);
        }
    };
    let mut out = io::stdout().lock();
    for argument in std::env::args_os().skip(1) {
        let path = PathBuf::from(argument);
        let Ok(text) = std::fs::read_to_string(&path) else {
            let _ = writeln!(
                io::stderr(),
                "rails_verdicts: a file cannot be read as UTF-8"
            );
            return ExitCode::from(2);
        };
        let rows: BTreeSet<String> = rails
            .refusals(&text)
            .into_iter()
            .map(|refusal| refusal.row.to_string())
            .collect();
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let line = serde_json::json!({ "file": name, "rows": rows });
        if writeln!(out, "{line}").is_err() {
            return ExitCode::from(2);
        }
    }
    ExitCode::SUCCESS
}
