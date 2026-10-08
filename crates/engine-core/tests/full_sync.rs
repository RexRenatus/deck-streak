//! The full-sync choice and the core's two reads it compares (SPEC-357 A2-A11).
//!
//! The reads run on collections each test builds with the engine's own API: reviews are answered
//! and a collection is marked synced through a test-only engine handle of the test's own, never
//! through the core's door, and the dispatcher then reads it the way the native adapter does. The
//! rule's tests build the id sets they compare directly, so each names the ids it expects.

#![allow(
    clippy::expect_used,
    clippy::print_stdout,
    reason = "a failed fixture should fail its test, and an enumerating test prints what it examined"
)]

mod support;

use std::collections::BTreeSet;
use std::path::Path;

use anki::card::CardId;
use anki::collection::{Collection, CollectionBuilder};
use anki::scheduler::answering::{CardAnswer, Rating};
use anki::timestamp::TimestampMillis;
use anki_proto::backend::BackendError;
use anki_proto::backend::backend_error::Kind;
use anki_proto::sync::SyncCollectionResponse;
use anki_proto::sync::sync_collection_response::ChangesRequired;
use deck_streak_engine_core::dispatch::{Dispatcher, Refusal};
use deck_streak_engine_core::full_sync::{
    Confirmed, Counted, Counts, Direction, IdSets, Losses, Offer, Ready, SnapshotAnswer, Unsynced,
};
use deck_streak_engine_core::table::Transport;
use prost::Message;

/// `BackendCollectionService.OpenCollection`.
const OPEN_COLLECTION: (u32, u32) = (3, 0);

/// The test's own engine handle on a closed collection file: the door the tests use to make
/// reviews and to mark a sync, which the core does not offer an adapter.
fn engine(collection: &Path) -> Collection {
    CollectionBuilder::new(collection)
        .build()
        .expect("the engine opens the test's collection")
}

/// Answers `card` Good, as the engine's own reviewer does: one review-log row, unsynced.
fn answer(collection: &Path, card: i64) {
    let mut col = engine(collection);
    let states = col
        .get_scheduling_states(CardId(card))
        .expect("the engine reads the card's next states");
    col.answer_card(&mut CardAnswer {
        card_id: CardId(card),
        current_state: states.current,
        new_state: states.good,
        rating: Rating::Good,
        answered_at: TimestampMillis::now(),
        milliseconds_taken: 1000,
        custom_data: None,
        from_queue: false,
    })
    .expect("the engine answers the card");
    col.close(None).expect("the engine closes the collection");
}

/// Marks every row synced and the collection's last sync after its last change, as a finished
/// normal sync leaves them. The stamps move back a second first, so a later change reads after
/// the sync.
fn mark_synced(collection: &Path) {
    let col = engine(collection);
    let db = col.storage.db();
    for sql in [
        "update revlog set usn = 0",
        "update cards set usn = 0",
        "update notes set usn = 0",
        "update col set mod = mod - 1000, scm = scm - 1000",
        "update col set ls = max(mod, scm)",
    ] {
        db.execute(sql, [])
            .expect("the test marks the collection synced");
    }
    col.close(None).expect("the engine closes the collection");
}

/// Changes the collection's schema after its last sync, as a schema edit does.
fn change_schema(collection: &Path) {
    let col = engine(collection);
    col.storage
        .db()
        .execute("update col set scm = ls + 1000", [])
        .expect("the test changes the schema stamp");
    col.close(None).expect("the engine closes the collection");
}

/// The ids the test's own handle reads, and the upload re-check stamp the support computes apart
/// from the core: the oracle the core's read is compared with.
fn held(collection: &Path) -> IdSets {
    let col = engine(collection);
    let db = col.storage.db();
    let ids = |sql: &str| -> BTreeSet<i64> {
        let mut statement = db.prepare(sql).expect("the test's read prepares");
        statement
            .query_map([], |row| row.get(0))
            .expect("the test's read runs")
            .map(|id| id.expect("an id reads"))
            .collect()
    };
    let sets = IdSets {
        reviews: ids("select id from revlog"),
        cards: ids("select id from cards"),
        notes: ids("select id from notes"),
        modified: support::stamp(&col),
    };
    col.close(None).expect("the engine closes the collection");
    sets
}

/// A native dispatcher started from the default init, with no sync credential and no endpoint,
/// with `synthetic`'s collection open.
fn open(synthetic: &support::Synthetic) -> Dispatcher {
    let dispatcher =
        Dispatcher::start(Transport::Native, &[]).expect("the engine starts from the default init");
    let (service, method) = OPEN_COLLECTION;
    dispatcher
        .run(service, method, &support::open_request(synthetic))
        .expect("the native dispatcher opens the collection");
    dispatcher
}

#[test]
fn the_id_reads_take_every_row() {
    let synthetic = support::synthetic("id-reads");
    let [first, second] = synthetic.cards;
    answer(&synthetic.collection, first);
    answer(&synthetic.collection, second);
    let expected = held(&synthetic.collection);
    let read = open(&synthetic).id_sets();
    assert_eq!(
        read,
        Ok(expected.clone()),
        "the core reads every review, card and note id and the modified stamp"
    );
    assert_eq!(
        (expected.reviews.len(), expected.cards, expected.notes.len()),
        (2, BTreeSet::from(synthetic.cards), 2),
        "the collection holds two reviews, its two cards and two notes"
    );
}

#[test]
fn the_unsynced_read_counts_reviews_and_changes_since_the_last_sync() {
    let reviewed = support::synthetic("unsynced-reviewed");
    let [first, second] = reviewed.cards;
    answer(&reviewed.collection, first);
    answer(&reviewed.collection, second);
    mark_synced(&reviewed.collection);
    answer(&reviewed.collection, first);
    let schema = support::synthetic("unsynced-schema");
    mark_synced(&schema.collection);
    change_schema(&schema.collection);
    let quiet = support::synthetic("unsynced-quiet");
    mark_synced(&quiet.collection);
    let read = [&reviewed, &schema, &quiet].map(|synthetic| open(synthetic).unsynced());
    assert_eq!(
        read,
        [
            Ok(Unsynced {
                reviews: 1,
                changed: true,
                schema: false
            }),
            Ok(Unsynced {
                reviews: 0,
                changed: false,
                schema: true
            }),
            Ok(Unsynced::default()),
        ],
        "three reviews, two of them synced, then one review since the sync; a schema changed after \
         its sync; a collection untouched since its sync"
    );
    assert_eq!(
        read.map(|unsynced| unsynced.map(|unsynced| unsynced.warns())),
        [Ok(true), Ok(true), Ok(false)],
        "an unsynced review warns, a schema change alone warns, and nothing unsynced does not"
    );
}

#[test]
fn each_unsynced_change_warns_on_its_own() {
    let alone = [
        Unsynced {
            reviews: 1,
            changed: false,
            schema: false,
        },
        Unsynced {
            reviews: 0,
            changed: true,
            schema: false,
        },
        Unsynced {
            reviews: 0,
            changed: false,
            schema: true,
        },
        Unsynced::default(),
    ];
    assert_eq!(
        alone.map(|unsynced| unsynced.warns()),
        [true, true, true, false],
        "an unsynced review, a changed collection and a changed schema each warn with neither of \
         the others, and nothing unsynced does not"
    );
}

/// A refusal's kind and message, decoded with the engine's own schema; `None` for a refusal the
/// dispatcher makes before the engine sees the call.
fn engine_error(refusal: Refusal) -> Option<(Kind, String)> {
    match refusal {
        Refusal::Engine { error } => {
            let error = BackendError::decode(error.as_slice())
                .expect("a refusal decodes as the engine's error");
            Some((error.kind(), error.message))
        }
        Refusal::NotAllowed { .. } | Refusal::NeedsGesture { .. } => None,
    }
}

/// The engine's own database error when its hash reads a schema stamp that is a fraction.
const FRACTION_REFUSED: &str = r#"DbError { info: "SqliteFailure(Error { code: Unknown, extended_code: 1 }, Some(\"Invalid function parameter type Real at index 1\"))", kind: Other }"#;

#[test]
fn a_reply_that_is_not_integers_is_the_engines_database_error() {
    let synthetic = support::synthetic("unreadable-stamp");
    let col = engine(&synthetic.collection);
    col.storage
        .db()
        .execute("update col set scm = 1.5", [])
        .expect("the test writes a schema stamp that is not an integer");
    col.close(None).expect("the engine closes the collection");
    assert_eq!(
        open(&synthetic).id_sets().map_err(engine_error),
        Err(Some((Kind::DbError, FRACTION_REFUSED.to_owned()))),
        "a schema stamp that is not an integer is refused by the engine's hash as its own \
         database error, and is never read as a guessed value"
    );
}

/// An answer that found a sealed snapshot, and one that did not.
const FOUND: SnapshotAnswer = SnapshotAnswer { found: true };
const NOT_FOUND: SnapshotAnswer = SnapshotAnswer { found: false };

/// Id sets the rule's tests name directly.
fn sets(reviews: &[i64], cards: &[i64], notes: &[i64], modified: i64) -> IdSets {
    IdSets {
        reviews: reviews.iter().copied().collect(),
        cards: cards.iter().copied().collect(),
        notes: notes.iter().copied().collect(),
        modified,
    }
}

/// The losses a test computed by hand.
fn losses(reviews: usize, cards: usize, notes: usize) -> Losses {
    Losses {
        reviews,
        cards,
        notes,
    }
}

/// The choice over `device` and `server` with both directions offered, confirmed on `direction`.
fn confirmed(device: &IdSets, server: &IdSets, direction: Direction) -> Confirmed {
    Counted::show(Offer::of(true, true), device.clone(), server.clone())
        .confirm(direction)
        .expect("an offered direction is confirmed")
}

#[test]
fn the_offer_follows_the_engines_answer() {
    let offered = [
        ChangesRequired::FullSync,
        ChangesRequired::FullUpload,
        ChangesRequired::FullDownload,
        ChangesRequired::NoChanges,
        ChangesRequired::NormalSync,
    ]
    .map(|required| {
        let mut answer = SyncCollectionResponse::default();
        answer.set_required(required);
        let offer = Offer::from_answer(&answer);
        (offer.upload, offer.download)
    });
    assert_eq!(
        offered,
        [
            (true, true),
            (true, false),
            (false, true),
            (false, false),
            (false, false)
        ],
        "a full sync offers both directions, a full upload the upload only, a full download the \
         download only, and no change or a normal sync neither"
    );
}

#[test]
fn each_direction_counts_the_ids_the_replaced_side_alone_holds() {
    // Review 3 was synced before the server was replaced without it: the device holds it and the
    // server does not, so a download loses it whatever its sequence number says.
    let device = sets(&[1, 2, 3, 5], &[10, 11], &[20, 21], 7);
    let server = sets(&[1, 2, 4], &[10, 12, 13], &[20], 9);
    assert_eq!(
        Counted::show(Offer::of(true, true), device.clone(), server.clone()).counts(),
        Counts {
            upload: Some(losses(1, 2, 0)),
            download: Some(losses(2, 1, 1)),
        },
        "an upload loses the server's review 4 and cards 12 and 13; a download loses the device's \
         reviews 3 and 5, card 11 and note 21"
    );
    assert_eq!(
        [Offer::of(true, false), Offer::of(false, true)].map(|offer| Counted::show(
            offer,
            device.clone(),
            server.clone()
        )
        .counts()),
        [
            Counts {
                upload: Some(losses(1, 2, 0)),
                download: None,
            },
            Counts {
                upload: None,
                download: Some(losses(2, 1, 1)),
            },
        ],
        "a direction the engine did not offer has no count"
    );
}

#[test]
fn a_direction_not_offered_is_refused_and_the_choice_kept() {
    let device = sets(&[1, 2], &[10], &[20], 7);
    let server = sets(&[1, 3], &[10], &[20], 9);
    let upload_only = Counted::show(Offer::of(true, false), device.clone(), server.clone());
    let download_only = Counted::show(Offer::of(false, true), device, server);
    assert_eq!(
        [
            upload_only.clone().confirm(Direction::Download),
            download_only.clone().confirm(Direction::Upload),
        ],
        [Err(upload_only.clone()), Err(download_only.clone())],
        "a direction the offer does not hold is refused, and the counted choice comes back unchanged"
    );
    assert_eq!(
        [
            upload_only.confirm(Direction::Upload).is_ok(),
            download_only.confirm(Direction::Download).is_ok(),
        ],
        [true, true],
        "the offered direction is confirmed"
    );
}

#[test]
fn a_backup_missing_an_id_of_the_replaced_side_is_refused() {
    let device = sets(&[1, 2, 3], &[10, 11], &[20, 21], 7);
    let server = sets(&[1, 4], &[10, 12], &[20, 22], 9);
    let download = confirmed(&device, &server, Direction::Download);
    let upload = confirmed(&device, &server, Direction::Upload);
    // Each short backup lacks one id of the side its write replaces, a review, a card or a note,
    // and holds every id of the side it keeps.
    let download_short = [
        sets(&[1, 2, 4], &[10, 11, 12], &[20, 21, 22], 7),
        sets(&[1, 2, 3, 4], &[10, 12], &[20, 21, 22], 7),
        sets(&[1, 2, 3, 4], &[10, 11, 12], &[20, 22], 7),
    ];
    let upload_short = [
        sets(&[1, 2, 3], &[10, 11, 12], &[20, 21, 22], 9),
        sets(&[1, 2, 3, 4], &[10, 11], &[20, 21, 22], 9),
        sets(&[1, 2, 3, 4], &[10, 11, 12], &[20, 21], 9),
    ];
    assert_eq!(
        download_short
            .each_ref()
            .map(|backup| download.clone().backed_up(backup).err()),
        [
            Some(download.clone()),
            Some(download.clone()),
            Some(download.clone())
        ],
        "a download's backup that lacks one of the device's review, card or note ids is refused, \
         and the confirmed choice is kept"
    );
    assert_eq!(
        upload_short
            .each_ref()
            .map(|backup| upload.clone().backed_up(backup).err()),
        [
            Some(upload.clone()),
            Some(upload.clone()),
            Some(upload.clone())
        ],
        "an upload's backup that lacks one of the server copy's ids is refused"
    );
    assert_eq!(
        [
            download.backed_up(&device).is_ok(),
            upload.backed_up(&server).is_ok(),
        ],
        [true, true],
        "a backup that holds every id of the replaced side is accepted"
    );
}

#[test]
fn an_upload_reaches_ready_only_after_the_snapshot_and_the_recheck() {
    // The device holds nothing the server lacks, so an upload loses the server's review 3, card 11
    // and note 21.
    let device = sets(&[1, 2], &[10], &[20], 7);
    let server = sets(&[1, 2, 3], &[10, 11], &[20, 21], 9);
    let backed = confirmed(&device, &server, Direction::Upload)
        .backed_up(&server)
        .expect("the server copy backs up the upload");
    assert_eq!(
        backed.clone().download_ready().err(),
        Some(backed.clone()),
        "an upload is not ready from its backup alone"
    );
    assert_eq!(
        backed.clone().snapshot_found(&NOT_FOUND).err(),
        Some(backed.clone()),
        "an upload whose snapshot was not found stays backed up"
    );
    let checked = backed
        .snapshot_found(&FOUND)
        .expect("a found snapshot checks the upload");
    // The fresh copies: unchanged; one more review under the same stamp; the same ids, restamped.
    let fresh = [
        server.clone(),
        sets(&[1, 2, 3, 4], &[10, 11], &[20, 21], 9),
        sets(&[1, 2, 3], &[10, 11], &[20, 21], 10),
    ];
    assert_eq!(
        fresh.map(|copy| {
            checked
                .clone()
                .rechecked(copy)
                .map(|_| ())
                .map_err(|counted| counted.counts())
        }),
        [
            Ok(()),
            Err(Counts {
                upload: Some(losses(2, 1, 1)),
                download: Some(losses(0, 0, 0)),
            }),
            Err(Counts {
                upload: Some(losses(1, 1, 1)),
                download: Some(losses(0, 0, 0)),
            }),
        ],
        "an unchanged copy is ready; a copy with a new review, or a new stamp, returns new counts"
    );
}

#[test]
fn a_download_needs_no_snapshot_and_refuses_one() {
    // The server holds nothing the device lacks, so a download loses the device's review 3, card
    // 11 and note 21.
    let device = sets(&[1, 2, 3], &[10, 11], &[20, 21], 7);
    let server = sets(&[1, 2], &[10], &[20], 9);
    let backed = confirmed(&device, &server, Direction::Download)
        .backed_up(&device)
        .expect("the device's collection backs up the download");
    assert_eq!(
        backed.clone().download_ready().map(|_| ()),
        Ok(()),
        "a download is ready from its backup"
    );
    assert_eq!(
        backed.clone().snapshot_found(&FOUND).err(),
        Some(backed),
        "a download refuses the snapshot step, even on a found snapshot"
    );
}

#[test]
fn a_write_refuses_a_device_row_its_backup_lacks() {
    let device = sets(&[1, 2], &[10, 11], &[20, 21], 7);
    let server = sets(&[1, 3], &[10], &[20], 9);
    let download = || {
        confirmed(&device, &server, Direction::Download)
            .backed_up(&device)
            .expect("the device's collection backs up the download")
            .download_ready()
            .expect("a download is ready from its backup")
    };
    let written = |ready: Ready, now: &IdSets| {
        ready
            .at_write(now)
            .map(|write| write.direction())
            .map_err(|counted| counted.counts())
    };
    // The device read at the write gained one row since its backup: a review, a card or a note.
    let gained = [
        sets(&[1, 2, 4], &[10, 11], &[20, 21], 8),
        sets(&[1, 2], &[10, 11, 12], &[20, 21], 8),
        sets(&[1, 2], &[10, 11], &[20, 21, 22], 8),
    ];
    assert_eq!(
        gained.each_ref().map(|now| written(download(), now)),
        [
            Err(Counts {
                upload: Some(losses(1, 0, 0)),
                download: Some(losses(2, 1, 1)),
            }),
            Err(Counts {
                upload: Some(losses(1, 0, 0)),
                download: Some(losses(1, 2, 1)),
            }),
            Err(Counts {
                upload: Some(losses(1, 0, 0)),
                download: Some(losses(1, 1, 2)),
            }),
        ],
        "a download whose device gained a row its backup lacks is refused, with new counts over \
         the device read now"
    );
    let upload = confirmed(&device, &server, Direction::Upload)
        .backed_up(&server)
        .expect("the server copy backs up the upload")
        .snapshot_found(&FOUND)
        .expect("a found snapshot checks the upload")
        .rechecked(server.clone())
        .expect("an unchanged server copy is ready");
    assert_eq!(
        [written(download(), &device), written(upload, &gained[0])],
        [Ok(Direction::Download), Ok(Direction::Upload)],
        "a download whose device is unchanged is written, and an upload admits a device gain, \
         which it sends"
    );
}

/// Each `Write {` construction in `text`, named by the fn it sits in, and those outside
/// `fn at_write`'s braces. Comments, the type's declaration and its impl are not constructions.
fn write_constructions(text: &str) -> (Vec<String>, Vec<String>) {
    let mut sites = Vec::new();
    let mut outside = Vec::new();
    let mut within = String::new();
    let mut at_write: Option<(usize, usize)> = None;
    for line in text.lines() {
        let code = line.split("//").next().unwrap_or_default();
        if let Some((_, after)) = code.split_once("fn ") {
            within = after
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if within == "at_write" {
                at_write = Some((0, 0));
            }
        }
        let built = code.contains("Write {")
            && !code.contains("struct Write {")
            && !code.contains("impl Write {")
            && !code.contains("-> Write {");
        if built {
            sites.push(within.clone());
            if at_write.is_none() {
                outside.push(within.clone());
            }
        }
        if let Some((opens, closes)) = at_write.as_mut() {
            *opens += code.matches('{').count();
            *closes += code.matches('}').count();
            if *opens > 0 && opens == closes {
                at_write = None;
            }
        }
    }
    (sites, outside)
}

#[test]
fn only_the_choice_makes_a_write() {
    let source = support::workspace().join("crates/engine-core/src/full_sync.rs");
    let text = std::fs::read_to_string(&source).expect("the rule's source reads");
    let (sites, outside) = write_constructions(&text);
    assert_eq!(
        outside,
        Vec::<String>::new(),
        "only Ready::at_write constructs a Write"
    );
    // The positive control: a copy with a Write built in a fn of its own is refused by that name.
    let plant = support::scratch("engine-core-full-sync", "only-the-choice").join("full_sync.rs");
    std::fs::write(
        &plant,
        format!(
            "{text}\nfn planted() -> Write {{\n    Write {{ direction: Direction::Upload }}\n}}\n"
        ),
    )
    .expect("the planted copy is written");
    let planted = std::fs::read_to_string(&plant).expect("the planted copy reads");
    assert_eq!(
        write_constructions(&planted).1,
        vec!["planted".to_owned()],
        "a Write built outside at_write is refused by the name of the fn that builds it"
    );
    support::examined("Write constructions", sites);
}
