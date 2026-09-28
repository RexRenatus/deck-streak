//! The content rails refuse every planted fixture by its own rail row and pass a clean reading note
//! (SPEC-042 A2, R3), hold the vendored `rails.json` to the rail kinds the port reads, and refuse a
//! control character with the adapter's own rail, naming the rail and the line and never the text.
//!
//! The cases after those read a note the way the pack's probe reads it, one reading rule each: where
//! a fence opens and closes, what a code span, a comment, a tag, an attribute and each form of link
//! take, how a path is decoded and where its extension starts, and the line each rail names. Each
//! pins behaviour no planted fixture reached, so a mutant of `rails.rs` that breaks it is caught
//! (SPEC-057 R1, R2).

// An integration test is test code: its helpers panic on an unreadable fixture, and it prints the
// examined count on purpose. clippy.toml's in-test allowances cover only `#[test]` bodies.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread;
use std::time::Duration;

use deck_streak_kernel::Verdict;
use deck_streak_vault::Rails;
use deck_streak_vault::rails::{CONTROL_CHARACTER, KNOWN_KEYS, VENDORED};

/// Prints how many items a check examined and refuses zero: a fixture set that stopped matching
/// must fail, never pass over the empty set (the tdd pack's examined contract).
fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}

/// The planted fixtures' directory.
fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/rails")
}

/// The text of the fixture `name`.
fn fixture(name: &str) -> String {
    fs::read_to_string(fixtures().join(name)).expect("a readable fixture")
}

/// The fixtures' index: the rail row each planted fixture holds, and the clean note.
struct Index {
    rows: BTreeMap<String, String>,
    clean: String,
}

fn index() -> Index {
    let text = fixture("rows.json");
    let value: serde_json::Value = serde_json::from_str(&text).expect("rows.json is JSON");
    let rows = value["rows"]
        .as_object()
        .expect("rows.json maps each rail row to its fixture")
        .iter()
        .map(|(row, file)| {
            let file = file.as_str().expect("a fixture's file name");
            (row.clone(), file.to_owned())
        })
        .collect();
    let clean = value["clean"]
        .as_str()
        .expect("the clean note's name")
        .to_owned();
    Index { rows, clean }
}

/// Every rail row that refuses `text`.
fn rows_refusing(rails: &Rails, text: &str) -> BTreeSet<String> {
    rails
        .refusals(text)
        .into_iter()
        .map(|refusal| refusal.row.to_string())
        .collect()
}

#[test]
fn every_rail_refuses_its_planted_fixture_and_a_clean_note_passes() {
    let rails = Rails::vendored().expect("the vendored rails.json reads as rails");
    let rows: Vec<String> = rails.rows().iter().map(ToString::to_string).collect();
    let rows = examined("rail row(s) of rails.json", rows);
    let index = index();
    assert_eq!(
        index.rows.keys().cloned().collect::<Vec<_>>(),
        rows,
        "rows.json plants one fixture for every rail row of rails.json, and for nothing else"
    );
    let mut refused = 0;
    for (row, file) in &index.rows {
        let text = fixture(file);
        assert_eq!(
            rows_refusing(&rails, &text),
            BTreeSet::from([row.clone()]),
            "the planted fixture {file} is refused by its own rail row {row}, and by no other"
        );
        match rails.check(&text) {
            Verdict::Refuse(refusal) => assert_eq!(refusal.row.as_str(), row, "{file}"),
            Verdict::Pass => panic!("the planted fixture {file} passed the rails"),
        }
        refused += 1;
    }
    assert_eq!(
        refused,
        rows.len(),
        "the examined count equals the rail rows"
    );
    let clean = fixture(&index.clean);
    assert_eq!(
        rails.check(&clean),
        Verdict::Pass,
        "the clean reading note passes the rails: {:?}",
        rails.refusals(&clean)
    );
}

#[test]
fn rails_json_holds_only_the_rail_kinds_the_port_reads() {
    let document: serde_json::Value = serde_json::from_str(VENDORED).expect("rails.json is JSON");
    let keys: BTreeSet<&str> = document
        .as_object()
        .expect("rails.json is an object")
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(
        keys,
        KNOWN_KEYS.into_iter().collect::<BTreeSet<_>>(),
        "a key rails.json gained is a new kind of rail, which the port needs code for (ADR-042)"
    );
}

#[test]
fn a_control_character_is_refused_by_the_adapters_own_rail_and_a_tab_passes() {
    let rails = Rails::vendored().expect("the vendored rails.json reads as rails");
    for code in [0x00_u32, 0x07, 0x0b, 0x0c, 0x1b, 0x7f, 0x85] {
        let control = char::from_u32(code).expect("a control character");
        let text = format!("A clean first line.\nA second line with {control} in it.\n");
        let refusals = rails.refusals(&text);
        assert_eq!(
            refusals
                .iter()
                .map(|refusal| (refusal.row.as_str(), refusal.line))
                .collect::<Vec<_>>(),
            vec![(CONTROL_CHARACTER, 2)],
            "U+{code:04X} is refused on its own line"
        );
    }
    assert_eq!(
        rails.check("A tab\tseparates these.\r\nA Windows line ends here.\n"),
        Verdict::Pass,
        "a tab, a carriage return and a line feed pass"
    );
}

#[test]
fn a_refusal_names_its_rail_and_line_and_never_the_text() {
    let rails = Rails::vendored().expect("the vendored rails.json reads as rails");
    let text = "A first line.\nA planted private-marker <% tp.user.command() %> line.\n";
    let Verdict::Refuse(refusal) = rails.check(text) else {
        panic!("a Templater command passed the rails");
    };
    assert_eq!((refusal.row.as_str(), refusal.line), ("templater_open", 2));
    let shown = refusal.to_string();
    assert_eq!(shown, "the rail templater_open refuses line 2");
    assert!(
        !shown.contains("private-marker"),
        "the refusal echoed the text: {shown}"
    );
}

/// How long one scan may run before a test calls it a hang. A scan of a few short lines ends in
/// microseconds, so one still running after this has stopped advancing its cursor: the test fails
/// then, rather than letting the scan grow its memory until the mutation tool's own timeout.
const HANG: Duration = Duration::from_secs(5);

/// The vendored rails.
fn vendored() -> Rails {
    Rails::vendored().expect("the vendored rails.json reads as rails")
}

/// Every refusal `rails` make of `text`, as each one's row and line, in `refusals`' own order. The
/// scan runs on its own thread, and one that panics or outlasts [`HANG`] fails the test.
fn refused(rails: &Rails, text: &str) -> Vec<(String, usize)> {
    let (sender, receiver) = mpsc::channel();
    let (rails, note) = (rails.clone(), text.to_owned());
    thread::spawn(move || {
        let refusals = rails
            .refusals(&note)
            .into_iter()
            .map(|refusal| (refusal.row.to_string(), refusal.line))
            .collect::<Vec<_>>();
        // A send fails only once the test has stopped waiting, having failed on a hang.
        let _ = sender.send(refusals);
    });
    match receiver.recv_timeout(HANG) {
        Ok(refusals) => refusals,
        Err(RecvTimeoutError::Timeout) => panic!("the scan of {text:?} ran past {HANG:?}"),
        Err(RecvTimeoutError::Disconnected) => panic!("the scan of {text:?} panicked"),
    }
}

/// Holds each case's note to exactly the refusals it names, row and line, and prints how many
/// cases it judged.
fn judge(rails: &Rails, what: &str, cases: &[(&str, &[(&str, usize)])]) {
    for (text, expected) in examined(what, cases.to_vec()) {
        let expected: Vec<(String, usize)> = expected
            .iter()
            .map(|(row, line)| ((*row).to_owned(), *line))
            .collect();
        assert_eq!(refused(rails, text), expected, "{what}: {text:?}");
    }
}

#[test]
fn each_kind_of_rail_refuses_the_line_it_stands_on() {
    // The raw scan, a tag in the prose, a fence's opening line and a code span each name their
    // own line, never the one before it.
    judge(
        &vendored(),
        "note(s) refused on their second line",
        &[
            (
                "A clean line.\nSee obsidian://open here.\n",
                &[("executable_schemes:obsidian", 2)],
            ),
            ("A clean line.\n<iframe>\n", &[("html_allow", 2)]),
            (
                "A clean line.\n```dataviewjs\nlet x = 1;\n```\n",
                &[("fence_known:dataviewjs", 2)],
            ),
            (
                "A clean line.\nThe count is `= this.file.name` today.\n",
                &[("inline_query_prefixes:=", 2)],
            ),
        ],
    );
}

#[test]
fn a_fence_opens_within_three_spaces_and_closes_on_a_bare_run_as_long() {
    judge(
        &vendored(),
        "fenced note(s)",
        &[
            // Up to three spaces may stand before an opening run; four open no fence.
            (
                "   ```dataviewjs\nlet x = 1;\n   ```\n",
                &[("fence_known:dataviewjs", 1)],
            ),
            ("    ```dataviewjs\nlet x = 1;\n", &[]),
            // A backtick run whose info holds a backtick opens no fence, so its line is prose; a
            // tilde run may hold one (CommonMark).
            ("```js`x\n<iframe>\n", &[("html_allow", 2)]),
            ("~~~ a`b\n<iframe>\n~~~\n", &[("fence_allow", 1)]),
            // The fence hides its code up to the run that closes it, and the prose after it is
            // read again.
            ("```\nlet x = 1;\n<iframe>\n```\n", &[]),
            ("```\nlet x = 1;\n```\n<iframe>\n", &[("html_allow", 4)]),
            // Up to three spaces may stand before a closing run; four close nothing.
            ("```\nlet x = 1;\n   ```\n<iframe>\n", &[("html_allow", 4)]),
            ("```\nlet x = 1;\n    ```\n<iframe>\n", &[]),
            // A blank line closes nothing, and neither does a run with text after it; spaces and
            // tabs after a run close.
            ("```\nlet x = 1;\n\n<iframe>\n```\n", &[]),
            ("```\nlet x = 1;\n```x\n<iframe>\n```\n", &[]),
            ("```\nlet x = 1;\n``` \t\n<iframe>\n", &[("html_allow", 4)]),
        ],
    );
}

#[test]
fn a_code_span_hides_its_code_from_the_prose_and_shows_it_to_the_query_rail() {
    judge(
        &vendored(),
        "note(s) with a code span",
        &[
            // A span hides its code, whatever its run of backticks, and only its own: the scan
            // resumes after its closing run.
            ("A span ``<iframe>`` hides a tag.\n", &[]),
            ("Then ```<p>``` shows.\n", &[]),
            ("`a` <iframe> `b`\n", &[("html_allow", 1)]),
            // A backtick that opens no span is prose.
            ("`5 is not code <iframe>\n", &[("html_allow", 1)]),
            // A run followed by another backtick closes nothing.
            ("`<p>`` more\n", &[("html_allow", 1)]),
            // The query rail reads a span's code, after a long line and at a line's end.
            (
                "Some text first: ``= x``\n",
                &[("inline_query_prefixes:=", 1)],
            ),
            ("Run `=x`\n", &[("inline_query_prefixes:=", 1)]),
        ],
    );
}

#[test]
fn a_comment_hides_what_it_holds_and_keeps_the_notes_lines() {
    judge(
        &vendored(),
        "note(s) with a comment",
        &[
            // An Obsidian comment and an HTML comment each hide what they hold, to their close.
            ("Text %% <iframe> %% more\n", &[]),
            ("Before <!-- a note<p>--> after.\n", &[]),
            // A comment keeps the note's lines, so a refusal after it names its own line.
            ("<!-- c -->\n<iframe>\n", &[("html_allow", 2)]),
            ("<!-- a\nb -->\n<iframe>\n", &[("html_allow", 3)]),
        ],
    );
}

#[test]
fn a_closing_tag_and_each_attribute_name_are_read_as_the_probe_reads_them() {
    judge(
        &vendored(),
        "note(s) with a tag",
        &[
            // A closing tag is a tag.
            ("</iframe>\n", &[("html_allow", 1)]),
            // An attribute name may start with a colon, and is then no allowed name.
            ("<span :class=\"x\">\n", &[("html_attributes_allow", 1)]),
            // A value, after spaces or none, quoted or bare, is never read as a name, and the
            // name after it is.
            ("<span title= \"onclick\">\n", &[]),
            ("<span title=onclick>\n", &[]),
            (
                "<span title=x onclick=y>\n",
                &[("html_attributes_allow", 1)],
            ),
        ],
    );
}

#[test]
fn a_wikilink_is_read_between_its_double_brackets() {
    judge(
        &vendored(),
        "note(s) with a wikilink",
        &[
            // Two brackets open it and two close it.
            ("![xy.base]]\n", &[]),
            ("![[view.base] and more\n", &[]),
            // The scan resumes right after one, so the next is read, and an empty one is read
            // once.
            (
                "[[abcdefgh]]![[view.base]]\n",
                &[("dynamic_embed_extensions:.base", 1)],
            ),
            ("An empty link [[]] here.\n", &[]),
        ],
    );
}

#[test]
fn a_markdown_links_destination_title_and_embed_are_read_as_the_probe_reads_them() {
    judge(
        &vendored(),
        "note(s) with a markdown link",
        &[
            // An embed of a dynamic view is refused as a wikilink's is; a label may be empty.
            (
                "![x](view.base)\n",
                &[("dynamic_embed_extensions:.base", 1)],
            ),
            (
                "[](javascript:x)\n",
                &[("executable_schemes:javascript", 1)],
            ),
            // An angled destination may hold a space.
            (
                "[x](<javascript:a b>)\n",
                &[("executable_schemes:javascript", 1)],
            ),
            // A title stands after spaces, and the link closes right after it.
            (
                "[x](javascript:y \"title\")\n",
                &[("executable_schemes:javascript", 1)],
            ),
            ("[x](javascript:y \"t\" )\n", &[]),
            ("![x](<view.base>\"t\")\n", &[]),
            // The scan resumes after the link, never inside it.
            (
                "[x](a \"t\")[y](javascript:z)\n",
                &[("executable_schemes:javascript", 1)],
            ),
            ("[x](a \"a[\")](javascript:z)\n", &[]),
            ("[a](x[)](javascript:z)\n", &[]),
            // A one-letter scheme is a drive: the destination is a path, cut at `#`.
            (
                "![x](C:view.base#top)\n",
                &[("dynamic_embed_extensions:.base", 1)],
            ),
        ],
    );
}
