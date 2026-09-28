//! The binary runs a role by the name its first argument gives and refuses any other with code 2,
//! two roles that open one fresh database at the same moment both start (SPEC-025 A14, A15, R1,
//! R11), and the `job` role runs a job of the table by its id and refuses an unknown one with code 2
//! (SPEC-027 A16, R7, R11); the `sync` job stops on a malformed scope before it syncs, paging with
//! the scope's reason code (SPEC-023 R2, R12); and the `data` role erases only with the confirmation
//! word and writes the export as one line of standard output (SPEC-021 A10, R8).

// An integration test is test code: its helpers panic on a failed child, and the examined count
// is printed on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::ffi::OsStr;
use std::fs;
use std::process::{Command, Output};
use std::sync::Arc;

use deck_streak_coordination::ledger::{CronLedger, Outcome, SqliteCronLedger};
use deck_streak_daemon::wiring::{DATABASE_FILE, StateDirectory, open_database};
use deck_streak_ingest::settings::{LAW_DECK_ROOT, SYNC_PASSWORD, SYNC_USERNAME};
use deck_streak_ingest::sync_runs::{
    ReasonCode, RunHistory, SqliteSyncRuns, SyncRun, SyncRunStore, Trigger,
};
use deck_streak_kernel::{Clock, Db, Offload, OffloadWorkers, StudyDay, SystemClock, UtcMillis};
use serde_json::Value;
use tokio::sync::Barrier;

/// Fresh databases the race test opens, two roles at a time. Opened with no lock, as at the
/// red-first commit, one role failed with `SQLITE_BUSY` in 29 of 30 runs of 24 rounds, as late as
/// the fifteenth round, and one run passed all 24; 64 rounds leave a regression no real chance.
const ROUNDS: usize = 64;

/// Prints how many items a check examined and refuses zero (the tdd pack's examined contract).
fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}

/// Runs the built binary with `arguments` and only the variables of `environment`.
fn deckstreakd(arguments: &[&str], environment: &[(&str, &OsStr)]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_deckstreakd"))
        .args(arguments)
        .env_clear()
        .envs(environment.iter().copied())
        .output()
        .expect("the binary runs")
}

/// Every line the binary wrote to stdout, as its journal priority and its JSON event; a line that
/// is not one is returned with no priority and `Null`.
fn events(output: &Output) -> Vec<(Option<String>, Value)> {
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|line| match line.find('{') {
            Some(3) => (
                Some(line[..3].to_owned()),
                serde_json::from_str(&line[3..]).unwrap_or(Value::Null),
            ),
            _ => (None, Value::Null),
        })
        .collect()
}

fn describe(output: &Output) -> String {
    format!(
        "{}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn the_binary_runs_a_role_by_name_and_refuses_an_unknown_one() {
    // An unknown role, no role, and a known one with an argument it does not take: each exits 2,
    // and its first line is an ERROR event whose usage names every role the binary knows.
    let refusals: [&[&str]; 3] = [&["frobnicate"], &[], &["api", "extra"]];
    for arguments in examined("refused invocation(s)", refusals.to_vec()) {
        let output = deckstreakd(arguments, &[]);
        assert_eq!(
            output.status.code(),
            Some(2),
            "{arguments:?}: {}",
            describe(&output)
        );
        let lines = events(&output);
        let first = lines.first().cloned().unwrap_or((None, Value::Null));
        assert_eq!(
            first.0.as_deref(),
            Some("<3>"),
            "{arguments:?}: {}",
            describe(&output)
        );
        let usage = first.1["message"].as_str().unwrap_or_default().to_owned();
        assert!(usage.starts_with("usage: deckstreakd <role>"), "{usage}");
        assert!(
            usage.contains("the roles are: api, bot, job, data;"),
            "{usage}"
        );
    }

    // A known role runs: the api role refuses to start without its listen address, naming the
    // setting from its first line, as a JSON event with its priority, and exits 1, not 2.
    let directory = tempfile::tempdir().expect("a temporary directory");
    let output = deckstreakd(
        &["api"],
        &[("STATE_DIRECTORY", directory.path().as_os_str())],
    );
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
    let lines = events(&output);
    let first = lines.first().cloned().unwrap_or((None, Value::Null));
    assert_eq!(first.0.as_deref(), Some("<3>"), "{}", describe(&output));
    let refusal = first.1.to_string();
    assert!(
        refusal.contains("DECKSTREAK_API_LISTEN"),
        "the refusal does not name the setting: {refusal}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn two_roles_opening_one_fresh_database_at_once_both_start() {
    for round in examined("round(s)", (1..=ROUNDS).collect()) {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let state = StateDirectory::new(directory.path()).expect("an absolute path");
        let workers = OffloadWorkers::new(2).expect("two workers is in range");
        let offload = Offload::new(workers, Arc::new(SystemClock));
        // Both roles start opening at the same moment, as two units systemd starts together.
        let together = Arc::new(Barrier::new(2));
        let roles: Vec<_> = (0..2)
            .map(|_| {
                let (state, offload, together) =
                    (state.clone(), offload.clone(), Arc::clone(&together));
                tokio::spawn(async move {
                    together.wait().await;
                    open_database(&offload, &state).await
                })
            })
            .collect();
        for (role, handle) in roles.into_iter().enumerate() {
            let opened = handle.await.expect("the role's task completes");
            assert!(
                opened.is_ok(),
                "round {round}: role {role} failed to start: {opened:?}"
            );
            let database = opened.expect("the database opened");
            // Each role reads the kernel's table: the migrations were applied, once.
            let generation = database.settings_generation().await;
            assert_eq!(generation.ok(), Some(0), "round {round}: role {role}");
            database.close().await;
        }
    }
}

#[tokio::test]
async fn the_job_role_runs_a_job_by_id_and_refuses_an_unknown_one() {
    // A job of the table runs by its id, once, and exits 0: each leaves its outcome in the ledger.
    // The watch runs first: on a fresh database it finds no sync attempt to call dead and no
    // maintenance fire to call off its slot, whatever the time; a maintenance run started by hand
    // off its slot is a drift the next check pages on, once.
    let directory = tempfile::tempdir().expect("a temporary directory");
    let state = [("STATE_DIRECTORY", directory.path().as_os_str())];
    for id in examined("job(s) run by id", vec!["liveness", "maintenance"]) {
        let output = deckstreakd(&["job", id], &state);
        assert_eq!(output.status.code(), Some(0), "{id}: {}", describe(&output));
    }
    let db = Db::open(&directory.path().join(DATABASE_FILE))
        .await
        .expect("the role's database opens");
    let ledger = SqliteCronLedger::new(db.clone());
    for id in ["maintenance", "liveness"] {
        let row = ledger
            .latest(id)
            .await
            .expect("the ledger reads")
            .unwrap_or_else(|| panic!("{id} ran and recorded its fire"));
        assert_eq!(
            (row.ok_count, row.last_outcome),
            (1, Outcome::Ok),
            "{id}: {row:?}"
        );
    }
    db.close().await;

    // An id the table does not hold, a missing id, and an extra argument: each exits 2, and its
    // first line is an ERROR event whose usage names every job of the table.
    let refusals: [&[&str]; 3] = [
        &["job", "frobnicate"],
        &["job"],
        &["job", "maintenance", "extra"],
    ];
    for arguments in examined("refused job invocation(s)", refusals.to_vec()) {
        let output = deckstreakd(arguments, &state);
        assert_eq!(
            output.status.code(),
            Some(2),
            "{arguments:?}: {}",
            describe(&output)
        );
        let lines = events(&output);
        let first = lines.first().cloned().unwrap_or((None, Value::Null));
        assert_eq!(
            first.0.as_deref(),
            Some("<3>"),
            "{arguments:?}: {}",
            describe(&output)
        );
        let usage = first.1["message"].as_str().unwrap_or_default().to_owned();
        assert!(
            usage.ends_with("the jobs are: sync, maintenance, liveness"),
            "{arguments:?}: {usage}"
        );
    }
}

/// A local offset, in minutes, that puts the system's now at about 12:30 local: the `sync` job's
/// 12:07 slot under a rollover at 12 then elapsed 23 minutes ago, inside its catch-up window,
/// whatever the time the test runs at. The job reads the system's clock, so the settings place its
/// fire.
fn offset_to_half_past_noon() -> String {
    const MINUTE_MS: i64 = 60_000;
    const DAY_MS: i64 = 86_400_000;
    let minute_of_day = SystemClock.now().epoch_millis().rem_euclid(DAY_MS) / MINUTE_MS;
    (12 * 60 + 30 - minute_of_day).to_string()
}

#[tokio::test]
async fn the_sync_job_pages_on_a_malformed_scope_before_it_syncs() {
    // The sync's own settings are valid: an endpoint on a loopback port nothing listens on, and a
    // credentials directory holding the synthetic account. The law root holds the deck separator,
    // which the scope refuses.
    let directory = tempfile::tempdir().expect("a temporary directory");
    let state = directory.path().join("state");
    let credentials = directory.path().join("credentials");
    for folder in [&state, &credentials] {
        fs::create_dir_all(folder).expect("a folder");
    }
    fs::write(credentials.join(SYNC_USERNAME), "synthetic-owner\n").expect("a credential");
    fs::write(credentials.join(SYNC_PASSWORD), "synthetic-password\n").expect("a credential");
    let offset = offset_to_half_past_noon();
    let output = deckstreakd(
        &["job", "sync"],
        &[
            ("STATE_DIRECTORY", state.as_os_str()),
            ("CREDENTIALS_DIRECTORY", credentials.as_os_str()),
            (
                "DECKSTREAK_SYNC_ENDPOINT",
                OsStr::new("http://127.0.0.1:9/"),
            ),
            ("DECKSTREAK_ROLLOVER_HOUR", OsStr::new("12")),
            ("DECKSTREAK_UTC_OFFSET_MINUTES", OsStr::new(&offset)),
            (LAW_DECK_ROOT, OsStr::new("Law\u{1f}Evidence")),
        ],
    );

    // The first failure of the job pages: the runner's code 1, and one ERROR line with the scope's
    // reason.
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
    let lines = events(&output);
    let pages: Vec<&Value> = lines
        .iter()
        .filter(|(priority, event)| {
            priority.as_deref() == Some("<3>") && event["message"] == "the job pages"
        })
        .map(|(_, event)| event)
        .collect();
    assert_eq!(pages.len(), 1, "{}", describe(&output));
    assert_eq!(
        (&pages[0]["job"], &pages[0]["reason"]),
        (&Value::from("sync"), &Value::from("scope_settings_refused")),
        "{}",
        describe(&output)
    );
    // The refusal names the setting and never its value, which is private configuration.
    let refusal = lines
        .iter()
        .find(|(_, event)| event["message"] == "the read's scope refuses it")
        .map_or_else(
            || panic!("the scope's refusal is logged: {}", describe(&output)),
            |(_, event)| event["refusal"].to_string(),
        );
    assert!(
        refusal.contains(LAW_DECK_ROOT) && !refusal.contains("Evidence"),
        "{refusal}"
    );

    // The ledger holds the fire as the job's error, and no sync ran: the job stopped before its
    // syncer existed, so no request reached the endpoint and no sync run was recorded.
    let db = Db::open(&state.join(DATABASE_FILE))
        .await
        .expect("the role's database opens");
    let row = SqliteCronLedger::new(db.clone())
        .latest("sync")
        .await
        .expect("the ledger reads")
        .unwrap_or_else(|| panic!("the sync job recorded its fire: {}", describe(&output)));
    assert_eq!(
        (row.ok_count, row.error_count, row.last_outcome),
        (0, 1, Outcome::Error),
        "{row:?}"
    );
    let history = SqliteSyncRuns::new(db.clone())
        .history()
        .await
        .expect("the sync record reads");
    assert_eq!(
        history,
        RunHistory {
            last: None,
            any_success: false,
        }
    );
    db.close().await;
}

/// A database in `directory`, where a role finds it, holding one synthetic sync run: the owner's
/// data an erase removes.
async fn with_one_sync_run(directory: &std::path::Path) {
    let db = Db::open(&directory.join(DATABASE_FILE))
        .await
        .expect("the role's database opens");
    SqliteSyncRuns::new(db.clone())
        .record(&SyncRun {
            trigger: Trigger::Owner,
            started_at: UtcMillis::from_epoch_millis(1_000),
            finished_at: UtcMillis::from_epoch_millis(2_000),
            study_day: StudyDay::from_epoch_day(20_000),
            outcome: Err(ReasonCode::ServerError),
            attempts: 3,
            full_download: false,
        })
        .await
        .expect("a synthetic run is recorded");
    db.close().await;
}

/// The sync record of the role's database in `directory`.
async fn sync_record(directory: &std::path::Path) -> RunHistory {
    let db = Db::open(&directory.join(DATABASE_FILE))
        .await
        .expect("the role's database opens");
    let history = SqliteSyncRuns::new(db.clone())
        .history()
        .await
        .expect("the sync record reads");
    db.close().await;
    history
}

#[tokio::test]
async fn the_data_role_erases_only_with_the_confirmation_word() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    with_one_sync_run(directory.path()).await;
    let state = [("STATE_DIRECTORY", directory.path().as_os_str())];

    // Every other form of the erase exits 2 before the database is touched, and its first line is
    // an ERROR event whose usage names the one form that erases.
    let refusals: [&[&str]; 5] = [
        &["data", "erase"],
        &["data", "erase", "--confirm"],
        &["data", "erase", "--confirm", "erase"],
        &["data", "erase", "ERASE"],
        &["data", "erase", "--confirm", "ERASE", "extra"],
    ];
    for arguments in examined("unconfirmed erase(s)", refusals.to_vec()) {
        let output = deckstreakd(arguments, &state);
        assert_eq!(
            output.status.code(),
            Some(2),
            "{arguments:?}: {}",
            describe(&output)
        );
        let lines = events(&output);
        let first = lines.first().cloned().unwrap_or((None, Value::Null));
        assert_eq!(first.0.as_deref(), Some("<3>"), "{}", describe(&output));
        let usage = first.1["message"].as_str().unwrap_or_default().to_owned();
        assert!(
            usage.contains("deckstreakd data erase --confirm ERASE"),
            "{arguments:?}: {usage}"
        );
    }
    let kept = sync_record(directory.path()).await;
    assert!(
        kept.last.is_some(),
        "a refused erase erased the sync record"
    );

    // With the word, the role erases: it exits 0, logs what it did, and the record is empty.
    let output = deckstreakd(&["data", "erase", "--confirm", "ERASE"], &state);
    assert_eq!(output.status.code(), Some(0), "{}", describe(&output));
    let erased = events(&output)
        .into_iter()
        .find(|(_, event)| event["message"] == "the owner's data was erased")
        .unwrap_or_else(|| panic!("the erase is logged: {}", describe(&output)));
    assert_eq!(erased.0.as_deref(), Some("<6>"), "{}", describe(&output));
    assert_eq!(
        sync_record(directory.path()).await,
        RunHistory {
            last: None,
            any_success: false,
        }
    );
}

#[tokio::test]
async fn the_data_role_writes_the_export_as_one_line_of_standard_output() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    with_one_sync_run(directory.path()).await;
    let state = [("STATE_DIRECTORY", directory.path().as_os_str())];
    let output = deckstreakd(&["data", "export"], &state);
    assert_eq!(output.status.code(), Some(0), "{}", describe(&output));
    // The export is the one line that does not open with a journal priority, and the only line.
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    assert!(
        stdout.ends_with("}\n"),
        "one object, ended by a newline: {stdout}"
    );
    let lines: Vec<&str> = examined("line(s) of standard output", stdout.lines().collect());
    assert_eq!(lines.len(), 1, "{}", describe(&output));
    let document: Value = serde_json::from_str(lines[0]).expect("the line is one JSON object");
    assert_eq!(document["schema"], "deckstreak.export.v1");
    let runs = document["sync_runs"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    assert_eq!(runs.len(), 1, "{document}");
    assert_eq!(
        (
            &runs[0]["trigger"],
            &runs[0]["reason"],
            &runs[0]["attempts"]
        ),
        (
            &Value::from("owner"),
            &Value::from("server_error"),
            &Value::from(3)
        )
    );
    // A singleton's one row is in it, and an exempt table is not.
    assert_eq!(document["settings_generation"][0]["generation"], 0);
    assert!(document.get("cron_fires").is_none(), "{document}");
}
