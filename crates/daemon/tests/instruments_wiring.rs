//! SPEC-094 R6 and R9: a role builds the instruments only when its settings allow, a conventions
//! file that refuses stops the start, and the api's late holder answers "not ready" until it is
//! filled. Every value is synthetic.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::ffi::OsStr;
use std::sync::Arc;

use deck_streak_coordination::instruments::{InstrumentService, Instruments};
use deck_streak_daemon::wiring::{LateInstruments, StateDirectory, instruments_for_role};
use deck_streak_ingest::settings::{STATE_DIRECTORY, SYNC_ENDPOINT};
use deck_streak_kernel::conventions::CONVENTIONS_FILE;
use deck_streak_kernel::{
    ConventionsError, Db, Environment, KernelError, Offload, OffloadWorkers, StudyDayRule,
    SystemClock,
};
use tempfile::TempDir;

fn offload() -> Offload {
    Offload::new(
        OffloadWorkers::new(1).expect("one worker"),
        Arc::new(SystemClock),
    )
}

async fn database(dir: &TempDir) -> Db {
    Db::open(&dir.path().join("ds.db")).await.expect("open")
}

#[tokio::test]
async fn a_role_with_valid_settings_gets_the_instruments() {
    let dir = TempDir::new().expect("dir");
    let state = StateDirectory::new(dir.path()).expect("absolute");
    let env = Environment::from_vars([
        (SYNC_ENDPOINT, OsStr::new("http://127.0.0.1:9/")),
        (STATE_DIRECTORY, dir.path().as_os_str()),
    ]);
    let built = instruments_for_role(
        &env,
        database(&dir).await,
        &state,
        offload(),
        StudyDayRule::default(),
    )
    .expect("the conventions load");
    let instruments = built.expect("the instruments are built");
    let listing = instruments.list().await.expect("the listing reads");
    assert_eq!(
        listing
            .iter()
            .map(|row| row.id.as_str())
            .collect::<Vec<_>>(),
        vec!["dark_fields"]
    );
}

#[tokio::test]
async fn a_role_whose_copy_settings_refuse_has_no_instruments() {
    let dir = TempDir::new().expect("dir");
    let state = StateDirectory::new(dir.path()).expect("absolute");
    let env = Environment::from_vars([(STATE_DIRECTORY, dir.path().as_os_str())]);
    let built = instruments_for_role(
        &env,
        database(&dir).await,
        &state,
        offload(),
        StudyDayRule::default(),
    );
    assert!(matches!(built, Ok(None)), "{built:?}");
}

#[tokio::test]
async fn an_unreadable_conventions_file_stops_the_start() {
    let dir = TempDir::new().expect("dir");
    let state = StateDirectory::new(dir.path()).expect("absolute");
    let missing = dir.path().join("absent.json");
    let env = Environment::from_vars([
        (SYNC_ENDPOINT, OsStr::new("http://127.0.0.1:9/")),
        (STATE_DIRECTORY, dir.path().as_os_str()),
        (CONVENTIONS_FILE, missing.as_os_str()),
    ]);
    let built = instruments_for_role(
        &env,
        database(&dir).await,
        &state,
        offload(),
        StudyDayRule::default(),
    );
    assert_eq!(
        built.err(),
        Some(ConventionsError::Unreadable {
            setting: CONVENTIONS_FILE
        })
    );
}

#[tokio::test]
async fn the_late_holder_answers_not_ready_until_it_is_filled() {
    let dir = TempDir::new().expect("dir");
    let late = LateInstruments::new();
    let before = late.list().await;
    assert!(
        matches!(
            before,
            Err(KernelError::Offload {
                operation: "instruments_not_ready"
            })
        ),
        "{before:?}"
    );
    let instruments = Instruments::new(
        database(&dir).await,
        dir.path(),
        Arc::new(SystemClock),
        StudyDayRule::default(),
        vec![],
    );
    late.fill(Arc::new(instruments));
    let after = late.list().await;
    assert!(after.is_ok(), "{after:?}");
}
