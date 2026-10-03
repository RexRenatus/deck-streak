//! The persona engine's templates, roster and outputs: the public templates load with their four
//! slots unfilled, a filled slot is refused, a roster outside the repository fills the slots, an
//! incomplete roster is refused, the golden readings open with the frontmatter the engine writes,
//! and a language output's band is the live band before the roster's; every template and roster
//! rule refuses by name (SPEC-044 A1 to A3, A6 to A8).

// An integration test is test code: its helpers panic on an unreadable fixture, and it prints the
// examined count on purpose. clippy.toml's in-test allowances cover only `#[test]` bodies.
#![allow(clippy::expect_used, clippy::print_stdout, clippy::panic)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use deck_streak_agent::{
    CefrBand, Duty, Frontmatter, LiveBand, MemoryPort, MemoryPorts, MemoryReader, MemorySource,
    PersonaError, ROSTER, Roster, RosterPath, Slot, Slots, Subject, SubjectKind, Template,
    TemplateId, TemplateSet, TopicKey, resolve_band,
};
use deck_streak_kernel::{Environment, PortFuture, SettingsError};
use serde_json::{Value, json};

/// A synthetic professor, obviously invented; the bio carries `&` and `<` to show no escaping.
const LAW_NAME: &str = "Professor Quill Testwell";
const LAW_BIO: &str = "A synthetic bio of an invented professor, evidence & procedure <plain>.";
const LAW_VOICE: &str = "A synthetic voice, measured and exact.";
const LAW_PERSONALITY: &str = "A synthetic personality that asks before it tells.";
/// A synthetic mentor, obviously invented.
const MENTOR_NAME: &str = "Mentor Vela Sample";

/// A synthetic roster: two invented personas, and two synthetic topics.
fn synthetic_roster() -> Value {
    json!({
        "schema": "deckstreak.agent.roster.v1",
        "personas": {
            "law-evidence": {
                "name": LAW_NAME,
                "bio": LAW_BIO,
                "voice": LAW_VOICE,
                "personality": LAW_PERSONALITY
            },
            "language-mentor-es": {
                "name": MENTOR_NAME,
                "bio": "A synthetic bio of an invented mentor.",
                "voice": "A synthetic voice, warm and clear.",
                "personality": "A synthetic personality that praises what went right."
            }
        },
        "topics": {
            "synthetic/law-topic": {"template": "law-evidence"},
            "synthetic/language-topic": {"template": "language-mentor-es", "cefr": "A2"}
        }
    })
}

/// An edit that breaks one rule of the synthetic roster.
type Edit = dyn Fn(&mut Value);

/// The public templates, compiled into the engine.
fn public() -> TemplateSet {
    TemplateSet::public().expect("the public templates load")
}

/// The topic `key`.
fn topic(key: &str) -> TopicKey {
    TopicKey::parse(key).expect("a synthetic topic key")
}

/// The refusal of the roster `roster`, or `None` when it reads.
fn refusal(roster: &Value) -> Option<PersonaError> {
    Roster::parse(&roster.to_string(), public()).err()
}

/// The repository's root, two levels above this crate.
fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

/// The public template `name`, read from `agent/personas/`.
fn template_text(name: &str) -> String {
    let path = repo().join("agent").join("personas").join(name);
    fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("the template {} cannot be read: {error}", path.display()))
}

/// Prints how many items a check examined and refuses zero (the tdd pack's examined contract).
fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}

/// The template's reason for refusing `text`, when it is a schema rule.
fn schema_refusal(text: &str) -> Option<&'static str> {
    match Template::parse(text) {
        Err(PersonaError::Template(reason)) => Some(reason),
        _ => None,
    }
}

#[test]
fn every_public_template_loads_with_its_roster_slots_unfilled() {
    let directory = repo().join("agent").join("personas");
    let mut files: Vec<PathBuf> = fs::read_dir(&directory)
        .expect("agent/personas is readable")
        .map(|entry| entry.expect("a directory entry").path())
        .filter(|path| path.to_string_lossy().ends_with(".persona.md"))
        .collect();
    files.sort();
    let files = examined("public templates in agent/personas", files);
    let public = TemplateSet::public().expect("the engine loads the templates it compiled in");
    assert!(!public.is_empty() && TemplateSet::default().is_empty());
    assert_eq!(
        public.len(),
        files.len(),
        "the engine carries every template of agent/personas, and no other"
    );
    let (mut law, mut language) = (0, 0);
    for file in &files {
        let text = fs::read_to_string(file).expect("a template is readable");
        let template = Template::parse(&text)
            .unwrap_or_else(|error| panic!("{} does not load: {error}", file.display()));
        let compiled = public
            .get(template.id())
            .unwrap_or_else(|| panic!("{} is not compiled into the engine", file.display()));
        assert_eq!(
            compiled.text(),
            text,
            "{} is compiled in unchanged",
            file.display()
        );
        for slot in Slot::ALL {
            assert!(
                template.body().contains(slot.token()),
                "{} carries {} unfilled",
                file.display(),
                slot.token()
            );
        }
        assert!(template.duties().contains(&Duty::DailyReading));
        assert_eq!(template.memory(), MemorySource::ALL);
        match template.subject().kind() {
            SubjectKind::Law => law += 1,
            SubjectKind::Language => {
                language += 1;
                assert!(template.lang().is_some_and(|tag| !tag.is_empty()));
            }
            other => panic!("{} is a {other:?} template", file.display()),
        }
    }
    assert_eq!(
        (law, language),
        (13, 5),
        "every law-professors and language-mentors template"
    );
}

#[test]
fn a_template_with_a_filled_slot_is_refused() {
    let pristine = template_text("law-evidence.persona.md");
    // A bio written into the template: the identity section no longer carries the slot.
    let bio = pristine.replace("{{bio}}", "A synthetic bio written into the template.");
    assert!(matches!(
        Template::parse(&bio),
        Err(PersonaError::SlotFilled { slot: "bio" })
    ));
    // A name written into the disclosure alone: the identity still carries it, the disclosure not.
    let at = pristine
        .rfind("{{name}}")
        .expect("the disclosure carries the name");
    let mut named = pristine.clone();
    named.replace_range(at..at + "{{name}}".len(), "Professor Invented");
    assert!(matches!(
        Template::parse(&named),
        Err(PersonaError::SlotFilled { slot: "name" })
    ));
    for slot in [Slot::Voice, Slot::Personality] {
        let filled = pristine.replace(slot.token(), "A synthetic line written into the template.");
        let refused = Template::parse(&filled);
        assert!(
            matches!(&refused, Err(PersonaError::SlotFilled { slot: name }) if *name == slot.name()),
            "{} filled: {refused:?}",
            slot.token()
        );
    }
    // A slot moved out of its own section is not in place, even though the text still holds it.
    let moved = pristine
        .replacen("{{bio}}", "", 1)
        .replacen("{{voice}}", "{{voice}} {{bio}}", 1);
    assert!(matches!(
        Template::parse(&moved),
        Err(PersonaError::SlotFilled { slot: "bio" })
    ));
    // A token that is not one of the four, beside a slot or unclosed.
    for unknown in ["{{voice}}{{age}}", "{{voice}} {{Name}}", "{{voice}} {{name"] {
        let planted = pristine.replacen("{{voice}}", unknown, 1);
        assert!(
            matches!(Template::parse(&planted), Err(PersonaError::UnknownSlot)),
            "{unknown} is refused"
        );
    }
    // The copy as the pack ships it loads.
    let loaded = Template::parse(&pristine).expect("the pristine template loads");
    assert_eq!(loaded.id().as_str(), "law-evidence");
}

#[test]
fn a_template_that_breaks_the_schema_is_refused() {
    let law = template_text("law-evidence.persona.md");
    let es = template_text("es.persona.md");
    let loaded = Template::parse(&law).expect("the law template loads");
    assert_eq!(loaded.subject().as_str(), "law/evidence");
    assert!(loaded.lang().is_none());
    assert!(loaded.body().starts_with("# {{name}}, Evidence\n"));
    assert!(!loaded.body().contains("schema:"));
    let mentor = Template::parse(&es).expect("the Spanish template loads");
    assert_eq!(mentor.lang(), Some("es"));
    assert_eq!(mentor.subject().kind(), SubjectKind::Language);
    let schema = "schema: \"phx.persona.template.v1\"";
    let duties = law
        .lines()
        .find(|line| line.starts_with("duties: "))
        .expect("the law template lists its duties");
    let cases = [
        (law.replacen("---\n", "", 1), "line 1 is not ---"),
        (
            law.replacen("\n---\n", "\n", 1),
            "the frontmatter is never closed",
        ),
        (
            law.replacen(schema, &format!("{schema}\nnot a key line"), 1),
            "a frontmatter line is not key: JSON",
        ),
        (
            law.replacen(schema, &format!("{schema}\n"), 1),
            "a frontmatter line is not key: JSON",
        ),
        (
            law.replacen("\"law/evidence\"", "law/evidence", 1),
            "a frontmatter value is not one JSON value",
        ),
        (
            law.replacen(schema, &format!("{schema}\n{schema}"), 1),
            "a frontmatter key is repeated",
        ),
        (
            law.replacen("template.v1", "template.v2", 1),
            "schema is not phx.persona.template.v1",
        ),
        (
            law.replacen("\"law-evidence\"", "\"Law Evidence\"", 1),
            "template is not a slug",
        ),
        (
            law.replacen("\"law/evidence\"", "\"lore/evidence\"", 1),
            "subject is not a known kind and an area",
        ),
        (
            es.replacen("lang: \"es\"\n", "", 1),
            "a language template names no lang",
        ),
        (
            es.replacen("lang: \"es\"", "lang: 7", 1),
            "a language template names no lang",
        ),
        (
            law.replacen(schema, &format!("{schema}\nlang: \"en\""), 1),
            "only a language template names a lang",
        ),
        (
            law.replacen("duties: [", "duties: [\"gossip\", ", 1),
            "a duty is off the registry",
        ),
        (
            law.replacen("duties: [", "duties: [7, ", 1),
            "duties is not a list of names",
        ),
        (
            law.replacen(duties, "duties: \"daily-reading\"", 1),
            "duties is not a list of names",
        ),
        (
            law.replacen(duties, "duties: []", 1),
            "the template offers no duty",
        ),
        (
            law.replacen("memory: [", "memory: [\"horoscope\", ", 1),
            "a memory source is off the list",
        ),
        (
            law.replacen("memory: [", "memory: [7, ", 1),
            "memory is not a list of names",
        ),
    ];
    for (text, reason) in &cases {
        assert_eq!(schema_refusal(text), Some(*reason), "refused as: {reason}");
    }
    assert!(matches!(
        TemplateSet::new(vec![loaded.clone(), loaded]),
        Err(PersonaError::Template("two templates share an id"))
    ));
}

#[test]
fn the_vocabulary_reads_back_by_its_names() {
    let duties: Vec<&str> = Duty::ALL.into_iter().map(Duty::name).collect();
    assert_eq!(
        duties,
        [
            "daily-reading",
            "drill-coach",
            "leech-doctor",
            "writing-tutor",
            "conversation-partner",
            "practice-questions"
        ]
    );
    for duty in Duty::ALL {
        assert_eq!(Duty::parse(duty.name()), Some(duty));
    }
    let bands: Vec<&str> = CefrBand::ALL.into_iter().map(CefrBand::name).collect();
    assert_eq!(bands, ["A1", "A2", "B1", "B2", "C1", "C2"]);
    for band in CefrBand::ALL {
        assert_eq!(CefrBand::parse(band.name()), Some(band));
    }
    let slots: Vec<(&str, &str)> = Slot::ALL
        .into_iter()
        .map(|slot| (slot.name(), slot.token()))
        .collect();
    assert_eq!(
        slots,
        [
            ("name", "{{name}}"),
            ("bio", "{{bio}}"),
            ("voice", "{{voice}}"),
            ("personality", "{{personality}}")
        ]
    );
    for slot in Slot::ALL {
        assert_eq!(Slot::parse(slot.name()), Some(slot));
    }
    let sources: Vec<&str> = MemorySource::ALL
        .into_iter()
        .map(MemorySource::name)
        .collect();
    assert_eq!(sources, ["leeches", "drill-grades", "lapses"]);
    for source in MemorySource::ALL {
        assert_eq!(
            MemorySource::parse(source.name()).expect("a listed source"),
            source
        );
    }
    let kinds: Vec<&str> = SubjectKind::ALL
        .into_iter()
        .map(SubjectKind::name)
        .collect();
    assert_eq!(kinds, ["language", "law", "test-prep", "general"]);
    for kind in SubjectKind::ALL {
        let subject = Subject::parse(&format!("{}/an-area-2", kind.name())).expect("a subject");
        assert_eq!(subject.kind(), kind);
        assert_eq!(subject.to_string(), format!("{}/an-area-2", kind.name()));
    }
    for refused in [
        "law",
        "law/",
        "/evidence",
        "law/Evidence",
        "law/evi--dence",
        "lore/evidence",
    ] {
        assert_eq!(Subject::parse(refused), None, "{refused} is not a subject");
    }
    assert_eq!(
        TemplateId::parse("law-evidence").map(|id| id.as_str().to_owned()),
        Some("law-evidence".to_owned())
    );
    for refused in ["", "-law", "law-", "law_evidence", "Law"] {
        assert_eq!(
            TemplateId::parse(refused),
            None,
            "{refused:?} is not a template id"
        );
    }
    assert!(Duty::parse("chores").is_none() && CefrBand::parse("D1").is_none());
    assert!(Slot::parse("age").is_none() && SubjectKind::parse("lore").is_none());
}

#[test]
fn a_roster_outside_the_repository_fills_the_four_slots() {
    let outside = tempfile::tempdir().expect("a temporary directory outside the repository");
    let file = outside.path().join("roster.json");
    fs::write(&file, synthetic_roster().to_string()).expect("the synthetic roster is written");
    let env = Environment::from_vars([("DECKSTREAK_AGENT_ROSTER", file.as_os_str())]);
    let path: RosterPath = env
        .required(ROSTER)
        .expect("the setting names the roster by an absolute path");
    let persona = Roster::load(&path, public())
        .and_then(|roster| roster.persona(&topic("synthetic/law-topic"), Duty::DailyReading));
    assert!(
        persona
            .as_ref()
            .is_ok_and(|persona| persona.text().contains(LAW_NAME)),
        "the roster fills the name: {persona:?}"
    );
    let persona = persona.expect("the synthetic law topic has a persona");
    let text = persona.text();
    for value in [LAW_BIO, LAW_VOICE, LAW_PERSONALITY] {
        assert!(text.contains(value), "the roster fills {value}, unescaped");
    }
    assert!(
        text.contains(&format!("{LAW_NAME} is an AI teaching persona")),
        "the disclosure names the persona"
    );
    assert!(!text.contains("{{"), "no slot token remains");
    assert!(text.starts_with(&format!("# {LAW_NAME}, Evidence\n")));
    assert_eq!(persona.template().as_str(), "law-evidence");
    assert_eq!(persona.subject().as_str(), "law/evidence");
    assert_eq!(persona.duty(), Duty::DailyReading);
    assert_eq!(persona.memory(), MemorySource::ALL);
    // Nothing of the roster reaches a line a log could print.
    let roster = Roster::load(&path, public()).expect("the roster reads");
    let shown = format!("{path:?} {persona:?} {roster:?}");
    assert_eq!(
        shown,
        "RosterPath(..) Persona(..) Roster { templates: 18, personas: 2, topics: 2 }"
    );
    let relative = Environment::from_vars([(ROSTER, "agent/roster.json")]);
    assert!(matches!(
        relative.required::<RosterPath>(ROSTER),
        Err(SettingsError::Malformed {
            expected: "an absolute file path",
            ..
        })
    ));
}

#[test]
fn instantiation_refuses_an_incomplete_roster() {
    let mut fifth = synthetic_roster();
    fifth["personas"]["law-evidence"]["age"] = json!("an invented age");
    assert!(
        matches!(refusal(&fifth), Some(PersonaError::UnknownSlot)),
        "a slot that is not one of the four is refused"
    );
    for slot in Slot::ALL {
        let mut missing = synthetic_roster();
        missing["personas"]["law-evidence"]
            .as_object_mut()
            .expect("an entry")
            .remove(slot.name());
        let mut blank = synthetic_roster();
        blank["personas"]["law-evidence"][slot.name()] = json!("   ");
        for unfilled in [missing, blank] {
            let refused = refusal(&unfilled);
            assert!(
                matches!(&refused, Some(PersonaError::SlotUnfilled { slot: name }) if *name == slot.name()),
                "{} unfilled: {refused:?}",
                slot.name()
            );
        }
    }
    let mut unfilled = synthetic_roster();
    unfilled["personas"]
        .as_object_mut()
        .expect("the personas")
        .remove("language-mentor-es");
    assert!(
        matches!(
            refusal(&unfilled),
            Some(PersonaError::SlotUnfilled { slot: "name" })
        ),
        "a topic whose template the roster fills no slot of is refused"
    );
    let mut stray = synthetic_roster();
    stray["personas"]["law-invented"] = stray["personas"]["law-evidence"].clone();
    assert!(matches!(
        refusal(&stray),
        Some(PersonaError::UnknownTemplate)
    ));
    let mut unbound = synthetic_roster();
    unbound["topics"]["synthetic/law-topic"]["template"] = json!("law-invented");
    assert!(matches!(
        refusal(&unbound),
        Some(PersonaError::UnknownTemplate)
    ));
    let message = refusal(&unbound)
        .expect("the roster is refused")
        .to_string();
    assert!(message.contains("template") && !message.contains("law-invented"));
    // A duty the topic's template does not offer: the language mentor sets no practice set.
    let roster =
        Roster::parse(&synthetic_roster().to_string(), public()).expect("the roster reads");
    let language = topic("synthetic/language-topic");
    assert!(matches!(
        roster.persona(&language, Duty::PracticeQuestions),
        Err(PersonaError::DutyNotOffered)
    ));
    assert!(matches!(
        roster.persona(&topic("synthetic/elsewhere"), Duty::DailyReading),
        Err(PersonaError::TopicUnbound)
    ));
    let mentor = roster
        .persona(&language, Duty::DailyReading)
        .expect("the mentor offers the daily reading");
    assert_eq!(mentor.band(), Some(CefrBand::A2));
    assert_eq!(mentor.lang(), Some("es"));
    assert!(mentor.text().contains(MENTOR_NAME));
}

#[test]
fn a_roster_that_breaks_its_schema_is_refused() {
    let roster = |edit: &Edit| {
        let mut roster = synthetic_roster();
        edit(&mut roster);
        match refusal(&roster) {
            Some(PersonaError::Roster(reason)) => reason,
            other => panic!("refused by a schema rule, not {other:?}"),
        }
    };
    assert!(matches!(
        Roster::parse("{not json", public()),
        Err(PersonaError::Roster("it is not JSON"))
    ));
    assert!(matches!(
        Roster::parse("[]", public()),
        Err(PersonaError::Roster("it is not a JSON object"))
    ));
    let cases: [(&Edit, &str); 12] = [
        (
            &|r| r["extra"] = json!(1),
            "it holds a key the schema does not name",
        ),
        (
            &|r| r["schema"] = json!("deckstreak.agent.roster.v2"),
            "schema is not deckstreak.agent.roster.v1",
        ),
        (&|r| r["personas"] = json!([]), "personas is not an object"),
        (
            &|r| r["personas"]["law-evidence"] = json!("a name"),
            "a persona entry is not an object",
        ),
        (&|r| r["topics"] = json!(7), "topics is not an object"),
        (
            &|r| r["topics"]["Synthetic/Law"] = json!({"template": "law-evidence"}),
            "a topic key is not a lowercase slug",
        ),
        (
            &|r| r["topics"]["synthetic/law-topic"] = json!(["law-evidence"]),
            "a topic entry is not an object",
        ),
        (
            &|r| r["topics"]["synthetic/law-topic"]["weight"] = json!(1),
            "a topic entry holds a key the schema does not name",
        ),
        (
            &|r| r["topics"]["synthetic/law-topic"] = json!({}),
            "a topic names no template",
        ),
        (
            &|r| {
                r["topics"]["synthetic/language-topic"]
                    .as_object_mut()
                    .expect("a topic")
                    .remove("cefr");
            },
            "a language topic names no cefr band",
        ),
        (
            &|r| r["topics"]["synthetic/law-topic"]["cefr"] = json!("B1"),
            "only a language topic names a cefr band",
        ),
        (
            &|r| r["topics"]["synthetic/language-topic"]["cefr"] = json!("D1"),
            "a topic's cefr is not a band from A1 to C2",
        ),
    ];
    for (edit, reason) in cases {
        assert_eq!(roster(edit), reason);
    }
}

#[test]
fn a_slot_that_is_not_one_line_of_plain_text_is_refused() {
    for (slot, value) in [
        (Slot::Name, "Two\nlines"),
        (Slot::Bio, "A\ttab"),
        (Slot::Voice, "A {{name}} token"),
        (Slot::Personality, "An opening {{ alone"),
        (Slot::Personality, "A closing }} alone"),
    ] {
        let mut roster = synthetic_roster();
        roster["personas"]["law-evidence"][slot.name()] = json!(value);
        let refused = refusal(&roster);
        assert!(
            matches!(&refused, Some(PersonaError::SlotNotPlainText { slot: name }) if *name == slot.name()),
            "{value:?}: {refused:?}"
        );
    }
    let mut numeric = synthetic_roster();
    numeric["personas"]["law-evidence"]["bio"] = json!(7);
    assert!(matches!(
        refusal(&numeric),
        Some(PersonaError::SlotUnfilled { slot: "bio" })
    ));
    let slots = Slots::new("A", "B", "C", "D").expect("four one-line slots");
    assert_eq!(format!("{slots:?}"), "Slots(..)");
    assert_eq!(
        Slot::ALL.map(|slot| slots.value(slot)),
        ["A", "B", "C", "D"]
    );
    let gone = tempfile::tempdir().expect("a temporary directory");
    let path = RosterPath::new(gone.path().join("roster.json")).expect("an absolute path");
    assert!(matches!(
        Roster::load(&path, public()),
        Err(PersonaError::RosterUnreadable(_))
    ));
    assert!(RosterPath::new("roster.json").is_none());
    assert_eq!(path.path(), gone.path().join("roster.json"));
    for refused in [
        "",
        "law/",
        "/law",
        "Law/evidence",
        "law//evidence",
        "law/evi dence",
    ] {
        assert_eq!(
            TopicKey::parse(refused),
            None,
            "{refused:?} is not a topic key"
        );
    }
    assert_eq!(topic("law/evidence-2").as_str(), "law/evidence-2");
}

#[test]
fn the_example_roster_reads_against_the_public_templates() {
    let example = repo().join("agent").join("roster.example.json");
    let path = RosterPath::new(
        example
            .canonicalize()
            .expect("agent/roster.example.json exists"),
    )
    .expect("an absolute path");
    let roster = Roster::load(&path, public()).expect("the example reads as the roster schema");
    let professor = roster
        .persona(&topic("example/law-topic"), Duty::DailyReading)
        .expect("the example binds its law topic");
    assert!(
        professor
            .text()
            .contains("Example Professor is the professor of Evidence.")
    );
    let mentor = roster
        .persona(&topic("example/language-topic"), Duty::WritingTutor)
        .expect("the example binds its language topic");
    assert_eq!(mentor.band(), Some(CefrBand::A2));
}

/// A fake source's port: it answers one synthetic entry for any subject.
struct FakeSource;

impl MemoryPort for FakeSource {
    fn read<'a>(&'a self, _subject: &'a Subject) -> PortFuture<'a, Vec<String>> {
        Box::pin(async { Ok(vec!["a synthetic entry".to_owned()]) })
    }
}

/// A fake live band's port: it answers its band for any subject.
struct FakeBand(Option<CefrBand>);

impl LiveBand for FakeBand {
    fn band<'a>(&'a self, _subject: &'a Subject) -> PortFuture<'a, Option<CefrBand>> {
        let band = self.0;
        Box::pin(async move { Ok(band) })
    }
}

#[tokio::test]
async fn the_golden_readings_open_with_the_frontmatter_the_engine_writes() {
    let directory = repo().join("agent").join("golden").join("daily-reading");
    let mut goldens = Vec::new();
    for kind in fs::read_dir(&directory).expect("the golden readings' directory") {
        let kind = kind.expect("a directory entry").path();
        for file in fs::read_dir(&kind).expect("a kind's directory") {
            let path = file.expect("a directory entry").path();
            if path.extension().is_some_and(|extension| extension == "md") {
                goldens.push(path);
            }
        }
    }
    goldens.sort();
    let goldens = examined("golden readings in agent/golden/daily-reading", goldens);
    let names: Vec<String> = goldens
        .iter()
        .map(|path| {
            path.strip_prefix(&directory)
                .expect("a golden under its directory")
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    assert_eq!(
        names,
        [
            "language/language-mentor-es.output.md",
            "law/law-evidence.output.md"
        ],
        "each golden reading is held to the engine's frontmatter below"
    );
    let roster =
        Roster::parse(&synthetic_roster().to_string(), public()).expect("the roster reads");
    // The law reading: the engine read the leeches and the lapses wired for it.
    let professor = roster
        .persona(&topic("synthetic/law-topic"), Duty::DailyReading)
        .expect("the professor");
    let ports = MemoryPorts::default()
        .wired(MemorySource::Leeches, Arc::new(FakeSource))
        .wired(MemorySource::Lapses, Arc::new(FakeSource));
    let mut reader = MemoryReader::new(&professor, &ports);
    reader
        .read_declared()
        .await
        .expect("the wired sources read");
    let band = resolve_band(&professor, None)
        .await
        .expect("no band to read");
    let law = Frontmatter::new(&professor, band, reader.reads());
    // The language reading: no source wired, and the roster's band.
    let mentor = roster
        .persona(&topic("synthetic/language-topic"), Duty::DailyReading)
        .expect("the mentor");
    let none = MemoryPorts::default();
    let mut empty = MemoryReader::new(&mentor, &none);
    empty.read_declared().await.expect("nothing to read");
    let band = resolve_band(&mentor, None)
        .await
        .expect("the roster's band");
    let language = Frontmatter::new(&mentor, band, empty.reads());
    for (path, frontmatter) in [(&goldens[1], law), (&goldens[0], language)] {
        let golden = fs::read_to_string(path).expect("a golden reading");
        let opening = format!("---\n{}", frontmatter.lines());
        assert!(
            golden.starts_with(&opening),
            "{} opens with the engine's frontmatter:\n{opening}",
            path.display()
        );
    }
}

#[tokio::test]
async fn the_cefr_band_comes_from_the_live_band_before_the_roster() {
    let roster =
        Roster::parse(&synthetic_roster().to_string(), public()).expect("the roster reads");
    let mentor = roster
        .persona(&topic("synthetic/language-topic"), Duty::DailyReading)
        .expect("the mentor");
    let live = FakeBand(Some(CefrBand::B1));
    let band = resolve_band(&mentor, Some(&live))
        .await
        .expect("the live band");
    assert_eq!(
        band,
        Some(CefrBand::B1),
        "the live band, when its port is wired and answers"
    );
    assert_eq!(
        resolve_band(&mentor, None)
            .await
            .expect("the roster's band"),
        Some(CefrBand::A2),
        "the roster's band, when no port is wired"
    );
    assert_eq!(
        resolve_band(&mentor, Some(&FakeBand(None)))
            .await
            .expect("the roster's band"),
        Some(CefrBand::A2),
        "the roster's band, when the port has none for the subject"
    );
    // The output names the band it used.
    let lines = Frontmatter::new(&mentor, band, &[]).lines();
    assert!(lines.contains("\ncefr: \"B1\"\nlang: \"es\"\n"), "{lines}");
    // A law persona carries no band, whether a live port or a caller offers one.
    let professor = roster
        .persona(&topic("synthetic/law-topic"), Duty::DailyReading)
        .expect("the professor");
    assert_eq!(
        resolve_band(&professor, Some(&live))
            .await
            .expect("no band"),
        None
    );
    let law = Frontmatter::new(&professor, Some(CefrBand::B1), &[]).lines();
    assert!(law.starts_with("schema: \"phx.persona.output.v1\"\npersona: \"law-evidence\"\n"));
    assert!(!law.contains("cefr") && !law.contains("lang"), "{law}");
}
