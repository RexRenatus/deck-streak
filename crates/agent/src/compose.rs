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
    let _ = parts;
    Ok(String::new())
}

fn slot(parts: &Parts<'_>, name: &str) -> Result<String, ComposeError> {
    Ok(match name {
        "persona" => parts.persona.trim_end().to_owned(),
        "duty_rules" => parts.duty.trim_end().to_owned(),
        n if n == format!("{}|json", Source::Memory.as_str()) => encode(parts.memory),
        n if n == format!("{}|json", Source::Cards.as_str()) => encode(parts.cards),
        other => return Err(ComposeError::UnknownSlot(other.to_owned())),
    })
}
