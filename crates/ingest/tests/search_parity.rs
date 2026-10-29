//! The read-only parity tool opens the collection through the kernel's read-only open, refuses a
//! writable open, leaves the source as it was, and answers counts only (SPEC-514 A6).

mod support;

use deck_streak_ingest::search_parity::{ParityCounts, ParityError, search_parity};
use support::synthetic::{PlannedCard, build_planned};

/// Searches over the planned collection, each with the cards both sides must select.
const SEARCHES: [(&str, usize); 6] = [
    ("deck:Spanish", 2),
    ("\"planned card\"", 3),
    ("-deck:Law", 2),
    ("deck:filtered", 1),
    ("is:new deck:Spanish::*", 1),
    ("back -card:2", 3),
];

#[tokio::test(flavor = "multi_thread")]
async fn the_parity_tool_opens_read_only_refuses_a_writable_open_and_prints_only_counts() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let source = dir.path().join("collection.anki2");
    let cards = [
        PlannedCard {
            id: 1_700_000_000_001,
            deck: "Spanish",
            filtered: false,
        },
        PlannedCard {
            id: 1_700_000_000_002,
            deck: "Spanish::Verbs",
            filtered: false,
        },
        PlannedCard {
            id: 1_700_000_000_003,
            deck: "Law",
            filtered: true,
        },
    ];
    build_planned(&source, &["Spanish", "Spanish::Verbs", "Law"], &cards, &[]);
    let before = std::fs::read(&source).expect("the source reads");

    for (index, (search, both)) in SEARCHES.iter().enumerate() {
        let scratch = dir.path().join(format!("scratch-{index}.anki2"));
        let counts = search_parity(&source, &scratch, search)
            .await
            .unwrap_or_else(|error| panic!("{search:?}: {error}"));
        assert_eq!(
            counts,
            ParityCounts {
                matched: *both,
                engine_only: 0,
                ds_only: 0,
            },
            "{search:?}"
        );
        assert_eq!(
            counts.to_string(),
            format!("match={both} engine-only=0 ds-only=0"),
            "the tool prints counts only"
        );
    }

    let refused = search_parity(&source, &source, "deck:Spanish").await;
    assert!(
        matches!(refused, Err(ParityError::WritableOpen)),
        "a writable open of the source is refused: {refused:?}"
    );
    let aliased = dir.path().join(".").join("collection.anki2");
    let refused = search_parity(&source, &aliased, "deck:Spanish").await;
    assert!(
        matches!(refused, Err(ParityError::WritableOpen)),
        "a scratch path that resolves to the source is refused: {refused:?}"
    );
    let refused = search_parity(&source, &dir.path().join("other.anki2"), "flag:1").await;
    assert!(
        matches!(refused, Err(ParityError::Search(_))),
        "{refused:?}"
    );
    let said = refused.map(|_| ()).unwrap_err().to_string();
    assert!(
        !said.contains("flag:1") && !said.contains("collection"),
        "{said}"
    );

    assert_eq!(
        std::fs::read(&source).expect("the source reads"),
        before,
        "the source is never written"
    );
    println!("examined {} search(es)", SEARCHES.len());
}

/// Builds a one-card collection at `dir`/collection.anki2 and returns its path.
fn one_card_source(dir: &std::path::Path) -> std::path::PathBuf {
    let source = dir.join("collection.anki2");
    let cards = [PlannedCard {
        id: 1_700_000_000_001,
        deck: "Spanish",
        filtered: false,
    }];
    build_planned(&source, &["Spanish"], &cards, &[]);
    source
}

#[tokio::test(flavor = "multi_thread")]
async fn a_scratch_path_hard_linked_to_the_source_is_refused_and_the_source_is_kept() {
    // A hard link is the source's own file under another name: the guard compares the device and
    // inode, so no spelling of the path lets the engine open the source writable.
    let dir = tempfile::tempdir().expect("a temporary directory");
    let source = one_card_source(dir.path());
    let before = std::fs::read(&source).expect("the source reads");
    let linked = dir.path().join("linked.anki2");
    std::fs::hard_link(&source, &linked).expect("a hard link to the source");

    let refused = search_parity(&source, &linked, "deck:Spanish").await;
    let after = std::fs::read(&source).expect("the source reads");
    println!(
        "hard link: result={refused:?} source_bytes_before={} after={} kept={}",
        before.len(),
        after.len(),
        after == before
    );
    assert!(
        after == before,
        "the source was written through a hard-linked scratch path"
    );
    assert!(
        matches!(refused, Err(ParityError::WritableOpen)),
        "a scratch path with the source's device and inode is refused: {refused:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_existing_scratch_path_is_never_truncated() {
    // The copy creates its scratch path and opens no existing one, so a file already at the path,
    // whatever it is, keeps its bytes and the tool gives no counts.
    let dir = tempfile::tempdir().expect("a temporary directory");
    let source = one_card_source(dir.path());
    let before = std::fs::read(&source).expect("the source reads");
    let scratch = dir.path().join("scratch.anki2");
    let kept: &[u8] = b"a file already at the scratch path";
    std::fs::write(&scratch, kept).expect("an existing scratch file");

    let refused = search_parity(&source, &scratch, "deck:Spanish").await;
    let found = std::fs::read(&scratch).expect("the scratch path reads");
    println!(
        "existing scratch: result={refused:?} scratch_bytes_before={} after={} kept={}",
        kept.len(),
        found.len(),
        found == kept
    );
    assert!(
        found == kept,
        "an existing scratch path was truncated and rewritten"
    );
    assert!(
        matches!(refused, Err(ParityError::Copy)),
        "the copy refuses an existing scratch path: {refused:?}"
    );
    assert!(
        std::fs::read(&source).expect("the source reads") == before,
        "the source is never written"
    );
}
