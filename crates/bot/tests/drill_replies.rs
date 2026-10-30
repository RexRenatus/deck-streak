//! The law drill replies' limits, by whole value (SPEC-110 R13, R14): a button's label, the
//! twelve-button keyboard and its "more" line, a view's prompt and its deferral line.

// An integration test is test code: its helpers panic on a failed check.
#![allow(clippy::expect_used)]

use deck_streak_bot::commands::Reply;
use deck_streak_bot::drill_commands::{list_reply, view_reply};
use deck_streak_coordination::drills::{DrillMeta, DrillView};
use frankenstein::types::{InlineKeyboardButton, InlineKeyboardMarkup};

fn meta(id: &str, title: &str, answered: bool) -> DrillMeta {
    DrillMeta {
        drill_id: id.to_owned(),
        kind: "irac".to_owned(),
        subject: "Torts".to_owned(),
        title: title.to_owned(),
        created: None,
        age_days: None,
        answered,
        deferred: false,
    }
}

fn keyboard(rows: &[(&str, &str)]) -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::builder()
        .inline_keyboard(
            rows.iter()
                .map(|(text, data)| {
                    vec![
                        InlineKeyboardButton::builder()
                            .text(*text)
                            .callback_data(*data)
                            .build(),
                    ]
                })
                .collect(),
        )
        .build()
}

#[test]
fn a_button_shows_forty_characters_of_the_title() {
    let title = "0123456789".repeat(4) + "X";
    let reply = list_reply("Drills", &[meta("d1", &title, false)]);
    assert_eq!(
        reply,
        Reply {
            text: "<b>Drills</b>".to_owned(),
            keyboard: Some(keyboard(&[(&"0123456789".repeat(4), "dv:d1")])),
        }
    );
}

#[test]
fn twelve_drills_say_nothing_more_and_thirteen_say_one() {
    let ids: Vec<String> = (0..13).map(|n| format!("d{n:02}")).collect();
    let all: Vec<DrillMeta> = ids.iter().map(|id| meta(id, id, false)).collect();
    let rows_of = |count: usize| -> InlineKeyboardMarkup {
        let pairs: Vec<(String, String)> = ids[..count]
            .iter()
            .map(|id| (id.clone(), format!("dv:{id}")))
            .collect();
        let borrowed: Vec<(&str, &str)> = pairs
            .iter()
            .map(|(a, b)| (a.as_str(), b.as_str()))
            .collect();
        keyboard(&borrowed)
    };
    assert_eq!(
        list_reply("Drills", &all[..12]),
        Reply {
            text: "<b>Drills</b>".to_owned(),
            keyboard: Some(rows_of(12)),
        }
    );
    assert_eq!(
        list_reply("Drills", &all),
        Reply {
            text: "<b>Drills</b>\n\u{2026}and 1 more".to_owned(),
            keyboard: Some(rows_of(12)),
        }
    );
}

fn view(prompt: &str, defer_reason: &str, answered: bool) -> DrillView {
    DrillView {
        meta: meta("d1", "Duty", answered),
        prompt: prompt.to_owned(),
        defer_reason: defer_reason.to_owned(),
        sections: Vec::new(),
        self_check: Vec::new(),
    }
}

#[test]
fn a_view_cuts_the_prompt_at_three_thousand_characters() {
    let long = "x".repeat(3_001);
    assert_eq!(
        view_reply(&view(&long, "", true)),
        Reply {
            text: format!("<b>Duty</b>\n{}", "x".repeat(3_000)),
            keyboard: None,
        }
    );
    let exact = "y".repeat(3_000);
    assert_eq!(
        view_reply(&view(&exact, "", true)).text,
        format!("<b>Duty</b>\n{exact}")
    );
}

#[test]
fn a_view_names_a_deferral_and_offers_an_answer_only_when_unanswered() {
    assert_eq!(
        view_reply(&view("P.", "later", false)),
        Reply {
            text: "<b>Duty</b>\nP.\nDeferred: later".to_owned(),
            keyboard: Some(keyboard(&[("Answer", "da:d1")])),
        }
    );
    assert_eq!(
        view_reply(&view("P.", "", true)),
        Reply {
            text: "<b>Duty</b>\nP.".to_owned(),
            keyboard: None,
        }
    );
}
