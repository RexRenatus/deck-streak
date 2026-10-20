//! SPEC-435 A10 to A13 (R14 to R17, ADR-449 D3): every SQL escape, shell, configured program and
//! environment read is listed with its reason.
//!
//! The population is every `.rs` file under `crates/*/src/`, every `crates/*/build.rs` and every
//! `.rs` file under `crates/*/examples/`, read as text, and the census names no file to skip. It
//! blanks `//` comments, nested `/* */` comments and string literals to spaces and a character
//! literal to `0` and spaces, keeping every line break; `#[cfg(test)]` items are examined, not
//! blanked.
//!
//! The arms, read in the blanked text: an SQL escape is `AssertSqlSafe` then `(`, so a `use` line
//! naming it is no site; an environment read is `env`, `::`, then `var`, `var_os`, `vars` or
//! `vars_os`, then `(`; and every `Command::new(` is examined, its first argument read from the
//! text as written. A string literal whose last `/` part is `bash`, `sh`, `dash`, `zsh`, `ksh` or
//! `fish` is a shell; an argument that is not a string literal is a configured program, which a
//! configuration can make a shell; any other string literal is examined and passes.
//!
//! The register is [`ARMS`]: one row per source and arm, with the count and the reason. The census
//! holds each arm's sites equal to the register file by file, and refuses a site no row lists, a
//! row whose count differs and a row it does not reach. It prints `examined N source(s)` and each
//! site as `<path>:<line> <arm>`, never a line's text, and refuses zero examined, which is VOID.
//! Every red is planted in `tests/fixtures/allow_list_arms/plants.rs.fixture` or in a temporary
//! tree, never in the product tree; the tree test reads the three source sets and runs in CI.

// An integration test is test code: its helpers panic on an unreadable file, and it prints the
// examined count on purpose. clippy.toml's in-test allowances cover only `#[test]` bodies.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// One row of the register: one source's sites of one arm, and why they stand.
#[expect(
    dead_code,
    reason = "a reviewer reads each row's reason; the census reads its counts"
)]
struct ArmRow {
    /// The source, from the repository's root.
    source: &'static str,
    /// The arm of R15.
    arm: &'static str,
    /// How many sites of the arm the source holds.
    count: usize,
    /// Why they stand.
    reason: &'static str,
}

/// Every SQL escape, shell, configured program and environment read of the tree (SPEC-435 R16).
const ARMS: [ArmRow; 11] = [
    ArmRow {
        source: "crates/ingest/src/reader.rs",
        arm: "sql-escape",
        count: 1,
        reason: "the reader's one door that runs a caller's statement on the read-only copy \
                 (SPEC-023 R1)",
    },
    ArmRow {
        source: "crates/agent/src/runner.rs",
        arm: "shell",
        count: 1,
        reason: "`bash <script> <file>` by argument vector, with no `-c` (ADR-043)",
    },
    ArmRow {
        source: "crates/daemon/src/snapshot_lister.rs",
        arm: "configured",
        count: 1,
        reason: "the archive's list command from configuration, run with a cleared environment",
    },
    ArmRow {
        source: "crates/vault/src/staged.rs",
        arm: "configured",
        count: 1,
        reason: "the vault gate's configured interpreter runs a pack probe; no model is called",
    },
    ArmRow {
        source: "crates/ingest/src/skip_write/tests.rs",
        arm: "configured",
        count: 1,
        reason: "a test module re-runs its own test binary",
    },
    ArmRow {
        source: "crates/daemon/src/main.rs",
        arm: "env-read",
        count: 1,
        reason: "`main` reads the process environment once (SPEC-020 R10)",
    },
    ArmRow {
        source: "crates/ingest/src/skip_write.rs",
        arm: "env-read",
        count: 1,
        reason: "the zone pin's `TZ` (SPEC-083)",
    },
    ArmRow {
        source: "crates/ingest/src/skip_write/tests.rs",
        arm: "env-read",
        count: 1,
        reason: "the re-run child reads its role",
    },
    ArmRow {
        source: "crates/progression/build.rs",
        arm: "env-read",
        count: 1,
        reason: "the build script's census switch",
    },
    ArmRow {
        source: "crates/fsrs7/examples/replay_report.rs",
        arm: "env-read",
        count: 1,
        reason: "an example that is its own `main` writes a CI step's summary file when one is set",
    },
    ArmRow {
        source: "crates/ingest/examples/engine_probe.rs",
        arm: "env-read",
        count: 2,
        reason: "an example that is its own `main` reads its endpoint and the credentials \
                 directory it hands to the kernel's credential loader",
    },
];

/// The sites of one source's text, by line (R15).
#[derive(Debug, Default)]
struct Arms {
    /// Each site, as its line and its arm.
    sites: Vec<(usize, &'static str)>,
    /// The line of each spawn of a literal program that is no shell: examined, and passing.
    passes: Vec<usize>,
}

/// A site the census found.
#[derive(Debug)]
struct ArmSite {
    /// The source, from the repository's root.
    path: String,
    /// Its line.
    line: usize,
    /// Its arm.
    arm: &'static str,
}

/// What the census read.
#[derive(Debug, Default)]
struct Census {
    /// How many sources it read.
    examined: usize,
    /// Every site, in path order, then by line.
    sites: Vec<ArmSite>,
}

/// The arms' sites of one source's text (R14, R15).
fn arms_of(text: &str) -> Arms {
    let written: Vec<char> = text.chars().collect();
    let blanked = blank(text);
    let lines: Vec<usize> = blanked
        .iter()
        .scan(1, |line, &c| {
            let current = *line;
            if c == '\n' {
                *line += 1;
            }
            Some(current)
        })
        .collect();
    let mut arms = Arms::default();
    let mut at = 0;
    while at < blanked.len() {
        if !is_word(blanked[at]) || (at > 0 && is_word(blanked[at - 1])) {
            at += 1;
            continue;
        }
        let word = word_from(&blanked, at);
        let line = lines[at];
        let after = skip_space(&blanked, at + word.len());
        if word == "AssertSqlSafe" && blanked.get(after) == Some(&'(') {
            arms.sites.push((line, "sql-escape"));
        } else if (word == "env" || word == "Command") && starts_with(&blanked, after, "::") {
            let callee_at = skip_space(&blanked, after + 2);
            let callee = word_from(&blanked, callee_at);
            let open = skip_space(&blanked, callee_at + callee.len());
            if blanked.get(open) == Some(&'(') {
                if word == "env" && matches!(callee.as_str(), "var" | "var_os" | "vars" | "vars_os")
                {
                    arms.sites.push((line, "env-read"));
                } else if word == "Command" && callee == "new" {
                    spawn(&written, open + 1, line, &mut arms);
                }
            }
        }
        at += word.len();
    }
    arms
}

/// Judges one `Command::new(` by its first argument, read from the text as written (R15).
fn spawn(written: &[char], from: usize, line: usize, arms: &mut Arms) {
    let Some(program) = literal_at(written, skip_space(written, from)) else {
        arms.sites.push((line, "configured"));
        return;
    };
    let name = program.rsplit('/').next().unwrap_or_default();
    if matches!(name, "bash" | "sh" | "dash" | "zsh" | "ksh" | "fish") {
        arms.sites.push((line, "shell"));
    } else {
        arms.passes.push(line);
    }
}

/// The content of the string literal that starts at `at`, if one does.
fn literal_at(chars: &[char], at: usize) -> Option<String> {
    let end = string_end(chars, at)?;
    let literal: String = chars[at..end].iter().collect();
    let open = literal.find('"')?;
    let close = literal.rfind('"')?;
    (close > open).then(|| literal[open + 1..close].to_owned())
}

/// The census of the three source sets under `root` (R14, R17).
fn census(root: &Path) -> Result<Census, String> {
    let mut sources = Vec::new();
    let crates = root.join("crates");
    if crates.is_dir() {
        let unreadable = |error: std::io::Error| format!("cannot read crates/: {error}");
        for entry in fs::read_dir(&crates).map_err(unreadable)? {
            let krate = entry.map_err(unreadable)?.path();
            if !krate.is_dir() {
                continue;
            }
            sources_under(&krate.join("src"), &mut sources)?;
            let build = krate.join("build.rs");
            if build.is_file() {
                sources.push(build);
            }
            sources_under(&krate.join("examples"), &mut sources)?;
        }
    }
    sources.sort();
    println!("examined {} source(s)", sources.len());
    if sources.is_empty() {
        return Err(
            "examined 0 source(s): the population is empty, which is VOID and never a pass"
                .to_owned(),
        );
    }
    let mut census = Census {
        examined: sources.len(),
        sites: Vec::new(),
    };
    let mut passes = 0;
    for source in &sources {
        let path = source
            .strip_prefix(root)
            .unwrap_or(source)
            .display()
            .to_string();
        let text =
            fs::read_to_string(source).map_err(|error| format!("cannot read {path}: {error}"))?;
        let found = arms_of(&text);
        for (line, arm) in found.sites {
            println!("{path}:{line} {arm}");
            census.sites.push(ArmSite {
                path: path.clone(),
                line,
                arm,
            });
        }
        for line in found.passes {
            println!("{path}:{line} passes: a literal program that is no shell");
            passes += 1;
        }
    }
    println!("spawns of a literal program that is no shell {passes}, examined and passing");
    Ok(census)
}

/// Every `.rs` file under `dir`, at any depth; a directory that does not exist holds none.
fn sources_under(dir: &Path, sources: &mut Vec<PathBuf>) -> Result<(), String> {
    if !dir.is_dir() {
        return Ok(());
    }
    let unreadable = |error: std::io::Error| format!("cannot read {}: {error}", dir.display());
    for entry in fs::read_dir(dir).map_err(unreadable)? {
        let path = entry.map_err(unreadable)?.path();
        if path.is_dir() {
            sources_under(&path, sources)?;
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            sources.push(path);
        }
    }
    Ok(())
}

/// Holds `sites` equal to `register`, file by file and arm by arm (R16). A refusal names its
/// reason on each of its lines.
fn judge(sites: &[ArmSite], register: &[ArmRow]) -> Result<(), String> {
    let mut refusals = Vec::new();
    for site in sites {
        let listed = register
            .iter()
            .any(|row| row.source == site.path && row.arm == site.arm);
        if !listed {
            refusals.push(format!(
                "unlisted: {}:{} {}",
                site.path, site.line, site.arm
            ));
        }
    }
    for row in register {
        let found = sites
            .iter()
            .filter(|site| site.path == row.source && site.arm == row.arm)
            .count();
        if found == 0 {
            refusals.push(format!("stale: {} {} {}", row.source, row.arm, row.count));
        } else if found != row.count {
            refusals.push(format!(
                "recounted: {} {} {found}, listed {}",
                row.source, row.arm, row.count
            ));
        }
    }
    if refusals.is_empty() {
        Ok(())
    } else {
        Err(refusals.join("\n"))
    }
}

/// The text with comments and string literals blanked to spaces and each character literal turned
/// into `0` and spaces, every line break kept (R14).
fn blank(text: &str) -> Vec<char> {
    let chars: Vec<char> = text.chars().collect();
    let mut blanked = chars.clone();
    let mut at = 0;
    while at < chars.len() {
        let next = chars.get(at + 1).copied();
        let (end, literal) = if chars[at] == '/' && next == Some('/') {
            (position(&chars, at, '\n').unwrap_or(chars.len()), false)
        } else if chars[at] == '/' && next == Some('*') {
            (comment_end(&chars, at), false)
        } else if let Some(end) = string_end(&chars, at) {
            (end, false)
        } else if let Some(end) = char_end(&chars, at) {
            (end, true)
        } else {
            at += 1;
            continue;
        };
        for c in &mut blanked[at..end] {
            if *c != '\n' {
                *c = ' ';
            }
        }
        if literal {
            blanked[at] = '0';
        }
        at = end;
    }
    blanked
}

/// The index after the `*/` that closes the comment opening at `open`, nested comments counted.
fn comment_end(chars: &[char], open: usize) -> usize {
    let mut depth = 0_usize;
    let mut at = open;
    while at < chars.len() {
        if starts_with(chars, at, "/*") {
            depth += 1;
            at += 2;
        } else if starts_with(chars, at, "*/") {
            depth -= 1;
            at += 2;
            if depth == 0 {
                return at;
            }
        } else {
            at += 1;
        }
    }
    chars.len()
}

/// The index after the string literal (plain, byte, C or raw) that starts at `at`, if one does.
fn string_end(chars: &[char], at: usize) -> Option<usize> {
    let mut cursor = at;
    if matches!(chars[at], 'b' | 'c' | 'r') && (at == 0 || !is_word(chars[at - 1])) {
        cursor += 1;
        if chars[at] != 'r' && chars.get(cursor) == Some(&'r') {
            cursor += 1;
        }
    }
    if cursor > at && chars[cursor - 1] == 'r' {
        let mut hashes = 0;
        while chars.get(cursor) == Some(&'#') {
            hashes += 1;
            cursor += 1;
        }
        if chars.get(cursor) != Some(&'"') {
            return None;
        }
        let close: Vec<char> = std::iter::once('"')
            .chain(std::iter::repeat_n('#', hashes))
            .collect();
        return Some(
            (cursor + 1..chars.len())
                .find(|&end| chars[end..].starts_with(&close))
                .map_or(chars.len(), |end| end + close.len()),
        );
    }
    if chars.get(cursor) != Some(&'"') {
        return None;
    }
    let mut scan = cursor + 1;
    while scan < chars.len() {
        match chars[scan] {
            '\\' => scan += 2,
            '"' => return Some(scan + 1),
            _ => scan += 1,
        }
    }
    Some(chars.len())
}

/// The index after the character or byte-character literal that starts at `at`, if one does; a
/// lifetime or a label is none.
fn char_end(chars: &[char], at: usize) -> Option<usize> {
    let mut quote = at;
    if chars[at] == 'b' && (at == 0 || !is_word(chars[at - 1])) && chars.get(at + 1) == Some(&'\'')
    {
        quote += 1;
    }
    if chars[quote] != '\'' {
        return None;
    }
    if chars.get(quote + 1) == Some(&'\\') {
        return position(chars, quote + 3, '\'').map(|close| close + 1);
    }
    (chars.get(quote + 2) == Some(&'\'')).then_some(quote + 3)
}

/// Whether `c` is a character of a word.
fn is_word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// The word that starts at `at`, empty when none does.
fn word_from(chars: &[char], at: usize) -> String {
    chars
        .get(at..)
        .unwrap_or_default()
        .iter()
        .take_while(|&&c| is_word(c))
        .collect()
}

/// The first index at or after `at` that is not white space.
fn skip_space(chars: &[char], at: usize) -> usize {
    let mut cursor = at;
    while chars.get(cursor).is_some_and(|c| c.is_whitespace()) {
        cursor += 1;
    }
    cursor
}

/// Whether `text` starts at `at`.
fn starts_with(chars: &[char], at: usize, text: &str) -> bool {
    text.chars()
        .enumerate()
        .all(|(offset, c)| chars.get(at + offset) == Some(&c))
}

/// The next index at or after `from` that holds `c`.
fn position(chars: &[char], from: usize, c: char) -> Option<usize> {
    chars
        .get(from..)?
        .iter()
        .position(|&found| found == c)
        .map(|offset| from + offset)
}

/// The repository's root, two levels above this crate.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The census's sites by source and arm, with how many each pair holds.
fn found(sites: &[ArmSite]) -> BTreeMap<(String, String), usize> {
    let mut found = BTreeMap::new();
    for site in sites {
        *found
            .entry((site.path.clone(), site.arm.to_owned()))
            .or_default() += 1;
    }
    found
}

/// The register's rows by source and arm, with the count each lists.
fn listed(register: &[ArmRow]) -> BTreeMap<(String, String), usize> {
    register
        .iter()
        .map(|row| ((row.source.to_owned(), row.arm.to_owned()), row.count))
        .collect()
}

#[test]
fn each_arm_finds_its_planted_site_and_nothing_else() {
    let plants = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/allow_list_arms/plants.rs.fixture");
    let text = fs::read_to_string(&plants).expect("the planted source is readable");
    let arms = arms_of(&text);

    assert_eq!(
        arms.sites,
        [
            (9, "sql-escape"),
            (10, "env-read"),
            (11, "env-read"),
            (12, "env-read"),
            (13, "env-read"),
            (15, "shell"),
            (16, "shell"),
            (17, "shell"),
            (18, "shell"),
            (19, "shell"),
            (20, "shell"),
            (21, "configured"),
        ],
        "each planted site by its line, and none in the use line, a comment, a literal or env!"
    );
    assert_eq!(
        arms.passes,
        [22],
        "the literal non-shell program is examined and passes"
    );
}

#[test]
fn an_unlisted_recounted_or_stale_arm_row_is_refused() {
    let site = |line: usize, arm: &'static str| ArmSite {
        path: "crates/planted/src/lib.rs".to_owned(),
        line,
        arm,
    };
    let sites = [site(3, "env-read"), site(5, "env-read"), site(9, "shell")];
    let row = |arm: &'static str, count: usize| ArmRow {
        source: "crates/planted/src/lib.rs",
        arm,
        count,
        reason: "a planted reason",
    };

    assert_eq!(
        judge(&sites, &[row("env-read", 2), row("shell", 1)]),
        Ok(()),
        "the positive control: every site listed with its count"
    );
    assert_eq!(
        judge(&sites, &[row("env-read", 2)]).err().as_deref(),
        Some("unlisted: crates/planted/src/lib.rs:9 shell"),
        "a site no row lists"
    );
    assert_eq!(
        judge(&sites, &[row("env-read", 1), row("shell", 1)])
            .err()
            .as_deref(),
        Some("recounted: crates/planted/src/lib.rs env-read 2, listed 1"),
        "a row whose count differs"
    );
    assert_eq!(
        judge(
            &sites,
            &[row("env-read", 2), row("shell", 1), row("sql-escape", 1)]
        )
        .err()
        .as_deref(),
        Some("stale: crates/planted/src/lib.rs sql-escape 1"),
        "a row the census does not reach"
    );
}

#[test]
fn every_arm_site_on_the_tree_is_listed_with_its_reason() {
    let census = census(&root()).expect("the census reads the three source sets");

    assert_eq!(
        judge(&census.sites, &ARMS),
        Ok(()),
        "every SQL escape, shell, configured program and environment read is listed"
    );
    assert_eq!(
        found(&census.sites),
        listed(&ARMS),
        "the sites equal ARMS, file by file and arm by arm"
    );
    assert_eq!(
        census.examined, 349,
        "the three source sets hold 349 sources"
    );
}

#[test]
fn an_empty_source_set_is_void_and_every_source_kind_is_read() {
    let empty = tempfile::tempdir().expect("a temporary directory");

    assert_eq!(
        census(empty.path()).err().as_deref(),
        Some("examined 0 source(s): the population is empty, which is VOID and never a pass"),
        "a tree with no source is refused"
    );
    let tree = tempfile::tempdir().expect("a temporary directory");
    let planted = tree.path().join("crates/planted");
    for (file, text) in [
        (
            "src/lib.rs",
            "fn zone() {\n    let _ = std::env::var_os(\"PLANTED_ZONE\");\n}\n",
        ),
        (
            "build.rs",
            "fn main() {\n    let _ = std::env::var(\"PLANTED_SWITCH\");\n}\n",
        ),
        (
            "examples/probe.rs",
            "fn main() {\n    let _ = std::env::vars();\n}\n",
        ),
    ] {
        let path = planted.join(file);
        fs::create_dir_all(path.parent().expect("a parent directory"))
            .expect("the planted directory is made");
        fs::write(&path, text).expect("a planted source is written");
    }
    let census = census(tree.path()).expect("the census reads the planted sources");
    let sites: Vec<(&str, usize, &str)> = census
        .sites
        .iter()
        .map(|site| (site.path.as_str(), site.line, site.arm))
        .collect();

    assert_eq!(
        sites,
        [
            ("crates/planted/build.rs", 2, "env-read"),
            ("crates/planted/examples/probe.rs", 2, "env-read"),
            ("crates/planted/src/lib.rs", 2, "env-read"),
        ],
        "a source under src/, a build script and an example are each read"
    );
    assert_eq!(census.examined, 3, "the three planted sources are read");
}
