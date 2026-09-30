//! Request capability is named by the compiler's resolved path, and the allow-list is audited
//! (SPEC-041 A18; #429).
//!
//! The census of `one_router.rs` (A15 to A17) reads the Bot API's method names as text, and text
//! cannot resolve a name: reqwest's client bound to a fresh name, or imported by its own type name,
//! names none of the tokens the census holds. Clippy resolves the path of every use, through a
//! `use` or a `type` alias, so `clippy.toml` names the four paths through which reqwest's client is
//! made (`disallowed-types` and `disallowed-methods`), and the workspace's `-D warnings` clippy
//! stage refuses a use of one anywhere. The legitimate transport site carries a scoped
//! `#[expect(..., reason = "...")]`, and those annotations are the allow-list this file audits:
//!
//! - `clippy.toml` names all four paths, and holds no `allow-invalid` entry;
//! - the only suppression of either lint anywhere in the workspace is an `#[expect]` with a reason
//!   at a named transport site;
//! - no lint group that holds them is suppressed, in any spelling a crate accepts: `clippy::style`,
//!   `clippy::all` and `warnings`, by an attribute (outer, inner, `cfg_attr`) or by a manifest's
//!   `[lints]` table or a compiler flag.
//!
//! The audit reads files as text, which is right here: it reads the allow-list, and clippy does the
//! resolving. Its population is generated: every spelling of a lint, in every form of an
//! attribute, at every place, planted in the bot's shipped sources.

#![allow(clippy::expect_used, clippy::print_stdout)]

use std::fs;
use std::path::{Path, PathBuf};

/// The transport, the one file that makes the bot's HTTP client.
const TRANSPORT: &str = "crates/bot/src/transport.rs";

/// The four paths `clippy.toml` names, each under the key that holds it.
const CLIPPY_RULES: [(&str, &str); 4] = [
    ("disallowed-types", "reqwest::Client"),
    ("disallowed-methods", "reqwest::get"),
    ("disallowed-methods", "reqwest::Client::new"),
    ("disallowed-methods", "reqwest::Client::builder"),
];

/// The two lints, as an attribute names them.
const LINTS: [&str; 2] = ["clippy::disallowed_types", "clippy::disallowed_methods"];

/// Every name that, in an `allow` or an `expect`, silences one of the two lints: the lints, and
/// each group that holds them (`style`, which `all` and `warnings` both contain).
const SILENCING: [&str; 5] = [
    "clippy::disallowed_types",
    "clippy::disallowed_methods",
    "clippy::style",
    "clippy::all",
    "warnings",
];

/// The named sites: the file, the lint, and the statement the attribute sits on. The transport
/// builds reqwest's client once, in its constructor, and hands it to the pinned client.
const SITES: [(&str, &str, &str); 2] = [
    (
        TRANSPORT,
        "clippy::disallowed_types",
        "let client = reqwest::Client::builder()",
    ),
    (
        TRANSPORT,
        "clippy::disallowed_methods",
        "let client = reqwest::Client::builder()",
    ),
];

/// A suppression of one of the two lints, or of a group that holds them.
#[derive(Debug, PartialEq)]
struct Suppression {
    line: usize,
    inner: bool,
    through_cfg_attr: bool,
    level: String,
    lints: Vec<String>,
    /// The attribute's arguments other than the lints and a `reason`.
    others: usize,
    reason: Option<String>,
    /// The first line of the statement or item the attribute sits on.
    header: String,
}

/// `text` with its comments blanked and its string and character literals emptied to `x`, with the
/// same lines and the same length in characters, so what remains is code and the quotes around a
/// literal. A whitespace-only literal stays blank, so an empty reason reads as empty.
fn code_view(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut at = 0;
    while at < chars.len() {
        let c = chars[at];
        let next = chars.get(at + 1).copied();
        if c == '/' && next == Some('/') {
            while at < chars.len() && chars[at] != '\n' {
                out.push(' ');
                at += 1;
            }
        } else if c == '/' && next == Some('*') {
            let mut depth = 0;
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
                    out.push(if chars[at] == '\n' { '\n' } else { ' ' });
                    at += 1;
                }
            }
        } else if let Some((quote, hashes)) = raw_string(&chars, at) {
            out.extend(&chars[at..=quote]);
            at = quote + 1;
            while at < chars.len() && !raw_end(&chars, at, hashes) {
                out.push(emptied(chars[at]));
                at += 1;
            }
            let end = (at + 1 + hashes).min(chars.len());
            out.extend(&chars[at..end]);
            at = end;
        } else if c == '"' {
            out.push('"');
            at += 1;
            while at < chars.len() && chars[at] != '"' {
                if chars[at] == '\\' && at + 1 < chars.len() {
                    out.push('x');
                    out.push(emptied(chars[at + 1]));
                    at += 2;
                } else {
                    out.push(emptied(chars[at]));
                    at += 1;
                }
            }
            if at < chars.len() {
                out.push('"');
                at += 1;
            }
        } else if c == '\'' && char_literal_end(&chars, at).is_some() {
            let end = char_literal_end(&chars, at).unwrap_or(at + 1);
            out.push('\'');
            out.extend(chars[at + 1..end - 1].iter().map(|_| 'x'));
            out.push('\'');
            at = end;
        } else {
            out.push(c);
            at += 1;
        }
    }
    out
}

/// A literal's character, kept as a line break or as a space, else `x`.
fn emptied(c: char) -> char {
    if c == '\n' {
        '\n'
    } else if c.is_whitespace() {
        ' '
    } else {
        'x'
    }
}

/// The opening quote and the `#` count of a raw string that starts at `at`.
fn raw_string(chars: &[char], at: usize) -> Option<(usize, usize)> {
    let word = at == 0 || !(chars[at - 1].is_alphanumeric() || chars[at - 1] == '_');
    let start = if chars[at] == 'b' && chars.get(at + 1) == Some(&'r') {
        at + 1
    } else {
        at
    };
    if !word || chars[start] != 'r' {
        return None;
    }
    let mut hashes = 0;
    while chars.get(start + 1 + hashes) == Some(&'#') {
        hashes += 1;
    }
    (chars.get(start + 1 + hashes) == Some(&'"')).then_some((start + 1 + hashes, hashes))
}

/// Whether a raw string of `hashes` `#`s closes at `at`.
fn raw_end(chars: &[char], at: usize, hashes: usize) -> bool {
    chars[at] == '"' && (1..=hashes).all(|n| chars.get(at + n) == Some(&'#'))
}

/// The character past a character literal that starts at `at`, or none for a lifetime.
fn char_literal_end(chars: &[char], at: usize) -> Option<usize> {
    if chars.get(at + 1) == Some(&'\\') {
        let close = (at + 2..chars.len().min(at + 12)).find(|&n| chars[n] == '\'')?;
        return Some(close + 1);
    }
    (chars.get(at + 2) == Some(&'\'') && chars.get(at + 1) != Some(&'\'')).then_some(at + 3)
}

/// The line, 1-based, of the character at `at` in `text`.
fn line_of(text: &[char], at: usize) -> usize {
    1 + text[..at].iter().filter(|&&c| c == '\n').count()
}

/// Every suppression of the two lints, or of a group that holds them, in the Rust `text`.
fn suppressions(text: &str) -> Vec<Suppression> {
    let code: Vec<char> = code_view(text).chars().collect();
    let mut found = Vec::new();
    let mut at = 0;
    while at < code.len() {
        if code[at] != '#' {
            at += 1;
            continue;
        }
        let mut open = at + 1;
        let inner = code.get(open) == Some(&'!');
        if inner {
            open += 1;
        }
        while code.get(open).is_some_and(|c| c.is_whitespace()) {
            open += 1;
        }
        if code.get(open) != Some(&'[') {
            at += 1;
            continue;
        }
        let close = matching(&code, open, '[', ']');
        let body: String = code[open + 1..close]
            .iter()
            .filter(|c| !c.is_whitespace())
            .collect();
        for (level, arguments) in levels(&body) {
            let lints: Vec<String> = arguments
                .iter()
                .filter(|argument| names_a_silenced_lint(argument))
                .cloned()
                .collect();
            if lints.is_empty() {
                continue;
            }
            let reason = arguments
                .iter()
                .find_map(|argument| argument.strip_prefix("reason="))
                .map(str::to_string);
            let others = arguments
                .iter()
                .filter(|argument| {
                    !names_a_silenced_lint(argument) && !argument.starts_with("reason=")
                })
                .count();
            found.push(Suppression {
                line: line_of(&code, at),
                inner,
                through_cfg_attr: body.starts_with("cfg_attr("),
                level,
                lints,
                others,
                reason,
                header: header_after(&code, close + 1),
            });
        }
        at = close + 1;
    }
    found
}

/// The index of the bracket that closes the one at `open`.
fn matching(code: &[char], open: usize, opener: char, closer: char) -> usize {
    let mut depth = 0;
    for (at, &c) in code.iter().enumerate().skip(open) {
        if c == opener {
            depth += 1;
        } else if c == closer {
            depth -= 1;
            if depth == 0 {
                return at;
            }
        }
    }
    code.len().saturating_sub(1)
}

/// Whether one argument of an attribute names one of the two lints or a group that holds them.
fn names_a_silenced_lint(argument: &str) -> bool {
    SILENCING.contains(&argument)
        || ["disallowed_types", "disallowed_methods", "style", "all"].contains(&argument)
}

/// Every `allow(..)` and `expect(..)` in an attribute's whitespace-free `body`, at any depth (a
/// `cfg_attr` holds them inside its own parentheses), each with its top-level arguments.
fn levels(body: &str) -> Vec<(String, Vec<String>)> {
    let chars: Vec<char> = body.chars().collect();
    let mut found = Vec::new();
    for level in ["allow", "expect"] {
        let needle: Vec<char> = format!("{level}(").chars().collect();
        for at in 0..chars.len().saturating_sub(needle.len() - 1) {
            let starts = at == 0 || matches!(chars[at - 1], ',' | '(');
            if starts && chars[at..at + needle.len()] == needle[..] {
                let open = at + needle.len() - 1;
                let close = matching(&chars, open, '(', ')');
                found.push((level.to_string(), split(&chars[open + 1..close])));
            }
        }
    }
    found
}

/// `text` cut at its top-level commas.
fn split(text: &[char]) -> Vec<String> {
    let mut parts = vec![String::new()];
    let mut depth = 0;
    for &c in text {
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' => depth -= 1,
            ',' if depth == 0 => {
                parts.push(String::new());
                continue;
            }
            _ => {}
        }
        if let Some(part) = parts.last_mut() {
            part.push(c);
        }
    }
    parts.retain(|part| !part.is_empty());
    parts
}

/// The first line of the statement or item at `at`, past any further attributes, with its
/// whitespace collapsed.
fn header_after(code: &[char], mut at: usize) -> String {
    loop {
        while code.get(at).is_some_and(|c| c.is_whitespace()) {
            at += 1;
        }
        if code.get(at) == Some(&'#') {
            let mut open = at + 1;
            if code.get(open) == Some(&'!') {
                open += 1;
            }
            if code.get(open) == Some(&'[') {
                at = matching(code, open, '[', ']') + 1;
                continue;
            }
        }
        break;
    }
    let line: String = code[at.min(code.len())..]
        .iter()
        .take_while(|&&c| c != '\n')
        .collect();
    line.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The refusals of one Rust source, and the named sites it carries as (lint, header).
fn audit_source(path: &str, text: &str) -> (Vec<String>, Vec<(String, String)>) {
    let mut refused = Vec::new();
    let mut sites = Vec::new();
    for suppression in suppressions(text) {
        let lint = suppression.lints.first().map_or("", String::as_str);
        let one_lint = suppression.lints.len() == 1 && LINTS.contains(&lint);
        let has_reason = suppression
            .reason
            .as_deref()
            .is_some_and(|reason| reason.trim_matches('"').chars().any(|c| c != ' '));
        let a_named_site = one_lint
            && SITES.contains(&(path, lint, suppression.header.as_str()))
            && suppression.level == "expect"
            && !suppression.inner
            && !suppression.through_cfg_attr
            && suppression.others == 0
            && has_reason;
        if a_named_site {
            sites.push((lint.to_string(), suppression.header));
        } else {
            refused.push(format!(
                "{path}:{}: {} of {} is not an `#[expect]` with a reason at a named site",
                suppression.line,
                suppression.level,
                suppression.lints.join(", ")
            ));
        }
    }
    (refused, sites)
}

/// The strings of the array under `key` in a `clippy.toml`, each a path (a bare element, or the
/// `path` of an inline table), with comments left out.
fn clippy_paths(text: &str, key: &str) -> Vec<String> {
    let code: Vec<char> = toml_code(text).chars().collect();
    let Some(start) = toml_key(&code, key) else {
        return Vec::new();
    };
    let Some(open) = (start..code.len()).find(|&n| code[n] == '[') else {
        return Vec::new();
    };
    let mut paths = Vec::new();
    let (mut at, mut depth) = (open, 0);
    while at < code.len() {
        match code[at] {
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            '"' | '\'' => {
                let quote = code[at];
                let end = (at + 1..code.len())
                    .find(|&n| code[n] == quote)
                    .unwrap_or(code.len() - 1);
                let value: String = code[at + 1..end].iter().collect();
                if string_is_a_path(&code, at) {
                    paths.push(value);
                }
                at = end;
            }
            _ => {}
        }
        at += 1;
    }
    paths
}

/// Whether the string at `at` is a path: a bare element, or the value of a `path` key.
fn string_is_a_path(code: &[char], at: usize) -> bool {
    let before: String = code[..at].iter().collect();
    let before = before.trim_end();
    match before.strip_suffix('=') {
        None => true,
        Some(key) => key.trim_end().ends_with("path"),
    }
}

/// The index just past `key =` at the start of a line, when the file sets it.
fn toml_key(code: &[char], key: &str) -> Option<usize> {
    let text: String = code.iter().collect();
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        let rest = line.trim_start();
        if let Some(after) = rest.strip_prefix(key)
            && after.trim_start().starts_with('=')
        {
            let position = offset + (line.len() - rest.len()) + key.len();
            return Some(text[..position].chars().count());
        }
        offset += line.len();
    }
    None
}

/// `text` with its TOML comments blanked, and its strings kept.
fn toml_code(text: &str) -> String {
    let mut out = String::new();
    for line in text.split_inclusive('\n') {
        let mut quote = None;
        let mut kept = String::new();
        for c in line.chars() {
            match (quote, c) {
                (None, '#') => break,
                (None, '"' | '\'') => quote = Some(c),
                (Some(open), _) if open == c => quote = None,
                _ => {}
            }
            kept.push(c);
        }
        out.push_str(&kept);
        if line.ends_with('\n') {
            out.push('\n');
        }
    }
    out
}

/// The refusals of a `clippy.toml`: a path it no longer names, and an `allow-invalid` entry, which
/// would let a path that resolves to nothing pass silently.
fn clippy_refusals(text: &str) -> Vec<String> {
    let mut refused = Vec::new();
    for (key, path) in CLIPPY_RULES {
        if !clippy_paths(text, key).iter().any(|named| named == path) {
            refused.push(format!("clippy.toml does not name {path} under {key}"));
        }
    }
    if toml_code(text).contains("allow-invalid") {
        refused.push("clippy.toml carries an allow-invalid entry".to_string());
    }
    refused
}

/// The refusals of a manifest that sets a group holding the two lints, or one of them, to allow.
fn manifest_refusals(path: &str, text: &str) -> Vec<String> {
    let mut refused = Vec::new();
    let mut in_lints = false;
    for (number, line) in toml_code(text).lines().enumerate() {
        let line = line.trim();
        if line.starts_with('[') {
            in_lints = line.contains("lints");
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim().trim_matches('"');
        let key = key.rsplit('.').next().unwrap_or(key).trim_matches('"');
        let silenced = ["disallowed_types", "disallowed_methods", "style", "all"].contains(&key);
        let lowers = value.contains("\"allow\"") || value.contains("\"expect\"");
        if in_lints && silenced && lowers {
            refused.push(format!("{path}:{}: sets {key} to allow", number + 1));
        }
    }
    refused
}

/// The refusals of a script, workflow or configuration file whose compiler flags silence a lint of
/// the two or a group that holds them, or cap every lint.
fn flag_refusals(path: &str, text: &str) -> Vec<String> {
    let mut refused = Vec::new();
    for (number, line) in text.lines().enumerate() {
        let tokens: Vec<&str> = line
            .split(|c: char| c.is_whitespace() || matches!(c, '"' | '\'' | ',' | '[' | ']'))
            .filter(|token| !token.is_empty())
            .collect();
        for (at, token) in tokens.iter().enumerate() {
            let spec = if let Some(rest) = token.strip_prefix("--allow") {
                Some(
                    rest.strip_prefix('=')
                        .map_or_else(|| tokens.get(at + 1).copied().unwrap_or(""), |value| value),
                )
            } else if let Some(rest) = token.strip_prefix("-A") {
                Some(if rest.is_empty() {
                    tokens.get(at + 1).copied().unwrap_or("")
                } else {
                    rest
                })
            } else {
                None
            };
            let caps = token.starts_with("--cap-lints");
            if caps || spec.is_some_and(names_a_silenced_lint) {
                refused.push(format!("{path}:{}: a flag silences the lints", number + 1));
            }
        }
    }
    refused
}

/// Every file of the workspace that can carry a suppression: its Rust sources and manifests, and
/// the scripts, workflows and configuration that pass compiler flags, as (relative path, text).
fn workspace_files(root: &Path) -> Vec<(String, String)> {
    let mut found = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(&directory).expect("a readable directory") {
            let entry = entry.expect("a directory entry");
            let kind = entry.file_type().expect("a file type");
            let name = entry.file_name().to_string_lossy().into_owned();
            let path = entry.path();
            if kind.is_dir() {
                if !["target", ".git", "node_modules"].contains(&name.as_str()) {
                    pending.push(path);
                }
                continue;
            }
            let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if kind.is_file()
                && ["rs", "toml", "sh", "yml", "yaml"].contains(&extension)
                && let Ok(text) = fs::read_to_string(&path)
            {
                let relative = path.strip_prefix(root).expect("under the root");
                found.push((relative.to_string_lossy().replace('\\', "/"), text));
            }
        }
    }
    found.sort();
    examined("workspace files that can carry a suppression", found)
}

/// Prints how many items a check examined and refuses zero (the tdd pack's examined contract).
fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}

/// The refusals of a set of files, and the named sites found in them.
fn audit(files: &[(String, String)]) -> (Vec<String>, Vec<(String, String, String)>) {
    let mut refused = Vec::new();
    let mut sites = Vec::new();
    for (path, text) in files {
        if is_rust(path) {
            let (faults, found) = audit_source(path, text);
            refused.extend(faults);
            sites.extend(
                found
                    .into_iter()
                    .map(|(lint, header)| (path.clone(), lint, header)),
            );
        } else if path.ends_with("Cargo.toml") {
            refused.extend(manifest_refusals(path, text));
        } else if path != "clippy.toml" {
            refused.extend(flag_refusals(path, text));
        }
    }
    (refused, sites)
}

/// Whether `path` names a Rust source.
fn is_rust(path: &str) -> bool {
    Path::new(path)
        .extension()
        .is_some_and(|extension| extension == "rs")
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the workspace root")
}

/// A lint's spellings: each name that silences, and the whitespace an attribute may carry inside it.
fn spellings() -> Vec<String> {
    let mut all: Vec<String> = SILENCING.iter().map(|lint| (*lint).to_string()).collect();
    all.push("clippy :: style".to_string());
    all.push("clippy::\n        disallowed_methods".to_string());
    all.push("disallowed_types".to_string());
    all
}

/// The forms an attribute takes around a lint spelling `lint`, as (form, is an inner attribute).
fn forms(lint: &str) -> Vec<(String, bool)> {
    vec![
        (format!("#[allow({lint})]"), false),
        (format!("#[expect({lint})]"), false),
        (format!("#[expect({lint}, reason = \"a reason\")]"), false),
        (format!("#[allow({lint}, reason = \"a reason\")]"), false),
        (format!("#[allow(unused, {lint})]"), false),
        (format!("#[allow({lint}, unused)]"), false),
        (format!("#[allow(\n    {lint},\n)]"), false),
        (format!("#[cfg_attr(test, allow({lint}))]"), false),
        (
            format!("#[cfg_attr(all(), expect({lint}, reason = \"r\"))]"),
            false,
        ),
        (format!("#![allow({lint})]"), true),
        (format!("#![expect({lint}, reason = \"a reason\")]"), true),
        (format!("#![cfg_attr(not(test), allow({lint}))]"), true),
    ]
}

/// A source with `attribute` planted: an inner attribute at the top, an outer one before a
/// function and before a statement in it (the three places a suppression takes hold).
fn plants(base: &str, attribute: &str, inner: bool) -> Vec<String> {
    if inner {
        return vec![format!("{attribute}\n{base}")];
    }
    vec![
        format!("{base}\n{attribute}\nfn planted() {{\n    let _ = 1;\n}}\n"),
        format!("{base}\nfn planted() {{\n    {attribute}\n    let _ = 1;\n}}\n"),
        format!(
            "{base}\nfn planted() {{\n    {attribute}\n    let client = reqwest::Client::new();\n}}\n"
        ),
    ]
}

#[test]
fn clippy_names_the_four_paths_that_make_reqwests_client() {
    let text = fs::read_to_string(root().join("clippy.toml")).expect("clippy.toml");
    assert_eq!(clippy_refusals(&text), Vec::<String>::new());
    assert_eq!(
        CLIPPY_RULES
            .iter()
            .filter(|(key, path)| clippy_paths(&text, key).iter().any(|named| named == path))
            .count(),
        4,
        "each of the four paths is named under its key"
    );

    // The population: each entry removed, commented out and moved under the other key, and an
    // `allow-invalid` added.
    let mut planted = 0;
    for (key, path) in CLIPPY_RULES {
        for member in [
            text.replacen(&format!("\"{path}\""), "\"unrelated::Path\"", 1),
            text.replacen(&format!("\"{path}\""), &format!("# \"{path}\""), 1),
            text.replacen(
                &format!("\"{path}\""),
                &format!("{{ path = \"{path}\", allow-invalid = true }}"),
                1,
            ),
            text.replacen(&format!("{key} ="), &format!("# {key} ="), 1),
        ] {
            assert_ne!(member, text, "{path} under {key} is planted");
            assert!(
                !clippy_refusals(&member).is_empty(),
                "{path} under {key}: a clippy.toml that lost it is refused"
            );
            planted += 1;
        }
    }
    assert_eq!(planted, 16, "four members of each of the four paths");
}

#[test]
fn the_only_suppression_of_the_rule_is_an_expect_at_a_named_transport_site() {
    let root = root();
    let files = workspace_files(&root);
    let (refused, sites) = audit(&files);
    assert_eq!(refused, Vec::<String>::new());
    let mut expected: Vec<(String, String, String)> = SITES
        .iter()
        .map(|(path, lint, header)| {
            (
                (*path).to_string(),
                (*lint).to_string(),
                (*header).to_string(),
            )
        })
        .collect();
    expected.sort();
    let mut found = sites;
    found.sort();
    assert_eq!(
        found, expected,
        "each named site is found once, and only they"
    );
    assert!(
        files.iter().any(|(path, _)| path == TRANSPORT),
        "the transport is among the {} files examined",
        files.len()
    );
    assert!(
        files.iter().filter(|(path, _)| is_rust(path)).count() > 100,
        "the census examined the workspace's Rust sources"
    );
}

#[test]
fn a_suppression_of_the_rule_or_of_its_group_is_refused_wherever_it_is_planted() {
    let root = root();
    let sources: Vec<(String, String)> = workspace_files(&root)
        .into_iter()
        .filter(|(path, _)| path.starts_with("crates/bot/src/") && is_rust(path))
        .collect();
    let on_disk = std::fs::read_dir(root.join("crates/bot/src"))
        .expect("the bot's source directory reads")
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "rs"))
        .count();
    assert!(on_disk > 0, "the bot has sources to examine");
    assert_eq!(
        sources.len(),
        on_disk,
        "every one of the bot's sources on disk is examined"
    );
    assert!(
        sources.iter().any(|(path, _)| path.ends_with("transport.rs")),
        "the transport source is among those examined"
    );

    let mut members = 0;
    let mut refused = 0;
    for (path, base) in &sources {
        assert_eq!(
            audit_source(path, base).0,
            Vec::<String>::new(),
            "{path} is clean"
        );
        for spelling in spellings() {
            for (attribute, inner) in forms(&spelling) {
                for planted in plants(base, &attribute, inner) {
                    members += 1;
                    let (faults, _) = audit_source(path, &planted);
                    if faults.iter().any(|fault| fault.contains("named site")) {
                        refused += 1;
                    }
                }
            }
        }
    }
    let population =
        sources.len() * spellings().len() * 12 * 3 - sources.len() * spellings().len() * 3 * 2;
    assert_eq!(
        members, population,
        "the population is generated, not listed"
    );
    assert_eq!(
        refused, members,
        "{refused} of {members} planted suppressions are refused"
    );
}

#[test]
fn a_suppression_in_a_manifest_or_a_flag_is_refused_and_a_look_alike_is_not() {
    let root = root();
    let files = workspace_files(&root);
    let manifests = files
        .iter()
        .filter(|(path, _)| path.ends_with("Cargo.toml"))
        .count();
    assert!(
        manifests > 10,
        "the workspace's {manifests} manifests are examined"
    );

    let mut members = 0;
    for name in ["disallowed_types", "disallowed_methods", "style", "all"] {
        for member in [
            format!("[lints.clippy]\n{name} = \"allow\"\n"),
            format!("[workspace.lints.clippy]\n{name} = {{ level = \"allow\", priority = -1 }}\n"),
            format!("[lints]\nclippy.{name} = \"allow\"\n"),
        ] {
            members += 1;
            assert!(
                !manifest_refusals("Cargo.toml", &member).is_empty(),
                "{member}"
            );
        }
    }
    for flag in [
        "cargo clippy -- -A clippy::disallowed_methods",
        "cargo clippy -- -Aclippy::disallowed_types",
        "cargo clippy -- --allow clippy::style",
        "cargo clippy -- --allow=clippy::all",
        "  rustflags = [\"-A\", \"warnings\"]",
        "  rustflags = [\"--cap-lints\", \"allow\"]",
    ] {
        members += 1;
        assert!(!flag_refusals("x.sh", flag).is_empty(), "{flag}");
    }
    assert_eq!(members, 18);

    // Look-alikes: a group at warn, a lint unrelated to the rule, a plain flag.
    assert!(
        manifest_refusals(
            "Cargo.toml",
            "[lints.clippy]\nall = { level = \"warn\", priority = -1 }\n"
        )
        .is_empty()
    );
    assert!(
        manifest_refusals("Cargo.toml", "[lints.clippy]\nunwrap_used = \"allow\"\n").is_empty()
    );
    assert!(flag_refusals("x.sh", "ls -A clippy.toml").is_empty());
    assert!(flag_refusals("x.sh", "cargo clippy -- -A clippy::module_name_repetitions").is_empty());
}

#[test]
fn the_audit_reads_code_and_only_code() {
    let named = "let client = reqwest::Client::builder()";
    let expect = |lint: &str, reason: &str| {
        format!("fn f() {{\n    #[expect({lint}, reason = \"{reason}\")]\n    {named}\n}}\n")
    };
    let (refused, sites) = audit_source(
        TRANSPORT,
        &expect("clippy::disallowed_methods", "the transport"),
    );
    assert_eq!(
        (refused, sites.len()),
        (Vec::<String>::new(), 1),
        "a named site is accepted"
    );

    for (case, text) in [
        (
            "another file",
            audit_source(
                "crates/bot/src/poll.rs",
                &expect("clippy::disallowed_methods", "r"),
            )
            .0,
        ),
        (
            "a blank reason",
            audit_source(TRANSPORT, &expect("clippy::disallowed_methods", "  ")).0,
        ),
        (
            "an empty reason",
            audit_source(TRANSPORT, &expect("clippy::disallowed_methods", "")).0,
        ),
        (
            "an allow",
            audit_source(
                TRANSPORT,
                &expect("clippy::disallowed_methods", "r").replace("expect", "allow"),
            )
            .0,
        ),
        (
            "another statement",
            audit_source(
                TRANSPORT,
                &expect("clippy::disallowed_methods", "r").replace(named, "let other = 1;"),
            )
            .0,
        ),
        (
            "a group at the named site",
            audit_source(TRANSPORT, &expect("clippy::style", "r")).0,
        ),
    ] {
        assert!(!text.is_empty(), "{case} is refused");
    }

    for (case, text) in [
        ("another lint", "#[allow(clippy::unwrap_used)]\nfn f() {}\n"),
        ("a comment", "// #[allow(clippy::style)]\nfn f() {}\n"),
        (
            "a block comment",
            "/* #![allow(clippy::all)] */\nfn f() {}\n",
        ),
        ("a string", "const S: &str = \"#[allow(clippy::style)]\";\n"),
        (
            "a raw string",
            "const S: &str = r#\"#![allow(warnings)]\"#;\n",
        ),
        (
            "a lifetime and a char",
            "fn f<'a>(x: &'a str) -> char { '#' }\n",
        ),
    ] {
        let (refused, _) = audit_source("crates/bot/src/poll.rs", text);
        assert!(
            refused.is_empty(),
            "{case} is not a suppression: {refused:?}"
        );
    }
}
