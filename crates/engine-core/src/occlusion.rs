//! Whether a card's question is an image occlusion question whose masks this app does not draw
//! (SPEC-380 R1, ADR-391 D1).

/// The engine's image occlusion mask layer, the canvas its own reviewer draws the masks on. It was
/// measured in the rendered question of a note the engine builds through its own image occlusion
/// interface, from a fixed image, fixed shapes and a fixed header, as
/// `<canvas id="image-occlusion-canvas"></canvas>` (SPEC-380 A4).
const MASK_LAYER: &str = "image-occlusion-canvas";

/// The attribute every image occlusion shape carries, measured on the same note's rendered question
/// as `<div class="cloze" data-ordinal="1" data-shape="rect" ...>` (SPEC-380 A4).
const SHAPE: &str = "data-shape=";

/// Whether the rendered `question` holds the engine's image occlusion mask layer or one of its
/// shapes (SPEC-380 R1): either marker alone marks it, because neither is drawn here without the
/// engine's own script, and no other question the engine renders holds either.
#[must_use]
pub fn masks_not_drawn(question: &str) -> bool {
    question.contains(MASK_LAYER) || question.contains(SHAPE)
}
