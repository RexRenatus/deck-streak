//! SPEC-345 A15 and A16 (R7, R10; ADR-356 D5, D8): outside the core, only the two UI adapters'
//! entry files name the owner's gesture, and the engine's write and door names appear only at the
//! held lines of SPEC-345 section 8; and the gesture is neither `Clone` nor `Copy`. SPEC-365 A7
//! and A8 (R2, R8; ADR-376 D9) hold the owner's answer the same way: only the entry files name it,
//! each builds it from a press and runs it, the engine's `answer_card` appears only at its held
//! fixture lines, and the answer is neither `Clone`, `Copy` nor `Default`.
//!
//! The census reads every member's `src`, `tests`, `examples` and `benches` and its `build.rs`,
//! with comments stripped and string literals kept, and judges every file outside the core. A held
//! line is a file, its exact trimmed text, how many times the file holds it and why, so a new
//! line, a held line with more text on it, a second copy and a line the tree no longer holds are
//! each refused by name. Callers planted in a scratch tree prove the census sees each kind.

#![allow(
    clippy::expect_used,
    clippy::print_stdout,
    reason = "a failed fixture should fail its test, and an enumerating test prints what it examined"
)]

mod support;

use std::collections::BTreeMap;
use std::fs;
use std::marker::PhantomData;
use std::path::Path;

use deck_streak_engine_core::answer::{Grade, OwnerAnswer};
use deck_streak_engine_core::gesture::{OwnerGesture, Target};

/// The engine's write and door names (SPEC-345 R10), the engine's own answer (SPEC-365 R8), and
/// its undo (SPEC-371 R13).
const ENGINE_NAMES: [&str; 24] = [
    "schedule_cards_as_new",
    "reschedule_cards_as_new",
    "reschedule_cards_as_new_defaults",
    "set_due_date",
    "remove_deck_config",
    "update_deck_configs",
    "upgrade_scheduler",
    "upgrade_to_v2_scheduler",
    "change_notetype",
    "change_notetype_of_notes",
    "remove_cards",
    "remove_cards_and_orphaned_notes",
    "remove_notes",
    "full_upload_or_download",
    "full_upload",
    "full_download",
    "grade_now",
    "remove_decks",
    "remove_decks_and_child_decks",
    "run_service_method",
    "run_db_command_bytes",
    "init_backend",
    "answer_card",
    "undo",
];

/// The gesture's names: its type, its one constructor, the dispatcher's exempt entry and the
/// full-sync choice's one-way door (SPEC-364 R3); and the answer's three (SPEC-365 R8).
const GESTURE_NAMES: [&str; 7] = [
    "OwnerGesture",
    "from_tap",
    "run_exempt",
    "OwnerAnswer",
    "from_press",
    "run_answer",
    "run_one_way",
];

/// The two UI adapters' exempt entries: outside the core, the only files that may name the gesture.
const ENTRY_FILES: [&str; 2] = ["crates/ffi/src/engine.rs", "crates/web-engine/src/wasm.rs"];

/// What each entry file's code owes: it builds the gesture from the tap and runs it, and it builds
/// the answer from the press and runs it (SPEC-365 R8).
const ENTRY_CALLS: [&str; 4] = [
    "OwnerGesture::from_tap(",
    ".run_exempt(",
    "OwnerAnswer::from_press(",
    ".run_answer(",
];

/// The core, whose own lines R10 does not hold (SPEC-345 section 8).
const CORE: &str = "crates/engine-core/";

/// The directories of a member the census reads, beside its `build.rs`.
const SOURCE_DIRECTORIES: [&str; 4] = ["src", "tests", "examples", "benches"];

/// Why the mirror's own port may name the engine: it is the engine's port in `ingest` (ADR-022).
const MIRROR_PORT: &str = "the mirror's own port method";
/// Why #600's census test may: its literals are data, not calls.
const SKIP_CENSUS_LITERAL: &str = "a string literal of #600's own census test: data, not a call";
/// Why SPEC-342's probes may: they run the engine against the engine's own sync server.
const SYNC_PROBE: &str = "SPEC-342 R10 and R11's probes against the engine's own sync server";
/// Why the web boundary census may: its `OWED` entry for the export names the export's statements
/// as string literals, data and not calls.
const BOUNDARY_CENSUS_LITERAL: &str =
    "a string literal of the web boundary census's own entry for the export: data, not a call";
/// Why the bot may name `undo`: its command calls the bot's own undo of a habit check-in, a
/// method of its own and not the engine's (SPEC-371 R13).
const BOT_OWN_UNDO: &str =
    "the bot's own undo of a habit check-in: its own method, not the engine's";
/// Why the notification router's test may: its table names the bot's command as a string literal.
const ROUTER_LITERAL: &str =
    "a string literal of the notification router's own table: data, not a call";
/// Why a fixture may answer a card through the engine's own `answer_card`: it builds a review
/// history in the test's own scratch collection, which no product path runs (ADR-376 D10).
const ANSWER_FIXTURE: &str =
    "a fixture's review in the test's own scratch collection, which no product path runs";

/// Each line outside the core that may name the engine or the gesture (SPEC-345 section 8): its
/// file, its exact trimmed text, how many times the file holds it, and why.
const HELD: [(&str, &str, usize, &str); 34] = [
    (
        "crates/ingest/src/engine.rs",
        "col.full_download(auth, engine_client())",
        1,
        "the mirror's full download, which writes only DeckStreak's read copy (SPEC-022 R6)",
    ),
    (
        "crates/ingest/src/engine.rs",
        ".set_due_date(&ids, spec, None)",
        1,
        "the mirror's skip-take write (#600), the adapter reaching the engine's own write",
    ),
    (
        "crates/ingest/src/skip_write.rs",
        "writer.set_due_date(path, &moved, &spec)",
        1,
        MIRROR_PORT,
    ),
    (
        "crates/ingest/src/sync.rs",
        "self.reopening(|| self.engine.full_download(&copy, login))",
        1,
        MIRROR_PORT,
    ),
    (
        "crates/ingest/examples/engine_probe.rs",
        ".block_on(RslibEngine.full_download(collection, &login()?))",
        1,
        MIRROR_PORT,
    ),
    (
        "crates/ingest/tests/engine_budget.rs",
        ".block_on(RslibEngine.full_download(&copy, &login))",
        2,
        MIRROR_PORT,
    ),
    (
        "crates/ingest/tests/sync.rs",
        ".block_on(RslibEngine.full_download(&copy, &login))",
        1,
        MIRROR_PORT,
    ),
    (
        "crates/ingest/tests/support/mod.rs",
        ".block_on(col.full_upload(auth, engine_client()))",
        1,
        "an upload to the scratch test server",
    ),
    (
        "crates/ingest/tests/undo_and_full_sync.rs",
        ".block_on(col.full_download(auth, engine_client()))",
        4,
        SYNC_PROBE,
    ),
    (
        "crates/ingest/tests/undo_and_full_sync.rs",
        ".block_on(col.full_upload(auth, engine_client()))",
        2,
        SYNC_PROBE,
    ),
    (
        "crates/ingest/tests/undo_and_full_sync.rs",
        ".block_on(a.full_upload(auth, engine_client()))",
        1,
        SYNC_PROBE,
    ),
    (
        "crates/ingest/tests/undo_and_full_sync.rs",
        "col.undo().expect(\"the answer is undone\");",
        2,
        SYNC_PROBE,
    ),
    (
        "crates/ingest/tests/undo_and_full_sync.rs",
        "matches!(col.undo(), Err(AnkiError::UndoEmpty)),",
        1,
        SYNC_PROBE,
    ),
    (
        "crates/bot/src/commands.rs",
        "Some(\"undo\") => self.undo().await,",
        1,
        BOT_OWN_UNDO,
    ),
    (
        "crates/notifications/tests/one_router.rs",
        "(\"Commands::undo\", \"send\"),",
        1,
        ROUTER_LITERAL,
    ),
    (
        "crates/ingest/tests/skip_write.rs",
        ".set_due_date(",
        1,
        "#600's test reaching the mirror's port",
    ),
    (
        "crates/ingest/tests/preset_zero_upload.rs",
        ".set_due_date(&scene.fixture.copy(), &[PRESET_CARDS[2].0], \"5\")",
        1,
        "the preset upload test reaching the mirror's port on its own scratch fixture copy",
    ),
    (
        "crates/ingest/tests/skip_census.rs",
        "\".set_due_date(\",",
        1,
        SKIP_CENSUS_LITERAL,
    ),
    (
        "crates/ingest/tests/skip_census.rs",
        "\".full_upload(\",",
        1,
        SKIP_CENSUS_LITERAL,
    ),
    (
        "crates/ingest/tests/skip_census.rs",
        r"col.set_due_date(&ids, &spec, None);\n\",
        1,
        SKIP_CENSUS_LITERAL,
    ),
    (
        "crates/ingest/tests/skip_census.rs",
        r#"["crates/ingest/src/engine.rs:11 .set_due_date("],"#,
        1,
        SKIP_CENSUS_LITERAL,
    ),
    (
        "crates/ffi/tests/exempt.rs",
        "engine.run_exempt(write, target, input)",
        1,
        "the native adapter's own test of its exempt entry (A17, A18)",
    ),
    (
        "crates/web-engine/tests/boundary.rs",
        "\"run_exempt\",",
        1,
        BOUNDARY_CENSUS_LITERAL,
    ),
    (
        "crates/web-engine/tests/boundary.rs",
        "\"let gesture = OwnerGesture::from_tap(write, target).map_err(refuse)?;\",",
        1,
        BOUNDARY_CENSUS_LITERAL,
    ),
    (
        "crates/web-engine/tests/boundary.rs",
        "\"dispatcher()?.run_exempt(gesture, input).map_err(refuse)\",",
        1,
        BOUNDARY_CENSUS_LITERAL,
    ),
    (
        "crates/web-engine/tests/boundary.rs",
        "\"OwnerAnswer::from_press(shown.card, pressed(grade))\",",
        1,
        BOUNDARY_CENSUS_LITERAL,
    ),
    (
        "crates/web-engine/tests/boundary.rs",
        "\".run_answer(answer, &request.encode_to_vec())\",",
        1,
        BOUNDARY_CENSUS_LITERAL,
    ),
    (
        "crates/web-engine/tests/boundary.rs",
        "\"OwnerGesture::from_tap(ExemptWrite::Undo, Target::Card(card))\",",
        1,
        BOUNDARY_CENSUS_LITERAL,
    ),
    (
        "crates/web-engine/tests/boundary.rs",
        "\".run_exempt(gesture, &recorded.encode_to_vec())\",",
        1,
        BOUNDARY_CENSUS_LITERAL,
    ),
    (
        "crates/web-engine/tests/boundary.rs",
        "\"OwnerGesture::from_tap(ExemptWrite::OneWaySync, Target::Collection)\",",
        1,
        BOUNDARY_CENSUS_LITERAL,
    ),
    (
        "crates/web-engine/tests/boundary.rs",
        "\".run_one_way(\",",
        1,
        BOUNDARY_CENSUS_LITERAL,
    ),
    (
        "crates/ingest/tests/support/mod.rs",
        "col.answer_card(&mut CardAnswer {",
        2,
        ANSWER_FIXTURE,
    ),
    (
        "crates/ingest/tests/undo_and_full_sync.rs",
        "col.answer_card(&mut CardAnswer {",
        1,
        ANSWER_FIXTURE,
    ),
    (
        "crates/ingest/tests/skip_census.rs",
        "\".answer_card(\",",
        1,
        SKIP_CENSUS_LITERAL,
    ),
];

/// Whether `file` is one of the two adapters' entry files.
fn is_entry(file: &str) -> bool {
    ENTRY_FILES.contains(&file)
}

/// Whether a line the census found is the held line `held`: by its exact text, so a held line
/// with more on it is a new line.
fn held_match(line: &str, held: &str) -> bool {
    line == held
}

/// Whether a held line found `found` times holds its count: exactly, so a second copy is a new
/// caller.
fn holds(count: usize, found: usize) -> bool {
    count == found
}

fn is_ident(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// The index just past the string or character literal that starts at `at`, if one does: a
/// quoted string, a raw string (`r"…"`, `r#"…"#`, with a `b` or `c` prefix), or a character
/// literal. A lifetime is not a literal.
fn literal_end(chars: &[char], at: usize) -> Option<usize> {
    let before = |back: usize| at.checked_sub(back).and_then(|i| chars.get(i)).copied();
    match chars[at] {
        '"' => {
            let mut end = at + 1;
            while end < chars.len() {
                match chars[end] {
                    '\\' => end += 2,
                    '"' => return Some(end + 1),
                    _ => end += 1,
                }
            }
            Some(chars.len())
        }
        'r' => {
            let prefix_ok = match before(1) {
                None => true,
                Some(c) if !is_ident(c) => true,
                Some('b' | 'c') => !before(2).is_some_and(is_ident),
                Some(_) => false,
            };
            let hashes = chars[at + 1..].iter().take_while(|c| **c == '#').count();
            if !prefix_ok || chars.get(at + 1 + hashes) != Some(&'"') {
                return None;
            }
            let mut end = at + hashes + 2;
            while end < chars.len() {
                let closes = chars[end] == '"'
                    && chars[end + 1..]
                        .iter()
                        .take(hashes)
                        .filter(|c| **c == '#')
                        .count()
                        == hashes;
                if closes {
                    return Some(end + 1 + hashes);
                }
                end += 1;
            }
            Some(chars.len())
        }
        '\'' => {
            if chars.get(at + 1) == Some(&'\\') {
                let close = chars.get(at + 3..)?.iter().position(|c| *c == '\'')?;
                Some(at + 3 + close + 1)
            } else if chars.get(at + 2) == Some(&'\'') {
                Some(at + 3)
            } else {
                None
            }
        }
        _ => None,
    }
}

/// A source with every comment removed and every newline kept, so each line keeps its number,
/// beside a mark of which characters sit inside a string or character literal.
struct Stripped {
    text: Vec<char>,
    literal: Vec<bool>,
}

fn strip(source: &str) -> Stripped {
    let chars: Vec<char> = source.chars().collect();
    let mut text = Vec::with_capacity(chars.len());
    let mut literal = Vec::with_capacity(chars.len());
    let mut at = 0;
    while at < chars.len() {
        let next = chars.get(at + 1).copied();
        if chars[at] == '/' && next == Some('/') {
            while at < chars.len() && chars[at] != '\n' {
                at += 1;
            }
        } else if chars[at] == '/' && next == Some('*') {
            let mut depth = 0_usize;
            while at < chars.len() {
                let pair = (chars[at], chars.get(at + 1).copied());
                if pair == ('/', Some('*')) {
                    depth += 1;
                    at += 2;
                } else if pair == ('*', Some('/')) {
                    depth -= 1;
                    at += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    if chars[at] == '\n' {
                        text.push('\n');
                        literal.push(false);
                    }
                    at += 1;
                }
            }
        } else if let Some(end) = literal_end(&chars, at) {
            let end = end.min(chars.len());
            text.extend_from_slice(&chars[at..end]);
            literal.extend(std::iter::repeat_n(true, end - at));
            at = end;
        } else {
            text.push(chars[at]);
            literal.push(false);
            at += 1;
        }
    }
    Stripped { text, literal }
}

/// The line of the character at `at`, from zero.
fn line_of(text: &[char], at: usize) -> usize {
    text[..at].iter().filter(|c| **c == '\n').count()
}

/// The nearest character before `at` that is not a blank, with its index.
fn before_blanks(text: &[char], at: usize) -> Option<(usize, char)> {
    text[..at]
        .iter()
        .enumerate()
        .rev()
        .find(|(_, c)| !c.is_whitespace())
        .map(|(i, c)| (i, *c))
}

/// The nearest character at or after `at` that is not a blank.
fn after_blanks(text: &[char], at: usize) -> Option<char> {
    text[at..].iter().copied().find(|c| !c.is_whitespace())
}

/// What one file names: the lines that name an engine write or door, the lines that name the
/// gesture, and the lines that open a `#[path]` or `include!` of a core file, each from zero.
#[derive(Default)]
struct Names {
    engine: Vec<usize>,
    gesture: Vec<usize>,
    includes: Vec<usize>,
}

/// Reads one file's stripped text: an engine name as a call (`.name(`), a path (`::name`) or a
/// name in a `use` tree, which may brace over several lines; a gesture name as any identifier;
/// and a `#[path]` or `include!` whose span names the core.
fn names(stripped: &Stripped) -> Names {
    let text = &stripped.text;
    let code = |at: usize| !stripped.literal[at];
    let mut found = Names::default();
    let mut in_use = vec![false; text.len()];
    let mut at = 0;
    while at < text.len() {
        if !is_ident(text[at]) {
            at += 1;
            continue;
        }
        let start = at;
        while at < text.len() && is_ident(text[at]) {
            at += 1;
        }
        let word: String = text[start..at].iter().collect();
        if word.starts_with(|c: char| c.is_ascii_digit()) {
            continue;
        }
        if word == "use" && code(start) {
            let mut end = at;
            while end < text.len() && !(text[end] == ';' && code(end)) {
                in_use[end] = true;
                end += 1;
            }
        }
        if ENGINE_NAMES.contains(&word.as_str()) {
            let before = before_blanks(text, start);
            let called =
                before.is_some_and(|(_, c)| c == '.') && after_blanks(text, at) == Some('(');
            let pathed = before.is_some_and(|(i, c)| c == ':' && i > 0 && text[i - 1] == ':');
            if called || pathed || in_use[start] {
                found.engine.push(line_of(text, start));
            }
        }
        if GESTURE_NAMES.contains(&word.as_str()) {
            found.gesture.push(line_of(text, start));
        }
        let opens = match word.as_str() {
            "path" => before_blanks(text, start).is_some_and(|(i, c)| {
                c == '[' && before_blanks(text, i).is_some_and(|(_, c)| c == '#')
            }),
            "include" => text.get(at) == Some(&'!'),
            _ => false,
        };
        if opens && code(start) {
            let close = if word == "path" { ']' } else { ')' };
            let span: String = text[at..]
                .iter()
                .enumerate()
                .take_while(|(i, c)| !(**c == close && code(at + i)))
                .map(|(_, c)| *c)
                .collect();
            if span.contains("engine-core") || span.contains("engine_core") {
                found.includes.push(line_of(text, start));
            }
        }
    }
    found
}

/// Every file the census reads under `root`: each member's `build.rs`, and every `.rs` file under
/// its `src`, `tests`, `examples` and `benches`, as a path from `root` with `/` between parts.
fn files(root: &Path) -> Vec<String> {
    fn walk(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries {
            let path = entry.expect("a directory entry reads").path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push(path);
            }
        }
    }
    let mut found = Vec::new();
    let members = fs::read_dir(root.join("crates")).expect("the tree has a crates directory");
    for member in members {
        let member = member.expect("a member entry reads").path();
        if !member.is_dir() {
            continue;
        }
        let build = member.join("build.rs");
        if build.is_file() {
            found.push(build);
        }
        for directory in SOURCE_DIRECTORIES {
            walk(&member.join(directory), &mut found);
        }
    }
    let mut found: Vec<String> = found
        .iter()
        .map(|path| {
            let relative = path
                .strip_prefix(root)
                .expect("each file sits under the root");
            relative
                .components()
                .map(|part| part.as_os_str().to_str().expect("a source path is UTF-8"))
                .collect::<Vec<_>>()
                .join("/")
        })
        .collect();
    found.sort();
    found
}

/// What the census found in one tree.
struct Census {
    /// Every file it read.
    files: Vec<String>,
    /// The entry files whose code builds a gesture from a tap and runs it.
    entries: Vec<String>,
    /// Every line outside the core it judged, as `file:line: text`.
    callers: Vec<String>,
    /// What it refused, each by name.
    refused: Vec<String>,
}

/// Reads every file of `root` and judges each outside the core against `held`.
fn census(root: &Path, held: &[(&str, &str, usize, &str)]) -> Census {
    let files = files(root);
    let mut entries = Vec::new();
    let mut callers = Vec::new();
    let mut refused = Vec::new();
    let mut groups: BTreeMap<(String, String), usize> = BTreeMap::new();
    for file in files.iter().filter(|file| !file.starts_with(CORE)) {
        let source =
            fs::read_to_string(root.join(file)).unwrap_or_else(|error| panic!("{file}: {error}"));
        let lines: Vec<&str> = source.lines().collect();
        let stripped = strip(&source);
        if is_entry(file) {
            let code: String = stripped
                .text
                .iter()
                .zip(&stripped.literal)
                .filter(|(c, literal)| !**literal && !c.is_whitespace())
                .map(|(c, _)| *c)
                .collect();
            if ENTRY_CALLS.iter().all(|call| code.contains(call)) {
                entries.push(file.clone());
            }
        }
        let found = names(&stripped);
        let mut judged: Vec<usize> = found.engine;
        if !is_entry(file) {
            judged.extend(found.gesture);
        }
        judged.sort_unstable();
        judged.dedup();
        for line in judged {
            let text = lines.get(line).map_or("", |text| text.trim());
            callers.push(format!("{file}:{}: {text}", line + 1));
            *groups.entry((file.clone(), text.to_owned())).or_default() += 1;
        }
        for line in found.includes {
            let text = lines.get(line).map_or("", |text| text.trim());
            refused.push(format!("{file}:{}: includes a core file: {text}", line + 1));
        }
    }
    let mut matched = vec![0_usize; held.len()];
    for ((file, text), found) in &groups {
        let row = held
            .iter()
            .position(|(held_file, line, _, _)| held_file == file && held_match(text, line));
        match row {
            Some(row) => matched[row] += found,
            None => refused.push(format!("{file}: `{text}` found {found}, held 0")),
        }
    }
    for ((file, line, count, _), found) in held.iter().zip(matched) {
        if !holds(*count, found) {
            refused.push(format!("{file}: `{line}` found {found}, held {count}"));
        }
    }
    Census {
        files,
        entries,
        callers,
        refused,
    }
}

/// Writes `text` as the whole of `file` under `root`.
fn write(root: &Path, file: &str, text: &str) {
    let path = root.join(file);
    fs::create_dir_all(path.parent().expect("a planted file has a directory"))
        .expect("a planted directory");
    fs::write(&path, text).expect("a planted file");
}

/// A scratch tree that holds each held line as often as it is held, and both entry files' calls,
/// so the census passes it before anything is planted.
fn held_tree(root: &Path) {
    let mut planted: BTreeMap<&str, String> = BTreeMap::new();
    for (file, line, count, _) in HELD {
        for _ in 0..count {
            let text = planted.entry(file).or_default();
            text.push_str(line);
            text.push('\n');
        }
    }
    for file in ENTRY_FILES {
        planted.entry(file).or_default().push_str(
            "let gesture = OwnerGesture::from_tap(write, target)?;\n\
             self.dispatcher.run_exempt(gesture, &input)\n\
             let answer = OwnerAnswer::from_press(card, grade);\n\
             self.dispatcher.run_answer(answer, &input)\n",
        );
    }
    for (file, text) in planted {
        write(root, file, &text);
    }
}

/// Appends `text` to `file` under `root`.
fn append(root: &Path, file: &str, text: &str) {
    let path = root.join(file);
    let held = fs::read_to_string(&path).unwrap_or_default();
    write(root, file, &format!("{held}{text}"));
}

#[test]
fn no_non_ui_caller_reaches_an_exempt_function() {
    let found = census(&support::workspace(), &HELD);
    assert_eq!(
        found.entries,
        ENTRY_FILES.map(String::from).to_vec(),
        "each UI adapter's entry file builds the gesture from the tap and runs it"
    );
    for caller in &found.callers {
        println!("{caller}");
    }
    support::examined(
        "file(s) under every member's src, tests, examples, benches and build.rs",
        found.files.clone(),
    );
    let callers = support::examined("caller line(s) outside the core", found.callers.clone());
    assert_eq!(
        found.refused,
        Vec::<String>::new(),
        "outside the core, only the entry files name the gesture, and the engine only at the held lines"
    );
    assert_eq!(
        callers.len(),
        HELD.iter().map(|(_, _, count, _)| count).sum::<usize>(),
        "the census found each held line as often as it is held"
    );
    planted();
}

/// Plants into the scratch tree under `root` one caller, include, extended line, second copy or
/// stale line per file, beside comment-only lines the census must not refuse.
fn plant(root: &Path) {
    append(
        root,
        "crates/daemon/src/main.rs",
        "// OwnerGesture::from_tap(write, target) in a comment is prose\n\
         /* dispatcher.run_exempt(gesture, &input) /* nested */ col.remove_notes(&ids) */\n\
         /// col.remove_notes(&ids)\n\
         // OwnerAnswer::from_press(card, grade), .run_answer(answer, &input) and col.answer_card(&mut answer)\n\
         let gesture = OwnerGesture::from_tap(ExemptWrite::Forget, Target::Card(card));\n",
    );
    append(
        root,
        "crates/bot/src/lib.rs",
        "dispatcher.run_exempt(gesture, &input)\n",
    );
    append(
        root,
        "crates/coordination/src/lib.rs",
        "backend.run_service_method(13, 17, &input)\n",
    );
    append(
        root,
        "crates/ingest/src/engine.rs",
        "col.remove_notes(&ids)\n",
    );
    append(
        root,
        "crates/ingest/src/lib.rs",
        "use anki::backend::{\n    Backend,\n    init_backend,\n};\n",
    );
    append(
        root,
        "crates/ingest/src/core.rs",
        "#[path = \"../../engine-core/src/gesture.rs\"]\nmod gesture;\n",
    );
    append(
        root,
        "crates/kernel/build.rs",
        "include!(\"../engine-core/src/table.rs\");\n",
    );
    append(
        root,
        "crates/api/src/main.rs",
        "let backend = anki::backend::init_backend(&message);\n",
    );
    append(
        root,
        "crates/mcp/src/lib.rs",
        "let name = \"run_exempt\";\n",
    );
    append(
        root,
        "crates/progression/benches/bench.rs",
        "engine.full_upload(auth, client)\n",
    );
    append(
        root,
        "crates/ffi/src/face.rs",
        "self.dispatcher.run_exempt(gesture, &input)\n",
    );
    write(
        root,
        "crates/ingest/tests/sync.rs",
        ".block_on(RslibEngine.full_download(&copy, &login)); col.remove_notes(&ids);\n",
    );
    append(
        root,
        "crates/ingest/src/skip_write.rs",
        "writer.set_due_date(path, &moved, &spec)\n",
    );
    append(
        root,
        "crates/coordination/src/lib.rs",
        "let answer = OwnerAnswer::from_press(card, Grade::Good);\n",
    );
    append(
        root,
        "crates/bot/src/lib.rs",
        "dispatcher.run_answer(answer, &input)\n",
    );
    append(
        root,
        "crates/ingest/src/engine.rs",
        "col.answer_card(&mut answer)\n",
    );
    for crate_name in ["daemon", "bot", "coordination", "ingest"] {
        write(
            root,
            &format!("crates/{crate_name}/src/one_way.rs"),
            "dispatcher.run_one_way(gesture, write, &auth)\n",
        );
    }
}

/// What the census refuses in the planted tree, one line per plant.
fn planted_refusals() -> Vec<&'static str> {
    vec![
        "crates/daemon/src/main.rs: `let gesture = OwnerGesture::from_tap(ExemptWrite::Forget, Target::Card(card));` found 1, held 0",
        "crates/bot/src/lib.rs: `dispatcher.run_exempt(gesture, &input)` found 1, held 0",
        "crates/coordination/src/lib.rs: `backend.run_service_method(13, 17, &input)` found 1, held 0",
        "crates/ingest/src/engine.rs: `col.remove_notes(&ids)` found 1, held 0",
        "crates/ingest/src/lib.rs: `init_backend,` found 1, held 0",
        "crates/ingest/src/core.rs:1: includes a core file: #[path = \"../../engine-core/src/gesture.rs\"]",
        "crates/kernel/build.rs:1: includes a core file: include!(\"../engine-core/src/table.rs\");",
        "crates/api/src/main.rs: `let backend = anki::backend::init_backend(&message);` found 1, held 0",
        "crates/mcp/src/lib.rs: `let name = \"run_exempt\";` found 1, held 0",
        "crates/progression/benches/bench.rs: `engine.full_upload(auth, client)` found 1, held 0",
        "crates/ffi/src/face.rs: `self.dispatcher.run_exempt(gesture, &input)` found 1, held 0",
        "crates/ingest/tests/sync.rs: `.block_on(RslibEngine.full_download(&copy, &login)); col.remove_notes(&ids);` found 1, held 0",
        "crates/ingest/tests/sync.rs: `.block_on(RslibEngine.full_download(&copy, &login))` found 0, held 1",
        "crates/ingest/src/skip_write.rs: `writer.set_due_date(path, &moved, &spec)` found 2, held 1",
        "crates/coordination/src/lib.rs: `let answer = OwnerAnswer::from_press(card, Grade::Good);` found 1, held 0",
        "crates/bot/src/lib.rs: `dispatcher.run_answer(answer, &input)` found 1, held 0",
        "crates/ingest/src/engine.rs: `col.answer_card(&mut answer)` found 1, held 0",
        "crates/daemon/src/one_way.rs: `dispatcher.run_one_way(gesture, write, &auth)` found 1, held 0",
        "crates/bot/src/one_way.rs: `dispatcher.run_one_way(gesture, write, &auth)` found 1, held 0",
        "crates/coordination/src/one_way.rs: `dispatcher.run_one_way(gesture, write, &auth)` found 1, held 0",
        "crates/ingest/src/one_way.rs: `dispatcher.run_one_way(gesture, write, &auth)` found 1, held 0",
    ]
}

/// The census over a scratch tree: it passes the held tree, then refuses by name each caller,
/// include, extended line, second copy and stale line planted into it, and no comment.
fn planted() {
    let root = support::scratch("engine-core-containment", "planted");
    held_tree(&root);
    let control = census(&root, &HELD);
    assert_eq!(
        (control.entries, control.refused),
        (ENTRY_FILES.map(String::from).to_vec(), Vec::new()),
        "the scratch tree holds each held line at its count and both entries, and passes"
    );
    plant(&root);
    let mut refused = census(&root, &HELD).refused;
    refused.sort();
    let mut expected = planted_refusals();
    expected.sort_unstable();
    assert_eq!(
        refused, expected,
        "each planted caller, include, extended line, second copy and stale line is refused by name, and no comment is"
    );
    support::examined("planted refusal(s)", refused);
}

/// SPEC-371 A19 (R13; ADR-382 D9): a planted `col.undo();` outside the entry files is refused by
/// name, and its comment is not; and over the tree, the census counts `undo` only at its held
/// lines.
#[test]
fn a_planted_undo_outside_the_entry_files_is_refused_by_name() {
    let root = support::scratch("engine-core-containment", "planted-undo");
    held_tree(&root);
    append(
        &root,
        "crates/coordination/src/lib.rs",
        "// col.undo(); in a comment is prose\ncol.undo();\n",
    );
    let refused = census(&root, &HELD).refused;
    assert_eq!(
        refused,
        vec!["crates/coordination/src/lib.rs: `col.undo();` found 1, held 0".to_owned()],
        "a planted undo outside the entry files is refused by name, and its comment is not"
    );
    support::examined("planted undo refusal(s)", refused);
    let mut undo: Vec<String> = census(&support::workspace(), &HELD)
        .callers
        .iter()
        .filter_map(|caller| {
            let (file, rest) = caller.split_once(':')?;
            let (_, text) = rest.split_once(": ")?;
            (text.contains(".undo(") || text.contains("::undo")).then(|| format!("{file}: {text}"))
        })
        .collect();
    undo.sort();
    assert_eq!(
        undo,
        vec![
            "crates/bot/src/commands.rs: Some(\"undo\") => self.undo().await,",
            "crates/ingest/tests/undo_and_full_sync.rs: col.undo().expect(\"the answer is undone\");",
            "crates/ingest/tests/undo_and_full_sync.rs: col.undo().expect(\"the answer is undone\");",
            "crates/ingest/tests/undo_and_full_sync.rs: matches!(col.undo(), Err(AnkiError::UndoEmpty)),",
            "crates/notifications/tests/one_router.rs: (\"Commands::undo\", \"send\"),",
        ],
        "outside the core, the census counts undo only at its four held entries"
    );
    support::examined("undo line(s) outside the core", undo);
}

/// Which of `Clone`, `Copy` and `Default` a type implements, read at compile time: an inherent
/// constant applies only where its bound holds, and the trait's `false` answers everywhere else.
struct Probe<T>(PhantomData<T>);

trait NotClone {
    const CLONE: bool = false;
}
trait NotCopy {
    const COPY: bool = false;
}
trait NotDefault {
    const DEFAULT: bool = false;
}
impl<T> NotClone for Probe<T> {}
impl<T> NotCopy for Probe<T> {}
impl<T> NotDefault for Probe<T> {}
impl<T: Clone> Probe<T> {
    const CLONE: bool = true;
}
impl<T: Copy> Probe<T> {
    const COPY: bool = true;
}
impl<T: Default> Probe<T> {
    const DEFAULT: bool = true;
}

/// `(Clone, Copy, Default)` for one concrete type.
macro_rules! traits {
    ($type:ty) => {
        (
            <Probe<$type>>::CLONE,
            <Probe<$type>>::COPY,
            <Probe<$type>>::DEFAULT,
        )
    };
}

#[test]
fn the_gesture_is_neither_clone_nor_copy() {
    let probed = vec![
        ("OwnerGesture", traits!(OwnerGesture)),
        ("Target", traits!(Target)),
        ("Vec<u8>", traits!(Vec<u8>)),
        ("File", traits!(std::fs::File)),
    ];
    assert_eq!(
        probed,
        vec![
            ("OwnerGesture", (false, false, false)),
            ("Target", (true, true, false)),
            ("Vec<u8>", (true, false, true)),
            ("File", (false, false, false)),
        ],
        "the gesture is neither Clone, Copy nor Default; the controls read as each type is"
    );
    support::examined("type(s) probed for Clone, Copy and Default", probed);
}

#[test]
fn the_owner_answer_is_neither_clone_nor_copy() {
    let probed = vec![
        ("OwnerAnswer", traits!(OwnerAnswer)),
        ("Grade", traits!(Grade)),
        ("Vec<u8>", traits!(Vec<u8>)),
        ("File", traits!(std::fs::File)),
    ];
    assert_eq!(
        probed,
        vec![
            ("OwnerAnswer", (false, false, false)),
            ("Grade", (true, true, false)),
            ("Vec<u8>", (true, false, true)),
            ("File", (false, false, false)),
        ],
        "the answer is neither Clone, Copy nor Default; the controls read as each type is"
    );
    support::examined("type(s) probed for Clone, Copy and Default", probed);
}
