//! A persona reads memory only through a reader built for its own subject: another subject's read
//! is refused before any port is asked, the journal can never be named, and an output declares
//! exactly the reads the reader made (SPEC-044 A4, A5).

// An integration test is test code: its helpers panic on an unreadable fixture or a poisoned lock.
#![allow(clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};

use deck_streak_agent::{
    Duty, Frontmatter, MemoryError, MemoryPort, MemoryPorts, MemoryRead, MemoryReader,
    MemorySource, Persona, PersonaError, Recall, Slots, Subject, Template,
};
use deck_streak_kernel::{KernelError, PortFuture};

/// A fake source's port: it answers `entries`, and records each subject it is asked for.
struct FakeSource {
    entries: Vec<String>,
    asked: Mutex<Vec<String>>,
}

impl FakeSource {
    fn new(entries: &[&str]) -> Arc<Self> {
        Arc::new(Self {
            entries: entries.iter().map(|entry| (*entry).to_owned()).collect(),
            asked: Mutex::new(Vec::new()),
        })
    }

    /// Every subject the reader asked this source for, in order.
    fn asked(&self) -> Vec<String> {
        self.asked.lock().expect("the fake's lock").clone()
    }
}

impl MemoryPort for FakeSource {
    fn read<'a>(&'a self, subject: &'a Subject) -> PortFuture<'a, Vec<String>> {
        Box::pin(async move {
            self.asked
                .lock()
                .expect("the fake's lock")
                .push(subject.as_str().to_owned());
            Ok(self.entries.clone())
        })
    }
}

/// A source whose port fails.
struct FailingSource;

impl MemoryPort for FailingSource {
    fn read<'a>(&'a self, _subject: &'a Subject) -> PortFuture<'a, Vec<String>> {
        Box::pin(async {
            Err(KernelError::Offload {
                operation: "a synthetic read",
            })
        })
    }
}

/// The public template `name`, read from `agent/personas/`.
fn template_text(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("agent")
        .join("personas")
        .join(name);
    fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("the template {} cannot be read: {error}", path.display()))
}

/// A persona of the template `text` for the daily reading, with synthetic slots.
fn persona_of(text: &str) -> Persona {
    let slots = Slots::new(
        "Professor Quill Testwell",
        "A synthetic bio of an invented professor.",
        "A synthetic voice.",
        "A synthetic personality.",
    )
    .expect("four synthetic slots");
    Template::parse(text)
        .expect("the template loads")
        .instantiate(&slots, Duty::DailyReading, None)
        .expect("the template offers the daily reading")
}

/// The evidence professor: it declares leeches, drill grades and lapses.
fn professor() -> Persona {
    persona_of(&template_text("law-evidence.persona.md"))
}

/// The tokens a rendered frontmatter's `memory` line declares.
fn declared(frontmatter: &str) -> Vec<String> {
    let line = frontmatter
        .lines()
        .find_map(|line| line.strip_prefix("memory: "))
        .expect("the frontmatter has a memory line");
    serde_json::from_str(line).expect("the memory line is one JSON list")
}

#[tokio::test]
async fn the_declared_memory_equals_the_reads_made() {
    let persona = professor();
    let leeches = FakeSource::new(&["a synthetic leech: an invented card"]);
    let lapses = FakeSource::new(&["a synthetic lapse"]);
    let ports = MemoryPorts::default()
        .wired(MemorySource::Leeches, leeches.clone())
        .wired(MemorySource::Lapses, lapses.clone());
    let mut reader = MemoryReader::new(&persona, &ports);
    let recalled = reader
        .read_declared()
        .await
        .expect("the wired sources read");
    let sources: Vec<MemorySource> = recalled.iter().map(Recall::source).collect();
    assert_eq!(sources, [MemorySource::Leeches, MemorySource::Lapses]);
    assert_eq!(
        format!("{:?}", recalled[0]),
        "Recall { source: Leeches, entries: 1 }"
    );
    // What each source was actually asked is the ground truth the declaration must equal.
    let mut made = Vec::new();
    for (name, source) in [("leeches", &leeches), ("lapses", &lapses)] {
        made.extend(
            source
                .asked()
                .into_iter()
                .map(|subject| format!("{name}@{subject}")),
        );
    }
    assert_eq!(made, ["leeches@law/evidence", "lapses@law/evidence"]);
    let frontmatter = Frontmatter::new(&persona, None, reader.reads());
    assert_eq!(declared(&frontmatter.render()), made);
    // A source read twice is declared once.
    reader
        .read_declared()
        .await
        .expect("the sources read again");
    let again = Frontmatter::new(&persona, None, reader.reads()).render();
    assert_eq!(declared(&again), made);
    // With no source wired, nothing is read and the output declares exactly that.
    let none = MemoryPorts::default();
    let mut empty = MemoryReader::new(&persona, &none);
    let nothing = empty.read_declared().await.expect("nothing to read");
    assert_eq!(nothing.len(), 0);
    let rendered = Frontmatter::new(&persona, None, empty.reads()).render();
    assert!(rendered.contains("\nmemory: []\n---\n"), "{rendered}");
    assert_eq!(declared(&rendered), Vec::<String>::new());
}

#[tokio::test]
async fn a_read_of_another_subjects_memory_is_refused() {
    let persona = professor();
    let leeches = FakeSource::new(&["a synthetic leech"]);
    let ports = MemoryPorts::default().wired(MemorySource::Leeches, leeches.clone());
    let mut reader = MemoryReader::new(&persona, &ports);
    let other = Subject::parse("language/es").expect("another subject");
    let refused = reader.read(MemorySource::Leeches, &other).await;
    assert!(
        matches!(refused, Err(MemoryError::OtherSubject)),
        "another subject's read is refused: {refused:?}"
    );
    assert!(leeches.asked().is_empty(), "no input: no port was asked");
    assert!(reader.reads().is_empty(), "no output: nothing is recorded");
    // Its own subject reads, and is recorded.
    let own = reader
        .read(MemorySource::Leeches, persona.subject())
        .await
        .expect("its own subject reads");
    assert_eq!(
        own.map(|recall| recall.entries().to_vec()),
        Some(vec!["a synthetic leech".to_owned()])
    );
    assert_eq!(leeches.asked(), ["law/evidence"]);
    let tokens: Vec<String> = reader.reads().iter().map(MemoryRead::token).collect();
    assert_eq!(tokens, ["leeches@law/evidence"]);
    assert_eq!(reader.reads()[0].source(), MemorySource::Leeches);
    assert_eq!(reader.reads()[0].subject(), persona.subject());
    // The journal can be named neither as a source nor in a template's memory.
    for journal in ["journal", "diary"] {
        assert!(
            matches!(MemorySource::parse(journal), Err(PersonaError::Journal)),
            "{journal} is never a source"
        );
    }
    let with_journal =
        template_text("law-evidence.persona.md").replacen("memory: [", "memory: [\"journal\", ", 1);
    assert!(matches!(
        Template::parse(&with_journal),
        Err(PersonaError::Journal)
    ));
}

#[tokio::test]
async fn a_source_the_template_does_not_declare_is_refused() {
    let narrow = template_text("law-evidence.persona.md").replacen(
        "memory: [\"leeches\", \"drill-grades\", \"lapses\"]",
        "memory: [\"leeches\"]",
        1,
    );
    let persona = persona_of(&narrow);
    assert_eq!(persona.memory(), [MemorySource::Leeches]);
    let lapses = FakeSource::new(&["a synthetic lapse"]);
    let ports = MemoryPorts::default().wired(MemorySource::Lapses, lapses.clone());
    let mut reader = MemoryReader::new(&persona, &ports);
    assert_eq!(reader.subject().as_str(), "law/evidence");
    let refused = reader.read(MemorySource::Lapses, persona.subject()).await;
    assert!(
        matches!(refused, Err(MemoryError::Undeclared)),
        "an undeclared source is refused: {refused:?}"
    );
    assert!(lapses.asked().is_empty(), "no port was asked");
    // Its declared source is served by no port here, so it reads nothing and records nothing.
    let nothing = reader.read_declared().await.expect("nothing to read");
    assert_eq!(nothing.len(), 0);
    assert!(reader.reads().is_empty());
}

#[tokio::test]
async fn a_source_that_fails_records_no_read() {
    let persona = professor();
    let lapses = FakeSource::new(&["a synthetic lapse"]);
    let ports = MemoryPorts::default()
        .wired(MemorySource::Leeches, Arc::new(FailingSource))
        .wired(MemorySource::Lapses, lapses.clone());
    let mut reader = MemoryReader::new(&persona, &ports);
    let failed = reader.read_declared().await;
    assert!(
        matches!(
            failed,
            Err(MemoryError::Source(KernelError::Offload { .. }))
        ),
        "a failing source fails the read: {failed:?}"
    );
    assert!(reader.reads().is_empty(), "the failed read is not declared");
    assert!(
        lapses.asked().is_empty(),
        "the reading stopped at the failure"
    );
    let recalled = reader
        .read(MemorySource::Lapses, persona.subject())
        .await
        .expect("another source still reads");
    assert!(recalled.is_some_and(|recall| recall.source() == MemorySource::Lapses));
}
