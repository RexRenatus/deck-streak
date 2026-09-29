//! A secret is read from the credentials directory and never from the environment, and a missing
//! credential refuses start by its id (SPEC-020 A11, A12; the rust-service pack's
//! `rs.no-secret-env` holds the production code to the same rule). An empty credential, no bytes
//! or only the one trailing newline the loader trims, refuses start the same way, and its refusal
//! carries the id and nothing else (SPEC-066 A1 to A3).
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
use deck_streak_kernel::{
    CredentialError, CredentialLoader, CredentialsDirectory, Redactor, Secret,
};

/// The credential the tests load, and synthetic values for it.
const ID: &str = "sync-login";
const FROM_THE_FILE: &str = "tidal-orchid-velvet";
const FROM_THE_ENVIRONMENT: &str = "copper-sparrow-drift";

/// The two forms of an empty credential: no bytes, and only the one trailing newline the loader
/// trims (SPEC-066 R1).
const EMPTY_FORMS: [&str; 2] = ["", "\n"];
/// A synthetic value planted where a careless refusal could pick one up: the credentials
/// directory's name, and a sibling credential the same loader reads first.
const SENTINEL: &str = "quartz-lantern-sentinel";
/// The sibling credential that holds the sentinel.
const SIBLING: &str = "sync-password";

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
        secret.as_ref().map(Secret::expose).ok(),
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
    // What its Debug shows instead: the type, and never the value.
    assert_eq!(format!("{secret:?}"), "Ok(Secret(..))");
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

#[test]
fn a_credential_that_exists_and_cannot_be_read_is_unreadable_and_not_missing() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    // A directory of the credential's name exists, and reading it as a file fails for a reason
    // other than the file's absence.
    fs::create_dir(directory.path().join(ID)).expect("a directory of that name");
    let refused = loader(directory.path(), &Redactor::new()).load(ID);
    assert!(
        matches!(refused, Err(CredentialError::Unreadable { id: ID, .. })),
        "an unreadable credential was reported as {refused:?}"
    );
}

#[test]
fn a_secret_shows_no_value_in_debug() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    fs::write(directory.path().join(ID), FROM_THE_FILE).expect("written");
    let secret = loader(directory.path(), &Redactor::new())
        .load(ID)
        .expect("the credential loads");
    assert_eq!(format!("{secret:?}"), "Secret(..)");
}

#[test]
fn an_empty_credential_refuses_start_by_its_id() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let loader = loader(directory.path(), &Redactor::new());
    for form in EMPTY_FORMS {
        fs::write(directory.path().join(ID), form).expect("written");
        let refused = loader.load(ID);
        assert!(
            matches!(refused, Err(CredentialError::Empty { id: ID })),
            "{form:?} was not refused as an empty {ID}: {refused:?}"
        );
        assert_eq!(
            refused.map(|_| ()).map_err(|refusal| refusal.to_string()),
            Err(format!(
                "the credential {ID} is empty in the credentials directory"
            )),
            "{form:?}"
        );
    }
}

#[test]
fn a_missing_credential_keeps_its_refusal_and_a_value_loads_unchanged() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let loader = loader(directory.path(), &Redactor::new());
    // No file of the id: it is missing, and the empty refusal does not take its place.
    let refused = loader.load(ID);
    assert!(
        matches!(refused, Err(CredentialError::Missing { id: ID })),
        "{refused:?}"
    );
    // A path of the id that exists and cannot be read as a file, here a directory, is refused as
    // unreadable: neither missing nor empty takes its place.
    fs::create_dir(directory.path().join(SIBLING)).expect("a directory at the id's path");
    let unreadable = loader.load(SIBLING);
    assert!(
        matches!(
            unreadable,
            Err(CredentialError::Unreadable { id: SIBLING, .. })
        ),
        "{unreadable:?}"
    );
    assert_eq!(
        unreadable
            .map(|_| ())
            .map_err(|refusal| refusal.to_string()),
        Err(format!("the credential {SIBLING} cannot be read"))
    );
    // A value of one character or more loads unchanged, less one trailing newline: a lone
    // character, and a file of two newlines, which holds one newline as its value.
    for (content, value) in [
        ("x", "x"),
        ("x\n", "x"),
        ("\n\n", "\n"),
        (FROM_THE_FILE, FROM_THE_FILE),
    ] {
        fs::write(directory.path().join(ID), content).expect("written");
        let secret = loader.load(ID);
        assert_eq!(
            secret.as_ref().map(Secret::expose).ok(),
            Some(value),
            "{content:?}: {secret:?}"
        );
    }
}

#[test]
fn an_empty_refusal_names_the_id_and_never_a_value() {
    // The sentinel is the credentials directory's name, and the value of a sibling credential the
    // same loader reads first; the refusal of the empty credential beside it carries neither.
    let scratch = tempfile::tempdir().expect("a temporary directory");
    let directory = scratch.path().join(SENTINEL);
    fs::create_dir(&directory).expect("the credentials directory");
    let loader = loader(&directory, &Redactor::new());
    fs::write(directory.join(SIBLING), format!("{SENTINEL}\n")).expect("written");
    let sibling = loader.load(SIBLING);
    assert_eq!(sibling.as_ref().map(Secret::expose).ok(), Some(SENTINEL));
    for form in EMPTY_FORMS {
        fs::write(directory.join(ID), form).expect("written");
        let said = loader
            .load(ID)
            .map(|_| ())
            .map_err(|refusal| (refusal.to_string(), format!("{refusal:?}")));
        // The id, and only the id: the message, and the variant's own Debug.
        assert_eq!(
            said,
            Err((
                format!("the credential {ID} is empty in the credentials directory"),
                format!("Empty {{ id: {ID:?} }}"),
            )),
            "{form:?}"
        );
        let (display, debug) = said.err().unwrap_or_default();
        assert!(
            !display.contains(SENTINEL) && !debug.contains(SENTINEL),
            "{form:?}: the refusal carries the sentinel: {display} / {debug}"
        );
    }
}
