//! A card's face, completed as the engine's own reviewer completes it (SPEC-348 R2, R4; ADR-359
//! D2).
//!
//! The engine renders a card partially; its reviewer then joins each side's nodes, extracts the
//! question's AV tags, puts the question's extracted text in `{{FrontSide}}` and extracts the
//! answer's. The core does the same, strips the tags from the text it shows, turns them into sound
//! and speech clips, and rewrites the text's media under [`crate::media`]'s rules.

use anki::backend::Backend;
use anki::card_rendering::{extract_av_tags, strip_av_tags};
use anki::text::html_to_text_line;
use anki_proto::card_rendering::rendered_template_node::Value;
use anki_proto::card_rendering::{
    AvTag, RenderCardResponse, RenderExistingCardRequest, RenderedTemplateNode, TtsTag, av_tag,
};
use anki_proto::cards::{Card, CardId};
use anki_proto::deck_config::deck_config::Config;
use anki_proto::deck_config::{DeckConfig, DeckConfigId};
use anki_proto::decks::deck::Kind;
use anki_proto::decks::{Deck, DeckId};
use prost::Message;

use crate::dispatch::Refusal;
use crate::media::{Budget, Reader};

/// `CardRenderingService.RenderExistingCard`, the engine's render of a stored card.
const RENDER_EXISTING_CARD: (u32, u32) = (27, 6);
/// `CardsService.GetCard`, read for the card's deck and its home deck.
const GET_CARD: (u32, u32) = (5, 0);
/// `DecksService.GetDeck`, read for the deck's preset.
const GET_DECK: (u32, u32) = (7, 8);
/// `DeckConfigService.GetDeckConfig`, read for the preset's audio flags.
const GET_DECK_CONFIG: (u32, u32) = (11, 1);
/// The preset a deck that is not a normal deck reads: the engine's default, which it never
/// removes.
const DEFAULT_PRESET: i64 = 1;
/// The platform's default speech rate, and the range it holds a rate to (SPEC-348 P3).
const DEFAULT_RATE: f32 = 0.5;
/// The slowest rate the platform speaks at.
const MINIMUM_RATE: f32 = 0.0;
/// The fastest rate the platform speaks at.
const MAXIMUM_RATE: f32 = 1.0;
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
        /// The voices the tag's `voices=` names, in its order; none when it names none.
        voices: Vec<String>,
    },
}

/// A completed face.
#[derive(Debug, Clone, PartialEq)]
pub struct Face {
    /// The side's display text: no AV tag, and every media reference a `data:` URL or emptied.
    pub text: String,
    /// The note type's CSS.
    pub css: String,
    /// The card's template index: 0 for its note type's first template.
    pub ordinal: u32,
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

/// Runs one call on the engine directly, past the table: the face's own reads and render, which
/// are the same on both transports.
fn call<T: Message + Default>(
    backend: &Backend,
    (service, method): (u32, u32),
    request: &impl Message,
) -> Result<T, Refusal> {
    let reply = backend
        .run_service_method(service, method, &request.encode_to_vec())
        .map_err(engine_error)?;
    T::decode(reply.as_slice()).map_err(|_| engine_error(Vec::new()))
}

/// The preset of `card`'s deck: its home deck's when it sits in a filtered deck.
fn preset(backend: &Backend, card: i64) -> Result<Config, Refusal> {
    let card: Card = call(backend, GET_CARD, &CardId { cid: card })?;
    let deck_id = match card.original_deck_id {
        0 => card.deck_id,
        home => home,
    };
    let deck: Deck = call(backend, GET_DECK, &DeckId { did: deck_id })?;
    let preset_id = match deck.kind {
        Some(Kind::Normal(normal)) => normal.config_id,
        _ => DEFAULT_PRESET,
    };
    let preset: DeckConfig = call(backend, GET_DECK_CONFIG, &DeckConfigId { dcid: preset_id })?;
    Ok(preset.config.unwrap_or_default())
}

/// A TTS tag in the platform's terms: one plain line, the language with a hyphen, and the
/// engine's speed scaled to the platform's rate and held to its range.
fn speech(tag: &TtsTag) -> Clip {
    let language = tag.lang.replace('_', "-");
    let rate = (tag.speed * DEFAULT_RATE).clamp(MINIMUM_RATE, MAXIMUM_RATE);
    Clip::Speech {
        text: html_to_text_line(&tag.field_text, false).into_owned(),
        language,
        rate,
        voices: Vec::new(),
    }
}

/// A side's AV tags as clips: a sound the budget admits as audio, and every TTS tag. A sound it
/// refuses is left out, and its name joins the omitted ones.
fn clips(tags: &[AvTag], budget: &mut Budget<'_>) -> Vec<Clip> {
    tags.iter()
        .filter_map(|tag| match tag.value.as_ref()? {
            av_tag::Value::SoundOrVideo(name) => budget.sound(name).map(|bytes| Clip::Sound {
                name: name.clone(),
                bytes,
            }),
            av_tag::Value::Tts(tag) => Some(speech(tag)),
        })
        .collect()
}

/// Completes `card`'s face for `side` as the engine's own reviewer does (SPEC-348 R2 to R4).
pub(crate) fn complete(
    backend: &Backend,
    card: i64,
    side: Side,
    autoplay: bool,
    media: &dyn Reader,
) -> Result<Face, Refusal> {
    let preset = preset(backend, card)?;
    let request = RenderExistingCardRequest {
        card_id: card,
        browser: false,
        partial_render: true,
    };
    let rendered: RenderCardResponse = call(backend, RENDER_EXISTING_CARD, &request)?;
    let tr = backend.i18n();
    let question_raw = join(&rendered.question_nodes, None);
    let (question_extracted, question_tags) = extract_av_tags(question_raw.as_str(), true, tr);
    let answer_raw = join(&rendered.answer_nodes, Some(&question_extracted));
    let (_, answer_tags) = extract_av_tags(answer_raw.as_str(), false, tr);
    let shown_question = strip_av_tags(question_raw.as_str());
    let shown = match side {
        Side::Question => shown_question,
        Side::Answer => strip_av_tags(join(&rendered.answer_nodes, Some(&shown_question))),
    };

    let mut budget = Budget::new(media);
    let text = budget.rewrite(&shown);
    let (shown_clips, replay) = match side {
        Side::Question => {
            let question = clips(&question_tags, &mut budget);
            (question.clone(), question)
        }
        Side::Answer => {
            let mut replay = Vec::new();
            if !preset.skip_question_when_replaying_answer {
                replay = clips(&question_tags, &mut budget);
            }
            let answer = clips(&answer_tags, &mut budget);
            replay.extend(answer.iter().cloned());
            (answer, replay)
        }
    };
    let wished = autoplay;
    let allowed = !preset.disable_autoplay;
    let autoplay = if wished && allowed {
        shown_clips
    } else {
        Vec::new()
    };
    Ok(Face {
        text,
        css: rendered.css,
        ordinal: 0,
        autoplay,
        replay,
        omitted: budget.into_omitted(),
    })
}
