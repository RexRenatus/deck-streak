//! Whether a card was made with AI assistance (SPEC-388 A1 to A9).
//!
//! The rule's tests write each tag set literally, as the engine stores a note's tags, so no
//! expected answer is computed by `carries`, and `MARKER` is compared with the literal
//! `ai-assisted`. The read's tests build each collection with the engine's own API, as the
//! dispatcher's tests build theirs, and their oracle is the engine's own note, read through the
//! engine's own interface on the same collection file and never through the core. Each asserts
//! that oracle before the core's answer, so each is red at the base on the core's answer alone. A
//! package's note time is a fixed literal, never the clock, as the package builder writes the
//! decision's time (ADR-151).

#![allow(
    clippy::expect_used,
    clippy::print_stdout,
    reason = "a failed fixture should fail its test, and the shared support module prints what an \
              enumerating test examined"
)]

mod support;

use std::path::{Path, PathBuf};

use anki::collection::{Collection, CollectionBuilder};
use anki::decks::DeckId;
use anki::import_export::package::{ExportAnkiPackageOptions, ImportAnkiPackageOptions};
use anki::notes::NoteId;
use anki::sync::login::{SyncAuth as EngineAuth, sync_login};
use anki_proto::collection::{CloseCollectionRequest, OpenCollectionRequest};
use anki_proto::sync::SyncCollectionResponse;
use anki_proto::sync::sync_collection_response::ChangesRequired;
use deck_streak_engine_core::ai_assisted::{MARKER, carries};
use deck_streak_engine_core::dispatch::{Dispatcher, Refusal};
use deck_streak_engine_core::table::Transport;
use prost::Message;
use support::sync_server::{self, PASSWORD, SERVER, SyncServer, USERNAME};

/// `BackendCollectionService.OpenCollection`.
const OPEN_COLLECTION: (u32, u32) = (3, 0);
/// `BackendCollectionService.CloseCollection`.
const CLOSE_COLLECTION: (u32, u32) = (3, 1);
/// A note id no collection here holds: the engine numbers notes by the instant they were added.
const LACKING: i64 = 1;
/// The note time of the first package of a note, in seconds.
const FIRST_PACKAGE: i64 = 1_700_000_000;
/// The note time of the package that adds the marker: newer than the first.
const MARKING_PACKAGE: i64 = 1_700_000_060;
/// The note time of the package that takes the marker away: newer than the marking one.
const UNMARKING_PACKAGE: i64 = 1_700_000_120;

/// A collection built for one test with the engine's own API: one Basic note per tag list, in the
/// default deck, closed. Returns the file and each note's id, in the order of `tags`.
fn collection(dir: &Path, tags: &[&[&str]]) -> (PathBuf, Vec<i64>) {
    let path = dir.join("collection.anki2");
    let mut col = CollectionBuilder::new(&path)
        .build()
        .expect("the engine creates the collection");
    let basic = col
        .get_notetype_by_name("Basic")
        .expect("the note types are read")
        .expect("the engine creates its stock Basic note type");
    let mut notes = Vec::new();
    for (index, tags) in tags.iter().enumerate() {
        let mut note = basic.new_note();
        note.set_field(0, format!("synthetic front {index}"))
            .expect("the front is set");
        note.set_field(1, format!("synthetic back {index}"))
            .expect("the back is set");
        note.tags = tags.iter().map(|tag| (*tag).to_owned()).collect();
        col.add_note(&mut note, DeckId(1))
            .expect("the engine adds the note to the default deck");
        notes.push(note.id.0);
    }
    col.close(None).expect("the engine closes the collection");
    (path, notes)
}

/// The test's own engine handle on a closed collection file.
fn engine(path: &Path) -> Collection {
    CollectionBuilder::new(path)
        .build()
        .expect("the engine opens the test's collection")
}

/// The oracle: `note`'s tags as the engine's own note read answers them on the file, sorted, or
/// `None` when the collection lacks the note.
fn engine_tags(path: &Path, note: i64) -> Option<Vec<String>> {
    let col = engine(path);
    let tags = col
        .storage
        .get_note(NoteId(note))
        .expect("the engine reads the note")
        .map(|note| {
            let mut tags = note.tags;
            tags.sort();
            tags
        });
    col.close(None).expect("the engine closes the collection");
    tags
}

/// The oracle for an imported note: the id the engine gave the note with `guid` in the file, and
/// its tags as the engine's own note read answers them, sorted.
fn engine_note(path: &Path, guid: &str) -> (i64, Vec<String>) {
    let col = engine(path);
    let id: i64 = col
        .storage
        .db()
        .query_row("select id from notes where guid = ?", [guid], |row| {
            row.get(0)
        })
        .expect("the file holds the packaged note");
    let mut tags = col
        .storage
        .get_note(NoteId(id))
        .expect("the engine reads the note")
        .expect("the note exists")
        .tags;
    tags.sort();
    col.close(None).expect("the engine closes the collection");
    (id, tags)
}

/// The core's answer for each of `notes`, through a web dispatcher with the file open, closed after.
fn core_reads(path: &Path, notes: &[i64]) -> Vec<Result<bool, Refusal>> {
    let dispatcher =
        Dispatcher::start(Transport::Web, &[]).expect("the engine starts from the default init");
    let open = OpenCollectionRequest {
        collection_path: path.to_str().expect("a scratch path is UTF-8").to_owned(),
        ..OpenCollectionRequest::default()
    };
    dispatcher
        .run(OPEN_COLLECTION.0, OPEN_COLLECTION.1, &open.encode_to_vec())
        .expect("the web dispatcher opens the collection");
    let answers = notes
        .iter()
        .map(|note| dispatcher.ai_assisted(*note))
        .collect();
    dispatcher
        .run(
            CLOSE_COLLECTION.0,
            CLOSE_COLLECTION.1,
            &CloseCollectionRequest::default().encode_to_vec(),
        )
        .expect("the web dispatcher closes the collection");
    answers
}

/// Owned tag lists, for comparing with the oracle's.
fn owned(lists: &[&[&str]]) -> Vec<Vec<String>> {
    lists
        .iter()
        .map(|tags| tags.iter().map(|tag| (*tag).to_owned()).collect())
        .collect()
}

#[test]
fn the_marker_is_the_tag_ai_assisted() {
    assert_eq!(
        MARKER, "ai-assisted",
        "the marker is the literal tag ai-assisted"
    );
    assert_eq!(
        [carries("ai-assisted"), carries(" ai-assisted ")],
        [true, true],
        "a note whose only tag is ai-assisted is marked, bare or as the engine stores its tags"
    );
}

#[test]
fn a_note_among_other_tags_is_marked() {
    assert_eq!(
        [
            carries(" ai-assisted grammar vocab "),
            carries(" grammar ai-assisted vocab "),
            carries(" grammar vocab ai-assisted "),
        ],
        [true, true, true],
        "ai-assisted marks a note at the start, in the middle and at the end of its tags"
    );
}

#[test]
fn the_marker_is_read_in_any_case() {
    assert_eq!(
        [
            carries(" AI-Assisted "),
            carries(" AI-ASSISTED "),
            carries(" grammar Ai-Assisted "),
        ],
        [true, true, true],
        "the marker is read without regard to ASCII case, as the engine compares tags"
    );
}

#[test]
fn only_the_whole_tag_marks_a_note() {
    assert_eq!(
        [
            carries(""),
            carries(" "),
            carries(" pre-ai-assisted "),
            carries(" ai-assisted-review "),
            carries(" ai-assisted::draft "),
            carries(" ai "),
            carries(" assisted "),
            carries(" ai assisted "),
        ],
        [false; 8],
        "no tags, a longer tag, a child tag or a part of the tag leaves a note unmarked"
    );
}

#[test]
fn the_core_reads_the_engines_own_tags() {
    let dir = support::scratch("engine-core-ai-assisted", "core-reads-tags");
    let (path, notes) = collection(&dir, &[&["ai-assisted"], &["vocab"]]);

    let oracle: Vec<Option<Vec<String>>> =
        notes.iter().map(|note| engine_tags(&path, *note)).collect();
    let answers = core_reads(&path, &notes);

    assert_eq!(
        oracle,
        owned(&[&["ai-assisted"], &["vocab"]])
            .into_iter()
            .map(Some)
            .collect::<Vec<_>>(),
        "the engine's own read of the file holds the marked note's tag and the other note's"
    );
    assert_eq!(
        answers,
        vec![Ok(true), Ok(false)],
        "the core's answer for the marked note and for the note tagged vocab is the engine's \
         reading of each note's tags"
    );
}

#[test]
fn a_note_the_collection_lacks_reads_unmarked() {
    let dir = support::scratch("engine-core-ai-assisted", "lacking-note");
    let (path, _) = collection(&dir, &[&["ai-assisted"]]);

    let oracle = engine_tags(&path, LACKING);
    let answers = core_reads(&path, &[LACKING]);

    assert_eq!(
        oracle, None,
        "the engine's own read finds no note {LACKING}"
    );
    assert_eq!(
        answers,
        vec![Ok(false)],
        "a note the collection lacks reads unmarked, with no refusal"
    );
}

/// Exports `source` whole, notes only, by the engine's own package export, and returns how many
/// notes the package holds.
fn export(source: &Path, package: &Path) -> usize {
    let mut col = engine(source);
    let exported = col
        .export_apkg(package, ExportAnkiPackageOptions::default(), "", None)
        .expect("the engine exports the package");
    col.close(None).expect("the engine closes the collection");
    exported
}

/// Imports `package` into `target` by the engine's own package import, with its default options:
/// a packaged note updates the collection's note only when the package's is newer.
fn import(target: &Path, package: &Path) {
    let mut col = CollectionBuilder::new(target)
        .with_desktop_media_paths()
        .build()
        .expect("the engine opens the target collection");
    col.import_apkg(package, ImportAnkiPackageOptions::default())
        .expect("the engine imports the package");
    col.close(None).expect("the engine closes the collection");
}

/// Writes `tags` on `note` through the engine's own note update, when they differ from its own.
fn retag(path: &Path, note: i64, tags: &[&str]) {
    let mut col = engine(path);
    let mut edited = col
        .storage
        .get_note(NoteId(note))
        .expect("the engine reads the note")
        .expect("the note exists");
    if edited
        .tags
        .iter()
        .map(String::as_str)
        .ne(tags.iter().copied())
    {
        edited.tags = tags.iter().map(|tag| (*tag).to_owned()).collect();
        col.update_note(&mut edited)
            .expect("the engine updates the note's tags");
    }
    col.close(None).expect("the engine closes the collection");
}

/// Gives the source's `note` a package's fixed note time, in seconds, as the package builder
/// writes the decision's time rather than the clock's.
fn stamp(source: &Path, note: i64, time: i64) {
    let col = engine(source);
    col.storage
        .db()
        .execute("update notes set mod = ?1 where id = ?2", [time, note])
        .expect("the package's note time is written");
    col.close(None).expect("the engine closes the collection");
}

/// A test's directory with its `parts` made under it, each empty.
fn directories(test: &str, parts: &[&str]) -> PathBuf {
    let dir = support::scratch("engine-core-ai-assisted", test);
    for part in parts {
        std::fs::create_dir_all(dir.join(part)).expect("a directory of the test's");
    }
    dir
}

/// The source's note `note`'s guid, read by the engine.
fn guid(source: &Path, note: i64) -> String {
    let col = engine(source);
    let guid = col
        .storage
        .get_note(NoteId(note))
        .expect("the engine reads the note")
        .expect("the note exists")
        .guid;
    col.close(None).expect("the engine closes the collection");
    guid
}

#[test]
fn the_marker_survives_the_engines_package_export_and_import() {
    let dir = directories("export-import", &["source", "target"]);
    let (source, notes) = collection(&dir.join("source"), &[&["ai-assisted"], &["vocab"]]);
    for note in &notes {
        stamp(&source, *note, FIRST_PACKAGE);
    }
    let guids: Vec<String> = notes.iter().map(|note| guid(&source, *note)).collect();
    let package = dir.join("cards.apkg");
    let target = dir.join("target").join("collection.anki2");

    assert_eq!(export(&source, &package), 2, "the package holds both notes");
    import(&target, &package);
    let (ids, oracle): (Vec<i64>, Vec<Vec<String>>) =
        guids.iter().map(|guid| engine_note(&target, guid)).unzip();
    let answers = core_reads(&target, &ids);

    assert_eq!(
        oracle,
        owned(&[&["ai-assisted"], &["vocab"]]),
        "the engine's own read of the fresh collection holds each imported note's tags as packaged"
    );
    assert_eq!(
        answers,
        vec![Ok(true), Ok(false)],
        "the core reads the imported marked note marked and the other unmarked"
    );
}

#[test]
fn a_newer_package_moves_the_marker_both_ways() {
    let dir = directories("newer-package", &["source", "target"]);
    let (source, notes) = collection(&dir.join("source"), &[&["vocab"]]);
    let [note] = notes[..] else {
        panic!("the source holds one note, not {notes:?}");
    };
    let guid = guid(&source, note);
    let target = dir.join("target").join("collection.anki2");
    let packages: [(&[&str], i64, &str); 3] = [
        (&["vocab"], FIRST_PACKAGE, "first.apkg"),
        (&["ai-assisted", "vocab"], MARKING_PACKAGE, "marking.apkg"),
        (&["vocab"], UNMARKING_PACKAGE, "unmarking.apkg"),
    ];

    let mut oracle = Vec::new();
    let mut answers = Vec::new();
    for (tags, time, name) in packages {
        retag(&source, note, tags);
        stamp(&source, note, time);
        let package = dir.join(name);
        assert_eq!(export(&source, &package), 1, "{name} holds the one note");
        import(&target, &package);
        let (id, tags) = engine_note(&target, &guid);
        oracle.push(tags);
        answers.extend(core_reads(&target, &[id]));
    }

    assert_eq!(
        oracle,
        owned(&[&["vocab"], &["ai-assisted", "vocab"], &["vocab"]]),
        "the engine's own read holds the note's tags as each newer package wrote them: the \
         marking package adds the marker and the newer unmarked one takes it away"
    );
    assert_eq!(
        answers,
        vec![Ok(false), Ok(true), Ok(false)],
        "the core reads the note unmarked, then marked after the marking package, then unmarked \
         after the newer package without the marker"
    );
}

/// The synthetic account logged in at `server` by the engine's own login, called directly.
fn logged_in(server: &SyncServer) -> EngineAuth {
    let endpoint = server.endpoint();
    let login = sync_server::runtime()
        .block_on(sync_login(
            USERNAME,
            PASSWORD,
            Some(endpoint.to_owned()),
            engine_client(),
        ))
        .unwrap_or_else(|error| panic!("the engine's own login: {error}"));
    EngineAuth {
        hkey: login.hkey,
        endpoint: Some(endpoint.parse().expect("the server's endpoint is a URL")),
        io_timeout_secs: None,
    }
}

/// A fresh HTTP client of the engine's own type, built by its `Default`, as the one-way tests
/// build one.
fn engine_client<Client: Default>() -> Client {
    Client::default()
}

/// The server's collection replaced by `path`'s, by the engine's own full upload.
fn full_upload(auth: &EngineAuth, path: &Path) {
    sync_server::runtime()
        .block_on(engine(path).full_upload(auth.clone(), engine_client()))
        .unwrap_or_else(|error| panic!("the engine's own upload: {error}"));
}

/// A new collection at `path` holding the server's, by the engine's own full download.
fn full_download(auth: &EngineAuth, path: &Path) {
    sync_server::runtime()
        .block_on(engine(path).full_download(auth.clone(), engine_client()))
        .unwrap_or_else(|error| panic!("the engine's own download: {error}"));
}

/// The engine's own normal sync of `path`, which needs no full sync.
fn normal_sync(auth: &EngineAuth, path: &Path) {
    let mut col = engine(path);
    let output = sync_server::runtime()
        .block_on(col.normal_sync(auth.clone(), engine_client()))
        .unwrap_or_else(|error| panic!("the engine's own normal sync: {error}"));
    col.close(None).expect("the engine closes the collection");
    let response: SyncCollectionResponse = output.into();
    assert_eq!(
        response.required(),
        ChangesRequired::NoChanges,
        "the device's changes travel by a normal sync"
    );
}

#[test]
fn the_marker_survives_full_and_normal_sync() {
    if sync_server::role().as_deref() == Some(SERVER) {
        return sync_server::serve();
    }
    let server = SyncServer::start("the_marker_survives_full_and_normal_sync");
    let auth = logged_in(&server);
    let dir = directories("full-and-normal-sync", &["first", "second"]);
    let (first, notes) = collection(&dir.join("first"), &[&["ai-assisted"], &["vocab"]]);
    let [marked, later] = notes[..] else {
        panic!("the first device holds two notes, not {notes:?}");
    };
    let second = dir.join("second").join("collection.anki2");

    full_upload(&auth, &first);
    full_download(&auth, &second);
    let mut oracle = vec![engine_tags(&second, marked), engine_tags(&second, later)];
    let mut answers = core_reads(&second, &[marked, later]);
    retag(&first, later, &["ai-assisted", "vocab"]);
    normal_sync(&auth, &first);
    normal_sync(&auth, &second);
    oracle.push(engine_tags(&second, later));
    answers.extend(core_reads(&second, &[later]));

    assert_eq!(
        oracle,
        owned(&[&["ai-assisted"], &["vocab"], &["ai-assisted", "vocab"]])
            .into_iter()
            .map(Some)
            .collect::<Vec<_>>(),
        "the engine's own read of the second device holds the marked note's tag after the full \
         download, and the marker added on the first device after the normal syncs"
    );
    assert_eq!(
        answers,
        vec![Ok(true), Ok(false), Ok(true)],
        "the core reads the marked note marked on the second device, and the note marked later \
         unmarked before the normal syncs and marked after them"
    );
}
