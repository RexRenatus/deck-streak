//! The archive's list command, bounded (SPEC-377 R14; B9): its words run with no shell and an empty
//! environment, the credential's file path its last argument, killed at its time bound, read to its
//! byte bound, and any refusal answering nothing; wired only when its setting and its credential are
//! both present.
//!
//! Every command is a stand-in program the test writes under its own temporary directory: an
//! executable `sh` script whose words are the test's, never a shell string the lister builds.

// An integration test is test code: its helpers panic on a failed fixture, and the examined counts
// are printed on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use deck_streak_api::snapshot_routes::SnapshotLister;
use deck_streak_api::{ApiState, Readiness};
use deck_streak_daemon::role_api::with_snapshot_lister;
use deck_streak_daemon::snapshot_lister::{
    ARCHIVE_LIST_COMMAND, ARCHIVE_LIST_CREDENTIAL, CommandLister, ListCommand,
};
use deck_streak_kernel::{CredentialsDirectory, Environment, Setting, SettingsError};

/// One test at a time: a stand-in written while another test forks could be held open in the
/// fork's child, and refuse to run.
static SERIAL: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Prints how many items a check examined and refuses zero (the tdd pack's examined contract).
fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}

/// An executable stand-in named `name` in `dir`, running `body` under `/bin/sh`.
fn standin(dir: &Path, name: &str, body: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, format!("#!/bin/sh\n{body}\n")).expect("the stand-in is written");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).expect("it is executable");
    path
}

/// The command a setting with `text` names.
fn command(text: &str) -> ListCommand {
    ListCommand::parse(text).expect("an absolute program")
}

/// A lister over `text` with the credential path `credential`, killed at one second and read to
/// 64 bytes.
fn small(text: &str, credential: &Path) -> CommandLister {
    CommandLister::bounded(
        command(text),
        credential.to_path_buf(),
        Duration::from_secs(1),
        64,
    )
}

fn owned(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| (*name).to_owned()).collect()
}

#[tokio::test]
async fn the_listing_is_the_commands_lines() {
    let _serial = SERIAL.lock().await;
    let dir = tempfile::tempdir().expect("a temporary directory");
    let program = standin(
        dir.path(),
        "lists",
        "printf 'sync-a.tar.age\\n\\n  sync-a.sha256.age  \\n'",
    );
    let lister = small(
        &program.display().to_string(),
        &dir.path().join("credential"),
    );
    assert_eq!(
        lister.list().await,
        Some(owned(&["sync-a.tar.age", "sync-a.sha256.age"]))
    );
}

#[tokio::test]
async fn the_words_run_with_no_shell_and_the_credential_path_last() {
    let _serial = SERIAL.lock().await;
    let dir = tempfile::tempdir().expect("a temporary directory");
    let program = standin(
        dir.path(),
        "echoes",
        "printf '%s\\n' \"$@\"\nprintf '%s\\n' \"${HOME:-no-home}\"",
    );
    let credentials = dir.path().join("credentials");
    fs::create_dir(&credentials).expect("the credential directory");
    fs::write(credentials.join(ARCHIVE_LIST_CREDENTIAL), "synthetic").expect("the credential");
    let env = Environment::from_vars([(
        ARCHIVE_LIST_COMMAND,
        format!("{} list a;b $(true) * 'q'", program.display()),
    )]);
    let directory = CredentialsDirectory::new(&credentials).expect("an absolute directory");
    let lister = CommandLister::configured(&env, &directory)
        .expect("the setting is well formed")
        .expect("the command and the credential are both present");
    let path = credentials
        .join(ARCHIVE_LIST_CREDENTIAL)
        .display()
        .to_string();
    // Each word arrives as itself, the credential's path last, and the environment is empty.
    assert_eq!(
        lister.list().await,
        Some(owned(&[
            "list", "a;b", "$(true)", "*", "'q'", &path, "no-home"
        ]))
    );
}

#[tokio::test]
async fn a_command_past_its_time_bound_answers_unknown() {
    let _serial = SERIAL.lock().await;
    let dir = tempfile::tempdir().expect("a temporary directory");
    let credential = dir.path().join("credential");
    let quick = standin(dir.path(), "quick", "printf 'sync-a.tar.age\\n'");
    assert_eq!(
        small(&quick.display().to_string(), &credential)
            .list()
            .await,
        Some(owned(&["sync-a.tar.age"])),
        "a command inside its bound is listed"
    );
    let slow = standin(
        dir.path(),
        "slow",
        "printf 'sync-a.tar.age\\n'\nexec /bin/sleep 5",
    );
    let started = Instant::now();
    let answer = tokio::time::timeout(
        Duration::from_secs(30),
        small(&slow.display().to_string(), &credential).list(),
    )
    .await
    .expect("the lister answers inside the test's own bound");
    assert_eq!(answer, None, "a command past its bound answers nothing");
    assert!(
        started.elapsed() < Duration::from_secs(4),
        "the command was killed at its one second, not waited for: {:?}",
        started.elapsed()
    );
}

#[tokio::test]
async fn a_command_past_its_time_bound_is_stopped_not_left_running() {
    let _serial = SERIAL.lock().await;
    let dir = tempfile::tempdir().expect("a temporary directory");
    let credential = dir.path().join("credential");
    // The control: a command inside its bound runs to its end and writes its mark.
    let shown = dir.path().join("shown");
    let quick = standin(
        dir.path(),
        "quick",
        &format!(
            "printf 'sync-a.tar.age\\n'\nprintf 'late' > '{}'",
            shown.display()
        ),
    );
    assert_eq!(
        small(&quick.display().to_string(), &credential)
            .list()
            .await,
        Some(owned(&["sync-a.tar.age"])),
        "a command inside its bound is listed"
    );
    assert!(shown.exists(), "the control's mark was not written");
    // A command still running at its one second is stopped there, so its shell never wakes to
    // write the mark it would write at two.
    let late = dir.path().join("late");
    let slow = standin(
        dir.path(),
        "slow",
        &format!("/bin/sleep 2\nprintf 'late' > '{}'", late.display()),
    );
    assert_eq!(
        small(&slow.display().to_string(), &credential).list().await,
        None,
        "a command past its bound answers nothing"
    );
    tokio::time::sleep(Duration::from_secs(3)).await;
    assert!(
        !late.exists(),
        "the command outlived its listing and wrote its mark"
    );
}

#[tokio::test]
async fn output_past_its_bound_answers_unknown() {
    let _serial = SERIAL.lock().await;
    let dir = tempfile::tempdir().expect("a temporary directory");
    let credential = dir.path().join("credential");
    // 63 characters and a newline are the 64 bytes the bound reads.
    let name = "x".repeat(63);
    let full = standin(dir.path(), "full", &format!("printf '{name}\\n'"));
    assert_eq!(
        small(&full.display().to_string(), &credential).list().await,
        Some(owned(&[&name])),
        "64 bytes are read whole"
    );
    let over = standin(dir.path(), "over", &format!("printf 'y{name}\\n'"));
    assert_eq!(
        small(&over.display().to_string(), &credential).list().await,
        None,
        "65 bytes are past the bound"
    );
}

#[tokio::test]
async fn a_refused_command_answers_unknown() {
    let _serial = SERIAL.lock().await;
    let dir = tempfile::tempdir().expect("a temporary directory");
    let credential = dir.path().join("credential");
    let listed = standin(dir.path(), "listed", "printf 'sync-a.tar.age\\n'\nexit 0");
    assert_eq!(
        small(&listed.display().to_string(), &credential)
            .list()
            .await,
        Some(owned(&["sync-a.tar.age"])),
        "a command that exits 0 is listed"
    );
    let refusals = examined(
        "refusals",
        vec![
            (
                "a non-zero exit",
                standin(dir.path(), "refuses", "printf 'sync-a.tar.age\\n'\nexit 3"),
            ),
            (
                "output that is not UTF-8",
                standin(dir.path(), "binary", "printf 'sync-\\377.tar.age\\n'"),
            ),
            ("a program that is absent", dir.path().join("absent")),
        ],
    );
    for (what, program) in refusals {
        assert_eq!(
            small(&program.display().to_string(), &credential)
                .list()
                .await,
            None,
            "{what} answers nothing"
        );
    }
}

#[test]
fn the_lister_is_wired_only_with_its_command_and_its_credential() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let directory = CredentialsDirectory::new(dir.path()).expect("an absolute directory");
    let set = Environment::from_vars([(ARCHIVE_LIST_COMMAND, "/usr/bin/lister ls")]);
    let unset = Environment::from_vars([("DECKSTREAK_OTHER", "set")]);
    assert!(
        CommandLister::configured(&unset, &directory)
            .expect("well formed")
            .is_none(),
        "no setting, no lister"
    );
    assert!(
        CommandLister::configured(&set, &directory)
            .expect("well formed")
            .is_none(),
        "no credential, no lister"
    );
    fs::create_dir(dir.path().join(ARCHIVE_LIST_CREDENTIAL)).expect("a directory by its name");
    assert!(
        CommandLister::configured(&set, &directory)
            .expect("well formed")
            .is_none(),
        "a directory is no credential"
    );
    let other = tempfile::tempdir().expect("a temporary directory");
    fs::write(other.path().join(ARCHIVE_LIST_CREDENTIAL), "synthetic").expect("the credential");
    let held = CredentialsDirectory::new(other.path()).expect("an absolute directory");
    let lister = CommandLister::configured(&set, &held).expect("well formed");
    assert!(lister.is_some(), "both present, a lister");
    let relative = Environment::from_vars([(ARCHIVE_LIST_COMMAND, "lister ls")]);
    assert!(
        matches!(
            CommandLister::configured(&relative, &held),
            Err(SettingsError::Malformed {
                setting: ARCHIVE_LIST_COMMAND,
                ..
            })
        ),
        "a program that is not absolute refuses start by the setting's name"
    );
    // The composed state names the lister only when one is wired.
    let wired = format!(
        "{:?}",
        with_snapshot_lister(ApiState::new(Readiness::new()), lister)
    );
    assert!(wired.ends_with(", snapshot: true }"), "{wired}");
    let off = format!(
        "{:?}",
        with_snapshot_lister(ApiState::new(Readiness::new()), None)
    );
    assert!(!off.contains("snapshot"), "{off}");
}

#[test]
fn a_relative_program_is_refused_with_the_shape_spelled_out() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let directory = CredentialsDirectory::new(dir.path()).expect("an absolute directory");
    let relative = Environment::from_vars([(ARCHIVE_LIST_COMMAND, "lister ls")]);
    let refusal = match CommandLister::configured(&relative, &directory) {
        Err(error) => error.to_string(),
        Ok(_) => panic!("a relative program must refuse start"),
    };
    let shape = "an absolute program and its arguments";
    assert!(
        refusal.ends_with(&format!("it must be {shape}")),
        "{refusal}"
    );
    assert!(ListCommand::parse("/usr/bin/lister ls -l").is_some());
    assert!(ListCommand::parse("lister ls").is_none());
    assert!(ListCommand::parse("").is_none());
}
