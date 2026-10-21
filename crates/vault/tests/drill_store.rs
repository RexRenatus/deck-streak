//! The drill tables are exported and erased and no note is (SPEC-110 A20), and the grade row
//! refuses XP outside its band (A23).

// An integration test is test code: its helpers panic, and it prints the examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::fs;

use deck_streak_kernel::data_rights::{DataRights, Disposition};
use deck_streak_kernel::{Db, StudyDay, UtcMillis};
use deck_streak_vault::data_rights::{VAULT_CONTEXT, VaultDataRights};
use deck_streak_vault::drill_store::{
    DRILL_ANSWERS_TABLE, DRILL_GRADES_TABLE, GradeRow, Surface, has_answer, insert_answer,
    recent_grades, record_grade,
};

fn at() -> UtcMillis {
    UtcMillis::from_epoch_millis(1_770_000_000_000)
}

fn grade(id: &str, xp: i64) -> GradeRow {
    GradeRow {
        drill_id: id.to_owned(),
        drill_type: "irac".to_owned(),
        subject: "Torts".to_owned(),
        xp,
        study_day: StudyDay::from_epoch_day(20_500),
    }
}

async fn database() -> (tempfile::TempDir, Db) {
    let dir = tempfile::tempdir().expect("a temp dir");
    let db = Db::open(&dir.path().join("deck_streak.db"))
        .await
        .expect("the database");
    (dir, db)
}

#[tokio::test]
async fn the_drill_tables_are_exported_and_erased_and_no_note() {
    let (dir, db) = database().await;
    let note = dir.path().join("note.md");
    fs::write(&note, "an owner's drill note").expect("a note");
    let mut tx = db.write().await.expect("a write");
    assert!(
        insert_answer(
            &mut tx,
            "irac-1",
            StudyDay::from_epoch_day(20_500),
            Surface::Bot,
            at()
        )
        .await
        .expect("an answer")
    );
    assert!(
        record_grade(&mut tx, &grade("irac-1", 17), at())
            .await
            .expect("a grade")
    );
    tx.commit().await.expect("committed");

    let port = VaultDataRights;
    let declaration = port.declaration().expect("a declaration");
    assert_eq!(declaration.context(), VAULT_CONTEXT);
    for table in [DRILL_ANSWERS_TABLE, DRILL_GRADES_TABLE] {
        assert!(
            matches!(
                declaration.disposition(table),
                Some(Disposition::ExportAndErase)
            ),
            "{table} is exported and erased"
        );
    }
    let mut tx = db.write().await.expect("a write");
    let exported = port.export(&mut tx).await.expect("an export");
    for table in [DRILL_ANSWERS_TABLE, DRILL_GRADES_TABLE] {
        let rows = exported
            .iter()
            .find(|t| t.table == table)
            .expect("the table exported");
        assert_eq!(rows.rows.len(), 1, "one exported row of {table}");
    }
    port.erase(&mut tx).await.expect("an erase");
    assert!(!has_answer(&mut tx, "irac-1").await.expect("a read"));
    assert!(
        recent_grades(&mut tx, "Torts", 10)
            .await
            .expect("a read")
            .is_empty()
    );
    drop(tx);
    assert_eq!(
        fs::read_to_string(&note).expect("the note remains"),
        "an owner's drill note",
        "an erase deletes no note"
    );
}

#[tokio::test]
async fn the_grade_row_refuses_xp_outside_its_band() {
    let (_dir, db) = database().await;
    let mut tx = db.write().await.expect("a write");
    for xp in [9, 26, 0, -1] {
        let refused = record_grade(&mut tx, &grade("bad", xp), at()).await;
        assert!(refused.is_err(), "xp {xp} is refused by the table");
    }
    assert!(
        record_grade(&mut tx, &grade("low", 10), at())
            .await
            .expect("ten")
    );
    assert!(
        record_grade(&mut tx, &grade("high", 25), at())
            .await
            .expect("twenty-five")
    );
    assert!(
        !record_grade(&mut tx, &grade("high", 12), at())
            .await
            .expect("a repeat"),
        "a second grade of one drill changes nothing"
    );
}
