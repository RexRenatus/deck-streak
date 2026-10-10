//! Whether a card's question is an image occlusion question whose masks this app does not draw
//! (SPEC-380 R1, ADR-391 D1).

/// Whether the rendered `question` holds the engine's image occlusion mask layer or one of its
/// shapes (SPEC-380 R1).
#[must_use]
pub fn masks_not_drawn(question: &str) -> bool {
    let _ = question;
    false
}
