//! SPEC-094 R9, ADR-095 amendment: the template token scan ends a pass that does not advance and
//! keeps nothing from it. Every reader here is synthetic.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use deck_streak_insights::dark_fields::{Step, token_step, tokens_with};

/// Reports how many tokens a scan examined, so an empty scan cannot pass unseen.
#[allow(clippy::print_stdout)]
fn examined(what: &str, count: usize) -> usize {
    println!("examined {count} {what}");
    count
}

#[test]
fn a_reader_that_stays_ends_the_scan_with_nothing_kept() {
    let text = "{{a}} {{b}}";
    let mut calls = 0;
    let got = tokens_with(text, |_, at| -> Step {
        calls += 1;
        assert!(calls < 3, "the scan went on past a stalled step");
        (Some((2, 3)), at)
    });
    assert!(got.is_empty(), "{got:?}");
    assert_eq!(calls, 1, "ended on the first step");
}

#[test]
fn a_reader_that_steps_back_ends_the_scan() {
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
    assert_eq!(got, vec!["a"]);
    assert_eq!(calls, 2);
}

#[test]
fn a_reader_that_advances_by_one_keeps_every_token_in_order() {
    let text = "abcd";
    let got = tokens_with(text, |_, at| -> Step { (Some((at, at + 1)), at + 1) });
    assert_eq!(
        got,
        vec!["a", "b", "c"],
        "the last byte has no pair to start"
    );
    assert_eq!(examined("tokens", got.len()), 3);
}

#[test]
fn the_real_scan_finds_two_tokens_and_skips_a_broken_pair() {
    let got = tokens_with("x{{Front}} {{}} {{a{b}} {{Back}}", token_step);
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
