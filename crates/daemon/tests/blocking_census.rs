//! The blocking census (SPEC-436 R5 to R12 and R22, ADR-450 D2): a blocking call inside an async
//! body of a shipped source is a site, keyed by its file, its function, its resolved path and its
//! ordinal. Today's sites are listed, and the census starts at `Level::Warn` (ADR-450 D5).

mod shipped_sources;

use proc_macro2::TokenTree;
use shipped_sources::{Key, Level, keyed, planted, repository_sources, shipped, verdict};

/// The rule this census names in its lines.
const RULE: &str = "blocking-in-async";

/// The census's level (R22). Moving it to `Fail` is its own delivery (R24).
const LEVEL: Level = Level::Warn;

/// Today's sites, SPEC-436 section 1, in its order (R22). Their cures are tracked apart.
const LISTED: &[&str] = &[
    "crates/agent/src/gate.rs check std::fs::remove_file #1",
    "crates/agent/src/gate.rs check std::fs::remove_file #2",
    "crates/agent/src/gate.rs check_input std::fs::remove_file #1",
    "crates/agent/src/runner.rs run std::fs::remove_file #1",
    "crates/ingest/src/skip_write.rs steps std::fs::copy #1",
    "crates/ingest/src/skip_write.rs backup std::fs::copy #1",
    "crates/ingest/src/skip_write.rs backup std::fs::set_permissions #1",
    "crates/ingest/src/skip_write.rs backup std::fs::rename #1",
    "crates/ingest/src/skip_write.rs restore_check std::fs::read #1",
    "crates/ingest/src/skip_write.rs restore_check std::fs::read #2",
    "crates/ingest/src/skip_write.rs restore_check std::fs::write #1",
];

/// What the census found: the keys, the files it examined and the async bodies it walked.
#[derive(Default)]
struct Found {
    keys: Vec<Key>,
    files: Vec<String>,
    bodies: usize,
}

/// One file's walk: the sites it met, each `(function, form)`, and the async bodies it entered.
#[derive(Default)]
struct Census {
    sites: Vec<(String, String)>,
    bodies: usize,
}

/// The census over a list of (path, text) (R19).
fn find(files: &[(String, String)]) -> Result<Found, String> {
    let sources = shipped(files)?;
    let mut found = Found::default();
    for source in &sources {
        let mut census = Census::default();
        walk(&source.tokens, "", false, &mut census);
        found.keys.extend(keyed(&source.path, &census.sites));
        found.bodies += census.bodies;
        found.files.push(source.path.clone());
    }
    eprintln!("examined {} shipped files", found.files.len());
    eprintln!("examined {} async bodies", found.bodies);
    Ok(found)
}

/// Walks one token list.
fn walk(_tokens: &[TokenTree], _function: &str, _in_async: bool, _census: &mut Census) {}

/// The found keys, as they print.
fn printed(found: &Found) -> Vec<String> {
    found.keys.iter().map(ToString::to_string).collect()
}

/// An async fn calling every member of the blocking set by its full path (A5).
const QUALIFIED: &str = r#"use std::sync::mpsc;

async fn qualified(url: &str) {
    std::fs::read("a");
    std::fs::read_to_string("a");
    std::fs::read_dir("a");
    std::fs::write("a", "b");
    std::fs::copy("a", "b");
    std::fs::rename("a", "b");
    std::fs::remove_file("a");
    std::fs::remove_dir("a");
    std::fs::remove_dir_all("a");
    std::fs::create_dir("a");
    std::fs::create_dir_all("a");
    std::fs::metadata("a");
    std::fs::symlink_metadata("a");
    std::fs::set_permissions("a", mode);
    std::fs::canonicalize("a");
    std::fs::File::open("a");
    std::fs::File::create("a");
    std::fs::File::create_new("a");
    std::fs::OpenOptions::new();
    std::thread::sleep(pause);
    std::net::TcpStream::connect(url);
    std::net::TcpListener::bind(url);
    std::net::UdpSocket::bind(url);
    std::process::Command::new("a");
    std::io::stdin();
    reqwest::blocking::get(url);
    futures::executor::block_on(work);
    runtime.block_on(work);
    receiver.recv();
}
"#;

/// An async fn calling members through each `use` form (A5).
const IMPORTED: &str = r#"use std::fs::read;
use std::fs::{copy, File};
use std::fs::{self};
use std::thread::sleep as nap;

async fn imported() {
    read("a");
    copy("a", "b");
    File::open("a");
    fs::write("a", "b");
    nap(pause);
    read("b");
}
"#;

/// A5's keys, spelt literally.
const FOUND_IN_FN: [&str; 35] = [
    "crates/planted/src/qualified.rs qualified std::fs::read #1",
    "crates/planted/src/qualified.rs qualified std::fs::read_to_string #1",
    "crates/planted/src/qualified.rs qualified std::fs::read_dir #1",
    "crates/planted/src/qualified.rs qualified std::fs::write #1",
    "crates/planted/src/qualified.rs qualified std::fs::copy #1",
    "crates/planted/src/qualified.rs qualified std::fs::rename #1",
    "crates/planted/src/qualified.rs qualified std::fs::remove_file #1",
    "crates/planted/src/qualified.rs qualified std::fs::remove_dir #1",
    "crates/planted/src/qualified.rs qualified std::fs::remove_dir_all #1",
    "crates/planted/src/qualified.rs qualified std::fs::create_dir #1",
    "crates/planted/src/qualified.rs qualified std::fs::create_dir_all #1",
    "crates/planted/src/qualified.rs qualified std::fs::metadata #1",
    "crates/planted/src/qualified.rs qualified std::fs::symlink_metadata #1",
    "crates/planted/src/qualified.rs qualified std::fs::set_permissions #1",
    "crates/planted/src/qualified.rs qualified std::fs::canonicalize #1",
    "crates/planted/src/qualified.rs qualified std::fs::File::open #1",
    "crates/planted/src/qualified.rs qualified std::fs::File::create #1",
    "crates/planted/src/qualified.rs qualified std::fs::File::create_new #1",
    "crates/planted/src/qualified.rs qualified std::fs::OpenOptions::new #1",
    "crates/planted/src/qualified.rs qualified std::thread::sleep #1",
    "crates/planted/src/qualified.rs qualified std::net::TcpStream::connect #1",
    "crates/planted/src/qualified.rs qualified std::net::TcpListener::bind #1",
    "crates/planted/src/qualified.rs qualified std::net::UdpSocket::bind #1",
    "crates/planted/src/qualified.rs qualified std::process::Command::new #1",
    "crates/planted/src/qualified.rs qualified std::io::stdin #1",
    "crates/planted/src/qualified.rs qualified reqwest::blocking::get #1",
    "crates/planted/src/qualified.rs qualified futures::executor::block_on #1",
    "crates/planted/src/qualified.rs qualified .block_on #1",
    "crates/planted/src/qualified.rs qualified .recv #1",
    "crates/planted/src/imported.rs imported std::fs::read #1",
    "crates/planted/src/imported.rs imported std::fs::copy #1",
    "crates/planted/src/imported.rs imported std::fs::File::open #1",
    "crates/planted/src/imported.rs imported std::fs::write #1",
    "crates/planted/src/imported.rs imported std::thread::sleep #1",
    "crates/planted/src/imported.rs imported std::fs::read #2",
];

#[test]
fn every_blocking_call_is_found_in_an_async_fn_body_however_it_is_named() {
    let files = planted(&[
        ("crates/planted/src/qualified.rs", QUALIFIED),
        ("crates/planted/src/imported.rs", IMPORTED),
    ]);
    let found = find(&files).expect("the census reads the planted files");
    assert_eq!(printed(&found), FOUND_IN_FN);
    assert_eq!(
        found.files,
        [
            "crates/planted/src/qualified.rs",
            "crates/planted/src/imported.rs"
        ]
    );
    assert_eq!(found.bodies, 2);
}

#[test]
fn an_async_block_and_an_async_closure_are_async_bodies() {
    let files = planted(&[(
        "crates/planted/src/blocks.rs",
        r#"fn plain_block() -> impl Future<Output = ()> {
    async {
        std::fs::read("a");
    }
}

fn moved_block(path: String) {
    let job = async move {
        std::fs::write(path, "b");
    };
}

fn boxed(path: String) -> BoxFuture<'static, ()> {
    Box::pin(async move {
        std::fs::remove_file(path);
    })
}

fn closures() {
    let first = async |path| {
        std::fs::copy(path, "b");
    };
    let second = async move |path| std::fs::rename(path, "b");
    std::fs::metadata("a");
}
"#,
    )]);
    let found = find(&files).expect("the census reads the planted file");
    assert_eq!(
        printed(&found),
        [
            "crates/planted/src/blocks.rs plain_block std::fs::read #1",
            "crates/planted/src/blocks.rs moved_block std::fs::write #1",
            "crates/planted/src/blocks.rs boxed std::fs::remove_file #1",
            "crates/planted/src/blocks.rs closures std::fs::copy #1",
            "crates/planted/src/blocks.rs closures std::fs::rename #1",
        ]
    );
    assert_eq!(found.bodies, 5);
}

#[test]
fn a_rail_a_nested_fn_test_code_and_the_async_equivalents_are_not_blocking_sites() {
    let files = planted(&[
        (
            "crates/planted/src/quiet.rs",
            r#"use tokio::fs::read;
use tokio::time::sleep;
use tokio::process::Command;

async fn quiet(path: String) {
    tokio::task::spawn_blocking(move || std::fs::read(path));
    tokio::task::block_in_place(|| {
        let nested = async { std::fs::write("a", "b"); };
        std::fs::copy("a", "b");
    });
    self.offload.run(move || std::fs::rename("a", "b"));
    fn helper() {
        std::fs::remove_file("a");
    }
    read("a").await;
    sleep(pause).await;
    Command::new("a");
    let handed = std::fs::read;
    std::fs::Permissions::from_mode(0o600);
    // std::fs::remove_dir("a");
    let text = "std::fs::create_dir(a)";
    std::fs::metadata("a");
}

fn synchronous() {
    std::fs::canonicalize("a");
}

const ITEM: Job = Job::new(async { std::fs::read_dir("a") });

#[cfg(test)]
mod tests {
    async fn inline() {
        std::fs::read("a");
    }
}

#[test]
fn marked() {
    let job = async { std::fs::read("a"); };
}

#[cfg(test)]
mod outside;

#[cfg(test)]
#[path = "elsewhere_tests.rs"]
mod named;
"#,
        ),
        (
            "crates/planted/src/quiet/outside.rs",
            "async fn outside() { std::fs::read(\"a\"); }\n",
        ),
        (
            "crates/planted/src/elsewhere_tests.rs",
            "async fn named() { std::fs::read(\"a\"); }\n",
        ),
    ]);
    let found = find(&files).expect("the census reads the planted files");
    assert_eq!(
        printed(&found),
        ["crates/planted/src/quiet.rs quiet std::fs::metadata #1"]
    );
    assert_eq!(found.files, ["crates/planted/src/quiet.rs"]);
}

#[test]
fn the_repository_lists_exactly_todays_blocking_sites() {
    let files = repository_sources().expect("the tree's sources read");
    let found = find(&files).expect("the census reads the tree");
    assert_eq!(printed(&found), LISTED);
    assert!(
        found
            .files
            .iter()
            .any(|file| file == "crates/coordination/src/recompute/mod.rs")
    );
    assert!(
        !found
            .files
            .iter()
            .any(|file| file == "crates/ingest/src/skip_write/tests.rs")
    );
    assert!(
        !found
            .files
            .iter()
            .any(|file| file == "crates/vault/src/unicode_other_tests.rs")
    );
    assert_eq!(LEVEL, Level::Warn);
    assert!(!found.files.is_empty());
    assert!(found.bodies > 0);
    assert_eq!(verdict(RULE, LEVEL, &found.keys, LISTED), Ok(Vec::new()));
}

#[test]
fn the_level_decides_whether_an_unlisted_site_fails_and_a_stale_entry_always_does() {
    let site = Key {
        file: "crates/planted/src/level.rs".to_owned(),
        function: "level".to_owned(),
        form: "std::fs::read".to_owned(),
        ordinal: 1,
    };
    let found = [site];
    assert_eq!(
        verdict(RULE, Level::Warn, &found, &[]),
        Ok(vec![
            "WARN blocking-in-async crates/planted/src/level.rs level std::fs::read #1".to_owned()
        ])
    );
    assert_eq!(
        verdict(RULE, Level::Fail, &found, &[]),
        Err(vec![
            "FAIL blocking-in-async crates/planted/src/level.rs level std::fs::read #1".to_owned()
        ])
    );
    let listed = [
        "crates/planted/src/level.rs level std::fs::read #1",
        "crates/planted/src/gone.rs gone std::fs::read #1",
    ];
    let stale =
        vec!["STALE blocking-in-async crates/planted/src/gone.rs gone std::fs::read #1".to_owned()];
    assert_eq!(
        verdict(RULE, Level::Warn, &found, &listed),
        Err(stale.clone())
    );
    assert_eq!(verdict(RULE, Level::Fail, &found, &listed), Err(stale));
    assert_eq!(
        verdict(RULE, Level::Fail, &found, &listed[..1]),
        Ok(Vec::new())
    );
}

#[test]
fn the_blocking_census_refuses_an_empty_population() {
    assert_eq!(find(&[]).err(), Some("examined 0 shipped files".to_owned()));
    let files = planted(&[(
        "crates/planted/src/sync.rs",
        "fn sync() { std::fs::read(\"a\"); }\n",
    )]);
    assert_eq!(
        find(&files).err(),
        Some("examined 0 async bodies".to_owned())
    );
}
