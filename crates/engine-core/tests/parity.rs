//! Each adapter against its transport's column of the core's table (SPEC-345 A5, A6).
//!
//! The adapters keep their own tables, checked first (ADR-356 D3). Both are std-only, so this test
//! compiles them as they are, by path, and compares them pair by pair with the core: a pair added
//! to one side alone fails here by its name. The second test reads the adapters' sources, comments
//! stripped, for the transport each starts its dispatcher on.

mod support;

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use deck_streak_engine_core::table::{ORDINARY, Transport};

#[path = "../../ffi/src/allow_list.rs"]
#[allow(
    dead_code,
    reason = "the native adapter's table is read here; its lookup is the adapter's"
)]
mod allow_list;

#[path = "../../web-engine/src/study.rs"]
#[allow(
    dead_code,
    reason = "the web engine's study calls are read here; its rating rule is the web engine's"
)]
mod study;

/// The core's column for `transport`: each admitted pair with the engine's name for it.
fn column(transport: Transport) -> BTreeSet<(u32, u32, String)> {
    ORDINARY
        .iter()
        .filter(|row| row.admits(transport))
        .map(|row| (row.service, row.method, row.name.to_owned()))
        .collect()
}

/// `Service.MethodName` as the web engine names a call: `method_name`.
fn snake(name: &str) -> String {
    let method = name.rsplit('.').next().unwrap_or(name);
    let mut out = String::new();
    for (index, letter) in method.chars().enumerate() {
        if letter.is_ascii_uppercase() && index > 0 {
            out.push('_');
        }
        out.push(letter.to_ascii_lowercase());
    }
    out
}

/// Asserts the core's column and an adapter's table are one set, naming each side's extra pairs.
fn assert_equal<T: Ord + std::fmt::Debug>(adapter: &str, core: &BTreeSet<T>, table: &BTreeSet<T>) {
    assert_eq!(
        core,
        table,
        "{adapter}: in the adapter's table only: {:?}; in the core's column only: {:?}",
        table.difference(core).collect::<Vec<_>>(),
        core.difference(table).collect::<Vec<_>>()
    );
}

#[test]
fn each_adapter_table_equals_its_transport_column() {
    let native: BTreeSet<(u32, u32, String)> = support::examined(
        "native allow-list entry(ies)",
        allow_list::ALLOW_LIST.to_vec(),
    )
    .iter()
    .map(|call| (call.service, call.method, call.name.to_owned()))
    .collect();
    assert_equal(
        "crates/ffi/src/allow_list.rs",
        &column(Transport::Native),
        &native,
    );

    let web: BTreeSet<(u32, u32, String)> =
        support::examined("web study call(s)", study::STUDY_CALLS.to_vec())
            .iter()
            .map(|&(service, method, name)| (service, method, name.to_owned()))
            .collect();
    let core: BTreeSet<(u32, u32, String)> = column(Transport::Web)
        .into_iter()
        .map(|(service, method, name)| (service, method, snake(&name)))
        .collect();
    assert_equal("crates/web-engine/src/study.rs", &core, &web);
}

/// Every `.rs` file under `dir`, in path order.
fn sources(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut entries: Vec<PathBuf> = fs::read_dir(dir)
        .expect("a source directory reads")
        .map(|entry| entry.expect("a directory entry reads").path())
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            found.extend(sources(&path));
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            found.push(path);
        }
    }
    found
}

/// A source's text with its line and block comments removed, so a comment that names a transport
/// counts for nothing.
fn code(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    loop {
        let line = rest.find("//");
        let block = rest.find("/*");
        let line_first = match (line, block) {
            (None, None) => {
                out.push_str(rest);
                return out;
            }
            (Some(line), Some(block)) => line < block,
            (Some(_), None) => true,
            (None, Some(_)) => false,
        };
        if let (true, Some(at)) = (line_first, line) {
            out.push_str(&rest[..at]);
            rest = rest[at..].find('\n').map_or("", |end| &rest[at + end..]);
        } else if let Some(at) = block {
            out.push_str(&rest[..at]);
            rest = rest[at..]
                .find("*/")
                .map_or("", |end| &rest[at + end + 2..]);
        }
    }
}

#[test]
fn each_adapter_starts_its_dispatcher_on_its_own_transport() {
    let crates = support::workspace().join("crates");
    for (adapter, own, other, entry) in [
        (
            "ffi",
            "Transport::Native",
            "Transport::Web",
            "src/engine.rs",
        ),
        (
            "web-engine",
            "Transport::Web",
            "Transport::Native",
            "src/wasm.rs",
        ),
    ] {
        let root = crates.join(adapter);
        let files = support::examined(
            &format!("source file(s) of crates/{adapter}/src"),
            sources(&root.join("src")),
        );
        let naming = |token: &str| -> Vec<String> {
            files
                .iter()
                .filter(|path| {
                    code(&fs::read_to_string(path).expect("a source reads")).contains(token)
                })
                .map(|path| {
                    path.strip_prefix(&root)
                        .expect("a source sits under its crate")
                        .to_string_lossy()
                        .into_owned()
                })
                .collect()
        };
        assert_eq!(
            (naming(own), naming(other)),
            (vec![entry.to_owned()], Vec::<String>::new()),
            "crates/{adapter} starts its dispatcher on {own} in {entry} alone, and never names {other}"
        );
    }
}
