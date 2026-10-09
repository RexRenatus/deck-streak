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
use std::os::unix::net::UnixDatagram;
use std::process::{Command, Output};
use std::sync::Arc;

use deck_streak_coordination::ledger::{CronLedger, Outcome, SqliteCronLedger};
use deck_streak_daemon::wiring::{DATABASE_FILE, StateDirectory, open_database};
use deck_streak_ingest::settings::{LAW_DECK_ROOT, SYNC_PASSWORD, SYNC_USERNAME};
use deck_streak_ingest::state::{RefusalReason, SqliteIngestState};
use deck_streak_ingest::sync_runs::{
    ReasonCode, RunHistory, SqliteSyncRuns, SyncRun, SyncRunStore, Trigger,
};
use deck_streak_kernel::{
    Clock, Db, Environment, Offload, OffloadWorkers, SettingsError, StudyDay, SystemClock,
    UtcMillis,
};
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
            usage.contains("the roles are: api, bot, job, data, mcp, preset;"),
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

#[test]
fn only_the_name_data_runs_the_data_role() {
    // A name that is not `data`, with the words of a real data command after it, is an unknown role:
    // exit 2 and the usage line, and no command of the data role runs.
    let refusals: [&[&str]; 3] = [
        &["frobnicate", "export"],
        &["api", "export"],
        &["exporter", "erase", "--confirm", "ERASE"],
    ];
    for arguments in examined("refused invocation(s)", refusals.to_vec()) {
        let output = deckstreakd(arguments, &[]);
        assert_eq!(
            output.status.code(),
            Some(2),
            "{arguments:?}: {}",
            describe(&output)
        );
        let lines = events(&output);
        let usage = lines
            .first()
            .map(|line| line.1["message"].as_str().unwrap_or_default().to_owned())
            .unwrap_or_default();
        assert!(usage.starts_with("usage: deckstreakd <role>"), "{usage}");
    }
}

#[tokio::test]
async fn the_open_lock_is_a_file_of_its_own_beside_the_database() {
    // SPEC-025 R11 names it: `deck_streak.db-open.lock`, never the database file itself, whose
    // descriptors' closing would drop SQLite's own POSIX locks.
    let directory = tempfile::tempdir().expect("a temporary directory");
    let state = StateDirectory::new(directory.path()).expect("an absolute path");
    let workers = OffloadWorkers::new(2).expect("two workers is in range");
    let offload = Offload::new(workers, Arc::new(SystemClock));
    let database = open_database(&offload, &state)
        .await
        .expect("the database opens");
    database.close().await;
    assert!(
        directory.path().join("deck_streak.db-open.lock").is_file(),
        "no open lock named deck_streak.db-open.lock beside the database"
    );
}

#[test]
fn a_relative_state_directory_is_refused_naming_the_shape_it_must_have() {
    let refused = StateDirectory::from_env(&Environment::from_vars([(
        "STATE_DIRECTORY",
        "relative/state",
    )]));
    assert_eq!(
        refused,
        Err(SettingsError::Malformed {
            setting: "STATE_DIRECTORY",
            expected: "an absolute directory path",
        })
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
            usage
                .ends_with("the jobs are: sync, maintenance, liveness, drill_postback, held_flush"),
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

/// The messages of `output` that say the owner's stored request was served or refused.
fn request_events(output: &Output) -> Vec<Value> {
    events(output)
        .into_iter()
        .map(|(_, event)| event)
        .filter(|event| {
            event["message"]
                .as_str()
                .is_some_and(|message| message.starts_with("the owner's request"))
        })
        .collect()
}

#[tokio::test]
async fn only_the_sync_job_serves_the_owners_stored_request() {
    // A request is stored, as the bot leaves it. The scope is malformed on purpose, so a served
    // request ends in a refusal the log names, without a network.
    let directory = tempfile::tempdir().expect("a temporary directory");
    let state = directory.path().join("state");
    let credentials = directory.path().join("credentials");
    for folder in [&state, &credentials] {
        fs::create_dir_all(folder).expect("a folder");
    }
    fs::write(credentials.join(SYNC_USERNAME), "synthetic-owner\n").expect("a credential");
    fs::write(credentials.join(SYNC_PASSWORD), "synthetic-password\n").expect("a credential");
    let db = Db::open(&state.join(DATABASE_FILE))
        .await
        .expect("the role's database opens");
    SqliteIngestState::new(db.clone())
        .request_rescore(SystemClock.now())
        .await
        .expect("the request is stored");
    db.close().await;
    let offset = offset_to_half_past_noon();
    let environment = [
        ("STATE_DIRECTORY", state.as_os_str()),
        ("CREDENTIALS_DIRECTORY", credentials.as_os_str()),
        (
            "DECKSTREAK_SYNC_ENDPOINT",
            OsStr::new("http://127.0.0.1:9/"),
        ),
        ("DECKSTREAK_ROLLOVER_HOUR", OsStr::new("12")),
        ("DECKSTREAK_UTC_OFFSET_MINUTES", OsStr::new(&offset)),
        (LAW_DECK_ROOT, OsStr::new("Law\u{1f}Evidence")),
    ];

    // Another job leaves the request where it is.
    let other = deckstreakd(&["job", "liveness"], &environment);
    assert_eq!(other.status.code(), Some(0), "{}", describe(&other));
    assert!(request_events(&other).is_empty(), "{}", describe(&other));

    // The sync job serves it: one owner cycle, refused here for the scope, and the log says so.
    let output = deckstreakd(&["job", "sync"], &environment);
    let served = request_events(&output);
    assert_eq!(served.len(), 1, "{}", describe(&output));
    assert_eq!(
        (&served[0]["message"], &served[0]["reason"]),
        (
            &Value::from("the owner's request was refused"),
            &Value::from("scope_settings_refused")
        ),
        "{}",
        describe(&output)
    );
}

/// A state directory holding one stored owner request, and the credentials the job reads.
async fn stored_request(directory: &std::path::Path) -> (std::path::PathBuf, std::path::PathBuf) {
    let state = directory.join("state");
    let credentials = directory.join("credentials");
    for folder in [&state, &credentials] {
        fs::create_dir_all(folder).expect("a folder");
    }
    fs::write(credentials.join(SYNC_USERNAME), "synthetic-owner\n").expect("a credential");
    fs::write(credentials.join(SYNC_PASSWORD), "synthetic-password\n").expect("a credential");
    let db = Db::open(&state.join(DATABASE_FILE))
        .await
        .expect("the role's database opens");
    SqliteIngestState::new(db.clone())
        .request_rescore(SystemClock.now())
        .await
        .expect("the request is stored");
    db.close().await;
    (state, credentials)
}

async fn stored_refusal(state: &std::path::Path) -> (bool, Option<String>) {
    let db = Db::open(&state.join(DATABASE_FILE))
        .await
        .expect("the role's database opens");
    let loaded = SqliteIngestState::new(db.clone())
        .load()
        .await
        .expect("the state reads");
    db.close().await;
    (
        loaded.rescore_pending,
        loaded
            .refusal
            .map(|refusal| refusal.reason.as_str().to_owned()),
    )
}

#[tokio::test]
async fn a_refused_owner_request_is_recorded_and_the_next_run_does_not_retry_it() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let (state, credentials) = stored_request(directory.path()).await;
    let offset = offset_to_half_past_noon();
    let environment = [
        ("STATE_DIRECTORY", state.as_os_str()),
        ("CREDENTIALS_DIRECTORY", credentials.as_os_str()),
        (
            "DECKSTREAK_SYNC_ENDPOINT",
            OsStr::new("http://127.0.0.1:9/"),
        ),
        ("DECKSTREAK_ROLLOVER_HOUR", OsStr::new("12")),
        ("DECKSTREAK_UTC_OFFSET_MINUTES", OsStr::new(&offset)),
        (LAW_DECK_ROOT, OsStr::new("Law\u{1f}Evidence")),
    ];
    let first = deckstreakd(&["job", "sync"], &environment);
    assert_eq!(request_events(&first).len(), 1, "{}", describe(&first));
    assert_eq!(
        stored_refusal(&state).await,
        (false, Some("scope_settings_refused".to_owned())),
        "{}",
        describe(&first)
    );
    let second = deckstreakd(&["job", "sync"], &environment);
    assert!(
        request_events(&second).is_empty(),
        "the refused request is not served again: {}",
        describe(&second)
    );
}

/// The codes the `ingest_state.refused_reason` `CHECK` allows, read from the migration that holds it.
fn stored_refusal_codes() -> Vec<String> {
    let migration = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../migrations/012801_ingest_refused_owner_request.sql"
    ))
    .expect("the migration reads");
    let list = migration
        .split("refused_reason IN (")
        .nth(1)
        .and_then(|rest| rest.split(')').next())
        .expect("the CHECK's list");
    list.split('\'')
        .skip(1)
        .step_by(2)
        .map(str::to_owned)
        .collect()
}

#[test]
fn every_code_the_owner_cycle_refuses_with_is_one_the_job_records() {
    // SPEC-128 R1, replacing the source scan (#396): the cycle refuses with a `RefusalReason` and
    // the job records that value, so the set to check is the enum's variants against the stored codes.
    let stored = stored_refusal_codes();
    assert_eq!(stored.len(), 8, "the stored codes: {stored:?}");
    for code in &stored {
        assert!(
            RefusalReason::parse(code).is_some(),
            "the migration stores {code}, which the enum cannot name"
        );
    }
    for reason in RefusalReason::ALL {
        assert!(
            stored.iter().any(|code| code == reason.as_str()),
            "{} is a variant the migration's CHECK refuses",
            reason.as_str()
        );
    }
}

/// Compile-time proof that the owner cycle refuses with the closed enum and nothing looser.
fn typed_refusal(
    cycle: &deck_streak_daemon::wiring::OwnerSyncCycle,
) -> impl std::future::Future<Output = Result<deck_streak_bot::SyncAnswer, RefusalReason>> + '_ {
    cycle.run()
}

#[test]
fn a_refusal_code_is_a_variant_of_the_closed_enum() {
    // SPEC-128 amendment (#396): the codes stored today are the enum's strings, byte for byte, no
    // two variants share one, and the owner cycle's only refusal type is the enum (a code outside
    // it does not compile).
    const DEV: [(&str, RefusalReason); 8] = [
        ("rescore_unrecorded", RefusalReason::RescoreUnrecorded),
        ("sync_settings_refused", RefusalReason::SyncSettingsRefused),
        (
            "credentials_directory_refused",
            RefusalReason::CredentialsDirectoryRefused,
        ),
        (
            "scope_settings_refused",
            RefusalReason::ScopeSettingsRefused,
        ),
        ("recompute_refused", RefusalReason::RecomputeRefused),
        ("sync_record_failed", RefusalReason::SyncRecordFailed),
        (
            "obligations_unreadable",
            RefusalReason::ObligationsUnreadable,
        ),
        ("recompute_failed", RefusalReason::RecomputeFailed),
    ];
    assert_eq!(RefusalReason::ALL.len(), DEV.len());
    for (code, reason) in DEV {
        assert_eq!(reason.as_str(), code);
        assert!(RefusalReason::ALL.contains(&reason), "{code} is listed");
    }
    let mut codes: Vec<&str> = RefusalReason::ALL.iter().map(|r| r.as_str()).collect();
    codes.sort_unstable();
    codes.dedup();
    assert_eq!(codes.len(), 8, "no two variants share a code");
    let _ = typed_refusal;
}

#[tokio::test]
async fn a_refusal_after_the_owners_run_answers_the_request_beside_the_run() {
    use deck_streak_daemon::sync_request::{Progress, RequestLedger as _, SqliteRequestLedger};
    let directory = tempfile::tempdir().expect("a temporary directory");
    let since = SystemClock.now();
    let (state, credentials) = stored_request(directory.path()).await;
    // The recompute writes the rollup after the sync's run is on record: a trigger that aborts that
    // write makes the recompute fail at that point, as a full disk or a locked file would.
    let db = Db::open(&state.join(DATABASE_FILE))
        .await
        .expect("the role's database opens");
    let mut write = db.write().await.expect("a write");
    sqlx::query(
        "CREATE TRIGGER refuse_rollup BEFORE INSERT ON daily_rollup \
         BEGIN SELECT RAISE(ABORT, 'planted'); END",
    )
    .execute(&mut *write)
    .await
    .expect("the trigger is planted");
    write.commit().await.expect("the trigger commits");
    db.close().await;
    let offset = offset_to_half_past_noon();
    let environment = [
        ("STATE_DIRECTORY", state.as_os_str()),
        ("CREDENTIALS_DIRECTORY", credentials.as_os_str()),
        (
            "DECKSTREAK_SYNC_ENDPOINT",
            OsStr::new("http://127.0.0.1:9/"),
        ),
        ("DECKSTREAK_ROLLOVER_HOUR", OsStr::new("12")),
        ("DECKSTREAK_UTC_OFFSET_MINUTES", OsStr::new(&offset)),
    ];
    let output = deckstreakd(&["job", "sync"], &environment);
    let db = Db::open(&state.join(DATABASE_FILE))
        .await
        .expect("the role's database opens");
    let progress = SqliteRequestLedger::new(db.clone())
        .progress(since)
        .await
        .expect("reads");
    let owner_run = SqliteSyncRuns::new(db.clone())
        .owner_run_since(since)
        .await
        .expect("reads");
    let loaded = SqliteIngestState::new(db.clone())
        .load()
        .await
        .expect("reads");
    db.close().await;
    println!(
        "A11 refusal={:?} owner_run={owner_run:?} progress={progress:?}",
        loaded.refusal
    );
    assert!(
        owner_run.is_some(),
        "the owner's run is on record: {}",
        describe(&output)
    );
    let refusal = loaded
        .refusal
        .unwrap_or_else(|| panic!("no refusal recorded: {}", describe(&output)));
    assert_eq!(
        refusal.reason,
        RefusalReason::RecomputeFailed,
        "the cycle's own refusal is recorded by its own code: {}",
        describe(&output)
    );
    assert!(
        matches!(
            progress,
            Progress::RefusedAfterRun { ref reason, .. } if reason == refusal.reason.as_str()
        ),
        "the refusal recorded after the run answers beside it: {progress:?}"
    );
}

#[tokio::test]
async fn a_refused_recompute_setup_is_recorded_for_the_owner() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let (state, credentials) = stored_request(directory.path()).await;
    let missing = directory.path().join("missing-courses.json");
    let offset = offset_to_half_past_noon();
    let environment = [
        ("STATE_DIRECTORY", state.as_os_str()),
        ("CREDENTIALS_DIRECTORY", credentials.as_os_str()),
        (
            "DECKSTREAK_SYNC_ENDPOINT",
            OsStr::new("http://127.0.0.1:9/"),
        ),
        ("DECKSTREAK_ROLLOVER_HOUR", OsStr::new("12")),
        ("DECKSTREAK_UTC_OFFSET_MINUTES", OsStr::new(&offset)),
        ("DECKSTREAK_COURSES_FILE", missing.as_os_str()),
    ];
    let output = deckstreakd(&["job", "sync"], &environment);
    assert_eq!(
        stored_refusal(&state).await,
        (false, Some("recompute_refused".to_owned())),
        "{}",
        describe(&output)
    );
}

#[tokio::test]
async fn two_planted_request_payloads_change_nothing_the_sync_job_serves() {
    // The doorbell's file holds a payload that pretends to command the job. The job reads the
    // stored flag and nothing from the file: with the flag clear it serves no owner request, with
    // the flag pending it serves exactly one, and the file stays byte for byte as planted.
    let cases: Vec<(bool, &[u8])> = examined(
        "planted payload cases",
        vec![
            (false, br#"{"trigger":"owner","force":true,"job":"sync"}"#),
            (true, b"{\"trigger\":\"none\",\"cancel\":true}\n\x00\xff"),
        ],
    );
    for (pending, payload) in cases {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let state = directory.path().join("state");
        let credentials = directory.path().join("credentials");
        let run = directory.path().join("run");
        for folder in [&state, &credentials, &run] {
            fs::create_dir_all(folder).expect("a folder");
        }
        let doorbell = run.join("request");
        fs::write(&doorbell, payload).expect("a planted payload");
        fs::write(credentials.join(SYNC_USERNAME), "synthetic-owner\n").expect("a credential");
        fs::write(credentials.join(SYNC_PASSWORD), "synthetic-password\n").expect("a credential");
        let db = Db::open(&state.join(DATABASE_FILE))
            .await
            .expect("the role's database opens");
        if pending {
            SqliteIngestState::new(db.clone())
                .request_rescore(SystemClock.now())
                .await
                .expect("the request is stored");
        }
        db.close().await;
        let offset = offset_to_half_past_noon();
        let environment = [
            ("STATE_DIRECTORY", state.as_os_str()),
            ("CREDENTIALS_DIRECTORY", credentials.as_os_str()),
            (
                "DECKSTREAK_SYNC_ENDPOINT",
                OsStr::new("http://127.0.0.1:9/"),
            ),
            ("DECKSTREAK_ROLLOVER_HOUR", OsStr::new("12")),
            ("DECKSTREAK_UTC_OFFSET_MINUTES", OsStr::new(&offset)),
            (LAW_DECK_ROOT, OsStr::new("Law\u{1f}Evidence")),
            ("DECKSTREAK_SYNC_REQUEST_PATH", doorbell.as_os_str()),
        ];
        let output = deckstreakd(&["job", "sync"], &environment);
        let served = request_events(&output);
        assert_eq!(
            served.len(),
            usize::from(pending),
            "flag pending={pending}: {}",
            describe(&output)
        );
        assert_eq!(
            fs::read(&doorbell).expect("the file stays"),
            payload,
            "the job leaves the request file untouched"
        );
    }
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

#[tokio::test]
async fn only_the_sync_job_loads_the_owners_conventions() {
    // The conventions file the setting names does not exist. The sync job builds the instruments,
    // so it refuses to start on it; a job that builds none never reads the file and runs.
    let directory = tempfile::tempdir().expect("a temporary directory");
    let state = directory.path().join("state");
    fs::create_dir_all(&state).expect("a folder");
    let missing = directory.path().join("absent-conventions.json");
    let environment = [
        ("STATE_DIRECTORY", state.as_os_str()),
        ("DECKSTREAK_CONVENTIONS_FILE", missing.as_os_str()),
    ];

    let sync = deckstreakd(&["job", "sync"], &environment);
    assert_eq!(sync.status.code(), Some(1), "{}", describe(&sync));
    let stopped: Vec<String> = events(&sync)
        .into_iter()
        .filter(|(_, event)| event["message"] == "the role stopped with an error")
        .map(|(_, event)| event["error"].to_string())
        .collect();
    assert_eq!(stopped.len(), 1, "{}", describe(&sync));
    assert!(
        stopped[0].contains("the job role") && stopped[0].contains("DECKSTREAK_CONVENTIONS_FILE"),
        "{stopped:?}"
    );

    let maintenance = deckstreakd(&["job", "maintenance"], &environment);
    assert_eq!(
        maintenance.status.code(),
        Some(0),
        "{}",
        describe(&maintenance)
    );
}

#[test]
fn the_mcp_role_is_a_known_role() {
    // `deckstreakd mcp` runs the mcp role: with no listen address it refuses to start naming the
    // setting from its first line, as a JSON event with its priority, and exits 1, not 2.
    let directory = tempfile::tempdir().expect("a temporary directory");
    let output = deckstreakd(
        &["mcp"],
        &[("STATE_DIRECTORY", directory.path().as_os_str())],
    );
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
    let lines = events(&output);
    let first = lines.first().cloned().unwrap_or((None, Value::Null));
    assert_eq!(first.0.as_deref(), Some("<3>"), "{}", describe(&output));
    let refusal = first.1.to_string();
    assert!(
        refusal.contains("DECKSTREAK_MCP_LISTEN"),
        "the refusal does not name the setting: {refusal}"
    );

    // The role takes no argument, and the usage line names it among the roles.
    let usage = deckstreakd(&["mcp", "extra"], &[]);
    assert_eq!(usage.status.code(), Some(2), "{}", describe(&usage));
    let lines = events(&usage);
    let first = lines.first().cloned().unwrap_or((None, Value::Null));
    let message = first.1["message"].as_str().unwrap_or_default().to_owned();
    assert!(
        message.contains("the roles are: api, bot, job, data, mcp, preset;"),
        "{message}"
    );
}

#[test]
fn the_mcp_role_that_cannot_open_its_database_says_stopping_on_its_notify_socket() {
    // The database's path holds a directory, so the open refuses after the role has bound its
    // listener and made its notifier: the role leaves before it serves, and `stop_before_serving`
    // is what tells systemd `STOPPING=1` on the way out.
    let directory = tempfile::tempdir().expect("a temporary directory");
    let state = directory.path().join("state");
    fs::create_dir_all(state.join(DATABASE_FILE)).expect("a directory where the database goes");
    let credentials = directory.path().join("credentials");
    fs::create_dir(&credentials).expect("the credentials directory");
    for (id, stem) in [
        ("mcp-core-token", "stopping-core"),
        ("mcp-law-track-token", "stopping-law"),
    ] {
        let value = format!("{stem}-{}", "k".repeat(32));
        fs::write(credentials.join(id), format!("{value}\n")).expect("a credential");
    }
    let socket_path = directory.path().join("notify.socket");
    let socket = UnixDatagram::bind(&socket_path).expect("the notify socket binds");
    socket
        .set_nonblocking(true)
        .expect("a non-blocking notify socket");

    let output = deckstreakd(
        &["mcp"],
        &[
            ("STATE_DIRECTORY", state.as_os_str()),
            ("CREDENTIALS_DIRECTORY", credentials.as_os_str()),
            ("DECKSTREAK_MCP_LISTEN", OsStr::new("127.0.0.1:0")),
            ("NOTIFY_SOCKET", socket_path.as_os_str()),
        ],
    );
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));

    // The child has exited, so every datagram it sent is already queued.
    let mut seen = Vec::new();
    let mut buffer = [0_u8; 256];
    while let Ok(length) = socket.recv(&mut buffer) {
        seen.push(String::from_utf8_lossy(&buffer[..length]).into_owned());
    }
    assert!(
        seen.iter().any(|message| message == "READY=1"),
        "the role was bound and ready before it refused; seen: {seen:?}"
    );
    assert_eq!(
        seen.iter()
            .filter(|message| message.as_str() == "STOPPING=1")
            .count(),
        1,
        "the role did not say STOPPING=1 once; seen: {seen:?}"
    );
}
