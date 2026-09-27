//! A data-rights declaration names each table once with its disposition, an exemption carries its
//! reason, and the kernel's own port resets the settings generation in place and exempts the
//! schema version table (SPEC-020 A24 to A26; CHARTER 13).

// An integration test is test code: its helpers panic on a failed fixture.
#![allow(clippy::expect_used)]

use deck_streak_kernel::{
    DataRights, DataRightsError, Db, Declaration, Disposition, KernelDataRights, TableRights,
};
use serde_json::{Map, Value, json};

/// A reset row, as a JSON object of columns.
fn row(value: Value) -> Map<String, Value> {
    match value {
        Value::Object(columns) => columns,
        other => panic!("a reset row is an object, not {other}"),
    }
}

#[test]
fn a_data_rights_declaration_names_each_table_once_with_a_disposition() {
    let declaration = Declaration::new(
        "habits",
        vec![
            TableRights {
                table: "minutes_log",
                disposition: Disposition::ExportAndErase,
            },
            TableRights {
                table: "habit_board",
                disposition: Disposition::ResetInPlace {
                    row: row(json!({"minutes": 0})),
                },
            },
            TableRights {
                table: "habit_keys",
                disposition: Disposition::Exempt {
                    reason: "synthetic: keeps an idempotency guard across an erase",
                },
            },
        ],
    )
    .expect("three tables, each named once");
    assert_eq!(declaration.context(), "habits");
    assert_eq!(
        declaration
            .tables()
            .iter()
            .map(|rights| rights.table)
            .collect::<Vec<_>>(),
        ["minutes_log", "habit_board", "habit_keys"]
    );
    assert_eq!(
        declaration.disposition("minutes_log"),
        Some(&Disposition::ExportAndErase)
    );
    assert_eq!(declaration.disposition("unknown_table"), None);

    // A table named twice is refused by its name: its export and its erase could disagree.
    let twice = Declaration::new(
        "habits",
        vec![
            TableRights {
                table: "minutes_log",
                disposition: Disposition::ExportAndErase,
            },
            TableRights {
                table: "minutes_log",
                disposition: Disposition::Exempt {
                    reason: "synthetic",
                },
            },
        ],
    );
    assert_eq!(
        twice,
        Err(DataRightsError::TableDeclaredTwice {
            context: "habits",
            table: "minutes_log",
        })
    );
}

#[test]
fn an_exempt_table_without_a_reason_is_refused() {
    for reason in ["", "   "] {
        let refused = Declaration::new(
            "coordination",
            vec![TableRights {
                table: "cron_fires",
                disposition: Disposition::Exempt { reason },
            }],
        );
        assert_eq!(
            refused,
            Err(DataRightsError::ExemptWithoutReason {
                context: "coordination",
                table: "cron_fires",
            }),
            "{reason:?}"
        );
    }
    // With its reason, the same exemption is accepted.
    let kept = Declaration::new(
        "coordination",
        vec![TableRights {
            table: "cron_fires",
            disposition: Disposition::Exempt {
                reason: "never erased: an erase must not re-arm the catch-up double-send guard",
            },
        }],
    );
    assert_eq!(kept.map(|declaration| declaration.tables().len()), Ok(1));
}

#[tokio::test]
async fn the_kernel_port_resets_the_settings_generation_and_exempts_the_schema_table() {
    let port = KernelDataRights;
    let declaration = port
        .declaration()
        .expect("the kernel's declaration is valid");
    assert_eq!(declaration.context(), "kernel");
    assert_eq!(declaration.tables().len(), 2, "{declaration:?}");
    let reset = declaration.disposition("settings_generation");
    assert!(
        matches!(reset, Some(Disposition::ResetInPlace { .. })),
        "settings_generation is reset in place: {reset:?}"
    );
    let Some(Disposition::ResetInPlace { row: reset_row }) = reset else {
        return;
    };
    assert_eq!(Value::Object(reset_row.clone()), json!({"generation": 0}));
    let schema = declaration.disposition("_sqlx_migrations");
    assert!(
        matches!(schema, Some(Disposition::Exempt { reason }) if reason.contains("schema version")),
        "the schema version table is exempt, with its reason: {schema:?}"
    );

    // Against a migrated database: an export carries the moved generation, and an erase resets it
    // in place and leaves the schema version table as it was.
    let directory = tempfile::tempdir().expect("a temporary directory");
    let db = Db::open(&directory.path().join("deckstreak.db"))
        .await
        .expect("the database opens");
    let mut write = db.write().await.expect("a write");
    Db::bump_settings_generation(&mut write)
        .await
        .expect("a bump");
    Db::bump_settings_generation(&mut write)
        .await
        .expect("a bump");
    write.commit().await.expect("committed");
    let migrations = || async {
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM _sqlx_migrations")
            .fetch_one(db.reader())
            .await
            .expect("the schema version table")
    };
    let applied = migrations().await;
    assert!(applied > 0);

    let mut read = db.reader().acquire().await.expect("a connection");
    let exported = port.export(&mut read).await.expect("the export");
    assert_eq!(
        exported.iter().map(|table| table.table).collect::<Vec<_>>(),
        ["settings_generation"]
    );
    assert_eq!(exported[0].rows.len(), 1);
    assert_eq!(exported[0].rows[0]["generation"], 2);

    let mut erase = db.write().await.expect("a write");
    port.erase(&mut erase).await.expect("the erase");
    erase.commit().await.expect("committed");
    let after = port
        .export(&mut read)
        .await
        .expect("the export after the erase");
    assert_eq!(
        after[0].rows.len(),
        1,
        "the singleton is reset, never deleted"
    );
    for (column, value) in reset_row {
        assert_eq!(&after[0].rows[0][column], value, "{column}");
    }
    assert_eq!(
        migrations().await,
        applied,
        "the schema version table is exempt"
    );
}
