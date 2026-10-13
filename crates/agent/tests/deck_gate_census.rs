//! SPEC-381 A9 (R6): every AI call site passes the deck gate or carries no deck content.
//!
//! The census reads every production source under `crates/*/src` with its comments and string
//! literals blanked, except the agent launch script's name, which it reads as text wherever a
//! source writes it. A source whose path names `drill` or `readings_tree`, in any case, is skipped
//! by its path before any open and only counted (`fenced N`): the drill surface is on hold, and
//! the fail-closed gate in `decide` is the control for a call the census cannot see (SPEC-381
//! section 5, #46).
//!
//! It finds every site of these shapes: the launch script's name, an `impl … Runner for`, a
//! `Command::new(` process launch, an `impl … MemoryPort for`, an MCP `#[tool]`, a production
//! `DutyEngine {` literal, and a runner's `runner.run(` call, which is how the job and instrument
//! runners are found. It holds the sites it found equal to [`DECLARED`], where each is `deck-gate`
//! or `not-deck-content` with its reason, and it pins that `decide` asks the deck gate before
//! `compose(` and before `self.runner.run(`. It refuses each planted fixture under
//! `tests/fixtures/census/` by name before it judges the tree, prints `examined N` and refuses
//! zero, and prints a site as `path:line:shape`, never its text. It follows
//! `crates/mcp/tests/guard_census.rs`.

// An integration test is test code: its helpers panic on a failed fixture, and the counts and
// the sites are printed on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::fs;
use std::path::{Path, PathBuf};

/// The agent launch script's name, read as text wherever a source writes it.
const LAUNCH_SCRIPT: &str = "run-headless.sh";

/// The source `decide` lives in, whose order the census pins.
const DUTY: &str = "crates/agent/src/duty.rs";

/// The deck gate's call, as `decide` makes it.
const GATE_CALL: [&str; 6] = ["self", ".", "deck_gate", ".", "judge", "("];
/// The composition of the prompt.
const COMPOSE_CALL: [&str; 2] = ["compose", "("];
/// The runner's one call.
const RUNNER_CALL: [&str; 6] = ["self", ".", "runner", ".", "run", "("];

/// What a site is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Shape {
    /// The agent launch script's name.
    LaunchScript,
    /// An `impl … Runner for` header.
    RunnerImpl,
    /// A `Command::new(` process launch.
    ProcessLaunch,
    /// An `impl … MemoryPort for` header.
    MemoryPort,
    /// An MCP `#[tool(..)]` or `#[tool]` attribute.
    McpTool,
    /// A `DutyEngine {` literal.
    EngineLiteral,
    /// A runner's `runner.run(` call.
    RunnerCall,
}

/// Every shape, in the order a site is judged.
const SHAPES: [Shape; 7] = [
    Shape::LaunchScript,
    Shape::RunnerImpl,
    Shape::ProcessLaunch,
    Shape::MemoryPort,
    Shape::McpTool,
    Shape::EngineLiteral,
    Shape::RunnerCall,
];

impl Shape {
    /// The shape's name, as a site prints it.
    const fn name(self) -> &'static str {
        match self {
            Self::LaunchScript => "launch-script",
            Self::RunnerImpl => "runner-impl",
            Self::ProcessLaunch => "process-launch",
            Self::MemoryPort => "memory-port",
            Self::McpTool => "mcp-tool",
            Self::EngineLiteral => "engine-literal",
            Self::RunnerCall => "runner-call",
        }
    }
}

/// Why a declared site may stand.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Class {
    /// The site reaches a model only through `decide`, after the deck gate.
    DeckGate,
    /// The site reaches no model, or carries no deck content to one.
    NotDeckContent,
}

/// The sites of one shape one source holds, and why they stand.
struct Declared {
    /// The source, from the workspace root.
    path: &'static str,
    /// The shape.
    shape: Shape,
    /// How many sites of the shape the source holds.
    count: usize,
    /// Whether the deck gate stands before it, or it carries no deck content.
    class: Class,
    /// Why.
    reason: &'static str,
}

/// Every AI call site of the tree, declared.
const DECLARED: [Declared; 13] = [
    Declared {
        path: "crates/agent/src/duty.rs",
        shape: Shape::RunnerCall,
        count: 1,
        class: Class::DeckGate,
        reason: "decide's one runner call, which the census pins after the deck gate",
    },
    Declared {
        path: "crates/agent/src/runner.rs",
        shape: Shape::LaunchScript,
        count: 2,
        class: Class::DeckGate,
        reason: "the process runner names the script it launches; only decide reaches it",
    },
    Declared {
        path: "crates/agent/src/runner.rs",
        shape: Shape::RunnerImpl,
        count: 1,
        class: Class::DeckGate,
        reason: "the one runner that reaches a model; only decide calls it, after the deck gate",
    },
    Declared {
        path: "crates/agent/src/runner.rs",
        shape: Shape::ProcessLaunch,
        count: 1,
        class: Class::DeckGate,
        reason: "the runner's launch of the script; only decide reaches it, after the deck gate",
    },
    Declared {
        path: "crates/agent/src/gate.rs",
        shape: Shape::ProcessLaunch,
        count: 1,
        class: Class::NotDeckContent,
        reason: "the output gate's probe: it runs a pack's check over a reply, and no model",
    },
    Declared {
        path: "crates/vault/src/staged.rs",
        shape: Shape::ProcessLaunch,
        count: 1,
        class: Class::NotDeckContent,
        reason: "the vault gate's probe over a staged note: it runs a pack's check, and no model",
    },
    Declared {
        path: "crates/daemon/src/snapshot_lister.rs",
        shape: Shape::ProcessLaunch,
        count: 1,
        class: Class::NotDeckContent,
        reason: "the snapshot lister: it runs the archive's list command, which returns snapshot names and times",
    },
    Declared {
        path: "crates/daemon/src/role_job.rs",
        shape: Shape::RunnerCall,
        count: 2,
        class: Class::NotDeckContent,
        reason: "the job runner: it runs a scheduled job's work, and no model",
    },
    Declared {
        path: "crates/coordination/src/instruments.rs",
        shape: Shape::RunnerImpl,
        count: 1,
        class: Class::NotDeckContent,
        reason: "the instrument runner: it computes a day's instruments, and no model",
    },
    Declared {
        path: "crates/coordination/src/instruments.rs",
        shape: Shape::RunnerCall,
        count: 1,
        class: Class::NotDeckContent,
        reason: "the instrument runner's call: it computes a day's instruments, and no model",
    },
    Declared {
        path: "crates/ingest/src/skip_write/tests.rs",
        shape: Shape::ProcessLaunch,
        count: 1,
        class: Class::NotDeckContent,
        reason: "a test module: it runs its own test binary again",
    },
    Declared {
        path: "crates/daemon/src/wiring.rs",
        shape: Shape::MemoryPort,
        count: 1,
        class: Class::NotDeckContent,
        reason: "the one memory port: an exercise type and its XP, never a card's text",
    },
    Declared {
        path: "crates/mcp/src/tools.rs",
        shape: Shape::McpTool,
        count: 1,
        class: Class::NotDeckContent,
        reason: "get_law_track: aggregates only (streak, XP, level and counts)",
    },
];

/// Where the planted fixtures live, from the crate's root.
const PLANTS: &str = "tests/fixtures/census";

/// The planted sites, each with the sites the census must refuse in it, as `line:shape`.
const SITE_PLANTS: [(&str, &[&str]); 4] = [
    (
        "a-second-launch.rs.fixture",
        &["10:process-launch", "11:launch-script"],
    ),
    ("an-undeclared-memory.rs.fixture", &["10:memory-port"]),
    ("an-undeclared-runner.rs.fixture", &["11:runner-impl"]),
    ("an-undeclared-tool.rs.fixture", &["10:mcp-tool"]),
];

/// The planted `decide` that reaches the prompt and the runner before the deck gate.
const ORDER_PLANT: &str = "a-runner-before-the-gate.rs.fixture";

/// What the census must say of [`ORDER_PLANT`].
const ORDER_REFUSALS: [&str; 2] = [
    "decide reaches `compose(` before the deck gate",
    "decide reaches `self.runner.run(` before the deck gate",
];

/// A `decide` that asks the deck gate first, which the pin must pass.
const IN_ORDER: &str = "async fn decide(&self) -> u8 {\n    \
    if self.deck_gate.judge(&input.scope).await.refused() {\n        return 0;\n    }\n    \
    let prompt = compose(&input.parts);\n    \
    let reply = self.runner.run(&prompt, &duty.caps).await;\n    1\n}\n";

/// Prints how many items a check examined and refuses zero (the tdd pack's examined contract).
fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}

/// Every `.rs` file under `directory`, in path order. Only names are listed here; no file is
/// opened.
fn rust_files(directory: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut pending = vec![directory.to_path_buf()];
    while let Some(next) = pending.pop() {
        for entry in fs::read_dir(&next).expect("a readable directory") {
            let path = entry.expect("a directory entry").path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

/// Whether a source's path names the drill surface, which the census never opens.
fn fenced_by_name(relative: &str) -> bool {
    let lower = relative.to_ascii_lowercase();
    lower.contains("drill") || lower.contains("readings_tree")
}

/// The workspace's root.
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("the workspace root")
        .to_path_buf()
}

/// Every production source under `crates/*/src`, from the root, but the drill-named ones, and how
/// many drill-named ones were skipped by their path.
fn production_sources(root: &Path) -> (Vec<String>, usize) {
    let mut sources: Vec<PathBuf> = fs::read_dir(root.join("crates"))
        .expect("the crates directory")
        .map(|entry| entry.expect("a crate").path().join("src"))
        .filter(|src| src.is_dir())
        .collect();
    sources.sort();
    let mut kept = Vec::new();
    let mut fenced = 0;
    for src in sources {
        for path in rust_files(&src) {
            let relative = path
                .strip_prefix(root)
                .expect("a source under the root")
                .to_string_lossy()
                .replace('\\', "/");
            if fenced_by_name(&relative) {
                fenced += 1;
            } else {
                kept.push(relative);
            }
        }
    }
    (kept, fenced)
}

/// `text` with every comment and every string or character literal blanked, its line breaks kept,
/// so a line of the code is the text's.
fn code_of(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let blank = |c: char| if c == '\n' { '\n' } else { ' ' };
    let mut out = String::with_capacity(text.len());
    let mut at = 0;
    while at < chars.len() {
        let c = chars[at];
        let next = chars.get(at + 1).copied();
        let after_ident = at > 0 && identifier(chars[at - 1]);
        if c == '/' && next == Some('/') {
            while at < chars.len() && chars[at] != '\n' {
                out.push(' ');
                at += 1;
            }
        } else if c == '/' && next == Some('*') {
            let mut depth = 0_usize;
            while at < chars.len() {
                if chars[at] == '/' && chars.get(at + 1) == Some(&'*') {
                    depth += 1;
                    out.push_str("  ");
                    at += 2;
                } else if chars[at] == '*' && chars.get(at + 1) == Some(&'/') {
                    depth -= 1;
                    out.push_str("  ");
                    at += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    out.push(blank(chars[at]));
                    at += 1;
                }
            }
        } else if c == 'r' && !after_ident && matches!(next, Some('"' | '#')) {
            // A raw string: r"..." or r#"..."#, with as many hashes closing it as opened it.
            let mut hashes = 0;
            let mut cursor = at + 1;
            while chars.get(cursor) == Some(&'#') {
                hashes += 1;
                cursor += 1;
            }
            if chars.get(cursor) == Some(&'"') {
                out.push('r');
                out.extend(std::iter::repeat_n(' ', cursor - at));
                at = cursor + 1;
                while at < chars.len() {
                    let closes = chars[at] == '"'
                        && (1..=hashes).all(|offset| chars.get(at + offset) == Some(&'#'));
                    if closes {
                        out.extend(std::iter::repeat_n(' ', hashes + 1));
                        at += hashes + 1;
                        break;
                    }
                    out.push(blank(chars[at]));
                    at += 1;
                }
            } else {
                out.push(c);
                at += 1;
            }
        } else if c == '"' {
            out.push(' ');
            at += 1;
            while at < chars.len() && chars[at] != '"' {
                let step = if chars[at] == '\\' { 2 } else { 1 };
                for skipped in chars.iter().skip(at).take(step) {
                    out.push(blank(*skipped));
                }
                at += step;
            }
            out.push(' ');
            at += 1;
        } else if c == '\'' && (next == Some('\\') || chars.get(at + 2) == Some(&'\'')) {
            // A character literal; a lifetime (`'a` with no closing quote) is code.
            out.push(' ');
            at += 1;
            while at < chars.len() && chars[at] != '\'' {
                let step = if chars[at] == '\\' { 2 } else { 1 };
                out.extend(std::iter::repeat_n(' ', step));
                at += step;
            }
            out.push(' ');
            at += 1;
        } else {
            out.push(c);
            at += 1;
        }
    }
    out
}

/// Whether `c` can be part of an identifier.
fn identifier(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Where `tokens` end when they start at `at` in `code`, any whitespace between two of them.
fn tokens_at(code: &[char], at: usize, tokens: &[&str]) -> Option<usize> {
    let mut cursor = at;
    for (index, token) in tokens.iter().enumerate() {
        if index > 0 {
            while code.get(cursor).is_some_and(|c| c.is_whitespace()) {
                cursor += 1;
            }
        }
        for expected in token.chars() {
            if code.get(cursor) != Some(&expected) {
                return None;
            }
            cursor += 1;
        }
    }
    Some(cursor)
}

/// Whether the clause that holds `at`, back to the last `;`, `{` or `}`, holds the word `impl`.
fn in_impl_header(code: &[char], at: usize) -> bool {
    let start = code[..at]
        .iter()
        .rposition(|c| matches!(c, ';' | '{' | '}'))
        .map_or(0, |found| found + 1);
    let clause: String = code[start..at].iter().collect();
    clause
        .split(|c: char| !identifier(c))
        .any(|word| word == "impl")
}

/// Whether an `impl … <trait> for` header names a trait ending in `name` at `at`.
fn impl_for(code: &[char], at: usize, name: &str) -> bool {
    let Some(end) = tokens_at(code, at, &[name]) else {
        return false;
    };
    let mut cursor = end;
    while code.get(cursor).is_some_and(|c| c.is_whitespace()) {
        cursor += 1;
    }
    cursor > end
        && tokens_at(code, cursor, &["for"])
            .is_some_and(|after| !code.get(after).copied().is_some_and(identifier))
        && in_impl_header(code, at)
}

/// Whether a `DutyEngine {` literal starts at `at`, rather than its declaration.
fn engine_literal(code: &[char], at: usize) -> bool {
    if at > 0 && identifier(code[at - 1]) {
        return false;
    }
    if tokens_at(code, at, &["DutyEngine", "{"]).is_none() {
        return false;
    }
    let before: String = code[..at].iter().collect();
    let previous = before
        .split(|c: char| !identifier(c))
        .rfind(|word| !word.is_empty());
    !matches!(
        previous,
        Some("struct" | "enum" | "union" | "impl" | "for" | "trait")
    )
}

/// Whether a site of `shape` starts at `at` in the blanked `code`.
fn site_at(code: &[char], at: usize, shape: Shape) -> bool {
    match shape {
        Shape::LaunchScript => false,
        Shape::RunnerImpl => code[at] == 'R' && impl_for(code, at, "Runner"),
        Shape::MemoryPort => code[at] == 'M' && impl_for(code, at, "MemoryPort"),
        Shape::ProcessLaunch => tokens_at(code, at, &["Command", "::", "new", "("]).is_some(),
        Shape::McpTool => tokens_at(code, at, &["#", "[", "tool"]).is_some_and(|end| {
            let mut cursor = end;
            while code.get(cursor).is_some_and(|c| c.is_whitespace()) {
                cursor += 1;
            }
            matches!(code.get(cursor), Some('(' | ']'))
        }),
        Shape::EngineLiteral => code[at] == 'D' && engine_literal(code, at),
        Shape::RunnerCall => tokens_at(code, at, &["runner", ".", "run", "("]).is_some(),
    }
}

/// Every site of `text`, as `(line, shape)`, in line order.
fn sites(text: &str) -> Vec<(usize, Shape)> {
    let mut found: Vec<(usize, Shape)> = text
        .lines()
        .enumerate()
        .flat_map(|(index, line)| {
            std::iter::repeat_n(
                (index + 1, Shape::LaunchScript),
                line.matches(LAUNCH_SCRIPT).count(),
            )
        })
        .collect();
    let code: Vec<char> = code_of(text).chars().collect();
    let mut line = 1;
    for (at, c) in code.iter().enumerate() {
        if *c == '\n' {
            line += 1;
            continue;
        }
        for shape in SHAPES {
            if site_at(&code, at, shape) {
                found.push((line, shape));
            }
        }
    }
    found.sort_unstable();
    found
}

/// How many sites of `shape` [`DECLARED`] declares for `path`.
fn declared_count(path: &str, shape: Shape) -> usize {
    DECLARED
        .iter()
        .filter(|declared| declared.path == path && declared.shape == shape)
        .map(|declared| declared.count)
        .sum()
}

/// What the census refuses in `path`'s `found` sites: each undeclared site as
/// `path:line:shape: undeclared`, and a declared count the source no longer holds.
fn judge(path: &str, found: &[(usize, Shape)]) -> Vec<String> {
    let mut undeclared: Vec<(usize, Shape)> = Vec::new();
    let mut miscounted = Vec::new();
    for shape in SHAPES {
        let lines: Vec<usize> = found
            .iter()
            .filter(|(_, site)| *site == shape)
            .map(|(line, _)| *line)
            .collect();
        let declared = declared_count(path, shape);
        if lines.len() == declared {
            continue;
        }
        if declared == 0 {
            undeclared.extend(lines.iter().map(|line| (*line, shape)));
        } else {
            miscounted.push(format!(
                "{path}:{}: declared {declared}, found {} at lines {lines:?}",
                shape.name(),
                lines.len()
            ));
        }
    }
    undeclared.sort_unstable();
    undeclared
        .into_iter()
        .map(|(line, shape)| format!("{path}:{line}:{}: undeclared", shape.name()))
        .chain(miscounted)
        .collect()
}

/// The index just past the bracket that closes the one at `open`.
fn closing(code: &[char], open: usize) -> usize {
    let mut depth = 0_usize;
    for (at, c) in code.iter().enumerate().skip(open) {
        if *c == '{' {
            depth += 1;
        } else if *c == '}' {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                return at + 1;
            }
        }
    }
    code.len()
}

/// The first place `tokens` start in `code` at an identifier's start.
fn first(code: &[char], tokens: &[&str]) -> Option<usize> {
    (0..code.len())
        .find(|&at| (at == 0 || !identifier(code[at - 1])) && tokens_at(code, at, tokens).is_some())
}

/// The body of the first `fn decide` in `code`, its braces included.
fn decide_body(code: &[char]) -> Option<&[char]> {
    let named = (0..code.len()).find(|&at| {
        (at == 0 || !identifier(code[at - 1]))
            && tokens_at(code, at, &["fn", "decide"])
                .is_some_and(|end| !code.get(end).copied().is_some_and(identifier))
    })?;
    let open = (named..code.len()).find(|&at| code[at] == '{')?;
    Some(&code[open..closing(code, open)])
}

/// What the pin refuses in `text`'s `decide`: no deck gate, or `compose(` or the runner's call
/// reached before it.
fn gate_order(text: &str) -> Vec<String> {
    let code: Vec<char> = code_of(text).chars().collect();
    let Some(body) = decide_body(&code) else {
        return vec!["no `fn decide` to pin".to_owned()];
    };
    let Some(gate) = first(body, &GATE_CALL) else {
        return vec!["decide calls no deck gate (`self.deck_gate.judge(`)".to_owned()];
    };
    let mut findings = Vec::new();
    for (name, tokens) in [
        ("compose(", &COMPOSE_CALL[..]),
        ("self.runner.run(", &RUNNER_CALL[..]),
    ] {
        match first(body, tokens) {
            None => findings.push(format!("decide makes no `{name}` call")),
            Some(at) if at < gate => {
                findings.push(format!("decide reaches `{name}` before the deck gate"));
            }
            Some(_) => {}
        }
    }
    findings
}

/// What the census refuses in its own declarations: a repeated entry, an empty count or reason, a
/// deck-gate entry outside the agent, or a declared source the census never opens.
fn declaration_findings() -> Vec<String> {
    let mut findings = Vec::new();
    for (index, declared) in DECLARED.iter().enumerate() {
        let name = format!("{}:{}", declared.path, declared.shape.name());
        if DECLARED[..index]
            .iter()
            .any(|earlier| earlier.path == declared.path && earlier.shape == declared.shape)
        {
            findings.push(format!("{name}: declared twice"));
        }
        if declared.count == 0 || declared.reason.trim().is_empty() {
            findings.push(format!("{name}: declared with no count or no reason"));
        }
        if declared.class == Class::DeckGate && !declared.path.starts_with("crates/agent/src/") {
            findings.push(format!(
                "{name}: a deck-gate site outside the agent's runner"
            ));
        }
        if fenced_by_name(declared.path) {
            findings.push(format!("{name}: a declared source the census never opens"));
        }
    }
    findings
}

/// The census refuses each planted fixture by name, and passes a `decide` in order.
fn refuses_every_plant_by_name() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut names: Vec<String> = fs::read_dir(crate_root.join(PLANTS))
        .expect("the plants")
        .map(|entry| {
            entry
                .expect("a plant")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    let mut expected: Vec<&str> = SITE_PLANTS
        .iter()
        .map(|(name, _)| *name)
        .chain([ORDER_PLANT])
        .collect();
    expected.sort_unstable();
    assert_eq!(examined("plant(s)", names), expected);

    for (name, refusals) in SITE_PLANTS {
        let path = format!("{PLANTS}/{name}");
        let text = fs::read_to_string(crate_root.join(&path)).expect("a readable plant");
        let refused: Vec<String> = refusals
            .iter()
            .map(|site| format!("{path}:{site}: undeclared"))
            .collect();
        assert_eq!(
            judge(&path, &sites(&text)),
            refused,
            "the census must refuse {name} by name"
        );
    }

    let text =
        fs::read_to_string(crate_root.join(PLANTS).join(ORDER_PLANT)).expect("a readable plant");
    assert_eq!(
        gate_order(&text),
        ORDER_REFUSALS,
        "the pin must refuse {ORDER_PLANT}"
    );
    assert_eq!(gate_order(IN_ORDER), Vec::<String>::new());
}

#[test]
fn every_ai_call_site_passes_the_deck_gate_or_carries_no_deck_content() {
    refuses_every_plant_by_name();

    let root = workspace_root();
    let (sources, fenced) = production_sources(&root);
    let sources = examined("production source(s)", sources);
    println!("fenced {fenced} drill-named source(s), skipped by their path and never opened");

    let mut findings = declaration_findings();
    for path in &sources {
        let text = fs::read_to_string(root.join(path)).expect("a readable source");
        let found = sites(&text);
        for (line, shape) in &found {
            println!("{path}:{line}:{}", shape.name());
        }
        findings.extend(judge(path, &found));
        if path == DUTY {
            findings.extend(
                gate_order(&text)
                    .into_iter()
                    .map(|finding| format!("{path}: {finding}")),
            );
        }
    }
    for declared in &DECLARED {
        if !sources.iter().any(|path| path == declared.path) {
            findings.push(format!(
                "{}:{}: declared, and the source was not examined",
                declared.path,
                declared.shape.name()
            ));
        }
    }
    if !sources.iter().any(|path| path == DUTY) {
        findings.push(format!("{DUTY}: not examined, so decide is not pinned"));
    }
    assert_eq!(findings, Vec::<String>::new(), "the census refused");
}
