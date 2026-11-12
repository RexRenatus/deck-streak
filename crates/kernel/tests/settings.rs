//! Settings refuse start by name and never by value, and an unset digest hour resolves as the
//! predecessor's did (SPEC-020 A3 to A7).

// An integration test is test code: its helpers panic on a malformed file, and the golden reader
// prints the examined count on purpose. clippy.toml's in-test allowances cover `#[test]` bodies.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use std::ffi::OsString;
use std::fs;
use std::os::unix::ffi::OsStringExt;
use std::path::Path;

use deck_streak_kernel::settings::{
    CREDENTIALS_DIRECTORY, DEFAULT_DIGEST_HOUR, DIGEST_HOUR, OFFLOAD_WORKERS, ROLLOVER_HOUR,
    UTC_OFFSET_MINUTES,
};
use deck_streak_kernel::study_day::DEFAULT_ROLLOVER_HOUR;
use deck_streak_kernel::{
    CredentialsDirectory, Environment, Hour, KernelSettings, Setting, SettingsError, UtcOffset,
};

/// An environment of `pairs`, as the daemon's `main` would hand it in.
fn env(pairs: &[(&str, &str)]) -> Environment {
    Environment::from_vars(pairs.iter().copied())
}

/// The hour of `digest.<key>` in the repository's notifications policy, `HH:MM`.
fn policy_hour(key: &str) -> u8 {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../notifications-policy.json");
    let text = fs::read_to_string(path).expect("notifications-policy.json is readable");
    let policy: serde_json::Value = serde_json::from_str(&text).expect("the policy is JSON");
    let time = policy["digest"][key]
        .as_str()
        .unwrap_or_else(|| panic!("the policy has digest.{key}"));
    let (hour, _) = time.split_once(':').expect("an HH:MM time");
    hour.parse().expect("a whole hour")
}

#[test]
fn an_explicit_digest_hour_before_the_rollover_hour_is_refused_by_name() {
    let refused = KernelSettings::from_env(&env(&[(ROLLOVER_HOUR, "17"), (DIGEST_HOUR, "13")]));
    assert_eq!(
        refused,
        Err(SettingsError::DigestBeforeRollover {
            digest: DIGEST_HOUR,
            rollover: ROLLOVER_HOUR,
        })
    );
    let refusal = refused.expect_err("an explicit digest hour before the rollover hour");
    let (text, debug) = (refusal.to_string(), format!("{refusal:?}"));
    assert!(
        text.contains(DIGEST_HOUR) && text.contains(ROLLOVER_HOUR),
        "the refusal names both settings: {text}"
    );
    for value in ["17", "13"] {
        assert!(
            !text.contains(value) && !debug.contains(value),
            "a value reached the refusal: {text} / {debug}"
        );
    }
    // A digest hour at the rollover hour is not before it.
    let at = KernelSettings::from_env(&env(&[(ROLLOVER_HOUR, "17"), (DIGEST_HOUR, "17")]))
        .expect("a digest at the rollover hour starts");
    assert_eq!(at.digest_hour, Hour::new(17).expect("an hour"));
}

#[test]
fn an_unset_digest_hour_resolves_as_the_predecessors_golden() {
    let (mut resolved, mut refused_earlier, mut refused_malformed) = (0, 0, 0);
    golden::each_case("digest_hour", |case| {
        let rollover = case.input["rollover_hour"]
            .as_i64()
            .expect("a rollover hour");
        let rollover_text = rollover.to_string();
        let mut pairs = vec![(ROLLOVER_HOUR, rollover_text.as_str())];
        if let Some(raw) = case.input["raw"].as_str() {
            pairs.push((DIGEST_HOUR, raw));
        }
        let port = KernelSettings::from_env(&env(&pairs))
            .map(|settings| i64::from(settings.digest_hour.get()));
        let predecessor = case.output.as_i64().expect("the predecessor's hour");
        let malformed = Err(SettingsError::Malformed {
            setting: DIGEST_HOUR,
            expected: Hour::SHAPE,
        });
        match case.class.as_deref() {
            // ADR-020's divergence: the predecessor fell back to its default for an unparsable
            // value and read Python-only spellings; DeckStreak refuses both, as it refuses every
            // malformed setting.
            Some("unparsable" | "python-only") => {
                assert_eq!(port, malformed, "{}", case.input);
                refused_malformed += 1;
            }
            // The predecessor resolved an hour out of 0 to 23, which its validation refused at
            // start (`config.py:Settings.validate`); DeckStreak refuses it as malformed.
            _ if !(0..=23).contains(&predecessor) => {
                assert_eq!(port, malformed, "{}", case.input);
                refused_malformed += 1;
            }
            // The predecessor resolved an explicit hour before the rollover, which its validation
            // refused at start; DeckStreak refuses it by name.
            _ if predecessor < rollover => {
                assert_eq!(
                    port,
                    Err(SettingsError::DigestBeforeRollover {
                        digest: DIGEST_HOUR,
                        rollover: ROLLOVER_HOUR,
                    }),
                    "{}",
                    case.input
                );
                refused_earlier += 1;
            }
            _ => {
                assert_eq!(port, Ok(predecessor), "the digest hour of {}", case.input);
                resolved += 1;
            }
        }
    });
    assert!(resolved > 0 && refused_earlier > 0 && refused_malformed > 0);
}

#[test]
fn a_missing_setting_is_refused_by_name() {
    let refused = CredentialsDirectory::from_env(&Environment::default());
    assert_eq!(
        refused,
        Err(SettingsError::Missing {
            setting: CREDENTIALS_DIRECTORY
        })
    );
    // A blank value is unset, as the predecessor read one.
    assert_eq!(
        CredentialsDirectory::from_env(&env(&[(CREDENTIALS_DIRECTORY, "  ")])),
        refused
    );
    // Any required setting of any context refuses start the same way, by its name.
    let refusal = Environment::default()
        .required::<Hour>(ROLLOVER_HOUR)
        .expect_err("an unset required setting");
    assert_eq!(
        refusal,
        SettingsError::Missing {
            setting: ROLLOVER_HOUR
        }
    );
    assert!(refusal.to_string().contains(ROLLOVER_HOUR), "{refusal}");
    // A set one is read.
    let directory = CredentialsDirectory::from_env(&env(&[(
        CREDENTIALS_DIRECTORY,
        "/run/credentials/example.service",
    )]))
    .expect("an absolute path");
    assert_eq!(
        directory.path(),
        Path::new("/run/credentials/example.service")
    );
}

#[test]
fn a_malformed_setting_is_refused_by_name_without_its_value() {
    // Each shape is the text the operator reads, compared as written and never through the
    // setting's constant.
    let hour = "a whole hour from 0 to 23";
    let malformed = [
        (ROLLOVER_HOUR, "quarter-past-seven", hour),
        (ROLLOVER_HOUR, "31", hour),
        (
            UTC_OFFSET_MINUTES,
            "-99999",
            "whole minutes east of UTC, from -720 to 840",
        ),
        (DIGEST_HOUR, "nineteen-ish", hour),
        (
            OFFLOAD_WORKERS,
            "4097",
            "a whole number of workers from 1 to 512",
        ),
    ];
    for (setting, value, expected) in malformed {
        let refused = KernelSettings::from_env(&env(&[(setting, value)]));
        assert_eq!(refused, Err(SettingsError::Malformed { setting, expected }));
        let refusal = refused.expect_err("a malformed setting refuses start");
        let (text, debug) = (refusal.to_string(), format!("{refusal:?}"));
        assert!(
            text.contains(setting) && text.contains(expected),
            "the refusal names the setting and its shape: {text}"
        );
        assert!(
            !text.contains(value) && !debug.contains(value),
            "{setting}'s value reached the refusal: {text} / {debug}"
        );
    }
    // The credentials directory is refused the same way.
    assert_eq!(
        CredentialsDirectory::from_env(&env(&[(CREDENTIALS_DIRECTORY, "run/credentials")])),
        Err(SettingsError::Malformed {
            setting: CREDENTIALS_DIRECTORY,
            expected: "an absolute directory path",
        })
    );
    // A value that is not UTF-8 is malformed too, and named by its setting alone.
    let bytes = Environment::from_vars([(
        OsString::from(ROLLOVER_HOUR),
        OsString::from_vec(vec![b'4', 0xff]),
    )]);
    assert_eq!(
        KernelSettings::from_env(&bytes),
        Err(SettingsError::Malformed {
            setting: ROLLOVER_HOUR,
            expected: Hour::SHAPE,
        })
    );
    // and well-formed values are read, their surrounding whitespace trimmed.
    let read = KernelSettings::from_env(&env(&[
        (ROLLOVER_HOUR, " 5 "),
        (UTC_OFFSET_MINUTES, "-90"),
        (OFFLOAD_WORKERS, "3"),
    ]))
    .expect("well-formed settings");
    assert_eq!(read.study_day_rule.rollover_hour().get(), 5);
    assert_eq!(read.study_day_rule.utc_offset().minutes(), -90);
    assert_eq!(read.offload_workers.get(), 3);
}

#[test]
fn the_default_rollover_and_digest_hours_equal_the_notifications_policy() {
    assert_eq!(DEFAULT_ROLLOVER_HOUR, policy_hour("rollover"));
    assert_eq!(DEFAULT_DIGEST_HOUR, policy_hour("at"));
    // and they are what an empty environment starts with.
    let defaults = KernelSettings::from_env(&Environment::default()).expect("the defaults start");
    assert_eq!(
        defaults.study_day_rule.rollover_hour().get(),
        DEFAULT_ROLLOVER_HOUR
    );
    assert_eq!(defaults.digest_hour.get(), DEFAULT_DIGEST_HOUR);
}

#[test]
fn the_environment_shows_its_names_in_debug_and_never_a_value() {
    let shown = format!("{:?}", env(&[("ONLY_NAME", "quiet-value")]));
    assert_eq!(shown, "Environment { names: [\"ONLY_NAME\"] }");
}

#[test]
fn an_offset_just_outside_the_bounds_is_refused_and_each_bound_is_admitted() {
    assert_eq!(
        UtcOffset::from_minutes(UtcOffset::MIN_MINUTES).map(UtcOffset::minutes),
        Some(-720)
    );
    assert_eq!(
        UtcOffset::from_minutes(UtcOffset::MAX_MINUTES).map(UtcOffset::minutes),
        Some(840)
    );
    assert_eq!(UtcOffset::from_minutes(UtcOffset::MIN_MINUTES - 1), None);
    assert_eq!(UtcOffset::from_minutes(UtcOffset::MAX_MINUTES + 1), None);
}
