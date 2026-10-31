//! The Kani harnesses as they were proved (SPEC-513 section 8; ADR-531 D7). `src/kani_proofs.rs`
//! compiles only under the Kani verifier (`#[cfg(kani)]`), so no native build holds a harness and
//! no native test can call one. This test reads the file as text instead and holds each harness,
//! and the slot helper K-bind draws from, to the text it was proved as: a harness changed in any
//! way fails here until this copy changes with it, and the change is proved again under Kani.

#![allow(clippy::print_stdout)]

/// The harness file, as the Kani verifier reads it.
const HARNESSES: &str = include_str!("../src/kani_proofs.rs");

/// Each function of the harness file, from its `fn` line to its closing brace, as proved.
const PROVED: [(&str, &str); 5] = [
    (
        "k_blob_path",
        r"fn k_blob_path() {
    let hash = MediaHash::from_bytes(kani::any());
    let path = hash.blob_path();
    assert_eq!(path[2], b'/');
    assert_eq!(path[..2], hash.hex()[..2]);
    assert_eq!(MediaHash::from_blob_path(&path), Some(hash));
}",
    ),
    (
        "any_slot",
        r"fn any_slot() -> Option<Binding> {
    if kani::any() {
        let state = if kani::any() {
            BindState::Live
        } else {
            BindState::Released
        };
        Some(Binding {
            hash: MediaHash::from_bytes(kani::any()),
            state,
        })
    } else {
        None
    }
}",
    ),
    (
        "k_bind",
        r"fn k_bind() {
    let slot = any_slot();
    let claimed = if kani::any() {
        Some(MediaHash::from_bytes(kani::any()))
    } else {
        None
    };
    let hash = MediaHash::from_bytes(kani::any());
    let (after, outcome) = step(slot, claimed, hash);
    match outcome {
        Ok(bound) => {
            assert_eq!(
                after,
                Some(Binding {
                    hash,
                    state: BindState::Live
                })
            );
            if let Some(before) = slot {
                assert_eq!(before.hash, hash);
                if before.state == BindState::Released {
                    assert_eq!(bound, Bound::Revived);
                }
            }
        }
        Err(_) => assert_eq!(after, slot),
    }
    if claimed.is_some_and(|claimed| claimed != hash) {
        assert_eq!(outcome, Err(BindRefusal::Mismatch));
    }
}",
    ),
    (
        "k_sweep",
        r"fn k_sweep() {
    let facts = Facts {
        referenced: kani::any(),
        pinned: kani::any(),
        imported: kani::any(),
        added_ms: kani::any(),
    };
    let policy = SweepPolicy {
        now_ms: kani::any(),
        grace_ms: kani::any(),
    };
    let young = facts.added_ms >= policy.now_ms
        || i128::from(policy.now_ms) - i128::from(facts.added_ms) < i128::from(policy.grace_ms);
    let expected = if facts.referenced {
        Fate::Keep(Keep::Live)
    } else if facts.pinned {
        Fate::Keep(Keep::Pinned)
    } else if young {
        Fate::Keep(Keep::InGrace)
    } else if facts.imported {
        Fate::Keep(Keep::Imported)
    } else {
        Fate::Delete
    };
    assert_eq!(fate(facts, policy), expected);
}",
    ),
    (
        "k_scan",
        r"fn k_scan() {
    let bytes: [u8; 4] = kani::any();
    kani::assume(bytes.iter().all(u8::is_ascii));
    if let Ok(text) = core::str::from_utf8(&bytes) {
        for name in scan(text) {
            assert!(!name.is_empty());
        }
    }
    let inner: u8 = kani::any();
    kani::assume(inner.is_ascii_graphic() && inner != b']');
    let tag = [b'[', b's', b'o', b'u', b'n', b'd', b':', inner, b']'];
    if let Ok(text) = core::str::from_utf8(&tag) {
        assert_eq!(scan(text), vec![String::from(char::from(inner))]);
    }
}",
    ),
];

/// The function `name` of `text`: from its `fn` line to the first line after it that is `}`.
fn function<'t>(text: &'t str, name: &str) -> Option<&'t str> {
    let start = text.find(&format!("\nfn {name}("))? + 1;
    let length = text[start..].find("\n}\n")? + 2;
    Some(&text[start..start + length])
}

#[test]
fn each_kani_harness_reads_as_it_was_proved() {
    for (name, proved) in PROVED {
        assert_eq!(function(HARNESSES, name), Some(proved), "{name}");
    }
    let planted = HARNESSES.replacen(
        "facts.added_ms >= policy.now_ms",
        "facts.added_ms < policy.now_ms",
        1,
    );
    assert_ne!(
        function(&planted, "k_sweep"),
        function(HARNESSES, "k_sweep"),
        "a planted change to a harness reads as a change"
    );
    println!(
        "examined {} of {} harness function(s)",
        PROVED.len(),
        PROVED.len()
    );
}
