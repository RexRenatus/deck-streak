//! The text a native client reads for each refusal (SPEC-336 R3).
//!
//! MUTATION COVERAGE, not red first: the refusal's text was green before this test, which was added
//! after the code to kill the mutant that answers every refusal with empty text (S33600). A refusal
//! carries its fields as data for a client that branches on them, and its text for a person who
//! reads the error a client shows; each variant's text names what refused the call and why.

use deck_streak_ffi::engine::{Engine, EngineRefusal};

/// `BackendCollectionService.CloseCollection`: a real engine call the allow-list leaves out.
const CLOSE_COLLECTION: (u32, u32) = (3, 1);

/// MUTATION COVERAGE (R3; S33600): each refusal reads as its own sentence, the one an unlisted call
/// gets from the entry point included, and none of them is empty.
#[test]
fn each_refusal_reads_as_its_own_sentence() {
    let unlisted = match Engine::new(Vec::new()) {
        Ok(engine) => engine
            .run(CLOSE_COLLECTION.0, CLOSE_COLLECTION.1, Vec::new())
            .map_err(|refusal| refusal.to_string()),
        Err(refusal) => Err(format!(
            "the engine did not start from its defaults: {refusal:?}"
        )),
    };
    let engine = EngineRefusal::Engine {
        error: vec![8, 1, 18, 3],
    };
    let start = EngineRefusal::Start {
        reason: String::from("a synthetic reason"),
    };
    assert_eq!(
        (unlisted, engine.to_string(), start.to_string()),
        (
            Err(String::from("service 3 method 1 is not on the allow-list")),
            String::from("the engine refused the call (4 bytes)"),
            String::from("the engine could not start: a synthetic reason"),
        )
    );
}
