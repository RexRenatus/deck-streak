//! A drill answered through one use case is answered once, whichever surface asks (SPEC-110 A6; R7).

// An integration test is test code: its helpers panic, and it prints the examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::fs;
use std::path::PathBuf;

use deck_streak_coordination::drills::{self, AnswerOutcome, DrillNotes, RealFs, Surface};
use deck_streak_kernel::{Db, Environment, Hour, StudyDayRule, UtcMillis, UtcOffset};
use deck_streak_vault::config::{ARCHIVE_FOLDER, READINGS_FOLDER, VAULT_ROOT};
use deck_streak_vault::{Rails, VaultSettings};
use tempfile::TempDir;

const NOTE: &str = "---\ntype: drill-irac\nsubject: \"Torts\"\ncreated: 2026-02-20\nstatus: active\n---\n# Duty of care\n\nThe prompt.\n\n## Free Recall\n\n## Self-Check\n\n- [ ] the rule stated\n- [ ] **Ready for grading**\n";

struct Fixture {
    _dir: TempDir,
    db: Db,
    notes: DrillNotes<RealFs>,
    note: PathBuf,
}

async fn fixture() -> Fixture {
    let dir = tempfile::tempdir().expect("a temp dir");
    let vault = dir.path().join("vault");
    let active = vault.join("11-Drills").join("Active");
    fs::create_dir_all(&active).expect("the active folder");
    let note = active.join("irac-1.md");
    fs::write(&note, NOTE).expect("a note");
    let env = Environment::from_vars([
        (VAULT_ROOT, vault.to_str().expect("a utf-8 path")),
        (READINGS_FOLDER, "12-Readings"),
        (ARCHIVE_FOLDER, "Archive"),
    ]);
    let settings = VaultSettings::from_env(&env).expect("settings");
    let db = Db::open(&dir.path().join("deck_streak.db"))
        .await
        .expect("the database");
    let notes = DrillNotes::open(&settings, RealFs, Rails::vendored().expect("the rails"))
        .expect("the vault opens");
    Fixture {
        _dir: dir,
        db,
        notes,
        note,
    }
}

#[tokio::test]
async fn a_second_answer_from_either_surface_is_refused() {
    let fx = fixture().await;
    let rule = StudyDayRule::default();
    let at = UtcMillis::from_epoch_millis(1_770_000_000_000);
    let first = drills::answer(
        &fx.notes,
        &fx.db,
        "irac-1",
        "The duty is owed.",
        Surface::Bot,
        rule,
        at,
    )
    .await
    .expect("the first answer");
    assert!(
        matches!(first, AnswerOutcome::Appended { .. }),
        "the first answer is appended: {first:?}"
    );
    let text = fs::read_to_string(&fx.note).expect("the note");
    assert_eq!(
        text.matches("## Your Answer").count(),
        1,
        "one answer heading"
    );
    for surface in [Surface::MiniApp, Surface::Bot] {
        let again = drills::answer(&fx.notes, &fx.db, "irac-1", "Another.", surface, rule, at)
            .await
            .expect("the repeat");
        assert_eq!(
            again,
            AnswerOutcome::AlreadyAnswered,
            "a repeat from {surface:?}"
        );
    }
    let after = fs::read_to_string(&fx.note).expect("the note");
    assert_eq!(after, text, "a refused answer leaves the note as it was");
    assert!(!after.contains("Another."), "the second text never lands");
    println!("examined 3 answer(s) through the one use case");
}

#[tokio::test]
async fn the_answer_heading_names_the_local_clock_of_the_rule() {
    let at = UtcMillis::from_epoch_millis(1_770_000_000_000);
    let hour = Hour::new(3).expect("an hour");
    for (offset, expected) in [
        (0, "2026-02-02 02:40"),
        (150, "2026-02-02 05:10"),
        (-300, "2026-02-01 21:40"),
    ] {
        let fx = fixture().await;
        let rule = StudyDayRule::new(hour, UtcOffset::from_minutes(offset).expect("an offset"));
        drills::answer(
            &fx.notes,
            &fx.db,
            "irac-1",
            "The duty is owed.",
            Surface::Bot,
            rule,
            at,
        )
        .await
        .expect("an answer");
        let written = fs::read_to_string(&fx.note).expect("the note");
        assert!(
            written.ends_with(&format!(
                "\n\n## Your Answer (via Telegram {expected})\n\nThe duty is owed.\n"
            )),
            "offset {offset}: {written}"
        );
    }
}
