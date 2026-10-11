//! Support shared by the adapter's integration tests (SPEC-336, SPEC-339): the synthetic
//! collection's builder, which the `harness-fixture` example also includes alone, the wire
//! helpers that encode each request and decode each response, the open request every test
//! sends first, and the parity collection's builder (SPEC-393 R15), which the `review-fixture`
//! example also includes alone.

pub mod parity;
pub mod synthetic;
pub mod wire;

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

pub use synthetic::Synthetic;

/// Builds the synthetic collection in a directory of its own under the target's scratch space.
pub fn synthetic(test: &str) -> Synthetic {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("the clock reads after the epoch")
        .as_nanos();
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("ffi-round-trip")
        .join(format!("{test}-{}-{stamp}", std::process::id()));
    synthetic::build(&dir).expect("the engine builds the synthetic collection")
}

fn text(path: &Path) -> &str {
    path.to_str().expect("a scratch path is UTF-8")
}

/// `OpenCollectionRequest` for a synthetic collection: its path, its media folder and its media
/// database.
pub fn open_request(synthetic: &Synthetic) -> Vec<u8> {
    let mut out = Vec::new();
    wire::put_bytes(&mut out, 1, text(&synthetic.collection).as_bytes());
    wire::put_bytes(
        &mut out,
        2,
        text(&synthetic.dir.join("collection.media")).as_bytes(),
    );
    wire::put_bytes(
        &mut out,
        3,
        text(&synthetic.dir.join("collection.media.db")).as_bytes(),
    );
    out
}
