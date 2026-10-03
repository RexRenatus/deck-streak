//! Curriculum's data-rights port lists its three tables as exported and erased, and an erase leaves
//! them empty (SPEC-077 A10, R18; SPEC-021): the port, as `privacy` drives it on one write, exports
//! every row of `language_progress`, `band_milestones` and `law_dues`, and its erase leaves none.
//! The store's mark of a band-up is set once (R7; ADR-303). Every row here is synthetic.

// An integration test is test code: its helpers panic on a failed fixture.
#![allow(clippy::expect_used)]

use deck_streak_curriculum::data_rights::{
    BAND_MILESTONES_TABLE, CurriculumDataRights, LANGUAGE_PROGRESS_TABLE, LAW_DUES_TABLE,
};
use deck_streak_curriculum::store::{self, NewMilestone, Recorded};
use deck_streak_kernel::{
    CourseCode, DataRights, Db, Disposition, ExportedTable, StudyDay, UtcMillis,
};
use serde_json::Value;

/// Rows in each of the three tables, written past the store as a recompute would leave them: two
/// courses, a silent baseline, an owed band-up and a celebrated one, and the law dues.
const SEEDS: [&str; 3] = [
    "INSERT INTO language_progress (course, name, flag, mastery_pct, current_band, mature_cards, \
     total_cards, current_unit, bands, updated_at, created_at) VALUES \
     ('al', 'Alpha', 'a', 61.5, 'A2', 12, 20, 7, \
      '[{\"band\":\"A1\",\"total\":8,\"mature\":8,\"pct\":90.0,\"achieved\":true}]', 2000, 1000), \
     ('be', 'Beta', 'b', 12.25, 'A1', 1, 9, NULL, '[]', 4000, 3000)",
    "INSERT INTO band_milestones (course, band, study_day, baseline, celebrated_at, created_at) \
     VALUES ('al', 'A1', 20000, 1, 1000, 1000), ('al', 'A2', 20003, 0, NULL, 5000), \
     ('be', 'A1', 20001, 1, 3000, 3000)",
    "INSERT INTO law_dues (id, study_day, backlog, due_today, updated_at, created_at) \
     VALUES (1, 20003, 4, 6, 5000, 1000)",
];

/// How many rows `table` holds, read past the port.
async fn count(db: &Db, table: &str) -> i64 {
    let statement = match table {
        LANGUAGE_PROGRESS_TABLE => "SELECT count(*) FROM language_progress",
        BAND_MILESTONES_TABLE => "SELECT count(*) FROM band_milestones",
        _ => "SELECT count(*) FROM law_dues",
    };
    sqlx::query_scalar(statement)
        .fetch_one(db.reader())
        .await
        .expect("the table's count")
}

/// The exported rows of `table`, each as the text of the columns named in `keys`.
fn exported(tables: &[ExportedTable], table: &str, keys: &[&str]) -> Vec<Vec<String>> {
    tables
        .iter()
        .filter(|exported| exported.table == table)
        .flat_map(|exported| exported.rows.iter())
        .map(|row| {
            keys.iter()
                .map(|key| match &row[*key] {
                    Value::String(text) => text.clone(),
                    other => other.to_string(),
                })
                .collect()
        })
        .collect()
}

#[tokio::test]
async fn the_curriculum_tables_are_exported_and_erased() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let db = Db::open(&directory.path().join("deck_streak.db"))
        .await
        .expect("the database opens");
    let mut write = db.write().await.expect("a write");
    for seed in SEEDS {
        sqlx::query(seed)
            .execute(&mut *write)
            .await
            .expect("the seed writes");
    }
    write.commit().await.expect("the seed commits");

    // The export carries every row of each table.
    let mut read = db.reader().acquire().await.expect("a read");
    let tables = CurriculumDataRights
        .export(&mut read)
        .await
        .expect("the export");
    drop(read);
    assert_eq!(
        exported(
            &tables,
            LANGUAGE_PROGRESS_TABLE,
            &["course", "current_band"]
        ),
        [["al", "A2"], ["be", "A1"]],
        "the export carries both courses' progress"
    );
    assert_eq!(
        exported(
            &tables,
            BAND_MILESTONES_TABLE,
            &["course", "band", "baseline"]
        ),
        [["al", "A1", "1"], ["al", "A2", "0"], ["be", "A1", "1"]],
        "the export carries every milestone, the baselines included"
    );
    assert_eq!(
        exported(
            &tables,
            LAW_DUES_TABLE,
            &["study_day", "backlog", "due_today"]
        ),
        [["20003", "4", "6"]],
        "the export carries the law dues"
    );

    // The port lists the three tables, each exported and erased.
    let declaration = CurriculumDataRights.declaration().expect("the declaration");
    let listed: Vec<(&str, bool)> = declaration
        .tables()
        .iter()
        .map(|rights| {
            (
                rights.table,
                matches!(rights.disposition, Disposition::ExportAndErase),
            )
        })
        .collect();
    assert_eq!(
        listed,
        [
            (LANGUAGE_PROGRESS_TABLE, true),
            (BAND_MILESTONES_TABLE, true),
            (LAW_DUES_TABLE, true),
        ]
    );

    // The erase runs on one write, as privacy drives it, and leaves every table empty.
    let mut write = db.write().await.expect("a write");
    CurriculumDataRights
        .erase(&mut write)
        .await
        .expect("the erase");
    write.commit().await.expect("the erase commits");
    for table in [
        LANGUAGE_PROGRESS_TABLE,
        BAND_MILESTONES_TABLE,
        LAW_DUES_TABLE,
    ] {
        assert_eq!(count(&db, table).await, 0, "the erase empties {table}");
    }
}

/// MUTATION COVERAGE (R7; ADR-303): a band-up is marked once. The first mark, set after the router
/// answers, marks the owed band-up and answers true; a second finds it marked, writes nothing and
/// answers false, so the stored instant stays the first one; a baseline, written marked, is never
/// marked again.
#[tokio::test]
async fn a_band_up_is_marked_once() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let db = Db::open(&directory.path().join("deck_streak.db"))
        .await
        .expect("the database opens");
    let course = CourseCode::new("al").expect("a course code");
    let mut write = db.write().await.expect("a write");
    for (band, baseline, at) in [("A1", true, 1_000), ("A2", false, 5_000)] {
        let milestone = NewMilestone {
            course: &course,
            band,
            study_day: StudyDay::from_epoch_day(20_000),
            baseline,
            at: UtcMillis::from_epoch_millis(at),
        };
        let recorded = store::record_milestone(&mut write, &milestone)
            .await
            .expect("the milestone records");
        assert_eq!(recorded, Recorded::New, "{band} is recorded");
    }
    let mut marks = Vec::new();
    for (band, at) in [("A2", 6_000), ("A2", 7_000), ("A1", 8_000)] {
        let marked = store::mark_band_up(&mut write, "al", band, UtcMillis::from_epoch_millis(at))
            .await
            .expect("the mark writes");
        marks.push((band, at, marked));
    }
    write.commit().await.expect("the marks commit");

    assert_eq!(
        marks,
        [
            ("A2", 6_000, true),
            ("A2", 7_000, false),
            ("A1", 8_000, false)
        ],
        "the owed band-up is marked once; a marked band-up and a baseline are not marked"
    );
    let mut read = db.reader().acquire().await.expect("a read");
    let stored: Vec<(String, Option<UtcMillis>)> = store::milestones(&mut read)
        .await
        .expect("the milestones")
        .into_iter()
        .map(|milestone| (milestone.band, milestone.celebrated_at))
        .collect();
    assert_eq!(
        stored,
        [
            ("A1".to_owned(), Some(UtcMillis::from_epoch_millis(1_000))),
            ("A2".to_owned(), Some(UtcMillis::from_epoch_millis(6_000))),
        ],
        "the band-up keeps its first mark and the baseline its own"
    );
}
