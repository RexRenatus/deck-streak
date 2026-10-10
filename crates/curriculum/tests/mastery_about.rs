//! The English law mastery sentence states the pillar's own figures (SPEC-408 R6, A5): the value
//! at no leeches, the step one leech takes off it, and the floor.

// An integration test is test code: it panics on an unreadable message file.
#![allow(clippy::expect_used, clippy::print_stdout)]

use deck_streak_curriculum::law::mastery_pillar;

#[test]
fn the_law_mastery_description_states_the_pillars_own_figures() {
    let messages: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../web/app/messages/en.json"
        ))
        .expect("the English message file"),
    )
    .expect("the English message file parses");
    let sentence = messages["law_mastery_about"].as_str().unwrap_or("");
    let found: Vec<&str> = sentence
        .split(|c: char| !c.is_ascii_digit())
        .filter(|run| !run.is_empty())
        .collect();
    let expected = [
        mastery_pillar(0),
        mastery_pillar(0) - mastery_pillar(1),
        mastery_pillar(1000),
    ]
    .map(|figure| format!("{figure:.0}"));
    assert_eq!(found, expected);
    println!("examined {} figures of the law sentence", found.len());
}
