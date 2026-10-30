//! The render call through the adapter (SPEC-339 A1, R10).
//!
//! The harness shows the queued card's question as the engine renders it, so the allow-list's
//! sixth pair is `CardRenderingService.RenderExistingCard`. The pair below is read from the
//! engine's generated dispatch at the pinned rev, not from the adapter's table: the backend card
//! rendering service is 27, and the render call, one of the collection service's methods it
//! fronts, is its method 6. A wrong entry in the table fails here.

// A failed setup step fails the test, as clippy.toml allows inside a test function.
#![allow(clippy::expect_used)]

#[expect(
    dead_code,
    reason = "this test renders the card the queue gives, so it reads neither id the builder returns"
)]
mod support;

use deck_streak_ffi::engine::{Engine, EngineRefusal};
use support::synthetic::FRONT;
use support::{open_request, synthetic, wire};

/// `BackendCollectionService.OpenCollection`, as the engine's generated dispatch numbers it.
const OPEN_COLLECTION: (u32, u32) = (3, 0);
/// `SchedulerService.GetQueuedCards`.
const GET_QUEUED_CARDS: (u32, u32) = (13, 3);
/// `CardRenderingService.RenderExistingCard`, reached through the backend card rendering service.
const RENDER_EXISTING_CARD: (u32, u32) = (27, 6);

fn call(
    engine: &Engine,
    (service, method): (u32, u32),
    input: Vec<u8>,
) -> Result<Vec<u8>, EngineRefusal> {
    engine.run(service, method, input)
}

/// Who refused a call, for an assertion that reads both kinds.
fn refused_by(result: &Result<Vec<u8>, EngineRefusal>) -> &'static str {
    match result {
        Ok(_) => "nobody",
        Err(EngineRefusal::NotAllowed { .. }) => "the allow-list",
        Err(EngineRefusal::Engine { .. }) => "the engine",
        Err(EngineRefusal::Start { .. }) => "the engine's start",
    }
}

/// `GetQueuedCardsRequest { fetch_limit: 1 }`.
fn queue_request() -> Vec<u8> {
    let mut out = Vec::new();
    wire::put_varint_field(&mut out, 1, 1);
    out
}

/// The id of the card at the head of a `QueuedCards` response.
fn head_card_id(queued: &[u8]) -> i64 {
    let head = wire::repeated(queued, 1)
        .into_iter()
        .next()
        .unwrap_or_default();
    wire::signed(&wire::bytes(&head, 1), 1)
}

/// `RenderExistingCardRequest { card_id, browser: false, partial_render: false }`: proto3 omits
/// the two false fields.
fn render_request(card_id: i64) -> Vec<u8> {
    let mut out = Vec::new();
    wire::put_varint_field(
        &mut out,
        1,
        u64::try_from(card_id).expect("a card id is positive"),
    );
    out
}

/// The question's text nodes of a `RenderCardResponse`, joined; a replacement node holds no text
/// field, so it adds nothing.
fn question_text(rendered: &[u8]) -> String {
    wire::repeated(rendered, 1)
        .iter()
        .map(|node| String::from_utf8(wire::bytes(node, 1)).expect("a text node is UTF-8"))
        .collect()
}

#[test]
fn a1_renders_the_queued_cards_question() {
    let synthetic = synthetic("render-a1");
    let engine = Engine::new(Vec::new()).expect("the engine starts from the default init message");
    let opened = call(&engine, OPEN_COLLECTION, open_request(&synthetic));
    let card_id = call(&engine, GET_QUEUED_CARDS, queue_request())
        .map(|bytes| head_card_id(&bytes))
        .unwrap_or_default();
    let rendered = call(&engine, RENDER_EXISTING_CARD, render_request(card_id));
    let shows_front = rendered
        .as_ref()
        .is_ok_and(|bytes| question_text(bytes).contains(FRONT));
    assert_eq!(
        (opened, refused_by(&rendered), shows_front),
        (Ok(Vec::new()), "nobody", true),
        "A1: RenderExistingCard renders the queued card, and its question's text nodes hold the synthetic front"
    );
}
