//! The public origin that turns linking on (SPEC-359 R1; A26, A27).
//!
//! Unset or blank, linking is off and every use case answers `linking_off`; set, the value must be
//! an `https` origin, and any other shape refuses start by the setting's name, never its value.

// An integration test is test code: its helpers panic on a failed fixture, and the examined counts
// are printed on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

mod support;

use deck_streak_identity::linking_config::PUBLIC_ORIGIN;
use deck_streak_identity::{LinkingConfig, PasskeyError, Refusal};
use deck_streak_kernel::Environment;
use serde_json::Value;

use support::fixture_with;

/// A link code's text the store never minted: 16 zero bytes as base64url.
const UNMINTED_CODE: &str = "AAAAAAAAAAAAAAAAAAAAAA";
/// A flow id no ceremony holds.
const UNKNOWN_FLOW: &str = "no-such-flow";
/// A passkey row id no row holds.
const UNKNOWN_ROW: i64 = 1;

/// Prints how many items a check examined and refuses zero (the tdd pack's examined contract).
fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}

/// The refusal a use case answered, if it answered one.
fn refusal<T>(answer: Result<T, PasskeyError>) -> Option<Refusal> {
    answer.err().and_then(|error| error.refusal())
}

#[tokio::test]
async fn linking_is_off_without_the_public_origin() {
    let mut configs = Vec::new();
    for value in [None, Some(""), Some("   ")] {
        let config = LinkingConfig::from_setting(value);
        assert!(
            config.is_ok(),
            "the public origin {value:?} refused start: unset or blank is linking off"
        );
        configs.push(config.expect("checked above"));
    }
    let unset = Environment::from_vars(Vec::<(String, String)>::new());
    let blank = Environment::from_vars([(PUBLIC_ORIGIN, "  ")]);
    for env in [unset, blank] {
        let config = LinkingConfig::from_env(&env);
        assert!(
            config.is_ok(),
            "an unset or blank {PUBLIC_ORIGIN} refused start: it is linking off"
        );
        configs.push(config.expect("checked above"));
    }

    for config in examined("configurations without a public origin", configs) {
        assert_eq!(config.relying_party().err(), Some(Refusal::LinkingOff));
        let fixture = fixture_with(config).await;
        let telegram = fixture
            .sessions
            .open(fixture.owner)
            .expect("a telegram session opens");
        let telegram = telegram.expose();
        let link = fixture.link_session();
        let db = &fixture.db;
        let passkeys = &fixture.passkeys;
        let answers = vec![
            ("mint", refusal(passkeys.mint_link_code(telegram))),
            (
                "redeem",
                refusal(passkeys.redeem_link_code(UNMINTED_CODE, None)),
            ),
            (
                "register start",
                refusal(passkeys.start_registration(db, &link).await),
            ),
            (
                "register finish",
                refusal(
                    passkeys
                        .finish_registration(db, &link, UNKNOWN_FLOW, &Value::Null)
                        .await,
                ),
            ),
            ("sign-in start", refusal(passkeys.start_sign_in(db).await)),
            (
                "sign-in finish",
                refusal(
                    passkeys
                        .finish_sign_in(db, None, UNKNOWN_FLOW, &Value::Null)
                        .await,
                ),
            ),
            ("methods", refusal(passkeys.methods(db).await)),
            (
                "remove",
                refusal(passkeys.remove(db, telegram, UNKNOWN_ROW).await),
            ),
        ];
        for (use_case, answered) in examined("linking use cases", answers) {
            assert_eq!(
                answered,
                Some(Refusal::LinkingOff),
                "the use case {use_case} ran with linking off"
            );
        }
    }
}

#[test]
fn a_public_origin_that_is_not_https_refuses_start() {
    let refused = examined(
        "values that are not an https origin",
        vec![
            "http://app.example",
            "https://app.example/",
            "https://app.example/link",
            "https://app.example?next=1",
            "https://app.example#top",
            "https://owner@app.example",
            "app.example",
            "ftp://app.example",
        ],
    );
    for value in refused {
        let error = LinkingConfig::from_setting(Some(value)).err();
        assert!(
            error.is_some(),
            "the public origin {value:?} was accepted: it is not an https origin"
        );
        let shown = error.expect("checked above").to_string();
        assert!(
            shown.contains(PUBLIC_ORIGIN),
            "the refusal of {value:?} does not name {PUBLIC_ORIGIN}: {shown}"
        );
        assert!(
            !shown.contains(value),
            "the refusal of {value:?} shows the value: {shown}"
        );
    }
    let from_env = LinkingConfig::from_env(&Environment::from_vars([(
        PUBLIC_ORIGIN,
        "http://app.example",
    )]))
    .err();
    assert!(
        from_env.is_some(),
        "{PUBLIC_ORIGIN} of http://app.example was accepted from the environment"
    );
    let shown = from_env.expect("checked above").to_string();
    assert!(shown.contains(PUBLIC_ORIGIN), "{shown}");
    assert!(!shown.contains("http://app.example"), "{shown}");

    for value in ["https://app.example", "https://app.example:8443"] {
        let config = LinkingConfig::from_setting(Some(value))
            .unwrap_or_else(|error| panic!("the https origin {value:?} was refused: {error}"));
        assert!(
            config.relying_party().is_ok(),
            "the https origin {value:?} turned linking off"
        );
    }
}
