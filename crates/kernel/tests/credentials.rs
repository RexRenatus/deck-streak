//! A secret is read from the credentials directory and never from the environment, and a missing
//! credential refuses start by its id (SPEC-020 A11, A12; the rust-service pack's
//! `rs.no-secret-env` holds the production code to the same rule).
//!
//! "Never from the environment" is measured on a child process: this test binary run again with
//! the credential's name set in its real environment, where the loader must still find nothing.

// An integration test is test code: its helpers panic on a failed file write or child.
#![allow(clippy::expect_used)]

use std::fs;
use std::path::Path;
use std::process::Command;

use deck_streak_kernel::redact::REDACTED;
use deck_streak_kernel::settings::CREDENTIALS_DIRECTORY;
use deck_streak_kernel::{CredentialError, CredentialLoader, CredentialsDirectory, Redactor};

/// The credential the tests load, and synthetic values for it.
const ID: &str = "sync-login";
const FROM_THE_FILE: &str = "tidal-orchid-velvet";
const FROM_THE_ENVIRONMENT: &str = "copper-sparrow-drift";

/// A loader over `directory`, registering with `redactor`.
fn loader(directory: &Path, redactor: &Redactor) -> CredentialLoader {
    CredentialLoader::new(
        CredentialsDirectory::new(directory).expect("an absolute directory"),
        redactor.clone(),
    )
}

#[test]
#[ignore = "a child process: a_secret_is_read_from_the_credentials_directory_and_never_from_the_environment runs it"]
fn child_finds_no_credential_in_its_environment() {
    // This process's environment carries the credential's name, with a value; its credentials
    // directory holds no file of that name.
    let directory = std::env::var_os(CREDENTIALS_DIRECTORY).expect("the parent sets it");
    let refused = loader(Path::new(&directory), &Redactor::new()).load(ID);
    assert!(
        matches!(refused, Err(CredentialError::Missing { id: ID })),
        "a credential missing from the directory was read from somewhere else: {refused:?}"
    );
}

#[test]
fn a_secret_is_read_from_the_credentials_directory_and_never_from_the_environment() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    fs::write(directory.path().join(ID), format!("{FROM_THE_FILE}\n")).expect("written");
    let redactor = Redactor::new();
    let loader = loader(directory.path(), &redactor);

    // The file's value, less its one trailing newline.
    let secret = loader.load(ID);
    assert_eq!(
        secret.as_ref().map(|secret| secret.expose()).ok(),
        Some(FROM_THE_FILE)
    );
    // It was registered before it was returned: a line carrying it leaves as the marker.
    assert_eq!(
        redactor.redact(&format!("sync as {FROM_THE_FILE} failed")),
        format!("sync as {REDACTED} failed")
    );
    assert!(
        !format!("{secret:?}").contains(FROM_THE_FILE),
        "a secret's Debug shows its value"
    );
    // One trailing newline is trimmed, and only one.
    fs::write(directory.path().join(ID), "two-newlines-value\n\n").expect("written");
    assert_eq!(
        loader.load(ID).expect("the credential loads").expose(),
        "two-newlines-value\n"
    );

    // A process whose environment carries the credential's name, and whose directory does not,
    // refuses it as missing.
    let empty = tempfile::tempdir().expect("an empty credentials directory");
    let child = Command::new(std::env::current_exe().expect("this test binary's path"))
        .args([
            "--exact",
            "child_finds_no_credential_in_its_environment",
            "--ignored",
            "--nocapture",
        ])
        .env(CREDENTIALS_DIRECTORY, empty.path())
        .env(ID, FROM_THE_ENVIRONMENT)
        .env("DECKSTREAK_SYNC_LOGIN", FROM_THE_ENVIRONMENT)
        .output()
        .expect("the child process runs");
    let stdout = String::from_utf8_lossy(&child.stdout);
    assert!(
        child.status.success() && stdout.contains("test result: ok. 1 passed"),
        "the child read the credential, or did not run:\n{stdout}\n{}",
        String::from_utf8_lossy(&child.stderr)
    );
}

#[test]
fn a_missing_credential_refuses_start_by_its_id() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let loader = loader(directory.path(), &Redactor::new());
    let refused = loader.load("telegram-bot-token");
    assert!(
        matches!(
            refused,
            Err(CredentialError::Missing {
                id: "telegram-bot-token"
            })
        ),
        "{refused:?}"
    );
    assert_eq!(
        refused.map(|_| ()).map_err(|refusal| refusal.to_string()),
        Err(
            "the credential telegram-bot-token is missing from the credentials directory"
                .to_owned()
        )
    );
    // An id that is not a plain file name in the directory is refused before anything is read.
    for id in ["", ".", "..", "../escape", "nested/id"] {
        let refused = loader.load(id);
        assert!(
            matches!(refused, Err(CredentialError::InvalidId { .. })),
            "{id:?} was read as a credential id: {refused:?}"
        );
    }
}
