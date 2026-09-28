//! The Studied stamp and the read tick each change only their own line and never untick (SPEC-042
//! A12, R9), a missing or doubled box line is reported with no write, and a body replacement keeps
//! the frontmatter and both box lines while refusing a note whose body the owner edited (A13, R10).

// An integration test is test code: its helpers panic on an unreadable file.
#![allow(clippy::expect_used)]

use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;

use deck_streak_kernel::{Environment, StudyDay};
use deck_streak_vault::config::{ARCHIVE_FOLDER, READINGS_FOLDER, VAULT_ROOT};
use deck_streak_vault::{
    BodyHash, BoxOutcome, Rails, ReadingsTree, RealFs, TopicKey, VaultError, VaultSettings,
};
use tempfile::TempDir;

const READINGS: &str = "12-Readings";
const ARCHIVE: &str = "Archive";
const DIGEST: &str = "5d41402abc4b2a76b9719d911017c592aaf1d7f2c3b4e5a69788796a5b4c3d2e";
const BODY: &str = "# Hearsay\n\nAn out-of-court statement offered for its truth.";

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

fn read(path: &PathBuf) -> String {
    fs::read_to_string(path).expect("a readable note")
}

#[test]
fn each_box_write_changes_only_its_own_line() {
    let (dir, tree) = vault();
    let day = StudyDay::from_epoch_day(20_000);
    let topic = TopicKey::new("law/evidence").expect("a topic key");
    let created = tree
        .create(day, &topic, DIGEST, BODY)
        .expect("the note is written");
    let path = dir.path().join(&created.path);
    let written = read(&path);
    assert!(
        written.ends_with(&format!("{BODY}\n\n- [ ] Studied\n- [ ] I read it\n")),
        "a new note ends with its body and two unticked boxes"
    );

    assert_eq!(
        tree.stamp_studied(day, &topic).expect("the stamp"),
        BoxOutcome::Ticked
    );
    let stamped = read(&path);
    assert_eq!(
        stamped,
        written.replace("- [ ] Studied\n", "- [x] Studied\n"),
        "the Studied stamp changed its own line and no other"
    );

    assert_eq!(
        tree.tick_read(day, &topic).expect("the tick"),
        BoxOutcome::Ticked
    );
    let ticked = read(&path);
    assert_eq!(
        ticked,
        stamped.replace("- [ ] I read it\n", "- [x] I read it\n"),
        "the read tick changed its own line and no other"
    );

    assert_eq!(
        (
            tree.stamp_studied(day, &topic).expect("a second stamp"),
            tree.tick_read(day, &topic).expect("a second tick"),
        ),
        (BoxOutcome::AlreadyTicked, BoxOutcome::AlreadyTicked),
        "a ticked box is never written again"
    );
    assert_eq!(read(&path), ticked, "neither box is ever unticked");

    // The owner's tick, made on a device, survives the Studied stamp.
    let other = TopicKey::new("law/torts").expect("a topic key");
    let created = tree
        .create(day, &other, DIGEST, BODY)
        .expect("a second note");
    let path = dir.path().join(&created.path);
    let owners = read(&path).replace("- [ ] I read it\n", "- [x] I read it\n");
    fs::write(&path, &owners).expect("the owner's tick");
    assert_eq!(
        tree.stamp_studied(day, &other).expect("the stamp"),
        BoxOutcome::Ticked
    );
    assert_eq!(
        read(&path),
        owners.replace("- [ ] Studied\n", "- [x] Studied\n")
    );
}

#[test]
fn a_missing_or_doubled_box_line_is_reported_with_no_write() {
    let (dir, tree) = vault();
    let day = StudyDay::from_epoch_day(20_000);
    let topic = TopicKey::new("law/evidence").expect("a topic key");
    let created = tree
        .create(day, &topic, DIGEST, BODY)
        .expect("the note is written");
    let path = dir.path().join(&created.path);
    let written = read(&path);

    let missing = written.replace("- [ ] Studied\n", "");
    fs::write(&path, &missing).expect("a note with no Studied line");
    let refused = tree
        .stamp_studied(day, &topic)
        .expect_err("a missing line is refused");
    assert_eq!(refused.to_string(), "box_anchor_missing");
    assert_eq!(read(&path), missing, "nothing was written");

    let doubled = written.replace("- [ ] I read it\n", "- [ ] I read it\n- [ ] I read it\n");
    fs::write(&path, &doubled).expect("a note with two read lines");
    let refused = tree
        .tick_read(day, &topic)
        .expect_err("a doubled line is refused");
    assert_eq!(refused.to_string(), "box_anchor_ambiguous");
    assert_eq!(read(&path), doubled, "nothing was written");
}

#[test]
fn a_body_replacement_keeps_the_boxes_and_refuses_an_edited_note() {
    let (dir, tree) = vault();
    let day = StudyDay::from_epoch_day(20_000);
    let topic = TopicKey::new("law/evidence").expect("a topic key");
    let created = tree
        .create(day, &topic, DIGEST, BODY)
        .expect("the note is written");
    let path = dir.path().join(&created.path);
    assert_eq!(created.body, BodyHash::of(BODY));
    // The owner ticked the note on a device before the regeneration.
    let before = read(&path).replace("- [ ] I read it\n", "- [x] I read it\n");
    fs::write(&path, &before).expect("the owner's tick");

    let second = "# Hearsay, again\n\nThe statement is offered to prove what it asserts.";
    let replaced = tree
        .replace_body(day, &topic, created.body, second)
        .expect("the body is replaced");

    assert_eq!(replaced, BodyHash::of(second));
    let after = read(&path);
    assert_eq!(
        after,
        before.replace(BODY, second),
        "the frontmatter and both box lines are kept byte for byte"
    );
    assert!(after.ends_with("\n\n- [ ] Studied\n- [x] I read it\n"));

    // The owner edits the body; a replacement is refused, and nothing is written.
    let edited = after.replace(second, &format!("{second}\nThe owner's own line."));
    fs::write(&path, &edited).expect("the owner's edit");
    let refused = tree
        .replace_body(day, &topic, replaced, "A third body.")
        .expect_err("an edited note is refused");
    assert!(
        matches!(refused, VaultError::NoteEdited),
        "refused as {refused:?}"
    );
    assert_eq!(refused.to_string(), "vault_note_edited");
    assert_eq!(read(&path), edited, "the owner's edit is never overwritten");
}
