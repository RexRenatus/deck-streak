//! ADR-022's memory and time budgets for Anki's engine, measured over its synthetic collection
//! (SPEC-022 A2, A3, and the incremental-sync budget of R3).
//!
//! Each memory test builds the collection, then runs the measured operation alone in a fresh
//! process that reads its own peak resident set (`VmHWM`) when the operation returns
//! (`support/mod.rs`). Each test also proves the operation happened, so a budget cannot pass by
//! measuring nothing: the queue holds the new cards the scheduler counts, the downloaded copy
//! holds the whole collection, and the incremental sync pulls the other client's reviews.
//!
//! Every test prints one `engine-budget` line, which `engine-measure.yml` gathers into its report.

mod support;

use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use deck_streak_ingest::engine::{AnkiEngine, RslibEngine, SyncLogin, SyncOutcome};
use support::synthetic::{self, Side};

/// ADR-022: the peak resident memory of opening the collection and resolving its new-card queue,
/// and of a full download, in KiB (256 MiB).
const MEMORY_BUDGET_KIB: u64 = 256 * 1024;
/// ADR-022: the wall time of an incremental sync that pulls 100 new reviews, in seconds.
const INCREMENTAL_BUDGET_SECONDS: f64 = 60.0;
/// ADR-022: the new reviews the incremental sync pulls.
const NEW_REVIEWS: usize = 100;

/// KiB as MiB. A resident set of 4 TiB or more reads as 4 TiB, far past any budget.
fn mib(kib: u64) -> f64 {
    f64::from(u32::try_from(kib).unwrap_or(u32::MAX)) / 1024.0
}

fn folder(scratch: &Path, name: &str) -> PathBuf {
    let folder = scratch.join(name);
    fs::create_dir_all(&folder).unwrap_or_else(|error| panic!("{}: {error}", folder.display()));
    folder
}

#[test]
fn opening_a_large_synthetic_collection_and_its_new_card_queue_stays_inside_the_memory_budget() {
    const TEST: &str = "opening_a_large_synthetic_collection_and_its_new_card_queue_stays_inside_the_memory_budget";
    if support::role().as_deref() == Some(support::MEASURE) {
        let queue = RslibEngine
            .new_card_queue(Path::new(&support::handed(support::COLLECTION)))
            .expect("the engine resolves today's new-card queue");
        support::report("vm_hwm_kib", support::peak_resident_kib());
        support::report("roots", queue.roots.len());
        support::report("new_cards", queue.new_card_total());
        support::report(
            "new_count",
            queue.roots.iter().map(|root| root.new_count).sum::<usize>(),
        );
        return;
    }
    let scratch = tempfile::tempdir().expect("a scratch directory");
    let collection = scratch.path().join("collection.anki2");
    let built = synthetic::build(&collection, Side::Client);
    assert_eq!(built.cards, synthetic::CARDS, "ADR-022's cards are built");
    assert_eq!(
        built.leaf_decks,
        synthetic::DECKS,
        "ADR-022's decks are built"
    );

    let report = support::measure(TEST, &[(support::COLLECTION, collection.as_os_str())]);
    let new_cards: usize = report.value("new_cards");
    assert_eq!(
        report.value::<usize>("roots"),
        synthetic::ROOTS.len(),
        "the queue is resolved for both top-level decks"
    );
    assert!(
        new_cards > 0,
        "the queue held no new card, so the budget measured nothing"
    );
    assert_eq!(
        new_cards,
        report.value::<usize>("new_count"),
        "the read holds every new card the scheduler counts for today"
    );
    let peak: u64 = report.value("vm_hwm_kib");
    println!(
        "engine-budget open-and-queue vm_hwm_mib={:.1} budget_mib={} new_cards={new_cards}",
        mib(peak),
        MEMORY_BUDGET_KIB / 1024
    );
    assert!(
        peak <= MEMORY_BUDGET_KIB,
        "opening the collection and resolving its queue peaked at {:.1} MiB, over ADR-022's {} MiB",
        mib(peak),
        MEMORY_BUDGET_KIB / 1024
    );
}

#[test]
fn a_full_download_of_the_large_synthetic_collection_stays_inside_the_memory_budget() {
    const TEST: &str =
        "a_full_download_of_the_large_synthetic_collection_stays_inside_the_memory_budget";
    match support::role().as_deref() {
        Some(support::SERVER) => return support::serve(),
        Some(support::MEASURE) => {
            let login = SyncLogin::new(
                support::handed(support::ENDPOINT),
                support::USERNAME,
                support::PASSWORD,
            );
            let copy = PathBuf::from(support::handed(support::COLLECTION));
            support::runtime()
                .block_on(RslibEngine.full_download(&copy, &login))
                .expect("the engine downloads the collection");
            support::report("vm_hwm_kib", support::peak_resident_kib());
            return;
        }
        _ => {}
    }
    let scratch = tempfile::tempdir().expect("a scratch directory");
    let base = scratch.path().join("server");
    let built = synthetic::build(&support::server_collection(&base), Side::Server);
    assert_eq!(built.cards, synthetic::CARDS, "ADR-022's cards are built");
    let copy = folder(scratch.path(), "copy").join("collection.anki2");
    let server = support::SyncServer::start(TEST, &base);

    let report = support::measure(
        TEST,
        &[
            (support::ENDPOINT, OsStr::new(server.endpoint())),
            (support::COLLECTION, copy.as_os_str()),
        ],
    );
    drop(server);
    let downloaded = synthetic::counts(&copy);
    assert_eq!(
        downloaded.cards,
        synthetic::CARDS,
        "the copy holds every card of the server's"
    );
    assert_eq!(
        downloaded.reviews,
        synthetic::REVIEWS,
        "the copy holds every review of the server's"
    );
    let peak: u64 = report.value("vm_hwm_kib");
    println!(
        "engine-budget full-download vm_hwm_mib={:.1} budget_mib={}",
        mib(peak),
        MEMORY_BUDGET_KIB / 1024
    );
    assert!(
        peak <= MEMORY_BUDGET_KIB,
        "a full download peaked at {:.1} MiB, over ADR-022's {} MiB",
        mib(peak),
        MEMORY_BUDGET_KIB / 1024
    );
}

#[test]
fn an_incremental_sync_of_one_hundred_new_reviews_stays_inside_the_time_budget() {
    const TEST: &str =
        "an_incremental_sync_of_one_hundred_new_reviews_stays_inside_the_time_budget";
    if support::role().as_deref() == Some(support::SERVER) {
        return support::serve();
    }
    let scratch = tempfile::tempdir().expect("a scratch directory");
    let base = scratch.path().join("server");
    synthetic::build(&support::server_collection(&base), Side::Server);
    let server = support::SyncServer::start(TEST, &base);
    let login = SyncLogin::new(server.endpoint(), support::USERNAME, support::PASSWORD);
    let runtime = support::runtime();
    let copy = folder(scratch.path(), "copy").join("collection.anki2");
    runtime
        .block_on(RslibEngine.full_download(&copy, &login))
        .expect("the copy starts as a full download");
    let before = synthetic::counts(&copy);
    let other = folder(scratch.path(), "other").join("collection.anki2");
    fs::copy(&copy, &other).expect("the other client starts from the same collection");
    support::review_on_another_client(&runtime, &other, server.endpoint(), NEW_REVIEWS);

    let started = Instant::now();
    let outcome = runtime
        .block_on(RslibEngine.normal_sync(&copy, &login))
        .expect("the copy syncs");
    let seconds = started.elapsed().as_secs_f64();
    let after = synthetic::counts(&copy);
    assert_eq!(
        outcome,
        SyncOutcome::Synced,
        "the copy exchanged changes with the server"
    );
    assert_eq!(
        after.reviews,
        before.reviews + NEW_REVIEWS,
        "the sync pulled the other client's reviews"
    );
    println!(
        "engine-budget incremental-sync seconds={seconds:.2} budget_seconds={INCREMENTAL_BUDGET_SECONDS}"
    );
    assert!(
        seconds <= INCREMENTAL_BUDGET_SECONDS,
        "the incremental sync took {seconds:.2} s, over ADR-022's {INCREMENTAL_BUDGET_SECONDS} s"
    );
}
