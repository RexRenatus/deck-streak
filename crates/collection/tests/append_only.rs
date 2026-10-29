//! The append-only log and the erase gate (SPEC-512 A4; ADR-530 D6): a review row, a retraction, a
//! client record and a field history row never change, and leave only while the gate reads open;
//! a missing gate reads closed; the server's review row needs its server order; and outside the
//! migrations only the data-rights port names the gate.

#![allow(clippy::expect_used, clippy::print_stdout)]

mod support;

use std::path::Path;

use support::{SERVER, base_store, device, examined, plant, raw, root, workspace_sources};

/// The append-only tables, each with an update the trigger must refuse.
const LOGS: [(&str, &str); 4] = [
    ("review_log", "UPDATE review_log SET ease = 1"),
    (
        "review_retraction",
        "UPDATE review_retraction SET at_ms = 1",
    ),
    ("review_client", "UPDATE review_client SET client = 2"),
    (
        "note_field_history",
        "UPDATE note_field_history SET html = 'x'",
    ),
];

/// The deletes, children first, so a delete under the open gate breaks no foreign key.
const DELETES: [(&str, &str); 4] = [
    ("review_client", "DELETE FROM review_client"),
    ("review_retraction", "DELETE FROM review_retraction"),
    ("review_log", "DELETE FROM review_log"),
    ("note_field_history", "DELETE FROM note_field_history"),
];

/// The one file, outside the migrations, that may name the gate.
const GATE_OWNER: &str = "crates/collection/src/data_rights.rs";

/// The table the census looks for.
const GATE: &str = "erase_gate";

#[test]
fn a_log_row_never_changes_and_leaves_only_while_the_erase_gate_is_open() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let path = dir.path().join("log.store");
    let mut findings = Vec::new();
    if let Err(why) = base_store(&path, SERVER) {
        findings.push(why);
    }
    let conn = raw(&path);
    let refused = |statement: &str, table: &str| match conn.execute_batch(statement) {
        Err(error) => error
            .to_string()
            .contains(&format!("{table} is append-only")),
        Ok(()) => false,
    };

    for (table, update) in examined("append-only table(s)", LOGS.to_vec()) {
        if !refused(update, table) {
            findings.push(format!("{table}: an update was not refused as append-only"));
        }
    }
    // The gate closed, then missing: each delete is refused.
    for (gate, setup) in [
        ("closed", "UPDATE erase_gate SET open = 0"),
        ("missing", "DELETE FROM erase_gate"),
    ] {
        if let Err(error) = conn.execute_batch(setup) {
            findings.push(format!("the gate could not be made {gate}: {error}"));
        }
        for (table, delete) in DELETES {
            if !refused(delete, table) {
                findings.push(format!(
                    "{table}: a delete under a {gate} gate was not refused"
                ));
            }
        }
    }
    // The gate open: each delete succeeds, and an update is still refused.
    if let Err(error) = conn.execute_batch("INSERT INTO erase_gate (id, open) VALUES (1, 1)") {
        findings.push(format!("the gate could not be opened: {error}"));
    }
    for (table, update) in LOGS {
        if !refused(update, table) {
            findings.push(format!(
                "{table}: an update under the open gate was not refused"
            ));
        }
    }
    for (table, delete) in DELETES {
        if let Err(error) = conn.execute_batch(delete) {
            findings.push(format!(
                "{table}: a delete under the open gate was refused: {error}"
            ));
        }
    }
    assert_eq!(findings, Vec::<String>::new(), "the append-only log");
}

#[test]
fn a_server_review_row_needs_its_server_order() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let row = "INSERT INTO review_log (id, origin, seq, card_id, note_id, deck_id, preset_id, \
               kind, ease, duration_ms, study_day) VALUES (128, zeroblob(16), 9, 1, 1, 1, 1, 1, 3, 0, 0)";
    let mut findings = Vec::new();
    let roles = [("server", SERVER, false), ("device", device(1), true)];
    for (name, role, admitted) in examined("role(s)", roles.to_vec()) {
        let path = dir.path().join(format!("{name}.store"));
        if let Err(why) = base_store(&path, role) {
            findings.push(format!("{name}: {why}"));
        }
        match (raw(&path).execute_batch(row), admitted) {
            (Ok(()), true) => {}
            (Err(error), false)
                if error
                    .to_string()
                    .contains("a server's review row needs its server_seq") => {}
            (outcome, _) => findings.push(format!(
                "{name}: a review row without its server order answered {outcome:?}"
            )),
        }
    }
    assert_eq!(findings, Vec::<String>::new(), "the server's order");
}

/// Every file under `root`'s crates that names the gate, other than its owner.
fn gate_namers(root: &Path) -> (usize, Vec<String>) {
    let sources = workspace_sources(root);
    let count = sources.len();
    let namers = sources
        .into_iter()
        .filter(|(path, text)| path != GATE_OWNER && text.contains(GATE))
        .map(|(path, _)| path)
        .collect();
    (count, namers)
}

#[test]
fn only_the_data_rights_port_names_the_erase_gate() {
    let (count, namers) = gate_namers(&root());
    examined("source file(s)", vec![(); count]);
    assert_eq!(namers, Vec::<String>::new(), "files naming the gate");
    let owner = std::fs::read_to_string(root().join(GATE_OWNER)).unwrap_or_default();
    assert!(owner.contains(GATE), "{GATE_OWNER} names the gate it opens");

    // A planted tree: a second file naming the gate is refused by its path.
    let planted = tempfile::tempdir().expect("a temporary directory");
    plant(
        planted.path(),
        GATE_OWNER,
        "const GATE: &str = \"erase_gate\";\n",
    );
    plant(
        planted.path(),
        "crates/collection/src/read.rs",
        "const OPEN: &str = \"UPDATE erase_gate SET open = 1\";\n",
    );
    let (_, namers) = gate_namers(planted.path());
    assert_eq!(
        namers,
        ["crates/collection/src/read.rs"],
        "the planted namer"
    );
}
