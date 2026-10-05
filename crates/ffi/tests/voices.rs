//! The voice choice (SPEC-348 R6, A9).
//!
//! The choices file is planted by hand with a damaged line between two good ones, in a directory
//! of its own under the target's scratch space. The test opens it, asks for each language's
//! choice against an installed set, asks for a picker's options, records and clears a choice, and
//! opens the file again to read what survived.

#![allow(clippy::expect_used, reason = "a failed fixture should fail its test")]

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
    let mut entries: Vec<String> = std::fs::read_dir(&dir)
        .expect("the scratch directory is read")
        .map(|entry| {
            entry
                .expect("an entry is read")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    entries.sort();
    assert_eq!(entries, ["voices.tsv"], "no temporary file is left behind");
}
