//! The one repair (SPEC-046 R7): a failed gate is named to the model once, and a second failure
//! ends the topic.

use crate::coverage::GateFailure;
use crate::state::ReadingGate;

/// How many attempts a topic gets: the first and one repair.
pub const MAX_ATTEMPTS: u32 = 2;

/// The shortest rejected line a finding may not repeat, in characters.
const QUOTE_MIN_CHARS: usize = 16;

/// What follows a failed attempt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Step {
    /// Try again with this engine-written repair text.
    Repair(String),
    /// End the topic on this gate.
    Fail(ReadingGate),
}

/// The spans a finding quotes, `'...'` or `"..."`, the way a probe's `repr` writes a fragment.
fn quoted_spans(finding: &str) -> impl Iterator<Item = &str> {
    ['\'', '"']
        .into_iter()
        .flat_map(move |mark| finding.split(mark).skip(1).step_by(2))
        .filter(|span| !span.trim().is_empty())
}

/// Whether a finding would carry rejected text or a fence marker back to the model.
fn quotes(finding: &str, rejected: &str) -> bool {
    finding.contains("<untrusted")
        || finding.contains("</untrusted")
        || quoted_spans(finding).any(|span| rejected.contains(span))
        || rejected
            .lines()
            .map(str::trim)
            .filter(|line| line.chars().count() >= QUOTE_MIN_CHARS)
            .any(|line| finding.contains(line))
}

/// What follows attempt number `attempt` failing with `failure`, whose output was `rejected`.
///
/// The repair text is the engine's own: the gate's name and its finding lines, less any line that
/// would repeat the rejected text.
#[must_use]
pub fn next(attempt: u32, failure: &GateFailure, rejected: &str) -> Step {
    if attempt >= MAX_ATTEMPTS {
        return Step::Fail(failure.gate);
    }
    let mut text = format!(
        "The previous reading failed the {} gate. Write the whole reading again and fix this:",
        failure.gate.as_str()
    );
    for finding in failure.findings.iter().filter(|f| !quotes(f, rejected)) {
        text.push_str("\n- ");
        text.push_str(finding);
    }
    Step::Repair(text)
}
