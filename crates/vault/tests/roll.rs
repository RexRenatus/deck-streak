//! A reading note rolls forward as the predecessor rolled it (SPEC-042 A4, R7), the owner's tick
//! and every line ending survive a roll byte for byte (A5), and a second roll-forward on the same
//! study day changes nothing while a malformed note is reported and its siblings still move (A7, R8).

// An integration test is test code: its helpers panic on an unreadable file, and it prints the
// examined count on purpose. clippy.toml's in-test allowances cover only `#[test]` bodies.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use deck_streak_kernel::{Environment, StudyDay};
use deck_streak_vault::config::{ARCHIVE_FOLDER, READINGS_FOLDER, VAULT_ROOT};
use deck_streak_vault::readings_tree::archive_folder;
use deck_streak_vault::{
    FolderName, Malformed, Rails, ReadingsTree, RealFs, RollFailure, RollFailureReason, TopicKey,
    VaultSettings, note,
};
use serde_json::Value;
use tempfile::TempDir;

const READINGS: &str = "12-Readings";
const ARCHIVE: &str = "Archive";
const DIGEST: &str = "5d41402abc4b2a76b9719d911017c592aaf1d7f2c3b4e5a69788796a5b4c3d2e";

/// Prints how many items a check examined and refuses zero (the tdd pack's examined contract).
fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}

/// A temporary vault with an empty readings folder, and the date tree over it.
fn vault() -> (TempDir, ReadingsTree<RealFs>) {
    let dir = tempfile::tempdir().expect("a temporary vault");
    fs::create_dir(dir.path().join(READINGS)).expect("the readings folder");
    let settings = VaultSettings::from_env(&Environment::from_vars([
        (VAULT_ROOT, dir.path().as_os_str().to_owned()),
        (READINGS_FOLDER, OsString::from(READINGS)),
        (ARCHIVE_FOLDER, OsString::from(ARCHIVE)),
    ]))
    .expect("the vault's settings");
    let rails = Rails::vendored().expect("the vendored rails");
    let tree = ReadingsTree::open(&settings, RealFs, rails).expect("the vault opens");
    (dir, tree)
}

/// Every file and folder under `root`, each file with its bytes, keyed by its relative path.
fn snapshot(root: &Path) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
    let mut found = BTreeMap::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in fs::read_dir(&dir).expect("a readable folder") {
            let path = entry.expect("a readable entry").path();
            let relative = path
                .strip_prefix(root)
                .expect("under the root")
                .to_path_buf();
            if path.is_dir() {
                pending.push(path);
                found.insert(relative, None);
            } else {
                found.insert(relative, Some(fs::read(&path).expect("a readable file")));
            }
        }
    }
    examined("vault entries", found.into_iter().collect())
        .into_iter()
        .collect()
}

/// `text` with every `{day:N}` token written as the ISO date of study day N, as the golden's adapter
/// wrote the predecessor's days.
fn expand(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(at) = rest.find("{day:") {
        out.push_str(&rest[..at]);
        let after = &rest[at + "{day:".len()..];
        let end = after.find('}').expect("a closed day token");
        let day: i64 = after[..end].parse().expect("an epoch day number");
        out.push_str(&StudyDay::from_epoch_day(day).to_string());
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    out
}

/// A note of `topic` for `day` as it sits in the vault, with `newline` line endings and the boxes
/// as the owner left them.
fn note_text(topic: &str, day: StudyDay, body: &str, read: bool, newline: &str) -> String {
    let read_box = if read {
        "- [x] I read it"
    } else {
        "- [ ] I read it"
    };
    let lines = [
        "---".to_owned(),
        "type: reading".to_owned(),
        format!("topic: {topic}"),
        format!("date: {day}"),
        format!("first_generated: {day}"),
        format!("last_rolled: {day}"),
        "rolls: 0".to_owned(),
        format!("digest: {DIGEST}"),
        format!("tags: [reading, {topic}]"),
        "ai_generated: true".to_owned(),
        "---".to_owned(),
        body.to_owned(),
        String::new(),
        "- [ ] Studied".to_owned(),
        read_box.to_owned(),
        String::new(),
    ];
    lines.join(newline)
}

#[test]
fn rolling_a_note_matches_the_parity_golden() {
    let mut refused = 0;
    let examined = golden::each_case("roll_note_text", |case| {
        let text = expand(case.input["text"].as_str().expect("a note's text"));
        let today = StudyDay::from_epoch_day(case.input["today"].as_i64().expect("an epoch day"));
        let rolled = note::roll(&text, today);
        match (&case.output["text"], &case.output["refused"]) {
            (Value::String(expected), _) if case.class.as_deref() != Some("python-only") => {
                assert_eq!(
                    rolled.as_deref(),
                    Ok(expand(expected).as_str()),
                    "the roll of {}",
                    case.input
                );
            }
            (Value::String(_), _) => {
                // ADR-042: a rolls spelling only Python's int reads is refused as malformed.
                assert_eq!(
                    rolled,
                    Err(Malformed::RollsNotAWholeNumber),
                    "the python-only case {}",
                    case.input
                );
                refused += 1;
            }
            (_, Value::String(exception)) if exception == "ValueError" => {
                assert_eq!(
                    rolled,
                    Err(Malformed::RollsNotAWholeNumber),
                    "the refused case {}",
                    case.input
                );
                refused += 1;
            }
            (_, Value::String(exception)) if exception == "MalformedReadingNoteError" => {
                assert!(
                    matches!(
                        rolled,
                        Err(Malformed::NoOpeningDelimiter | Malformed::NoClosingDelimiter)
                    ),
                    "the refused case {} rolled as {rolled:?}",
                    case.input
                );
                refused += 1;
            }
            (text, exception) => panic!("a case whose output is neither: {text} {exception}"),
        }
    });
    // The registry draws 53 cases, 9 of which the port refuses: 6 the predecessor refused too, and
    // 3 spellings only Python's int reads.
    assert_eq!(
        (examined.count, refused),
        (53, 9),
        "the cases examined and refused"
    );
}

#[test]
fn rolling_forward_keeps_the_owners_tick_byte_for_byte() {
    let (dir, tree) = vault();
    let yesterday = StudyDay::from_epoch_day(20_000);
    let today = StudyDay::from_epoch_day(20_001);
    let topic = TopicKey::new("law/evidence").expect("a topic key");
    let body = "Hearsay is an out-of-court statement.\r\nIts exceptions are listed by rule.";
    // The note as the owner's device left it: CRLF line endings, and the owner's tick.
    let source = note_text("law/evidence", yesterday, body, true, "\r\n");
    let source_path = dir.path().join(tree.note_path(yesterday, &topic));
    fs::create_dir(source_path.parent().expect("the day folder")).expect("yesterday's folder");
    fs::write(&source_path, &source).expect("yesterday's note");

    let report = tree
        .roll_forward(today, std::slice::from_ref(&topic))
        .expect("the roll-forward runs");

    assert_eq!(report.rolled, vec![tree.note_path(today, &topic)]);
    let rolled = fs::read_to_string(dir.path().join(tree.note_path(today, &topic)))
        .expect("today's rolled note");
    let expected = source.replace("rolls: 0\r\n", "rolls: 1\r\n").replace(
        &format!("last_rolled: {yesterday}\r\n"),
        &format!("last_rolled: {today}\r\n"),
    );
    assert_eq!(
        rolled, expected,
        "every byte but rolls and last_rolled is kept"
    );
    assert!(
        rolled.ends_with("\r\n- [ ] Studied\r\n- [x] I read it\r\n"),
        "the owner's tick is kept"
    );
    assert_eq!(
        rolled.matches("\r\n").count(),
        rolled.matches('\n').count(),
        "every line ending is still CRLF"
    );
    assert!(!source_path.exists(), "the source was moved, not copied");
}

#[test]
fn a_second_roll_forward_changes_nothing_and_a_malformed_note_is_reported() {
    let (dir, tree) = vault();
    let older = StudyDay::from_epoch_day(20_090);
    let yesterday = StudyDay::from_epoch_day(20_100);
    let today = StudyDay::from_epoch_day(20_101);
    let evidence = TopicKey::new("law/evidence").expect("a topic key");
    let torts = TopicKey::new("law/torts").expect("a topic key");
    let japanese = TopicKey::new("language/ja").expect("a topic key");
    let lsat = TopicKey::new("lsat").expect("a topic key");
    let write = |day: StudyDay, topic: &TopicKey, text: &str| {
        let path = dir.path().join(tree.note_path(day, topic));
        fs::create_dir_all(path.parent().expect("the day folder")).expect("a day folder");
        fs::write(path, text).expect("a note");
    };
    // Yesterday: a carried note, a note that is not carried, and a carried note whose frontmatter
    // never closes. An older day: a note left from a night that did not roll.
    write(
        yesterday,
        &evidence,
        &note_text(
            "law/evidence",
            yesterday,
            "A primer on hearsay.",
            false,
            "\n",
        ),
    );
    write(
        yesterday,
        &torts,
        &note_text("law/torts", yesterday, "A primer on duty.", true, "\n"),
    );
    let malformed =
        "---\ntype: reading\ntopic: language/ja\nrolls: 0\nA body with no closing line.\n";
    write(yesterday, &japanese, malformed);
    write(
        older,
        &lsat,
        &note_text("lsat", older, "A primer on flaws.", false, "\n"),
    );
    let readings = FolderName::new(READINGS).expect("a folder name");
    let archive = FolderName::new(ARCHIVE).expect("a folder name");
    let carried = [evidence.clone(), japanese.clone()];

    let first = tree
        .roll_forward(today, &carried)
        .expect("the first roll-forward");

    assert_eq!(first.rolled, vec![tree.note_path(today, &evidence)]);
    let mut archived = first.archived.clone();
    archived.sort();
    let mut expected = vec![
        archive_folder(&readings, &archive, yesterday).join(torts.file_name()),
        archive_folder(&readings, &archive, older).join(lsat.file_name()),
    ];
    expected.sort();
    assert_eq!(archived, expected, "the notes not carried are archived");
    assert_eq!(
        first.failed,
        vec![RollFailure {
            path: tree.note_path(yesterday, &japanese),
            reason: RollFailureReason::Malformed(Malformed::NoClosingDelimiter),
        }],
        "the malformed note is reported by path and a bounded reason"
    );
    assert_eq!(
        fs::read_to_string(dir.path().join(tree.note_path(yesterday, &japanese)))
            .expect("the malformed note stays"),
        malformed
    );
    let after_first = snapshot(dir.path());

    let second = tree
        .roll_forward(today, &carried)
        .expect("the second roll-forward");

    assert_eq!(
        snapshot(dir.path()),
        after_first,
        "a second roll-forward on the same study day changes nothing"
    );
    assert_eq!(
        (second.rolled.len(), second.archived.len(), second.failed),
        (0, 0, first.failed),
        "the second call moves nothing and reports the malformed note again"
    );
}
