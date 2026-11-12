//! The voice choice (SPEC-348 R6, A9).
//!
//! The choices file is planted by hand with a damaged line between two good ones, in a directory
//! of its own under the target's scratch space. The test opens it, asks for each language's
//! choice against an installed set, asks for a picker's options, records and clears a choice, and
//! opens the file again to read what survived.
//!
//! The parity tests (SPEC-393 A10 to A12) ask which voice speaks a TTS tag that requests voices,
//! with and without a kept choice, against the installed set and one more `en-US` voice whose name
//! holds a space.

#![allow(
    clippy::expect_used,
    clippy::print_stdout,
    reason = "a failed fixture should fail its test, and an enumerating test prints what it examined"
)]

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use deck_streak_ffi::voices::{Voice, VoiceChoices, VoiceQuality};

fn voice(identifier: &str, name: &str, language: &str, quality: VoiceQuality) -> Voice {
    Voice {
        identifier: identifier.to_owned(),
        name: name.to_owned(),
        language: language.to_owned(),
        quality,
    }
}

fn names(voices: &[Voice]) -> Vec<&str> {
    voices.iter().map(|voice| voice.name.as_str()).collect()
}

/// The installed set: three `en-US` voices, one of each quality, and one each of `en-AU`,
/// `fr-FR` and `de-DE`, in no order.
fn installed() -> Vec<Voice> {
    vec![
        voice("voice.fred", "Fred", "en-US", VoiceQuality::Default),
        voice("voice.karen", "Karen", "en-AU", VoiceQuality::Default),
        voice("voice.thomas", "Thomas", "fr-FR", VoiceQuality::Enhanced),
        voice("voice.zoe", "Zoe", "en-US", VoiceQuality::Premium),
        voice("voice.ava", "Ava", "en-US", VoiceQuality::Enhanced),
        voice("voice.anna", "Anna", "de-DE", VoiceQuality::Default),
    ]
}

/// A directory of the test's own under the target's scratch space.
fn scratch() -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("the clock reads after the epoch")
        .as_nanos();
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("ffi-voices")
        .join(format!("{}-{stamp}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    dir
}

/// Prints how many `what` the test examined and refuses none: a population that came back empty
/// judged nothing, and every assertion over it would pass.
fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}

#[test]
fn a_voice_choice_survives_and_follows_the_installed_set() {
    let dir = scratch();
    let file = dir.join("voices.tsv");
    std::fs::write(
        &file,
        "en-US\tvoice.zoe\nthis line is damaged\nfr-FR\tvoice.thomas\n",
    )
    .expect("the choices file is planted");
    let path = file.to_str().expect("a scratch path is UTF-8").to_owned();

    let installed = installed();

    let choices = VoiceChoices::open(path.clone());
    assert_eq!(
        choices.chosen("en-US".to_owned(), installed.clone()),
        Some("voice.zoe".to_owned()),
        "the line before the damaged one is read"
    );
    assert_eq!(
        choices.chosen("fr-FR".to_owned(), installed.clone()),
        Some("voice.thomas".to_owned()),
        "the damaged line is skipped, and the line after it is read"
    );
    let without_thomas: Vec<Voice> = installed
        .iter()
        .filter(|voice| voice.identifier != "voice.thomas")
        .cloned()
        .collect();
    assert_eq!(
        choices.chosen("fr-FR".to_owned(), without_thomas),
        None,
        "a chosen voice no longer installed is not chosen"
    );

    assert_eq!(
        names(&choices.options("en-US".to_owned(), installed.clone())),
        ["Zoe", "Ava", "Fred"],
        "the language's own voices, premium first"
    );
    assert_eq!(
        names(&choices.options("en-GB".to_owned(), installed.clone())),
        ["Zoe", "Ava", "Fred", "Karen"],
        "no en-GB voice: the primary subtag's, by quality, then by name"
    );
    assert_eq!(
        names(&choices.options("ja-JP".to_owned(), installed.clone())),
        Vec::<&str>::new(),
        "no voice shares the primary subtag"
    );

    choices
        .choose("de-DE".to_owned(), Some("voice.anna".to_owned()))
        .expect("a choice is recorded");
    choices
        .choose("en-US".to_owned(), None)
        .expect("a choice is cleared");

    let reopened = VoiceChoices::open(path);
    assert_eq!(
        reopened.chosen("de-DE".to_owned(), installed.clone()),
        Some("voice.anna".to_owned()),
        "a recorded choice survives reopening"
    );
    assert_eq!(
        reopened.chosen("en-US".to_owned(), installed.clone()),
        None,
        "a cleared choice stays cleared"
    );
    assert_eq!(
        reopened.chosen("fr-FR".to_owned(), installed),
        Some("voice.thomas".to_owned()),
        "a choice not touched survives the rewrite"
    );
    assert_eq!(
        std::fs::read_to_string(&file).expect("the choices file is read"),
        "de-DE\tvoice.anna\nfr-FR\tvoice.thomas\n",
        "the file is written whole, the damaged line gone"
    );
    let mut entries: Vec<String> = examined(
        "scratch entr(ies)",
        std::fs::read_dir(&dir)
            .expect("the scratch directory is read")
            .map(|entry| {
                entry
                    .expect("an entry is read")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect(),
    );
    entries.sort();
    assert_eq!(entries, ["voices.tsv"], "no temporary file is left behind");
}

/// MUTATION COVERAGE (SPEC-348 R6): a line with an empty language, an empty identifier or a second
/// tab is skipped, each by its own line, and the rewrite keeps only the lines that parse.
#[test]
fn a_line_with_an_empty_side_or_a_second_tab_is_skipped() {
    let dir = scratch();
    let file = dir.join("voices.tsv");
    std::fs::write(
        &file,
        "\tvoice.anna\nde-DE\t\nfr-FR\tvoice.thomas\textra\nen-US\tvoice.zoe\n",
    )
    .expect("the choices file is planted");
    let choices = VoiceChoices::open(file.to_str().expect("a scratch path is UTF-8").to_owned());
    choices
        .choose("ja-JP".to_owned(), Some("voice.kyoko".to_owned()))
        .expect("a choice is recorded");
    assert_eq!(
        std::fs::read_to_string(&file).expect("the choices file is read"),
        "en-US\tvoice.zoe\nja-JP\tvoice.kyoko\n",
        "only the lines that parse survive the rewrite"
    );
}

/// MUTATION COVERAGE (SPEC-348 R6): a choice that cannot be written is refused in a sentence that
/// names the reason.
#[test]
fn a_choice_that_cannot_be_written_is_refused_in_words() {
    let missing = scratch().join("no-such-directory").join("voices.tsv");
    let choices = VoiceChoices::open(
        missing
            .to_str()
            .expect("a scratch path is UTF-8")
            .to_owned(),
    );
    let refusal = choices
        .choose("en-US".to_owned(), Some("voice.zoe".to_owned()))
        .expect_err("the directory does not exist");
    assert!(
        refusal
            .to_string()
            .starts_with("the voice choice was not written: "),
        "the refusal reads as a sentence: {refusal}"
    );
}

/// The installed set, and the `en-US` voice `Parity Voice` a parity tag asks for by name.
fn parity_installed() -> Vec<Voice> {
    let mut installed = installed();
    installed.push(voice(
        "test.voice.parity",
        "Parity Voice",
        "en-US",
        VoiceQuality::Default,
    ));
    installed
}

/// Choices opened from a file that does not exist yet, so none is kept.
fn no_choices() -> std::sync::Arc<VoiceChoices> {
    let file = scratch().join("voices.tsv");
    VoiceChoices::open(file.to_str().expect("a scratch path is UTF-8").to_owned())
}

fn requested(entries: &[&str]) -> Vec<String> {
    entries.iter().map(|&entry| entry.to_owned()).collect()
}

/// RED-FIRST (SPEC-393 R8, A10): with no kept choice, the first requested entry naming a voice the
/// picker offers speaks: `Desk_Parity_Voice` names `Parity Voice` after its first `_`, the name's
/// space written `_`.
#[test]
fn a_requested_voice_speaks_when_no_choice_is_kept() {
    let choices = no_choices();
    assert_eq!(
        choices.voice_for(
            "en-US".to_owned(),
            requested(&["Absent_Voice", "Desk_Parity_Voice"]),
            parity_installed(),
        ),
        Some("test.voice.parity".to_owned()),
        "the second entry names the installed voice Parity Voice"
    );
}

/// NOT RED (SPEC-393 R8, A11): a kept choice that is installed speaks, though the request names
/// another installed voice.
#[test]
fn a_kept_choice_speaks_over_a_requested_voice() {
    let choices = no_choices();
    choices
        .choose("en-US".to_owned(), Some("voice.zoe".to_owned()))
        .expect("the choice is written");
    assert_eq!(
        choices.voice_for(
            "en-US".to_owned(),
            requested(&["Desk_Parity_Voice"]),
            parity_installed(),
        ),
        Some("voice.zoe".to_owned()),
        "the kept choice speaks over the requested voice"
    );
}

/// RED-FIRST (SPEC-393 R8, A12): an entry naming a voice of another language, or one not
/// installed, is passed over; an installed voice's identifier speaks before a later entry; and a
/// request whose every entry is passed over answers nothing.
#[test]
fn a_request_the_picker_would_not_offer_is_passed_over() {
    let choices = no_choices();
    assert_eq!(
        choices.voice_for(
            "en-US".to_owned(),
            requested(&["Thomas", "Absent_Voice", "voice.ava", "Zoe"]),
            parity_installed(),
        ),
        Some("voice.ava".to_owned()),
        "a French voice and an absent one are passed over, and the identifier speaks first"
    );
    assert_eq!(
        choices.voice_for(
            "en-US".to_owned(),
            requested(&["Thomas", "Absent_Voice", "voice.karen"]),
            parity_installed(),
        ),
        None,
        "no entry names a voice the picker offers for en-US"
    );
}
