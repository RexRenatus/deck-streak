//! A card's face, completed as the engine's own reviewer completes it (SPEC-348 R2, R4; ADR-359
//! D2).
//!
//! The engine renders a card partially; its reviewer then joins each side's nodes, extracts the
//! question's AV tags, puts the question's extracted text in `{{FrontSide}}` and extracts the
//! answer's. The core does the same, strips the tags from the text it shows, turns them into sound
//! and speech clips, and rewrites the text's media under [`crate::media`]'s rules.

use anki::backend::Backend;
use anki_proto::card_rendering::rendered_template_node::Value;
use anki_proto::card_rendering::{
    RenderCardResponse, RenderExistingCardRequest, RenderedTemplateNode,
};
use prost::Message;

use crate::dispatch::Refusal;
use crate::media::Reader;

/// `CardRenderingService.RenderExistingCard`, the engine's render of a stored card.
const RENDER_EXISTING_CARD: (u32, u32) = (27, 6);
/// The replacement the engine leaves for the reviewer to fill with the question.
const FRONT_SIDE: &str = "FrontSide";

/// The side of a card a face shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// The question.
    Question,
    /// The answer, which opens with the question when its template says `{{FrontSide}}`.
    Answer,
}

/// One thing a face plays.
#[derive(Debug, Clone, PartialEq)]
pub enum Clip {
    /// A sound file, by its name and bytes.
    Sound {
        /// The file's name, as the card names it.
        name: String,
        /// The file's bytes.
        bytes: Vec<u8>,
    },
    /// Text the platform speaks.
    Speech {
        /// The text, as one plain line.
        text: String,
        /// The language, in the platform's spelling (`en-US`).
        language: String,
        /// The platform's rate: the engine's speed times its default, held to its range.
        rate: f32,
    },
}

/// A completed face.
#[derive(Debug, Clone, PartialEq)]
pub struct Face {
    /// The side's display text: no AV tag, and every media reference a `data:` URL or emptied.
    pub text: String,
    /// The note type's CSS.
    pub css: String,
    /// The clips to play when the side is shown: none unless the client wishes it and the card's
    /// preset allows it.
    pub autoplay: Vec<Clip>,
    /// The clips a replay plays.
    pub replay: Vec<Clip>,
    /// The media names left out, each once.
    pub omitted: Vec<String>,
}

fn engine_error(error: Vec<u8>) -> Refusal {
    Refusal::Engine { error }
}

/// Joins a side's nodes: a text node as it is, a replacement by its current text, and the
/// `{{FrontSide}}` replacement by `front_side` when one is given.
fn join(nodes: &[RenderedTemplateNode], front_side: Option<&str>) -> String {
    nodes
        .iter()
        .map(|node| match &node.value {
            Some(Value::Text(text)) => text.as_str(),
            Some(Value::Replacement(replacement)) => match front_side {
                Some(front) if replacement.field_name == FRONT_SIDE => front,
                _ => replacement.current_text.as_str(),
            },
            None => "",
        })
        .collect()
}

/// Completes `card`'s face for `side` (stubbed: the joined text, with no clip).
pub(crate) fn complete(
    backend: &Backend,
    card: i64,
    side: Side,
    autoplay: bool,
    media: &dyn Reader,
) -> Result<Face, Refusal> {
    let _ = (autoplay, media);
    let (service, method) = RENDER_EXISTING_CARD;
    let request = RenderExistingCardRequest {
        card_id: card,
        browser: false,
        partial_render: true,
    };
    let reply = backend
        .run_service_method(service, method, &request.encode_to_vec())
        .map_err(engine_error)?;
    let rendered =
        RenderCardResponse::decode(reply.as_slice()).map_err(|_| engine_error(Vec::new()))?;
    let question = join(&rendered.question_nodes, None);
    let text = match side {
        Side::Question => question,
        Side::Answer => join(&rendered.answer_nodes, Some(&question)),
    };
    Ok(Face {
        text,
        css: rendered.css,
        autoplay: Vec::new(),
        replay: Vec::new(),
        omitted: Vec::new(),
    })
}
