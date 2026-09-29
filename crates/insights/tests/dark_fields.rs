//! SPEC-094 A9 to A11: Dark Fields equals the predecessor's goldens.
#![allow(clippy::expect_used, clippy::unwrap_used)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use deck_streak_ingest::structure::{DeclaredField, StructureReads, Template};
use deck_streak_insights::dark_fields::{
    self, DarkFields, DarkFieldsInput, build_report, config_tokens,
};
use deck_streak_insights::instrument::Instrument;
use serde_json::Value;

/// The bytes a golden's lowercase hex names.
fn unhex(text: &str) -> Vec<u8> {
    (0..text.len() / 2)
        .map(|at| {
            u8::from_str_radix(&text[at * 2..at * 2 + 2], 16)
                .unwrap_or_else(|_| panic!("not hex: {text}"))
        })
        .collect()
}

fn text(value: &Value) -> String {
    value.as_str().expect("a string").to_owned()
}

fn int(value: &Value) -> i64 {
    value.as_i64().expect("an integer")
}

fn input_of(case: &Value) -> DarkFieldsInput {
    let names = case["template_names"].as_array().expect("names");
    let configs = case["template_configs"].as_array().expect("configs");
    let fields = case["declared_fields"].as_array().expect("fields");
    let presence = case["presence"].as_array().expect("presence");
    DarkFieldsInput {
        template_names: names
            .iter()
            .map(|row| (int(&row[0]), int(&row[1]), text(&row[2]), text(&row[3])))
            .collect(),
        template_configs: configs
            .iter()
            .map(|row| {
                (
                    int(&row["ntid"]),
                    int(&row["ord"]),
                    row["hex"].as_str().map(unhex),
                )
            })
            .collect(),
        declared_fields: fields
            .iter()
            .map(|row| (int(&row["ntid"]), int(&row["ord"]), text(&row["name"])))
            .collect(),
        presence: presence
            .iter()
            .map(|row| ((int(&row[0]), int(&row[1])), int(&row[2])))
            .collect(),
        reviewed_note_count: int(&case["reviewed_note_count"]),
        failed_reads: case["failed_reads"]
            .as_array()
            .expect("failed reads")
            .iter()
            .map(text)
            .collect(),
        names_are_safe: false,
    }
}

#[test]
fn dark_fields_match_the_predecessors_golden() {
    let mut examined = 0;
    let mut dark = 0;
    let mut cold = 0;
    let mut failed = 0;
    golden::each_case("dark_fields", |case| {
        let report = build_report(&input_of(&case.input));
        examined += 1;
        let out = &case.output;
        let seen: Vec<(String, String, i64)> = report
            .dark_fields
            .iter()
            .map(|row| (row.note_type.clone(), row.field.clone(), row.reviewed_notes))
            .collect();
        let wanted: Vec<(String, String, i64)> = out["dark_fields"]
            .as_array()
            .expect("dark fields")
            .iter()
            .map(|row| (text(&row[0]), text(&row[1]), int(&row[2])))
            .collect();
        dark += usize::from(!wanted.is_empty());
        assert_eq!(seen, wanted, "{}", case.input);
        let unparseable: Vec<(String, i64)> = report
            .unparseable
            .iter()
            .map(|row| (row.note_type.clone(), row.note_type_id))
            .collect();
        let wanted: Vec<(String, i64)> = out["unparseable"]
            .as_array()
            .expect("unparseable")
            .iter()
            .map(|row| (text(&row[0]), int(&row[1])))
            .collect();
        assert_eq!(unparseable, wanted, "{}", case.input);
        assert_eq!(report.notetypes_checked, int(&out["notetypes_checked"]));
        assert_eq!(report.reviewed_note_count, int(&out["reviewed_note_count"]));
        cold += usize::from(report.is_cold);
        assert_eq!(report.is_cold, out["is_cold"].as_bool().unwrap());
        let reads: Vec<String> = out["failed_reads"]
            .as_array()
            .expect("failed reads")
            .iter()
            .map(text)
            .collect();
        failed += usize::from(!reads.is_empty());
        assert_eq!(report.failed_reads, reads, "{}", case.input);
    });
    assert!(examined >= 25, "examined only {examined} golden cases");
    assert!(dark >= 3, "only {dark} cases held a dark field");
    assert!(cold >= 1, "only {cold} cold cases");
    assert!(failed >= 1, "only {failed} failed-read cases");
}

#[test]
fn template_tokens_match_the_predecessors_golden() {
    let mut examined = 0;
    let mut failures = 0;
    let mut named = 0;
    golden::each_case("dark_fields_tokens", |case| {
        let config = case.input["hex"].as_str().map(unhex);
        let (tokens, failed) = config_tokens(config.as_deref());
        examined += 1;
        failures += usize::from(failed);
        let seen: Vec<String> = tokens.into_iter().collect();
        let wanted: Vec<String> = case.output["tokens"]
            .as_array()
            .expect("tokens")
            .iter()
            .map(text)
            .collect();
        named += usize::from(!wanted.is_empty());
        assert_eq!(seen, wanted, "{}", case.input);
        assert_eq!(failed, case.output["failed"].as_bool().unwrap());
    });
    assert!(examined >= 30, "examined only {examined} golden cases");
    assert!(failures >= 3, "only {failures} failed decodes");
    assert!(named >= 10, "only {named} cases named a token");
}

#[test]
fn the_dark_fields_constants_equal_the_predecessors() {
    let mut examined = 0;
    golden::each_case("dark_fields.constants", |case| {
        let name = case.input["name"].as_str().expect("a name");
        examined += 1;
        match name {
            "darkfields.MIN_DARK_NOTES" => {
                assert_eq!(dark_fields::MIN_DARK_NOTES, int(&case.output));
            }
            "darkfields.MAX_DARK_FIELDS_SHOWN" => {
                assert_eq!(
                    i64::try_from(dark_fields::MAX_DARK_FIELDS_SHOWN).unwrap(),
                    int(&case.output)
                );
            }
            "darkfields.MAX_UNPARSEABLE_SHOWN" => {
                assert_eq!(
                    i64::try_from(dark_fields::MAX_UNPARSEABLE_SHOWN).unwrap(),
                    int(&case.output)
                );
            }
            "darkfields._SECTION_PREFIXES" => {
                assert_eq!(dark_fields::SECTION_PREFIXES, case.output.as_str().unwrap());
            }
            "darkfields._Q_FORMAT_FIELD" => {
                assert_eq!(
                    i64::try_from(dark_fields::Q_FORMAT_FIELD).unwrap(),
                    int(&case.output)
                );
            }
            "darkfields._A_FORMAT_FIELD" => {
                assert_eq!(
                    i64::try_from(dark_fields::A_FORMAT_FIELD).unwrap(),
                    int(&case.output)
                );
            }
            "transfer.NOTES_BATCH_SIZE" => {
                assert_eq!(
                    i64::try_from(deck_streak_ingest::structure::NOTES_BATCH_SIZE).unwrap(),
                    int(&case.output)
                );
            }
            other => panic!("a constant this test does not know: {other}"),
        }
    });
    assert_eq!(examined, 7, "the golden holds seven constants");
    let mut names = 0;
    golden::each_case("dark_fields.special_names", |case| {
        let wanted: Vec<String> = case.output["names"]
            .as_array()
            .expect("names")
            .iter()
            .map(text)
            .collect();
        let mut seen: Vec<String> = dark_fields::SPECIAL_FIELD_NAMES
            .iter()
            .map(|name| (*name).to_owned())
            .collect();
        seen.sort();
        assert_eq!(seen, wanted);
        names += wanted.len();
    });
    assert_eq!(names, 6);
}

fn reads_with(dark: usize) -> StructureReads {
    let mut reads = StructureReads::default();
    reads.templates.push(Template {
        note_type_id: 1,
        ordinal: 0,
        note_type_name: "Basic".to_owned(),
        name: "Card 1".to_owned(),
        config: Some(unhex("0a0a7b7b46726f6e747d7d12")),
    });
    reads.reviewed_notes.insert(1);
    for ordinal in 0..=i64::try_from(dark).unwrap() {
        reads.fields.push(DeclaredField {
            note_type_id: 1,
            ordinal,
            name: if ordinal == 0 {
                "Front".to_owned()
            } else {
                format!("Extra {ordinal:03}")
            },
        });
        reads.presence.insert((1, ordinal), 5);
    }
    reads
}

#[test]
fn a_report_shows_at_most_the_cap_and_counts_the_rest() {
    let dark = 45;
    let view = DarkFields.build(&reads_with(dark));
    assert_eq!(view.dark_fields_total, dark);
    assert_eq!(view.dark_fields.len(), 40);
    assert_eq!(view.dark_fields[0].field, "Extra 001");
    assert_eq!(view.notetypes_checked, 1);
    assert!(DarkFields.failed_reads(&view).is_empty());
}

#[test]
fn names_made_safe_at_the_read_are_compared_made_safe() {
    let mut reads = reads_with(0);
    reads.fields[0].name = deck_streak_ingest::structure::safe_name("Q&A");
    reads.templates[0].config = {
        let text = b"{{Q&A}}";
        let mut config = vec![0x0a, u8::try_from(text.len()).unwrap()];
        config.extend_from_slice(text);
        Some(config)
    };
    let view = DarkFields.build(&reads);
    assert_eq!(view.dark_fields_total, 0);
    assert_eq!(view.notetypes_checked, 1);
}
