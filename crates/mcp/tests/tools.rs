//! SPEC-119 A33, A41 and A42 (section 14): `get_law_track` answers the roster golden's fields,
//! numbers only, with mastery rounded to 2 places; its name, annotations, parameters and output
//! fields equal its entry of `goldens/mcp_roster.json`; and a pending leech count or mastery answers
//! null, never 0 (R15 to R17; SPEC-077 R12; ADR-329 D7). The output schema admits each pending
//! number as null, as the SPEC's pending-null rule declares them nullable and required.
//!
//! Every call goes through the served stack with the law-track token, built from parts.

// An integration test is test code: its helpers panic on a failed fixture, and the examined count
// is printed on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

mod support;

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use std::collections::BTreeSet;
use std::sync::Arc;

use deck_streak_coordination::law::LawBlock;
use serde_json::{Map, Value, json};
use support::{ScriptedLaw, Served, call_law_track, full_block, law_token, list_tools};

/// The predecessor's roster entry of each tool, from the committed golden.
fn roster() -> Vec<Value> {
    let mut tools = Vec::new();
    let examined = golden::each_case("mcp_roster", |case| {
        tools.extend(
            case.output["tools"]
                .as_array()
                .expect("the roster's tools")
                .iter()
                .cloned(),
        );
    });
    assert_eq!(examined.count, 1, "the roster golden holds one case");
    tools
}

/// The roster golden's ordered output fields of `get_law_track`.
fn golden_fields() -> Vec<String> {
    let entry = roster()
        .into_iter()
        .find(|tool| tool["name"] == "get_law_track")
        .expect("the roster names get_law_track");
    entry["output_fields"]
        .as_array()
        .expect("the entry's output fields")
        .iter()
        .map(|field| field.as_str().expect("a field name").to_owned())
        .collect()
}

/// `get_law_track`'s structured answer over a law track answering `block`, and how many reads it
/// took.
async fn law_track_answer(block: LawBlock) -> (Map<String, Value>, usize) {
    let law = ScriptedLaw::answering(block);
    let served = Served::start(Arc::clone(&law)).await;
    let reply = served
        .post("/mcp", Some(&law_token()), &call_law_track())
        .await;
    assert_eq!(reply.status, 200, "get_law_track: {}", reply.text());
    let result = reply.result();
    assert_ne!(
        result["isError"],
        Value::Bool(true),
        "get_law_track answered a tool error: {result}"
    );
    let answer = result["structuredContent"]
        .as_object()
        .unwrap_or_else(|| panic!("no structured content: {result}"))
        .clone();
    (answer, law.reads())
}

#[tokio::test(flavor = "current_thread")]
async fn the_law_track_answers_numbers_only() {
    let (answer, reads) = law_track_answer(full_block()).await;
    assert_eq!(reads, 1, "one call reads the law track once");

    // Exactly the golden's fields.
    let fields: BTreeSet<String> = answer.keys().cloned().collect();
    let expected: BTreeSet<String> = golden_fields().into_iter().collect();
    assert_eq!(fields, expected, "the answer's fields: {answer:?}");

    // Numbers only, each the law block's, and mastery rounded to 2 places as the predecessor's
    // `server.py:_law_float` rounds it: Python's `round(0.125, 2)` is 0.12, half to even on the
    // exact binary value, where rounding half away from zero would give 0.13.
    for (field, value) in &answer {
        assert!(value.is_number(), "{field} is not a number: {value}");
    }
    assert_eq!(
        Value::Object(answer),
        json!({
            "streak": 12,
            "xp_today": 340,
            "total_xp": 98_765,
            "level": 7,
            "dues": 23,
            "leech_total": 4,
            "mastery": 0.12,
        })
    );
}

#[tokio::test(flavor = "current_thread")]
async fn the_pending_law_numbers_answer_null_never_zero() {
    let pending = LawBlock {
        leech_active: None,
        mastery: None,
        ..full_block()
    };
    let (answer, _) = law_track_answer(pending).await;

    // Both pending numbers are present, and null: never 0 and never 0.0 (SPEC-077 R12).
    assert_eq!(
        answer.get("leech_total"),
        Some(&Value::Null),
        "a pending leech count: {answer:?}"
    );
    assert_eq!(
        answer.get("mastery"),
        Some(&Value::Null),
        "a pending mastery: {answer:?}"
    );

    // The numbers that are known still answer.
    assert_eq!(answer.get("dues"), Some(&json!(23)));
    assert_eq!(answer.get("streak"), Some(&json!(12)));
}

#[tokio::test(flavor = "current_thread")]
async fn each_served_tool_matches_its_roster_golden_entry() {
    let served = Served::start(ScriptedLaw::answering(full_block())).await;
    let reply = served.post("/mcp", Some(&law_token()), &list_tools()).await;
    assert_eq!(reply.status, 200, "tools/list: {}", reply.text());
    let result = reply.result();
    let tools = result["tools"]
        .as_array()
        .unwrap_or_else(|| panic!("no tools: {result}"))
        .clone();
    let roster = roster();

    let names: Vec<&str> = tools
        .iter()
        .map(|tool| tool["name"].as_str().expect("a tool's name"))
        .collect();
    assert_eq!(names, vec!["get_law_track"], "the served tools");
    println!("examined {} served tool(s)", tools.len());

    for tool in &tools {
        let name = tool["name"].as_str().expect("a tool's name");
        let entry = roster
            .iter()
            .find(|entry| entry["name"] == name)
            .unwrap_or_else(|| panic!("{name} is not in the roster golden"));

        // The annotations, exactly.
        assert_eq!(
            tool["annotations"], entry["annotations"],
            "{name}'s annotations"
        );

        // The parameters: the golden's, less the predecessor's `token`, which R15 removes
        // (ADR-329: the one measured difference, never edited into the golden).
        let parameters: BTreeSet<String> = tool["inputSchema"]["properties"]
            .as_object()
            .map(|properties| properties.keys().cloned().collect())
            .unwrap_or_default();
        let mut expected: BTreeSet<String> = entry["input_schema"]["properties"]
            .as_object()
            .expect("the entry's parameters")
            .keys()
            .cloned()
            .collect();
        assert!(expected.remove("token"), "{name}'s golden names `token`");
        assert_eq!(parameters, expected, "{name}'s parameters");

        // The output fields, in the golden's order: the output schema requires each of them, a
        // pending one included, in that order.
        let required: Vec<String> = tool["outputSchema"]["required"]
            .as_array()
            .unwrap_or_else(|| panic!("{name} declares no required output: {tool}"))
            .iter()
            .map(|field| field.as_str().expect("a field name").to_owned())
            .collect();
        let fields: Vec<String> = entry["output_fields"]
            .as_array()
            .expect("the entry's output fields")
            .iter()
            .map(|field| field.as_str().expect("a field name").to_owned())
            .collect();
        assert_eq!(required, fields, "{name}'s output fields");
        let declared: BTreeSet<&String> = tool["outputSchema"]["properties"]
            .as_object()
            .expect("the output schema's properties")
            .keys()
            .collect();
        assert_eq!(
            declared,
            fields.iter().collect::<BTreeSet<_>>(),
            "{name}'s output properties"
        );
    }
}

/// Whether the JSON Schema `schema` admits `value` by its type: its `type`, one name or a list of
/// them, or any branch of its `anyOf`. A schema that names no type admits nothing here, so a field
/// the schema leaves blank cannot pass by saying nothing.
fn admits(schema: &Value, value: &Value) -> bool {
    if let Some(branches) = schema["anyOf"].as_array() {
        return branches.iter().any(|branch| admits(branch, value));
    }
    let names: Vec<&str> = match &schema["type"] {
        Value::String(name) => vec![name.as_str()],
        Value::Array(names) => names.iter().filter_map(Value::as_str).collect(),
        _ => Vec::new(),
    };
    names.iter().any(|name| match *name {
        "null" => value.is_null(),
        "integer" => value.is_i64() || value.is_u64(),
        "number" => value.is_number(),
        "boolean" => value.is_boolean(),
        "string" => value.is_string(),
        "array" => value.is_array(),
        "object" => value.is_object(),
        _ => false,
    })
}

#[tokio::test(flavor = "current_thread")]
async fn the_output_schema_admits_each_pending_number_as_null() {
    let served = Served::start(ScriptedLaw::answering(full_block())).await;
    let reply = served.post("/mcp", Some(&law_token()), &list_tools()).await;
    assert_eq!(reply.status, 200, "tools/list: {}", reply.text());
    let result = reply.result();
    let schema = result["tools"]
        .as_array()
        .and_then(|tools| tools.iter().find(|tool| tool["name"] == "get_law_track"))
        .map(|tool| tool["outputSchema"].clone())
        .unwrap_or_else(|| panic!("get_law_track declares no output schema: {result}"));

    // Every pending number at once: the dues before the first recompute stores them, and the
    // leeches and the mastery while the leech port is not wired. SPEC-119 declares the three
    // nullable and required, so a client that checks an answer against the schema accepts both.
    let pending = LawBlock {
        dues: None,
        leech_active: None,
        mastery: None,
        ..full_block()
    };
    let (pending_answer, _) = law_track_answer(pending).await;
    let (full_answer, _) = law_track_answer(full_block()).await;

    let fields = golden_fields();
    println!("examined {} output field(s)", fields.len());
    assert!(
        !fields.is_empty(),
        "examined 0 output fields: the population is empty, so nothing was judged"
    );
    for field in &fields {
        let declared = &schema["properties"][field];
        for answer in [&pending_answer, &full_answer] {
            let value = answer
                .get(field)
                .unwrap_or_else(|| panic!("the answer holds no {field}: {answer:?}"));
            assert!(
                admits(declared, value),
                "the output schema's {field} ({declared}) refuses the answered {value}"
            );
        }
    }

    // A number the ledger always answers stays a number: null is admitted only where it is pending.
    for field in ["streak", "xp_today", "total_xp", "level"] {
        let declared = &schema["properties"][field];
        assert!(
            !admits(declared, &Value::Null),
            "the output schema's {field} ({declared}) admits null"
        );
    }
}
