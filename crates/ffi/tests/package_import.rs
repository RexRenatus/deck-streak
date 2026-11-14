//! The deck package import through the native adapter (SPEC-681 A2 to A5, ADR-703). A valid
//! package's notes and cards enter the open collection through one allow-listed call, the import
//! pair is admitted on the Native column alone, and a package the adapter refuses, a file that is
//! no package or one whose inner compressed member decompresses past the bound, reads as a worded
//! error and writes no row. Each package is planted at run time: the valid one is the engine's own
//! export of a synthetic collection, its card's review scheduling kept, and each oversize member is
//! a zstd frame of run-length blocks, with no content size declared, that the test writes itself.
//! Every count is read from the collection file with the engine's own API after the adapter closed
//! it.

// A failed setup step fails the test, as clippy.toml allows inside a test function.
#![allow(clippy::expect_used)]

#[expect(
    dead_code,
    reason = "the import tests open the synthetic collection by its path and use some wire helpers"
)]
mod support;

use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anki::collection::CollectionBuilder;
use anki::import_export::package::ExportAnkiPackageOptions;
use deck_streak_engine_core::table::{self, Decision, Transport};
use deck_streak_ffi::allow_list;
use deck_streak_ffi::engine::{Engine, EngineRefusal};
use support::{Synthetic, open_request, synthetic, wire};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

/// `CollectionService.OpenCollection`, which every test opens its target with.
const OPEN_COLLECTION: (u32, u32) = (3, 0);
/// `ImportExportService.ImportAnkiPackage`, the one call the import makes (SPEC-681 R2).
const IMPORT: (u32, u32) = (39, 2);
/// The import pair's name in both tables.
const IMPORT_NAME: &str = "ImportExportService.ImportAnkiPackage";
/// One byte past the bound every zip entry and inner compressed member is held to (SPEC-681 R4).
const PAST_BOUND: u64 = (1 << 30) + 1;
/// The largest block a zstd frame holds, and the window the planted frame declares.
const BLOCK: u64 = 1 << 17;
/// A zstd block header's type field for a run-length block.
const RUN_LENGTH: u32 = 1 << 1;
/// The words a package with a member past the bound reads as (SPEC-681 R5).
const PAST_BOUND_WORDS: &str =
    "This package holds a file larger than the app imports (1 GiB), so nothing was imported.";
/// The words a file that is no package reads as (SPEC-681 R5).
const UNREADABLE_WORDS: &str =
    "This file is not an Anki deck package the app can read, so nothing was imported.";
/// The review interval, in days, the source card carries into the package.
const INTERVAL: i64 = 9;
/// A zstd frame whose header descriptor sets the reserved bit, so the decoder refuses it as it
/// reads, a few bytes into the member and far inside the bound.
const CORRUPT_FRAME: [u8; 8] = [0x28, 0xb5, 0x2f, 0xfd, 0x08, 0x00, 0x00, 0x00];
/// The field number the planted requests give the one field the import does not read.
const UNREAD_FIELD: u64 = 3;

/// The adapter, started from the default init message.
fn engine() -> Arc<Engine> {
    Engine::new(Vec::new()).expect("the engine starts from the default init message")
}

/// `ImportAnkiPackageRequest` with the package's path alone, as the app sends it: the adapter
/// writes the options.
fn import_request(package: &Path) -> Vec<u8> {
    let mut out = Vec::new();
    let path = package.to_str().expect("a scratch path is UTF-8");
    wire::put_bytes(&mut out, 1, path.as_bytes());
    out
}

/// Opens the target through the adapter, sends the one import call, closes the target by
/// dropping the engine, and returns what the import call answered.
fn import(target: &Synthetic, package: &Path) -> Result<Vec<u8>, EngineRefusal> {
    import_bytes(target, import_request(package))
}

/// Opens the target through the adapter, sends the import call with `request` as its bytes,
/// closes the target by dropping the engine, and returns what the import call answered.
fn import_bytes(target: &Synthetic, request: Vec<u8>) -> Result<Vec<u8>, EngineRefusal> {
    let engine = engine();
    engine
        .run(OPEN_COLLECTION.0, OPEN_COLLECTION.1, open_request(target))
        .expect("the adapter opens the target collection");
    let imported = engine.run(IMPORT.0, IMPORT.1, request);
    drop(engine);
    imported
}

/// A request whose package path follows one field the import does not read, written as `field`
/// gives it: the reader skips that field by its wire type and still finds the path.
fn request_after(field: &[u8], package: &Path) -> Vec<u8> {
    let mut request = field.to_vec();
    request.extend_from_slice(&import_request(package));
    request
}

/// One count the collection file answers, read with the engine's own API.
fn count(collection: &Path, sql: &str) -> i64 {
    let col = CollectionBuilder::new(collection)
        .build()
        .expect("the engine opens the collection");
    let counted: i64 = col
        .storage
        .db()
        .query_row(sql, [], |row| row.get(0))
        .expect("the engine counts the rows");
    col.close(None).expect("the engine closes the collection");
    counted
}

/// The notes and the cards the collection holds.
fn rows(collection: &Path) -> (i64, i64) {
    (
        count(collection, "select count() from notes"),
        count(collection, "select count() from cards"),
    )
}

/// The cards the collection holds in review with the source card's interval.
fn reviewing(collection: &Path) -> i64 {
    count(
        collection,
        &format!("select count() from cards where type = 2 and queue = 2 and ivl = {INTERVAL}"),
    )
}

/// Each note type the collection holds, by id and modification time.
fn note_types(collection: &Path) -> Vec<(i64, i64)> {
    let col = CollectionBuilder::new(collection)
        .build()
        .expect("the engine opens the collection");
    let held = {
        let db = col.storage.db();
        let mut statement = db
            .prepare("select id, mtime_secs from notetypes order by id")
            .expect("the engine reads the note types");
        statement
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .expect("the engine lists the note types")
            .collect::<Result<Vec<(i64, i64)>, _>>()
            .expect("each note type row reads")
    };
    col.close(None).expect("the engine closes the collection");
    held
}

/// The words a refusal carries: the engine refusal's message, or what was answered instead.
fn words(result: &Result<Vec<u8>, EngineRefusal>) -> String {
    match result {
        Err(EngineRefusal::Engine { error }) => {
            String::from_utf8_lossy(&wire::bytes(error, 1)).into_owned()
        }
        other => format!("not a worded refusal: {other:?}"),
    }
}

/// The engine's own export of a synthetic collection whose one card is in review: the valid
/// package every refusal stands beside.
fn valid_package(test: &str) -> PathBuf {
    let source = synthetic(&format!("{test}-source"));
    let package = source.dir.join("valid.apkg");
    let mut col = CollectionBuilder::new(source.collection.as_path())
        .build()
        .expect("the engine opens the source collection");
    let review = format!(
        "update cards set type = 2, queue = 2, ivl = {INTERVAL}, due = 1 where id = {}",
        source.card_id
    );
    col.storage
        .db()
        .execute(&review, [])
        .expect("the source card takes review scheduling");
    col.export_apkg(
        &package,
        ExportAnkiPackageOptions {
            with_scheduling: true,
            with_deck_configs: true,
            with_media: false,
            legacy: false,
        },
        "",
        None,
    )
    .expect("the engine exports the source collection");
    col.close(None)
        .expect("the engine closes the source collection");
    package
}

/// Every entry of a package, by name and decompressed bytes, in its order.
fn entries(package: &Path) -> Vec<(String, Vec<u8>)> {
    let mut archive = ZipArchive::new(File::open(package).expect("the package opens"))
        .expect("the package reads as a zip archive");
    (0..archive.len())
        .map(|index| {
            let mut entry = archive.by_index(index).expect("the entry opens");
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes).expect("the entry reads");
            (
                entry.name().expect("the entry name reads").into_owned(),
                bytes,
            )
        })
        .collect()
}

/// A package's entries with one entry's bytes replaced, its name and place kept.
fn replaced(package: &Path, name: &str, bytes: &[u8]) -> Vec<(String, Vec<u8>)> {
    entries(package)
        .into_iter()
        .map(|(entry, held)| {
            if entry == name {
                (entry, bytes.to_vec())
            } else {
                (entry, held)
            }
        })
        .collect()
}

/// Writes a package whose entries are stored as given, so each member's bytes are the test's own.
fn plant(package: &Path, members: &[(String, Vec<u8>)]) {
    let mut zip = ZipWriter::new(File::create(package).expect("the planted package is created"));
    for (name, bytes) in members {
        zip.start_file(
            name.as_str(),
            SimpleFileOptions::default().compression_method(CompressionMethod::Stored),
        )
        .expect("the package takes the entry");
        zip.write_all(bytes).expect("the entry takes its bytes");
    }
    zip.finish().expect("the planted package is closed");
}

/// A zstd frame of run-length blocks that decompresses to `size` zero bytes. Its header declares
/// no content size and no checksum, and a window of one block, so only a reader that counts the
/// bytes as they decompress sees how large the member is.
fn run_length_frame(size: u64) -> Vec<u8> {
    let mut frame = vec![0x28, 0xb5, 0x2f, 0xfd, 0x00, 0x38];
    let mut left = size;
    while left > 0 {
        let run = left.min(BLOCK);
        left -= run;
        let last = u32::from(left == 0);
        let length = u32::try_from(run).expect("a block holds at most one window");
        let header = (length << 3) | RUN_LENGTH | last;
        frame.extend_from_slice(&header.to_le_bytes()[..3]);
        frame.push(0);
    }
    frame
}

/// The media list of a current package, compressed as the engine writes it, naming one file that
/// the archive holds as its first media member.
fn media_list() -> Vec<u8> {
    let mut entry = Vec::new();
    wire::put_bytes(&mut entry, 1, b"planted.png");
    wire::put_varint_field(&mut entry, 2, 1);
    wire::put_bytes(&mut entry, 3, &[0; 20]);
    let mut list = Vec::new();
    wire::put_bytes(&mut list, 1, &entry);
    zstd::encode_all(list.as_slice(), 0).expect("zstd compresses the media list")
}

#[test]
fn a_valid_package_imports_its_notes_and_cards_in_one_call() {
    let package = valid_package("a2");
    let target = synthetic("a2-target");
    let imported = import(&target, &package);
    assert_eq!(
        rows(&target.collection),
        (2, 2),
        "the package's note and card join the target's own through the one import call, which \
         answered {:?}",
        imported.as_ref().map(Vec::len)
    );
    assert!(
        imported.is_ok(),
        "the one import call answers with the engine's response: {imported:?}"
    );
}

#[test]
fn the_import_keeps_scheduling_and_note_types_under_its_four_options() {
    let package = valid_package("options");
    let target = synthetic("options-target");
    let held = note_types(&target.collection);
    let imported = import(&target, &package);
    assert_eq!(
        reviewing(&target.collection),
        1,
        "the imported card keeps its review scheduling ({INTERVAL} days), as the adapter's options \
         keep it; the import call answered {:?}",
        imported.as_ref().map(Vec::len)
    );
    let after = note_types(&target.collection);
    assert!(
        held.iter().all(|row| after.contains(row)),
        "no note type the target held is merged or changed: before {held:?}, after {after:?}"
    );
}

#[test]
fn the_import_pair_is_admitted_on_the_native_column() {
    assert_eq!(
        allow_list::allowed(IMPORT.0, IMPORT.1).map(|call| call.name),
        Some(IMPORT_NAME),
        "the native allow-list admits service 39 method 2 as the package import"
    );
    assert_eq!(
        table::decide(Transport::Native, IMPORT.0, IMPORT.1),
        Decision::Admit,
        "the core's table admits the import pair on the Native column"
    );
    assert_eq!(
        table::decide(Transport::Web, IMPORT.0, IMPORT.1),
        Decision::NotAllowed,
        "the core's table refuses the import pair on the Web column"
    );
}

#[test]
fn an_unreadable_package_is_refused_in_words_with_no_row() {
    let target = synthetic("a4-target");
    let package = target.dir.join("unreadable.apkg");
    std::fs::write(&package, b"this file is no zip archive").expect("the planted file is written");
    let refused = import(&target, &package);
    assert_eq!(
        words(&refused),
        UNREADABLE_WORDS,
        "a file that is no package reads as the refusal's words"
    );
    assert_eq!(
        rows(&target.collection),
        (1, 1),
        "the refused file writes no note and no card"
    );
}

#[test]
fn an_oversize_collection_member_is_refused_before_any_row() {
    let target = synthetic("a5-collection-target");
    let package = target.dir.join("oversize-collection.apkg");
    let members = replaced(
        &valid_package("a5-collection"),
        "collection.anki21b",
        &run_length_frame(PAST_BOUND),
    );
    plant(&package, &members);
    let refused = import(&target, &package);
    assert_eq!(
        words(&refused),
        PAST_BOUND_WORDS,
        "a collection member that decompresses one byte past the bound reads as the bound's words"
    );
    assert_eq!(
        rows(&target.collection),
        (1, 1),
        "the refused package writes no note and no card"
    );
}

#[test]
fn an_oversize_media_member_is_refused_before_any_row() {
    let target = synthetic("a5-media-target");
    let package = target.dir.join("oversize-media.apkg");
    let mut members = replaced(&valid_package("a5-media"), "media", &media_list());
    members.push((String::from("0"), run_length_frame(PAST_BOUND)));
    plant(&package, &members);
    let refused = import(&target, &package);
    assert_eq!(
        words(&refused),
        PAST_BOUND_WORDS,
        "a media member that decompresses one byte past the bound reads as the bound's words"
    );
    assert_eq!(
        rows(&target.collection),
        (1, 1),
        "the refused package writes no note and no card"
    );
}

/// The reader skips a varint field (wire type 0) that comes before the package path.
#[test]
fn a_varint_field_before_the_path_is_skipped_and_the_package_imports() {
    let package = valid_package("varint-field");
    let target = synthetic("varint-field-target");
    let mut field = Vec::new();
    wire::put_varint_field(&mut field, UNREAD_FIELD, 1);
    let imported = import_bytes(&target, request_after(&field, &package));
    assert_eq!(
        rows(&target.collection),
        (2, 2),
        "a varint field before the path is skipped and the package imports; the import call \
         answered {:?}",
        imported.as_ref().map(Vec::len)
    );
}

/// The reader skips a 64-bit field (wire type 1) that comes before the package path.
#[test]
fn a_fixed64_field_before_the_path_is_skipped_and_the_package_imports() {
    let package = valid_package("fixed64-field");
    let target = synthetic("fixed64-field-target");
    let mut field = Vec::new();
    wire::put_varint(&mut field, (UNREAD_FIELD << 3) | 1);
    field.extend_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);
    let imported = import_bytes(&target, request_after(&field, &package));
    assert_eq!(
        rows(&target.collection),
        (2, 2),
        "a 64-bit field before the path is skipped and the package imports; the import call \
         answered {:?}",
        imported.as_ref().map(Vec::len)
    );
}

/// The reader skips a 32-bit field (wire type 5) that comes before the package path.
#[test]
fn a_fixed32_field_before_the_path_is_skipped_and_the_package_imports() {
    let package = valid_package("fixed32-field");
    let target = synthetic("fixed32-field-target");
    let mut field = Vec::new();
    wire::put_varint(&mut field, (UNREAD_FIELD << 3) | 5);
    field.extend_from_slice(&[1, 2, 3, 4]);
    let imported = import_bytes(&target, request_after(&field, &package));
    assert_eq!(
        rows(&target.collection),
        (2, 2),
        "a 32-bit field before the path is skipped and the package imports; the import call \
         answered {:?}",
        imported.as_ref().map(Vec::len)
    );
}

/// A member that fails to decode inside the bound is refused as unreadable, not as past the
/// bound: only a read that passed the bound reads as the bound's words.
#[test]
fn a_member_that_fails_inside_the_bound_reads_as_unreadable() {
    let target = synthetic("corrupt-member-target");
    let package = target.dir.join("corrupt-member.apkg");
    let members = replaced(
        &valid_package("corrupt-member"),
        "collection.anki21b",
        &CORRUPT_FRAME,
    );
    plant(&package, &members);
    let refused = import(&target, &package);
    assert_eq!(
        words(&refused),
        UNREADABLE_WORDS,
        "a collection member whose frame the decoder refuses inside the bound reads as the \
         unreadable words"
    );
    assert_eq!(
        rows(&target.collection),
        (1, 1),
        "the refused package writes no note and no card"
    );
}

/// A source-text guard: the crate root declares the package bound module once, public and
/// bare, with no attribute that hides it on the line before. Row S68114 mutates that
/// declaration, a change no behaviour test can see.
#[test]
fn the_crate_root_declares_the_package_bound_module_publicly() {
    let root = include_str!("../src/lib.rs");
    let lines: Vec<&str> = root.lines().collect();
    let at: Vec<usize> = (0..lines.len())
        .filter(|&index| lines[index] == "pub mod package_bound;")
        .collect();
    assert_eq!(
        at.len(),
        1,
        "the crate root declares `pub mod package_bound;` as one bare line"
    );
    let before = at[0].checked_sub(1).map_or("", |index| lines[index]);
    assert_ne!(
        before.trim(),
        "#[doc(hidden)]",
        "the line before `pub mod package_bound;` is no `#[doc(hidden)]` attribute"
    );
}
