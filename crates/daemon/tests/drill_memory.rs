//! The drill-grades memory port gives a subject's newest grades as types and XP only (SPEC-110
//! A15, R12; SPEC-044 R7).

#![allow(clippy::expect_used)]

use deck_streak_agent::{MemoryPort, Subject};
use deck_streak_daemon::wiring::{DRILL_GRADES_LIMIT, DrillGradesMemory};
use deck_streak_kernel::Db;

#[tokio::test]
async fn the_drill_grades_port_gives_types_and_xp_only() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let db = Db::open(&dir.path().join("deck_streak.db"))
        .await
        .expect("the database opens");
    let mine = Subject::parse("law/torts").expect("a subject");
    let other = Subject::parse("law/contracts").expect("a subject");
    let mut tx = db.write().await.expect("a write");
    for i in 0..13_i64 {
        let (id, subject) = if i == 6 {
            (format!("secret-{i}"), other.as_str().to_owned())
        } else {
            (format!("secret-{i}"), mine.as_str().to_owned())
        };
        sqlx::query(
            "INSERT INTO drill_grades (drill_id, drill_type, subject, xp, study_day, created_at)
             VALUES (?1, 'irac', ?2, ?3, 20500, ?4)",
        )
        .bind(id)
        .bind(subject)
        .bind(10 + i)
        .bind(1_000 + i)
        .execute(&mut *tx)
        .await
        .expect("a grade row");
    }
    tx.commit().await.expect("commit");

    let port = DrillGradesMemory::new(db.clone());
    let entries = port.read(&mine).await.expect("the port answers");
    assert_eq!(usize::try_from(DRILL_GRADES_LIMIT).expect("small"), 10);
    assert_eq!(entries.len(), 10, "at most ten entries: {entries:?}");
    assert_eq!(entries[0], "irac xp 22", "newest first");
    assert_eq!(entries[1], "irac xp 21");
    assert!(
        entries.iter().all(|line| !line.contains("secret")),
        "no drill id or text: {entries:?}"
    );
    assert!(
        !entries.contains(&"irac xp 16".to_owned()),
        "another subject's grade is not this subject's: {entries:?}"
    );
    let none = port
        .read(&Subject::parse("law/property").expect("a subject"))
        .await
        .expect("the port answers");
    assert!(none.is_empty());
    db.close().await;
}
