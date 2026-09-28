//! The persona engine's templates and roster: the public templates load with their four slots
//! unfilled, a filled slot is refused, and every template rule refuses by name (SPEC-044 A1, A3).

// An integration test is test code: its helpers panic on an unreadable fixture, and it prints the
// examined count on purpose. clippy.toml's in-test allowances cover only `#[test]` bodies.
#![allow(clippy::expect_used, clippy::print_stdout, clippy::panic)]

use std::fs;
use std::path::{Path, PathBuf};

use deck_streak_agent::{
    CefrBand, Duty, MemorySource, PersonaError, Slot, Subject, SubjectKind, Template, TemplateId,
    TemplateSet,
};

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
