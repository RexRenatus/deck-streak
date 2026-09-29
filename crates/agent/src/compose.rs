//! Prompt composition (SPEC-043 R9, A10, A11): persona-core's order, with untrusted text fenced.
//!
//! The order is the shared rules and the untrusted-content policy (the system files), then the
//! instantiated persona, then the duty's instructions, then the subject's memory as data, then the
//! untrusted inputs. The task prompt names each slot; a slot's value is written once and never
//! scanned again, so a value cannot introduce a slot of its own.

use crate::fence::{Source, encode};

/// The pieces a prompt is composed from. Only `memory` and `cards` are untrusted.
#[derive(Clone, Copy, Debug)]
pub struct Parts<'a> {
    /// The shared rules (a system file).
    pub rules: &'a str,
    /// The untrusted-content policy (a system file).
    pub policy: &'a str,
    /// The task prompt, with its `{{slot}}` markers.
    pub template: &'a str,
    /// The instantiated persona.
    pub persona: &'a str,
    /// The duty's instructions.
    pub duty: &'a str,
    /// The subject's memory, as data.
    pub memory: &'a str,
    /// Today's new cards, as data.
    pub cards: &'a str,
    /// The form's instruction (SPEC-046 R3, R4), written by the engine and trusted.
    pub form: &'a str,
    /// The prose target, written by the engine and trusted.
    pub word_target: &'a str,
    /// The repair on a second attempt, built from engine text only; empty on the first attempt.
    pub repair: &'a str,
}

/// Why a prompt could not be composed.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ComposeError {
    /// The task prompt names a slot this composer does not know.
    #[error("the task prompt names an unknown slot: {0}")]
    UnknownSlot(String),
    /// A trusted piece holds a fence marker, so it could be mistaken for untrusted text.
    #[error("a trusted piece holds an untrusted fence marker")]
    FenceInTrusted,
}

/// The composed prompt: the system files, then the task prompt with every slot filled.
///
/// # Errors
///
/// [`ComposeError`] when the task prompt names an unknown slot, or a trusted piece carries a fence
/// marker.
pub fn compose(parts: &Parts<'_>) -> Result<String, ComposeError> {
    for trusted in [
        parts.rules,
        parts.policy,
        parts.persona,
        parts.duty,
        parts.form,
        parts.word_target,
        parts.repair,
    ] {
        if trusted.contains("<untrusted") || trusted.contains("</untrusted") {
            return Err(ComposeError::FenceInTrusted);
        }
    }
    let mut out = String::new();
    out.push_str(parts.rules.trim_end());
    out.push_str("\n\n");
    out.push_str(parts.policy.trim_end());
    out.push_str("\n\n");
    let mut rest = parts.template;
    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let Some(end) = after.find("}}") else {
            return Err(ComposeError::UnknownSlot(after.to_owned()));
        };
        let name = &after[..end];
        out.push_str(&slot(parts, name)?);
        rest = &after[end + 2..];
    }
    out.push_str(rest);
    Ok(out)
}

fn slot(parts: &Parts<'_>, name: &str) -> Result<String, ComposeError> {
    Ok(match name {
        "persona" => parts.persona.trim_end().to_owned(),
        "duty_rules" => parts.duty.trim_end().to_owned(),
        "form" => parts.form.trim_end().to_owned(),
        "word_target" => parts.word_target.trim_end().to_owned(),
        "repair" => parts.repair.trim_end().to_owned(),
        n if n == format!("{}|json", Source::Memory.as_str()) => encode(parts.memory),
        n if n == format!("{}|json", Source::Cards.as_str()) => encode(parts.cards),
        other => return Err(ComposeError::UnknownSlot(other.to_owned())),
    })
}
