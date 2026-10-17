//! SPEC-094 R9, ADR-095 amendment: the template token scan refuses a pass that does not advance
//! by name, keeps nothing from it, and its template is named unparseable rather than judged on a
//! partial token set. Every reader here is synthetic.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::collections::BTreeSet;

use deck_streak_insights::dark_fields::{
    Step, TOKEN_NO_PROGRESS, config_tokens, config_tokens_with, token_step, tokens_with,
};

/// Reports how many tokens a scan examined, so an empty scan cannot pass unseen.
#[allow(clippy::print_stdout)]
fn examined(what: &str, count: usize) -> usize {
    println!("examined {count} {what}");
    count
}

#[test]
fn a_reader_that_stays_is_refused_with_nothing_kept() {
    let text = "{{a}} {{b}}";
    let mut calls = 0;
    let got = tokens_with(text, |_, at| -> Step {
        calls += 1;
        assert!(calls < 3, "the scan went on past a stalled step");
        (Some((2, 3)), at)
    });
    assert_eq!(got, Err(TOKEN_NO_PROGRESS));
    assert_eq!(calls, 1, "refused on the first step");
}

#[test]
fn a_reader_that_steps_back_is_refused() {
    let text = "{{a}} {{b}}";
    let mut calls = 0;
    let got = tokens_with(text, |_, at| -> Step {
        calls += 1;
        assert!(calls < 4, "the scan went on past a step back");
        if calls == 1 {
            (Some((2, 3)), at + 2)
        } else {
            (Some((2, 3)), at - 1)
        }
    });
    assert_eq!(
        got,
        Err(TOKEN_NO_PROGRESS),
        "the token kept before the step back is dropped"
    );
    assert_eq!(calls, 2);
}

#[test]
fn a_reader_that_advances_by_one_keeps_every_token_in_order() {
    let text = "abcd";
    let mut calls = 0;
    let got = tokens_with(text, |_, at| -> Step {
        calls += 1;
        assert!(calls < 5, "the scan read past the text's end");
        (Some((at, at + 1)), at + 1)
    })
    .unwrap();
    assert_eq!(
        got,
        vec!["a", "b", "c"],
        "the last byte has no pair to start"
    );
    assert_eq!(examined("tokens", got.len()), 3);
}

#[test]
fn the_real_scan_finds_two_tokens_and_skips_a_broken_pair() {
    let got = tokens_with("x{{Front}} {{}} {{a{b}} {{Back}}", token_step).unwrap();
    assert_eq!(got, vec!["Front", "Back"]);
    assert_eq!(examined("tokens", got.len()), 2);
}

#[test]
fn one_step_answers_the_token_range_and_the_position_after_it() {
    let bytes = b"{{ab}}c";
    assert_eq!(token_step(bytes, 0), (Some((2, 4)), 6));
    assert_eq!(token_step(bytes, 1), (None, 2));
    assert_eq!(token_step(b"{{}}", 0), (None, 1));
    assert_eq!(token_step(b"{{ab", 0), (None, 1));
}

/// A template config whose front is `{{Front}}` and whose back is `{{Back}}`.
const CONFIG: &[u8] = b"\x0a\x09{{Front}}\x12\x08{{Back}}";

#[test]
fn a_scan_that_refuses_marks_its_template_failed_with_no_tokens() {
    let mut calls = 0;
    let got = config_tokens_with(Some(CONFIG), |_, at| -> Step {
        calls += 1;
        assert!(calls < 3, "the scan went on past a stalled step");
        (None, at)
    });
    assert_eq!(got, (BTreeSet::new(), true));
    assert_eq!(calls, 2, "each side is refused on its first step");
}

#[test]
fn a_back_that_refuses_fails_the_template_its_front_read() {
    let mut calls = 0;
    let got = config_tokens_with(Some(CONFIG), |bytes, at| -> Step {
        calls += 1;
        assert!(calls < 4, "the scan went on past a stalled step");
        if bytes.starts_with(b"{{Back") {
            (None, at)
        } else {
            token_step(bytes, at)
        }
    });
    assert_eq!(
        got,
        (BTreeSet::new(), true),
        "the front's Front is not kept"
    );
}

#[test]
fn a_scan_that_advances_reads_both_sides_of_the_template() {
    let (tokens, failed) = config_tokens(Some(CONFIG));
    assert!(!failed);
    let names: Vec<&str> = tokens.iter().map(String::as_str).collect();
    assert_eq!(names, vec!["Back", "Front"]);
    assert_eq!(
        config_tokens_with(Some(CONFIG), token_step),
        (tokens, false)
    );
}
