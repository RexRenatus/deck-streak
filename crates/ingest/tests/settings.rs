//! Ingest's settings refuse start by name (SPEC-022 A14, R5, R13).

mod support;

use deck_streak_ingest::settings::{STATE_DIRECTORY, SYNC_ENDPOINT, SyncSettings};
use deck_streak_kernel::{Environment, SettingsError};

/// A state directory that names no real host path: settings are parsed, never opened, here.
const STATE: &str = "/state";

#[test]
fn a_missing_sync_endpoint_refuses_start_by_name() {
    let refused = SyncSettings::from_env(&Environment::from_vars([(STATE_DIRECTORY, STATE)]));
    assert!(
        matches!(refused, Err(SettingsError::Missing { setting }) if setting == SYNC_ENDPOINT),
        "a missing {SYNC_ENDPOINT} must refuse start by name, not {refused:?}"
    );
    let named = refused.map(|_| ()).unwrap_err().to_string();
    assert!(named.contains(SYNC_ENDPOINT), "{named}");
    // The same environment with the endpoint set starts, and the copy lives in the state
    // directory, so the refusal above is the endpoint's and nothing else's.
    let settings = SyncSettings::from_env(&Environment::from_vars([
        (STATE_DIRECTORY, STATE),
        (SYNC_ENDPOINT, "https://sync.example.invalid/"),
    ]))
    .expect("an endpoint and a state directory are all the settings a sync needs");
    assert!(settings.copy_path().starts_with(STATE));
}

#[test]
fn a_malformed_sync_endpoint_is_refused_without_its_value() {
    let value = "ftp://sync.example.invalid/";
    let refused = SyncSettings::from_env(&Environment::from_vars([
        (STATE_DIRECTORY, STATE),
        (SYNC_ENDPOINT, value),
    ]));
    assert!(
        matches!(refused, Err(SettingsError::Malformed { setting, .. }) if setting == SYNC_ENDPOINT),
        "{refused:?}"
    );
    let named = refused.map(|_| ()).unwrap_err().to_string();
    assert!(
        named.contains(SYNC_ENDPOINT) && !named.contains("example"),
        "{named}"
    );
}

#[test]
fn a_plain_http_endpoint_is_marked_cleartext() {
    let settings = |endpoint| {
        SyncSettings::from_env(&Environment::from_vars([
            (STATE_DIRECTORY, STATE),
            (SYNC_ENDPOINT, endpoint),
        ]))
        .unwrap()
    };
    assert!(
        settings("http://sync.example.invalid/")
            .endpoint()
            .is_cleartext()
    );
    assert!(
        !settings("https://sync.example.invalid/")
            .endpoint()
            .is_cleartext()
    );
    // The endpoint's Debug names no part of it: the address is private configuration.
    assert_eq!(
        format!("{:?}", settings("http://sync.example.invalid/").endpoint()),
        "SyncEndpoint(..)"
    );
}

#[test]
fn an_endpoint_that_names_no_host_is_refused() {
    let parse = |endpoint| {
        SyncSettings::from_env(&Environment::from_vars([
            (STATE_DIRECTORY, STATE),
            (SYNC_ENDPOINT, endpoint),
        ]))
        .is_ok()
    };
    assert!(parse("https://sync.example.invalid:8080/path"));
    assert!(!parse("http://"));
    assert!(!parse("http:///path"));
    assert!(!parse("http://:8080/"));
    assert!(!parse("http://sync example.invalid/"));
}

#[test]
fn a_cleartext_endpoint_logs_one_warning_that_names_the_setting_and_not_its_value() {
    let logs = support::logs::Logs::default();
    let settings = |endpoint| {
        SyncSettings::from_env(&Environment::from_vars([
            (STATE_DIRECTORY, STATE),
            (SYNC_ENDPOINT, endpoint),
        ]))
        .expect("a valid endpoint")
    };
    tracing::subscriber::with_default(logs.recorder(), || {
        settings("https://sync.example.invalid/").warn_if_cleartext();
    });
    assert!(logs.warnings().is_empty(), "{:?}", logs.events());
    tracing::subscriber::with_default(logs.recorder(), || {
        settings("http://sync.example.invalid/").warn_if_cleartext();
    });
    let warnings = logs.warnings();
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert_eq!(warnings[0].field("setting"), Some(SYNC_ENDPOINT));
    assert!(
        !format!("{:?}", warnings[0]).contains("example"),
        "{warnings:?}"
    );
}
