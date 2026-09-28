//! A staged run is applied only through the vault-duties pack's blocking classes: a run with a red
//! class is discarded and the vault's bytes are unchanged (SPEC-042 A10, R4), a green run is
//! applied, and a run that deletes, escapes its folders or overwrites a note is refused before the
//! gate is asked.

// An integration test is test code: its helpers panic on an unreadable file, and it prints the
// examined count on purpose. clippy.toml's in-test allowances cover only `#[test]` bodies.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::cell::Cell;
use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

use deck_streak_kernel::Verdict;
use deck_streak_vault::sha256;
use deck_streak_vault::{
    Discard, Executor, GateError, ProbeGate, Rails, RealFs, RedClass, RunGate, RunOutcome,
    RunRefusal,
};
use serde_json::{Value, json};
use tempfile::TempDir;

/// A layout whose periodic formats hold no date, as the pack's worked example uses.
fn layout() -> Value {
    json!({
        "schema": "phx.duty.vault.layout.v1",
        "periodic": {
            "daily": {"folder": "Daily", "format": "[Today]"},
            "weekly": {"folder": "Weekly", "format": "[This week]"}
        },
        "inbox": "90-Inbox",
        "folders": {"11-Drills": "title", "12-Readings": "title", "90-Inbox": "any"},
        "duties": {
            "daily-note": {"writes": ["@daily"]},
            "weekly-synthesis": {"writes": ["@weekly"]},
            "inbox-curator": {"moves_from": ["90-Inbox"], "moves_to": ["11-Drills", "12-Readings"]},
            "daily-reading": {"writes": ["12-Readings"]},
            "drill-coach": {"writes": ["11-Drills"]}
        },
        "journal": []
    })
}

/// The daily note a run stages, linking `reading` in its readings section.
fn daily_note(reading: &str) -> String {
    format!(
        "---\nschema: \"phx.duty.vault.note.v1\"\nduty: \"daily-note\"\ntags: [\"daily-note\"]\n---\n\
         ## Readings <!-- section:readings -->\n\n- [[{reading}]]\n\n\
         ## Drills <!-- section:drills -->\n\n- [[Hearsay drill]]\n\n\
         ## Inbox <!-- section:inbox -->\n\nNothing was filed from the inbox.\n"
    )
}

/// A daily-note run's record, with `ops`.
fn record(ops: &Value) -> Value {
    json!({
        "schema": "phx.duty.vault.run.v1",
        "duty": "daily-note",
        "vault": ["12-Readings/Evidence primer.md", "11-Drills/Hearsay drill.md"],
        "inputs": [
            {"path": "12-Readings/Evidence primer.md", "section": "readings"},
            {"path": "11-Drills/Hearsay drill.md", "section": "drills"}
        ],
        "layout": layout(),
        "ops": ops
    })
}

/// A staging directory holding `record` and the files `staged` names, at their vault paths.
fn stage(record: &Value, staged: &[(&str, &str)]) -> TempDir {
    let run = tempfile::tempdir().expect("a staging directory");
    fs::write(
        run.path().join("duty-run.json"),
        serde_json::to_string_pretty(record).expect("a record"),
    )
    .expect("the record");
    for (path, text) in staged {
        let file = run.path().join(path);
        fs::create_dir_all(file.parent().expect("a folder")).expect("a staged folder");
        fs::write(file, text).expect("a staged file");
    }
    run
}

/// A vault holding the notes the run's index names, and the daily-notes folder.
fn vault() -> TempDir {
    let vault = tempfile::tempdir().expect("a temporary vault");
    for (path, text) in [
        (
            "12-Readings/Evidence primer.md",
            "# Evidence primer\n\nA primer.\n",
        ),
        (
            "11-Drills/Hearsay drill.md",
            "# Hearsay drill\n\nA drill.\n",
        ),
    ] {
        let file = vault.path().join(path);
        fs::create_dir_all(file.parent().expect("a folder")).expect("a vault folder");
        fs::write(file, text).expect("a vault note");
    }
    fs::create_dir(vault.path().join("Daily")).expect("the daily-notes folder");
    vault
}

/// Prints how many items a check examined and refuses zero (the tdd pack's examined contract).
fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
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
            if path.is_symlink() {
                found.insert(relative, Some(b"a symbolic link".to_vec()));
            } else if path.is_dir() {
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

/// The gate as the pack's own vendored probe, reading `vault`.
fn probe_gate(vault: &Path) -> ProbeGate {
    let probe =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.packs/scripts/vault-duties-probe.py");
    ProbeGate::new("python3", probe)
        .expect("the pack's blocking classes")
        .with_vault(vault)
}

/// A gate that counts the runs it is asked about and finds each one green.
#[derive(Default)]
struct CountingGate {
    asked: Cell<usize>,
}

impl RunGate for CountingGate {
    fn judge(&self, _run_dir: &Path) -> Result<Verdict<RedClass>, GateError> {
        self.asked.set(self.asked.get() + 1);
        Ok(Verdict::Pass)
    }
}

fn rails() -> Rails {
    Rails::vendored().expect("the vendored rails")
}

/// A refused run: its operations, its staged files, and the refusal it must draw.
type RefusedRun<'a> = (Value, Vec<(&'a str, &'a str)>, RunRefusal);

#[test]
fn a_staged_run_with_a_red_class_leaves_the_vault_untouched() {
    let vault = vault();
    let before = snapshot(vault.path());
    // The note links a reading the vault does not hold: only the pack's note-links class sees it.
    let run = stage(
        &record(&json!([{"op": "create", "path": "Daily/Today.md"}])),
        &[("Daily/Today.md", &daily_note("Missing primer"))],
    );
    let gate = probe_gate(vault.path());
    let executor = Executor::new(RealFs, rails(), vault.path(), &gate).expect("the executor");

    let outcome = executor.apply(run.path()).expect("the executor runs");

    assert!(
        matches!(
            outcome,
            RunOutcome::Discarded(Discard::Red(ref red)) if red.class == "note-links"
        ),
        "the run was not discarded for its red note-links class: {outcome:?}"
    );
    assert_eq!(
        snapshot(vault.path()),
        before,
        "the vault's bytes are unchanged"
    );
}

#[test]
fn a_green_run_is_applied_through_the_gate() {
    let vault = vault();
    let note = daily_note("Evidence primer");
    let run = stage(
        &record(&json!([{"op": "create", "path": "Daily/Today.md"}])),
        &[("Daily/Today.md", &note)],
    );
    let gate = probe_gate(vault.path());
    let executor = Executor::new(RealFs, rails(), vault.path(), &gate).expect("the executor");

    let outcome = executor.apply(run.path()).expect("the executor runs");

    assert!(
        matches!(outcome, RunOutcome::Applied { ops: 1 }),
        "the green run was not applied: {outcome:?}"
    );
    assert_eq!(
        fs::read_to_string(vault.path().join("Daily/Today.md")).expect("the applied note"),
        note
    );
}

#[test]
fn a_run_that_deletes_escapes_or_overwrites_is_refused_before_the_gate() {
    let vault = vault();
    fs::write(vault.path().join("Daily/today.md"), "the owner's note\n").expect("an owner's note");
    let outside = tempfile::tempdir().expect("a folder outside the vault");
    symlink(outside.path(), vault.path().join("Weekly")).expect("a planted link");
    let note = daily_note("Evidence primer");
    let owners = "# Evidence primer\n\nA primer.\n";
    let cases: [RefusedRun; 6] = [
        (
            json!([{"op": "delete", "path": "12-Readings/Evidence primer.md"}]),
            vec![],
            RunRefusal::Verb { op: 1 },
        ),
        (
            json!([{"op": "create", "path": "Daily/../12-Readings/Planted.md"}]),
            vec![("Daily/../12-Readings/Planted.md", note.as_str())],
            RunRefusal::UnsoundPath { op: 1 },
        ),
        (
            json!([{"op": "create", "path": "12-Readings/Planted.md"}]),
            vec![("12-Readings/Planted.md", note.as_str())],
            RunRefusal::OutsideFolders { op: 1 },
        ),
        (
            json!([{"op": "create", "path": "Daily/Today.md"}]),
            vec![("Daily/Today.md", note.as_str())],
            RunRefusal::WouldOverwrite { op: 1 },
        ),
        (
            json!([
                {"op": "update", "path": "Daily/today.md", "sha256_before": sha256::hex(&sha256::digest(owners.as_bytes()))}
            ]),
            vec![("Daily/today.md", note.as_str())],
            RunRefusal::Changed { op: 1 },
        ),
        (
            json!([{"op": "create", "path": "Daily/Today.md"}]),
            vec![
                ("Daily/Today.md", note.as_str()),
                ("Daily/Extra.md", note.as_str()),
            ],
            RunRefusal::StagedWithoutOp,
        ),
    ];
    let before = snapshot(vault.path());
    for (ops, staged, expected) in cases {
        let run = stage(&record(&ops), &staged);
        let gate = CountingGate::default();
        let executor = Executor::new(RealFs, rails(), vault.path(), &gate).expect("the executor");

        let outcome = executor.apply(run.path()).expect("the executor runs");

        match outcome {
            RunOutcome::Discarded(Discard::Refused(refusal)) => {
                assert_eq!(refusal, expected, "the run {ops}");
            }
            other => panic!("the run {ops} was not refused: {other:?}"),
        }
        assert_eq!(
            gate.asked.get(),
            0,
            "the gate was asked about a refused run"
        );
    }
    // A daily note whose folder links out of the vault is refused too.
    let mut weekly = record(&json!([{"op": "create", "path": "Weekly/This week.md"}]));
    weekly["duty"] = json!("weekly-synthesis");
    let run = stage(&weekly, &[("Weekly/This week.md", &note)]);
    let gate = CountingGate::default();
    let executor = Executor::new(RealFs, rails(), vault.path(), &gate).expect("the executor");
    let outcome = executor.apply(run.path()).expect("the executor runs");
    assert!(
        matches!(
            outcome,
            RunOutcome::Discarded(Discard::Refused(RunRefusal::OutsideFolders { op: 1 }))
        ),
        "a write through a link out of the vault: {outcome:?}"
    );
    assert_eq!(
        snapshot(vault.path()),
        before,
        "no refused run changed the vault"
    );
}
