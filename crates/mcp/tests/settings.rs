//! SPEC-119 A2 to A6: the guard's credentials, read only through the credential loader, and every
//! refusal by the credential's id (R6, R7, R8; T15).

// An integration test is test code: its helpers panic on a failed fixture.
#![allow(clippy::expect_used)]

use std::fs;

use deck_streak_kernel::{CredentialError, CredentialLoader, CredentialsDirectory, Redactor};
use deck_streak_mcp::{Grants, McpError, Scopes};

const CORE: &str = "mcp-core-token";
const LAW_TRACK: &str = "mcp-law-track-token";

/// A token of `length` characters, built from parts at run time.
fn token(stem: &str, length: usize) -> String {
    let mut value = format!("settings-{stem}-");
    let fill = length.saturating_sub(value.chars().count());
    value.push_str(&"k".repeat(fill));
    value.truncate(length);
    value
}

/// How a credential sits in the credentials directory.
enum Credential {
    /// A file holding the value and the line feed systemd writes.
    Text(String),
    /// A file holding these bytes exactly.
    Bytes(Vec<u8>),
    /// A directory of the credential's name, which the loader cannot read as a file.
    Unreadable,
}

/// A credentials directory holding `credentials`.
fn directory(credentials: &[(&str, Credential)]) -> tempfile::TempDir {
    let directory = tempfile::tempdir().expect("a temporary directory");
    for (id, credential) in credentials {
        let path = directory.path().join(id);
        match credential {
            Credential::Text(value) => fs::write(path, format!("{value}\n")),
            Credential::Bytes(bytes) => fs::write(path, bytes),
            Credential::Unreadable => fs::create_dir(path),
        }
        .expect("a credential");
    }
    directory
}

/// The guard's grants loaded from `credentials`.
fn load(credentials: &[(&str, Credential)]) -> Result<Grants, McpError> {
    let directory = directory(credentials);
    let path = CredentialsDirectory::new(directory.path()).expect("an absolute path");
    Grants::load(&CredentialLoader::new(path, Redactor::new()))
}

/// The scope names each loaded grant holds, in load order.
fn scope_names(grants: &Grants) -> Vec<Vec<&'static str>> {
    grants.scopes().into_iter().map(Scopes::names).collect()
}

/// The broken forms of a credential the loader refuses: empty, unreadable and non-text.
fn broken() -> Vec<(&'static str, Credential)> {
    vec![
        ("empty", Credential::Bytes(b"\n".to_vec())),
        ("unreadable", Credential::Unreadable),
        ("non-text", Credential::Bytes(vec![0xff, 0xfe, 0xfd, b'\n'])),
    ]
}

#[test]
fn the_core_credential_is_required() {
    // A missing core credential refuses start by its id.
    let missing = load(&[(LAW_TRACK, Credential::Text(token("law", 40)))]);
    assert!(
        matches!(
            missing,
            Err(McpError::Credential(CredentialError::Missing { id: CORE }))
        ),
        "a missing core credential: {missing:?}"
    );

    // So does an empty, an unreadable and a non-text one.
    for (form, credential) in broken() {
        let refused = load(&[(CORE, credential)]);
        let by_id = match &refused {
            Err(McpError::Credential(CredentialError::Empty { id })) => {
                form == "empty" && *id == CORE
            }
            Err(McpError::Credential(CredentialError::Unreadable { id, .. })) => {
                form == "unreadable" && *id == CORE
            }
            Err(McpError::Credential(CredentialError::NotText { id })) => {
                form == "non-text" && *id == CORE
            }
            _ => false,
        };
        assert!(by_id, "a {form} core credential: {refused:?}");
    }

    // And a valid core credential alone starts, granting core.
    let grants =
        load(&[(CORE, Credential::Text(token("core", 40)))]).expect("the core token starts");
    assert_eq!(scope_names(&grants), vec![vec!["core"]]);
}

#[test]
fn a_missing_scope_credential_grants_nothing() {
    let grants = load(&[(CORE, Credential::Text(token("core", 40)))])
        .expect("a missing law-track credential still starts");
    assert_eq!(scope_names(&grants), vec![vec!["core"]]);

    let both = load(&[
        (CORE, Credential::Text(token("core", 40))),
        (LAW_TRACK, Credential::Text(token("law", 40))),
    ])
    .expect("both credentials start");
    assert_eq!(
        scope_names(&both),
        vec![vec!["core"], vec!["core", "law_track"]]
    );
}

#[test]
fn a_broken_scope_credential_refuses_start() {
    for (form, credential) in broken() {
        let refused = load(&[
            (CORE, Credential::Text(token("core", 40))),
            (LAW_TRACK, credential),
        ]);
        let by_id = match &refused {
            Err(McpError::Credential(CredentialError::Empty { id })) => {
                form == "empty" && *id == LAW_TRACK
            }
            Err(McpError::Credential(CredentialError::Unreadable { id, .. })) => {
                form == "unreadable" && *id == LAW_TRACK
            }
            Err(McpError::Credential(CredentialError::NotText { id })) => {
                form == "non-text" && *id == LAW_TRACK
            }
            _ => false,
        };
        assert!(by_id, "a {form} law-track credential: {refused:?}");
    }
}

#[test]
fn a_short_credential_refuses_start() {
    // 31 characters refuse start, by the credential's id.
    let short_core = load(&[(CORE, Credential::Text(token("core", 31)))]);
    assert!(
        matches!(short_core, Err(McpError::WeakCredential { id: CORE })),
        "a 31-character core token: {short_core:?}"
    );
    let short_law = load(&[
        (CORE, Credential::Text(token("core", 40))),
        (LAW_TRACK, Credential::Text(token("law", 31))),
    ]);
    assert!(
        matches!(short_law, Err(McpError::WeakCredential { id: LAW_TRACK })),
        "a 31-character law-track token: {short_law:?}"
    );

    // A token ending in a carriage return (a file written with CRLF) can never be presented, so it
    // refuses start as unpresentable, before its length is read.
    let carriage = load(&[(CORE, Credential::Text(format!("{}\r", token("core", 40))))]);
    assert!(
        matches!(
            carriage,
            Err(McpError::UnpresentableCredential { id: CORE })
        ),
        "a core token ending in a carriage return: {carriage:?}"
    );
    let short_carriage = load(&[
        (CORE, Credential::Text(token("core", 40))),
        (
            LAW_TRACK,
            Credential::Text(format!("{}\r", token("law", 20))),
        ),
    ]);
    assert!(
        matches!(
            short_carriage,
            Err(McpError::UnpresentableCredential { id: LAW_TRACK })
        ),
        "a short law-track token ending in a carriage return: {short_carriage:?}"
    );

    // And 32 characters start, for either credential.
    let grants = load(&[
        (CORE, Credential::Text(token("core", 32))),
        (LAW_TRACK, Credential::Text(token("law", 32))),
    ])
    .expect("32-character tokens start");
    assert_eq!(
        scope_names(&grants),
        vec![vec!["core"], vec!["core", "law_track"]]
    );
}

#[test]
fn two_credentials_with_one_value_refuse_start() {
    let value = token("shared", 40);
    let refused = load(&[
        (CORE, Credential::Text(value.clone())),
        (LAW_TRACK, Credential::Text(value)),
    ]);
    assert!(
        matches!(
            refused,
            Err(McpError::SharedCredential {
                first: CORE,
                second: LAW_TRACK
            })
        ),
        "two credentials with one value: {refused:?}"
    );

    // Two values that differ only in their last character are two grants.
    let mut other = token("shared", 40);
    other.pop();
    other.push('x');
    let distinct = load(&[
        (CORE, Credential::Text(token("shared", 40))),
        (LAW_TRACK, Credential::Text(other)),
    ])
    .expect("two values start");
    assert_eq!(
        scope_names(&distinct),
        vec![vec!["core"], vec!["core", "law_track"]]
    );
}
