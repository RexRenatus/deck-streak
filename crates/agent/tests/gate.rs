//! The probe gate (SPEC-043 A9 to A11): a probe's exit is read closed, and anything unproven fails.
#![allow(clippy::expect_used)]

mod support;

use std::path::PathBuf;

use deck_streak_agent::gate::{
    GateBuildError, GateClass, GateClassSpec, GateOutcome, OutputGate, ProbeGate,
};
use support::script;

fn probe() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake-probe.py")
}

fn spec(class: GateClass) -> GateClassSpec {
    GateClassSpec {
        probe: probe(),
        class,
        with_template: false,
    }
}

fn gate(dir: &std::path::Path, classes: &[GateClass]) -> ProbeGate {
    ProbeGate::new(
        dir.to_path_buf(),
        dir.to_path_buf(),
        classes.iter().map(|c| spec(*c)).collect(),
        Some(spec(GateClass::OutputInvisible)),
        true,
    )
    .expect("a gate with its classes and its input class")
}

#[tokio::test]
async fn a_clean_output_passes_every_class() {
    let dir = tempfile::tempdir().expect("a directory");
    let g = gate(
        dir.path(),
        &[
            GateClass::OutputLinks,
            GateClass::OutputInvisible,
            GateClass::NoHumanClaim,
        ],
    );
    assert_eq!(g.check("a plain reading", "tpl").await, GateOutcome::Passed);
}

#[tokio::test]
async fn the_first_failing_class_names_the_refusal() {
    let dir = tempfile::tempdir().expect("a directory");
    let g = gate(
        dir.path(),
        &[GateClass::OutputLinks, GateClass::NoHumanClaim],
    );
    let GateOutcome::Failed { class, findings } = g.check("see http://x.example", "tpl").await
    else {
        panic!("expected a refusal")
    };
    assert_eq!(class, GateClass::OutputLinks);
    assert_eq!(findings, ["output-links: finding", "examined 1"]);
}

#[tokio::test]
async fn a_probe_that_cannot_run_or_examined_nothing_fails_closed() {
    let dir = tempfile::tempdir().expect("a directory");
    // The fake probe knows three classes, so it answers `no-dates` as a class it does not know.
    let void = gate(dir.path(), &[GateClass::NoDates]);
    let GateOutcome::Failed { class, .. } = void.check("ok", "tpl").await else {
        panic!("VOID must not pass")
    };
    assert_eq!(class, GateClass::Void);
    let empty = script(dir.path(), "empty.py", "");
    std::fs::write(
        &empty,
        "import sys\nprint('output-links: ok')\nprint('examined 0')\n",
    )
    .expect("a probe");
    let g = ProbeGate::new(
        dir.path().to_path_buf(),
        dir.path().to_path_buf(),
        vec![GateClassSpec {
            probe: empty,
            class: GateClass::OutputLinks,
            with_template: false,
        }],
        None,
        false,
    )
    .expect("a gate over one class, for a duty that reads no input");
    let GateOutcome::Failed { class, .. } = g.check("ok", "tpl").await else {
        panic!("examined 0 must not pass")
    };
    assert_eq!(class, GateClass::ExaminedNothing);
    let missing = ProbeGate::new(
        dir.path().to_path_buf(),
        dir.path().to_path_buf(),
        vec![GateClassSpec {
            probe: dir.path().join("absent.py"),
            class: GateClass::OutputLinks,
            with_template: false,
        }],
        None,
        false,
    )
    .expect("a gate over one class, for a duty that reads no input");
    assert!(matches!(
        missing.check("ok", "t").await,
        GateOutcome::Failed { .. }
    ));
}

#[tokio::test]
async fn an_untrusted_input_with_an_invisible_character_is_refused_before_it_is_fenced() {
    let dir = tempfile::tempdir().expect("a directory");
    let g = gate(dir.path(), &[GateClass::OutputLinks]);
    assert_eq!(g.check_input("plain").await, GateOutcome::Passed);
    let GateOutcome::Failed { class, .. } = g.check_input("a\u{200b}b").await else {
        panic!("expected a refusal")
    };
    assert_eq!(class, GateClass::OutputInvisible);
}

#[tokio::test]
async fn a_template_flag_adds_the_template_as_a_second_subject() {
    let dir = tempfile::tempdir().expect("a directory");
    let recorder = dir.path().join("argv.py");
    std::fs::write(
        &recorder,
        format!(
            "import sys\nopen('{}/argv','w').write(' '.join(sys.argv[1:]))\nprint('examined 1')\n",
            dir.path().display()
        ),
    )
    .expect("a probe");
    let g = ProbeGate::new(
        dir.path().to_path_buf(),
        dir.path().to_path_buf(),
        vec![GateClassSpec {
            probe: recorder,
            class: GateClass::OutputContract,
            with_template: true,
        }],
        None,
        false,
    )
    .expect("a gate over one class, for a duty that reads no input");
    assert_eq!(g.check("out", "tpl").await, GateOutcome::Passed);
    let argv = std::fs::read_to_string(dir.path().join("argv")).expect("argv");
    assert_eq!(argv.matches("--subject").count(), 2);
    assert!(argv.ends_with("check output-contract"));
}

#[test]
fn an_empty_class_list_is_refused_at_construction() {
    let dir = tempfile::tempdir().expect("a directory");
    let refused = ProbeGate::new(
        dir.path().to_path_buf(),
        dir.path().to_path_buf(),
        Vec::new(),
        Some(spec(GateClass::OutputInvisible)),
        true,
    );
    assert_eq!(refused.err(), Some(GateBuildError::NoOutputClass));
    // The same call with one class is built: the refusal is the empty list and nothing else.
    let built = ProbeGate::new(
        dir.path().to_path_buf(),
        dir.path().to_path_buf(),
        vec![spec(GateClass::OutputLinks)],
        Some(spec(GateClass::OutputInvisible)),
        true,
    );
    assert!(built.is_ok());
}

#[test]
fn a_duty_that_reads_inputs_needs_an_input_class() {
    let dir = tempfile::tempdir().expect("a directory");
    let build = |input_class: Option<GateClassSpec>, reads_inputs: bool| {
        ProbeGate::new(
            dir.path().to_path_buf(),
            dir.path().to_path_buf(),
            vec![spec(GateClass::OutputLinks)],
            input_class,
            reads_inputs,
        )
    };
    assert_eq!(build(None, true).err(), Some(GateBuildError::NoInputClass));
    // A duty that reads no input needs none, and a duty that reads inputs is built with one.
    assert!(build(None, false).is_ok());
    assert!(build(Some(spec(GateClass::OutputInvisible)), true).is_ok());
}
