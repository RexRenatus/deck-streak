//! The drills' vault opens for a configured vault and for nothing else (SPEC-110 R13, R15).

#![allow(clippy::expect_used)]

use std::fs;

use deck_streak_coordination::drills::{ARCHIVE_FOLDER, DrillMeta, READINGS_FOLDER, VAULT_ROOT};
use deck_streak_daemon::drill_vault;
use deck_streak_kernel::{Environment, StudyDay};

const NOTE: &str = "---\ntype: drill-irac\nsubject: \"Torts\"\ncreated: 2026-02-20\nstatus: active\n---\n# Duty of care\n\nThe prompt.\n\n## Free Recall\n\n- [ ] **Ready for grading**\n";

fn env(root: &str) -> Environment {
    Environment::from_vars([
        (VAULT_ROOT, root),
        (READINGS_FOLDER, "12-Readings"),
        (ARCHIVE_FOLDER, "Archive"),
    ])
}

#[test]
fn a_configured_vault_opens_over_its_active_drills() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let vault = dir.path().join("vault");
    let active = vault.join("11-Drills").join("Active");
    fs::create_dir_all(&active).expect("the active folder");
    fs::write(active.join("irac-1.md"), NOTE).expect("a note");
    let notes = drill_vault::open(&env(vault.to_str().expect("a utf-8 path")))
        .expect("a configured vault opens");
    let listed = notes
        .list_active(StudyDay::from_epoch_day(20_500))
        .expect("the drills");
    assert_eq!(
        listed,
        vec![DrillMeta {
            drill_id: "irac-1".to_owned(),
            kind: "irac".to_owned(),
            subject: "Torts".to_owned(),
            title: "Duty of care".to_owned(),
            created: listed.first().and_then(|meta| meta.created),
            age_days: listed.first().and_then(|meta| meta.age_days),
            answered: false,
            deferred: false,
        }]
    );
    assert!(
        listed[0].created.is_some(),
        "the note's created date is read"
    );
}

#[test]
fn no_vault_and_a_missing_root_serve_no_drills() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let vault = dir.path().join("vault");
    fs::create_dir_all(vault.join("11-Drills").join("Active")).expect("the active folder");
    let root = vault.to_str().expect("a utf-8 path");
    let opened = drill_vault::open(&env(root)).expect("the same setting opens a real root");
    assert_eq!(
        opened.list_active(StudyDay::from_epoch_day(20_500)).ok(),
        Some(Vec::new())
    );
    let unset = Environment::from_vars(Vec::<(&str, &str)>::new());
    assert_eq!(drill_vault::open(&unset).map(|_| ()), None);
    let missing = dir.path().join("nowhere");
    let missing = drill_vault::open(&env(missing.to_str().expect("a utf-8 path")));
    assert_eq!(missing.map(|_| ()), None);
}
