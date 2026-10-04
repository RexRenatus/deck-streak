//! Reads both targets' lines and the web target's check from the directory it is given, writes
//! `report.md` there, and appends it to the run's summary when `GITHUB_STEP_SUMMARY` names one
//! (SPEC-342 R8). Any refusal is printed by name and exits 1; the rules are the crate's
//! `measure::report`, which this only calls (ADR-353 D6).

use std::fs::{self, OpenOptions};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use deck_streak_fsrs7::measure::report;

fn main() -> ExitCode {
    let Some(dir) = std::env::args_os().nth(1).map(PathBuf::from) else {
        eprintln!("usage: replay_report <report directory>");
        return ExitCode::from(2);
    };
    let read = |name: &str| fs::read_to_string(dir.join(name)).unwrap_or_default();
    let (text, verdict) =
        match report::render(&read("native.log"), &read("wasm.log"), &read("web-check")) {
            Ok(text) => (text, ExitCode::SUCCESS),
            Err(refusals) => {
                let mut text = String::from("## FSRS-7 replay time (SPEC-342): refused\n\n");
                for refusal in &refusals {
                    eprintln!("replay_report: {refusal}");
                    text.push_str(&format!("- {refusal}\n"));
                }
                (text, ExitCode::FAILURE)
            }
        };
    if let Err(error) = write(&dir.join("report.md"), &text, false) {
        eprintln!("replay_report: report.md: {error}");
        return ExitCode::FAILURE;
    }
    let summary = std::env::var_os("GITHUB_STEP_SUMMARY");
    if let Some(Err(error)) = summary.map(|summary| write(Path::new(&summary), &text, true)) {
        eprintln!("replay_report: the step summary: {error}");
        return ExitCode::FAILURE;
    }
    verdict
}

/// Writes `text` to `path`, appending when `append`.
fn write(path: &Path, text: &str, append: bool) -> std::io::Result<()> {
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .append(append)
        .truncate(!append)
        .open(path)?;
    file.write_all(text.as_bytes())
}
