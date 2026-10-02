//! The layout in force is the owner's file `DECKSTREAK_VAULT_LAYOUT` names, or the vendored default
//! when it is unset; a file that cannot be read or is not a layout refuses the start (SPEC-118 A22,
//! R4).

// An integration test is test code: its helpers panic on an unwritable fixture.
#![allow(clippy::expect_used)]

use std::fs;
use std::path::Path;

use deck_streak_kernel::Environment;
use deck_streak_vault::config::VAULT_LAYOUT;
use deck_streak_vault::{LayoutInForce, StartRefusal, VaultError};

fn naming(file: &Path) -> Environment {
    Environment::from_vars([(VAULT_LAYOUT, file.as_os_str())])
}

#[test]
fn the_layout_in_force_is_the_owners_or_the_default() {
    let unset = LayoutInForce::from_env(&Environment::from_vars(Vec::<(String, String)>::new()))
        .expect("the vendored layout");
    assert_eq!(
        unset.inbox, "90-Inbox",
        "unset, the vendored inbox is in force"
    );
    assert!(
        unset.journal.is_empty(),
        "the vendored layout names no journal"
    );
    assert_eq!(
        unset,
        LayoutInForce::vendored().expect("the vendored layout")
    );

    let dir = tempfile::tempdir().expect("a temporary directory");
    let owners = dir.path().join("layout.json");
    fs::write(
        &owners,
        r#"{"schema": "phx.duty.vault.layout.v1",
            "periodic": {"daily": {"folder": "Daily"}, "weekly": {"folder": "Weekly"}},
            "inbox": "/00-Capture/", "duties": {}, "journal": ["Journal", "Private/Diary/"]}"#,
    )
    .expect("the owner's layout");
    let set = LayoutInForce::from_env(&naming(&owners)).expect("the owner's layout");
    assert_eq!(
        set,
        LayoutInForce {
            inbox: "00-Capture".to_owned(),
            journal: vec!["Journal".to_owned(), "Private/Diary".to_owned()],
        },
        "set, the owner's layout is in force"
    );
    let root = dir.path().join("vault");
    assert_eq!(
        set.journal_paths(&root),
        vec![root.join("Journal"), root.join("Private/Diary")]
    );

    for text in [
        "not a layout",
        r#"{"journal": []}"#,
        r#"{"inbox": 7}"#,
        r#"{"inbox": ""}"#,
        r#"{"inbox": "../outside"}"#,
        r#"{"inbox": "90-Inbox", "journal": "Journal"}"#,
        r#"{"inbox": "90-Inbox", "journal": ["Journal/../.."]}"#,
    ] {
        fs::write(&owners, text).expect("a malformed layout");
        let refused = LayoutInForce::from_env(&naming(&owners));
        assert!(
            matches!(
                refused,
                Err(VaultError::Start(StartRefusal::LayoutMalformed))
            ),
            "{text} is not a layout: {refused:?}"
        );
    }

    let refused = LayoutInForce::from_env(&naming(&dir.path().join("absent.json")));
    assert!(
        matches!(
            refused,
            Err(VaultError::Start(StartRefusal::LayoutUnreadable))
        ),
        "an absent layout file refuses the start: {refused:?}"
    );
    let refused = LayoutInForce::from_env(&Environment::from_vars([(VAULT_LAYOUT, "layout.json")]));
    assert!(
        matches!(
            refused,
            Err(VaultError::Start(StartRefusal::LayoutUnreadable))
        ),
        "a relative layout path refuses the start: {refused:?}"
    );

    let example =
        fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.env.example"))
            .expect(".env.example");
    assert!(
        example
            .lines()
            .any(|line| line == format!("{VAULT_LAYOUT}=")),
        ".env.example names {VAULT_LAYOUT}, unset"
    );
}
