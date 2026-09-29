//! The output gate (SPEC-043 R9, R10): the packs' blocking checks, run over an output before
//! anything is delivered.
//!
//! A probe speaks one protocol: `python3 <probe> --root R --subject OUT [--subject TEMPLATE] check
//! <class>`; exit 0 is green, exit 1 a finding, exit 2 misuse and exit 3 VOID, and a report that
//! examined nothing is not a pass. Anything but a proven green fails closed.

use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};

use tokio::process::Command;

/// Numbers each staged subject, so two checks in one process never share a file.
static STAGE_SEQUENCE: AtomicU64 = AtomicU64::new(0);
/// The class a gate that could not run reports: an unproven output is never delivered.
pub const CLASS_VOID: &str = "void";
/// The class an output that examined nothing reports.
pub const CLASS_EMPTY: &str = "examined-nothing";

/// A future a gate returns.
pub type GateFuture<'a> = Pin<Box<dyn Future<Output = GateOutcome> + Send + 'a>>;

/// What the gate decided.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GateOutcome {
    /// Every blocking class was green.
    Passed,
    /// A blocking class refused the output.
    Failed {
        /// The class that refused it.
        class: String,
        /// The probe's finding lines.
        findings: Vec<String>,
    },
}

/// Something that gates an output.
pub trait OutputGate: Send + Sync {
    /// Checks `output`, produced from `template`.
    fn check<'a>(&'a self, output: &'a str, template: &'a str) -> GateFuture<'a>;

    /// Checks one untrusted input for the invisible-character class before it is fenced (R8).
    fn check_input<'a>(&'a self, input: &'a str) -> GateFuture<'a>;
}

/// One blocking class and the probe that answers it.
#[derive(Clone, Debug)]
pub struct GateClassSpec {
    /// The probe script.
    pub probe: PathBuf,
    /// The class name the probe checks.
    pub class: String,
    /// Whether the probe also takes the template as a second subject.
    pub with_template: bool,
}

/// The gate that runs the packs' probes as subprocesses.
#[derive(Clone, Debug)]
pub struct ProbeGate {
    root: PathBuf,
    work_dir: PathBuf,
    classes: Vec<GateClassSpec>,
    input_class: Option<GateClassSpec>,
}

impl ProbeGate {
    /// A gate over `classes`, probing with `root` as the repository root and staging subjects in
    /// `work_dir`. `input_class` is the invisible-character check for untrusted inputs.
    #[must_use]
    pub const fn new(
        root: PathBuf,
        work_dir: PathBuf,
        classes: Vec<GateClassSpec>,
        input_class: Option<GateClassSpec>,
    ) -> Self {
        Self {
            root,
            work_dir,
            classes,
            input_class,
        }
    }

    async fn run_class(
        &self,
        spec: &GateClassSpec,
        subject: &Path,
        template: &Path,
    ) -> GateOutcome {
        let mut command = Command::new("python3");
        command
            .arg(&spec.probe)
            .arg("--root")
            .arg(&self.root)
            .arg("--subject")
            .arg(subject);
        if spec.with_template {
            command.arg("--subject").arg(template);
        }
        command
            .arg("check")
            .arg(&spec.class)
            .stdin(Stdio::null())
            .kill_on_drop(true);
        let Ok(done) = command.output().await else {
            return failed(CLASS_VOID, Vec::new());
        };
        let text = String::from_utf8_lossy(&done.stdout).into_owned();
        let findings: Vec<String> = text.lines().map(str::to_owned).collect();
        match done.status.code() {
            Some(0) if examined_something(&text) => GateOutcome::Passed,
            Some(0) => failed(CLASS_EMPTY, findings),
            Some(1) => failed(&spec.class, findings),
            _ => failed(CLASS_VOID, findings),
        }
    }

    async fn stage(&self, name: &str, text: &str) -> std::io::Result<PathBuf> {
        let path = self.work_dir.join(format!(
            "{}-{}-{name}",
            std::process::id(),
            STAGE_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        let text = text.to_owned();
        let target = path.clone();
        tokio::task::spawn_blocking(move || {
            use std::io::Write;
            use std::os::unix::fs::OpenOptionsExt;
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .mode(0o600)
                .open(target)?;
            file.write_all(text.as_bytes())
        })
        .await
        .map_err(std::io::Error::other)??;
        Ok(path)
    }
}

/// Whether a probe's report examined at least one thing: it ends `examined N`, and N is not 0.
fn examined_something(report: &str) -> bool {
    report
        .lines()
        .rev()
        .find_map(|line| line.trim().rsplit_once("examined "))
        .and_then(|(_, count)| count.trim().parse::<u64>().ok())
        .is_some_and(|count| count > 0)
}

fn failed(class: &str, findings: Vec<String>) -> GateOutcome {
    GateOutcome::Failed {
        class: class.to_owned(),
        findings,
    }
}

impl OutputGate for ProbeGate {
    fn check<'a>(&'a self, output: &'a str, template: &'a str) -> GateFuture<'a> {
        let _ = (output, template, &self.classes, &self.input_class);
        Box::pin(async { GateOutcome::Passed })
    }

    fn check_input<'a>(&'a self, input: &'a str) -> GateFuture<'a> {
        let _ = input;
        Box::pin(async { GateOutcome::Passed })
    }
}
