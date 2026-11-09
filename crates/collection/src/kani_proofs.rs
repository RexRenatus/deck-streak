// @phx harness k_put ramp=report unwind=33

// The media store's Kani harness (SPEC-601 R8; ADR-621 D9), compiled only under the Kani
// verifier. It drives the blob store's shipped put flow over an abstract hasher and a staging
// whose every step may fail, and checks that a placed file holds bytes within the cap, was synced
// before its rename and never sits under a claimed hash its bytes miss. The ledger names it
// K-put; the hash itself is held by test with real SHA-256.

use deck_streak_media::MediaHash;

use crate::blob::{BlobError, Staging, put_flow};

/// A staging whose three steps each pass or refuse as the verifier chooses.
struct Steps {
    stages: bool,
    syncs: bool,
    places: bool,
    staged: bool,
    synced: bool,
    placed: Option<MediaHash>,
}

impl Steps {
    /// A staging whose stage, sync and rename pass as `stages`, `syncs` and `places` say.
    const fn new(stages: bool, syncs: bool, places: bool) -> Self {
        Self {
            stages,
            syncs,
            places,
            staged: false,
            synced: false,
            placed: None,
        }
    }
}

/// The refusal a failing step answers.
fn refused() -> BlobError {
    BlobError::Io(std::io::Error::from(std::io::ErrorKind::Other))
}

impl Staging for Steps {
    fn stage(&mut self, _bytes: &[u8]) -> Result<(), BlobError> {
        if !self.stages {
            return Err(refused());
        }
        self.staged = true;
        Ok(())
    }

    fn sync(&mut self) -> Result<(), BlobError> {
        if !self.staged || !self.syncs {
            return Err(refused());
        }
        self.synced = true;
        Ok(())
    }

    fn place(&mut self, hash: &MediaHash) -> Result<(), BlobError> {
        if !self.synced || !self.places {
            return Err(refused());
        }
        self.placed = Some(*hash);
        Ok(())
    }

    fn discard(&mut self) {
        self.staged = false;
    }
}

/// K-put: a put that answers a hash placed exactly that hash, after a sync, within the cap and
/// matching any claimed hash; a refused put placed nothing.
#[kani::proof]
#[kani::unwind(33)]
fn k_put() {
    let bytes: [u8; 2] = kani::any();
    let cap: u64 = kani::any();
    let hash = MediaHash::from_bytes(kani::any());
    let claimed = if kani::any() {
        Some(MediaHash::from_bytes(kani::any()))
    } else {
        None
    };
    let mut staging = Steps::new(kani::any(), kani::any(), kani::any());
    let outcome = put_flow(&mut staging, |_| hash, &bytes, cap, claimed.as_ref());
    match outcome {
        Ok(placed) => {
            assert_eq!(placed, hash);
            assert!(cap >= 2);
            assert!(claimed.is_none_or(|claimed| claimed == hash));
            assert!(staging.synced);
            assert_eq!(staging.placed, Some(hash));
        }
        Err(_) => assert_eq!(staging.placed, None),
    }
}
