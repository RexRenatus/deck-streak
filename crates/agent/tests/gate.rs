//! The probe gate (SPEC-043 A9 to A11): a probe's exit is read closed, and anything unproven fails.
#![allow(clippy::expect_used)]

mod support;

use std::path::PathBuf;

use deck_streak_agent::gate::{
    CLASS_EMPTY, CLASS_VOID, GateClassSpec, GateOutcome, OutputGate, ProbeGate,
};
use support::script;

fn probe() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake-probe.py")
}

fn spec(class: &str) -> GateClassSpec {
    GateClassSpec {
        probe: probe(),
        class: class.to_owned(),
        with_template: false,
    }
}

fn gate(dir: &std::path::Path, classes: &[&str]) -> ProbeGate {
    ProbeGate::new(
        dir.to_path_buf(),
        dir.to_path_buf(),
        classes.iter().map(|c| spec(c)).collect(),
        Some(spec("output-invisible")),
    )
}

#[tokio::test]
async fn a_clean_output_passes_every_class() {
    let dir = tempfile::tempdir().expect("a directory");
    let g = gate(
        dir.path(),
        &["output-links", "output-invisible", "output-identity"],
    );
    assert_eq!(g.check("a plain reading", "tpl").await, GateOutcome::Passed);
}

#[tokio::test]
async fn the_first_failing_class_names_the_refusal() {
    let dir = tempfile::tempdir().expect("a directory");
    let g = gate(dir.path(), &["output-links", "output-identity"]);
    let GateOutcome::Failed { class, findings } = g.check("see http://x.example", "tpl").await
    else {
        panic!("expected a refusal")
    };
    assert_eq!(class, "output-links");
    assert_eq!(findings, ["output-links: finding", "examined 1"]);
}

#[tokio::test]
async fn a_probe_that_cannot_run_or_examined_nothing_fails_closed() {
    let dir = tempfile::tempdir().expect("a directory");
    let void = gate(dir.path(), &["unknown-class"]);
    let GateOutcome::Failed { class, .. } = void.check("ok", "tpl").await else {
        panic!("VOID must not pass")
    };
    assert_eq!(class, CLASS_VOID);
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
            class: "output-links".to_owned(),
            with_template: false,
        }],
        None,
    );
    let GateOutcome::Failed { class, .. } = g.check("ok", "tpl").await else {
        panic!("examined 0 must not pass")
    };
    assert_eq!(class, CLASS_EMPTY);
    let missing = ProbeGate::new(
        dir.path().to_path_buf(),
        dir.path().to_path_buf(),
        vec![GateClassSpec {
            probe: dir.path().join("absent.py"),
            class: "x".to_owned(),
            with_template: false,
        }],
        None,
    );
    assert!(matches!(
        missing.check("ok", "t").await,
        GateOutcome::Failed { .. }
    ));
}

#[tokio::test]
async fn an_untrusted_input_with_an_invisible_character_is_refused_before_it_is_fenced() {
    let dir = tempfile::tempdir().expect("a directory");
    let g = gate(dir.path(), &["output-links"]);
    assert_eq!(g.check_input("plain").await, GateOutcome::Passed);
    let GateOutcome::Failed { class, .. } = g.check_input("a\u{200b}b").await else {
        panic!("expected a refusal")
    };
    assert_eq!(class, "output-invisible");
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
            class: "persona".to_owned(),
            with_template: true,
        }],
        None,
    );
    assert_eq!(g.check("out", "tpl").await, GateOutcome::Passed);
    let argv = std::fs::read_to_string(dir.path().join("argv")).expect("argv");
    assert_eq!(argv.matches("--subject").count(), 2);
    assert!(argv.ends_with("check persona"));
}
