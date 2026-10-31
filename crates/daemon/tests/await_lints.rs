//! The await lints' audit (SPEC-436 R1 to R4, ADR-450 D1): the workspace denies
//! `await_holding_lock` and `await_holding_refcell_ref` by name, and nothing in the tree lowers
//! either of them or the `suspicious` group that holds them. It reads its sources as data (R19);
//! only the repository tests read the tree.

use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use proc_macro2::{Delimiter, TokenStream, TokenTree};

/// The listed `expect` suppressions, each `(file, lint)` (R3): none.
const SUPPRESSIONS: &[(&str, &str)] = &[];

/// The root manifest's clippy table (R2).
const TABLE: &str = "[workspace.lints.clippy]";

/// A member manifest's clippy table (R3).
const MEMBER_TABLE: &str = "[lints.clippy]";

/// The lints the workspace denies by name (R1, R2).
const REQUIRED: &[&str] = &["await_holding_lock", "await_holding_refcell_ref"];

/// The levels that hold a required lint (R2).
const ACCEPTED: &[&str] = &["deny", "forbid"];

/// The names no attribute, member manifest or flag may lower (R3).
const AUDITED: &[&str] = &[
    "await_holding_lock",
    "await_holding_refcell_ref",
    "suspicious",
];

/// The levels that lower an audited name (R3).
const LOWERING: &[&str] = &["allow", "expect", "warn"];

/// The flags that lower a lint on a command line (R3), each also read attached when short.
const FLAGS: &[&str] = &["-A", "--allow", "-W", "--warn"];

/// The flag files: each directory's own files with the extension (R3).
const FLAG_FAMILIES: &[(&str, &str)] = &[
    ("scripts/", "sh"),
    (".github/workflows/", "yml"),
    (".cargo/", "toml"),
];

/// What the audit examined, and every lowering it refused.
#[derive(Default)]
struct Audit {
    manifests: Vec<String>,
    sources: usize,
    flags: Vec<String>,
    refused: Vec<String>,
}

/// A source's tokens; a file that does not lex fails, naming it.
fn lexed(path: &str, text: &str) -> Result<Vec<TokenTree>, String> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let stream =
        TokenStream::from_str(text).map_err(|error| format!("{path} does not lex: {error}"))?;
    Ok(stream.into_iter().collect())
}

/// Whether a path names a Rust source.
fn is_rust(path: &str) -> bool {
    Path::new(path)
        .extension()
        .is_some_and(|extension| extension == "rs")
}

/// A TOML line with its `#` comment removed; a `#` inside a string stays.
fn uncommented(line: &str) -> &str {
    let mut quoted = false;
    for (at, ch) in line.char_indices() {
        if ch == '"' {
            quoted = !quoted;
        } else if ch == '#' && !quoted {
            return &line[..at];
        }
    }
    line
}

/// The level a lint entry's value sets, written plainly or as an inline table's `level`, or
/// `missing` when it sets none.
fn level_of(value: &str) -> String {
    let value = value.strip_prefix('{').map_or(value, |table| {
        table
            .split_once("level")
            .map_or("", |(_, rest)| rest.trim_start().trim_start_matches('='))
    });
    let level = value
        .trim()
        .trim_start_matches('"')
        .split('"')
        .next()
        .unwrap_or_default();
    if level.is_empty() {
        "missing".to_owned()
    } else {
        level.to_owned()
    }
}

/// Each `name = level` entry of one TOML table, with comments removed (R2, R3).
fn table_entries(text: &str, table: &str) -> Vec<(String, String)> {
    let mut entries = Vec::new();
    let mut inside = false;
    for line in text.lines() {
        let line = uncommented(line).trim();
        if line.starts_with('[') {
            inside = line == table;
        } else if inside && let Some((name, value)) = line.split_once('=') {
            entries.push((name.trim().to_owned(), level_of(value.trim())));
        }
    }
    entries
}

/// The level the root manifest's clippy table gives each required lint, or `missing` (R2).
fn required_levels(text: &str) -> Vec<(String, String)> {
    let entries = table_entries(text, TABLE);
    REQUIRED
        .iter()
        .map(|lint| {
            let level = entries
                .iter()
                .rev()
                .find(|(name, _)| name == lint)
                .map_or_else(|| "missing".to_owned(), |(_, level)| level.clone());
            ((*lint).to_owned(), level)
        })
        .collect()
}

/// The root manifest's refusals, each naming its lint (R2).
fn manifest_refusals(text: &str) -> Vec<String> {
    required_levels(text)
        .into_iter()
        .filter(|(_, level)| !ACCEPTED.contains(&level.as_str()))
        .map(|(lint, level)| format!("Cargo.toml: {lint} is {level}"))
        .collect()
}

/// A member manifest's refusals: an audited name below `deny` in its `[lints.clippy]` (R3).
fn member_refusals(path: &str, text: &str) -> Vec<String> {
    table_entries(text, MEMBER_TABLE)
        .into_iter()
        .filter(|(name, level)| {
            AUDITED.contains(&name.as_str()) && !ACCEPTED.contains(&level.as_str())
        })
        .map(|(name, level)| format!("{path}: {name} = {level}"))
        .collect()
}

/// Whether a token is the punctuation `ch`.
fn is_char(token: Option<&TokenTree>, ch: char) -> bool {
    matches!(token, Some(TokenTree::Punct(punct)) if punct.as_char() == ch)
}

/// A source's refusals: every attribute that lowers an audited name (R3).
fn source_refusals(
    path: &str,
    text: &str,
    suppressions: &[(&str, &str)],
) -> Result<Vec<String>, String> {
    let mut refused = Vec::new();
    attributes(path, &lexed(path, text)?, suppressions, &mut refused);
    Ok(refused)
}

/// Every outer and inner attribute in a token list, at any depth. A comment is no token and a
/// string is one literal, so neither is ever an attribute (R3).
fn attributes(
    path: &str,
    tokens: &[TokenTree],
    suppressions: &[(&str, &str)],
    refused: &mut Vec<String>,
) {
    for (at, token) in tokens.iter().enumerate() {
        let TokenTree::Group(group) = token else {
            continue;
        };
        let inner: Vec<TokenTree> = group.stream().into_iter().collect();
        let mark = at.checked_sub(1).and_then(|place| tokens.get(place));
        let outer = is_char(mark, '#');
        let inner_mark = is_char(mark, '!')
            && is_char(at.checked_sub(2).and_then(|place| tokens.get(place)), '#');
        if group.delimiter() == Delimiter::Bracket && (outer || inner_mark) {
            lowerings(path, &inner, suppressions, refused);
        } else {
            attributes(path, &inner, suppressions, refused);
        }
    }
}

/// The lowerings one attribute's inside sets (R3): `allow`, `expect` or `warn` naming an audited
/// lint, or a `cfg_attr` holding one. An `expect` with a `reason` that `suppressions` lists by file
/// and lint is not one.
fn lowerings(
    path: &str,
    inner: &[TokenTree],
    suppressions: &[(&str, &str)],
    refused: &mut Vec<String>,
) {
    let [TokenTree::Ident(name), TokenTree::Group(arguments)] = inner else {
        return;
    };
    let arguments: Vec<TokenTree> = arguments.stream().into_iter().collect();
    if name == "cfg_attr" {
        for (at, token) in arguments.iter().enumerate() {
            if let (TokenTree::Ident(_), Some(TokenTree::Group(_))) = (token, arguments.get(at + 1))
            {
                lowerings(
                    path,
                    arguments.get(at..at + 2).unwrap_or_default(),
                    suppressions,
                    refused,
                );
            }
        }
        return;
    }
    let level = name.to_string();
    if !LOWERING.contains(&level.as_str()) {
        return;
    }
    let reason = arguments
        .iter()
        .any(|token| matches!(token, TokenTree::Ident(word) if word == "reason"));
    for window in arguments.windows(4) {
        if let [
            TokenTree::Ident(tool),
            first,
            second,
            TokenTree::Ident(lint),
        ] = window
            && tool == "clippy"
            && is_char(Some(first), ':')
            && is_char(Some(second), ':')
        {
            let lint = lint.to_string();
            let listed = level == "expect"
                && reason
                && suppressions
                    .iter()
                    .any(|(file, name)| *file == path && *name == lint);
            if AUDITED.contains(&lint.as_str()) && !listed {
                refused.push(format!("{path}: {level}(clippy::{lint})"));
            }
        }
    }
}

/// A flag file's refusals: a lowering flag beside an audited name, apart or attached (R3).
fn flag_refusals(path: &str, text: &str) -> Vec<String> {
    let words: Vec<&str> = text
        .split(|c: char| c.is_whitespace() || matches!(c, '"' | '\'' | ',' | '[' | ']' | '='))
        .filter(|word| !word.is_empty())
        .collect();
    let mut refused = Vec::new();
    for (at, word) in words.iter().enumerate() {
        for flag in FLAGS {
            let named = if word == flag {
                words.get(at + 1).copied()
            } else {
                word.strip_prefix(flag).filter(|_| flag.len() == 2)
            };
            if let Some(lint) = named.and_then(|name| name.strip_prefix("clippy::"))
                && AUDITED.contains(&lint)
            {
                refused.push(format!("{path}: {flag} clippy::{lint}"));
            }
        }
    }
    refused
}

/// Whether a path is a member's manifest, `crates/<member>/Cargo.toml`.
fn is_member_manifest(path: &str) -> bool {
    let parts: Vec<&str> = path.split('/').collect();
    parts.len() == 3 && parts.first() == Some(&"crates") && parts.get(2) == Some(&"Cargo.toml")
}

/// Whether a path is a flag file, directly in a flag family's directory with its extension (R3).
fn is_flag_file(path: &str) -> bool {
    FLAG_FAMILIES.iter().any(|(directory, extension)| {
        path.strip_prefix(directory).is_some_and(|name| {
            !name.contains('/')
                && Path::new(name)
                    .extension()
                    .is_some_and(|found| found == *extension)
        })
    })
}

/// The audit over a list of (path, text) (R2 to R4): the root manifest, the member manifests,
/// every Rust source under `crates/` and the flag files. It prints the three counts, then
/// refuses any of them at 0.
fn audit(files: &[(String, String)], suppressions: &[(&str, &str)]) -> Result<Audit, String> {
    let mut audit = Audit::default();
    for (path, text) in files {
        if path == "Cargo.toml" {
            audit.manifests.push(path.clone());
            audit.refused.extend(manifest_refusals(text));
        } else if is_member_manifest(path) {
            audit.manifests.push(path.clone());
            audit.refused.extend(member_refusals(path, text));
        } else if path.starts_with("crates/") && is_rust(path) {
            audit.sources += 1;
            audit
                .refused
                .extend(source_refusals(path, text, suppressions)?);
        } else if is_flag_file(path) {
            audit.flags.push(path.clone());
            audit.refused.extend(flag_refusals(path, text));
        }
    }
    eprintln!("examined {} manifests", audit.manifests.len());
    eprintln!("examined {} sources", audit.sources);
    eprintln!("examined {} flag files", audit.flags.len());
    let counts = [
        (audit.manifests.len(), "manifests"),
        (audit.sources, "sources"),
        (audit.flags.len(), "flag files"),
    ];
    if let Some((_, what)) = counts.iter().find(|(count, _)| *count == 0) {
        return Err(format!("examined 0 {what}"));
    }
    Ok(audit)
}

/// Planted files, in the shape the audit takes.
fn planted(files: &[(&str, &str)]) -> Vec<(String, String)> {
    files
        .iter()
        .map(|(path, text)| ((*path).to_owned(), (*text).to_owned()))
        .collect()
}

/// The repository's root.
fn root() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}

/// A directory's entries, sorted, each with its path from the repository root.
fn listed(directory: &str) -> Result<Vec<(String, PathBuf)>, String> {
    let listing =
        fs::read_dir(root().join(directory)).map_err(|error| format!("{directory}: {error}"))?;
    let mut paths = Vec::new();
    for entry in listing {
        let entry = entry.map_err(|error| format!("{directory}: {error}"))?;
        paths.push((
            format!("{directory}/{}", entry.file_name().to_string_lossy()),
            entry.path(),
        ));
    }
    paths.sort();
    Ok(paths)
}

/// A file's text, or an error naming it.
fn read(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))
}

/// The tree's audit population: the root manifest, each member's manifest, every Rust file under
/// `crates/`, and every file directly under `scripts/`, `.github/workflows/` and `.cargo/`.
fn repository() -> Result<Vec<(String, String)>, String> {
    let mut files = vec![("Cargo.toml".to_owned(), read(&root().join("Cargo.toml"))?)];
    let mut pending = listed("crates")?;
    while let Some((name, path)) = pending.pop() {
        if path.is_dir() {
            pending.extend(listed(&name)?);
        } else if is_rust(&name)
            || (name.matches('/').count() == 2 && name.ends_with("/Cargo.toml"))
        {
            files.push((name, read(&path)?));
        }
    }
    for directory in ["scripts", ".github/workflows", ".cargo"] {
        for (name, path) in listed(directory)? {
            if path.is_file() {
                files.push((name, read(&path)?));
            }
        }
    }
    Ok(files)
}

#[test]
fn the_workspace_denies_both_await_lints() {
    let text = fs::read_to_string(root().join("Cargo.toml")).expect("the root manifest reads");
    assert_eq!(
        required_levels(&text),
        [
            ("await_holding_lock".to_owned(), "deny".to_owned()),
            ("await_holding_refcell_ref".to_owned(), "deny".to_owned()),
        ]
    );
    assert_eq!(manifest_refusals(&text), Vec::<String>::new());
}

#[test]
fn a_missing_lowered_or_misplaced_await_lint_entry_is_refused() {
    let cases: [(&str, &[&str]); 11] = [
        (
            "[workspace.lints.clippy]\nawait_holding_lock = \"deny\"\n",
            &["Cargo.toml: await_holding_refcell_ref is missing"],
        ),
        (
            "[workspace.lints.clippy]\nawait_holding_lock = \"warn\"\nawait_holding_refcell_ref = \"deny\"\n",
            &["Cargo.toml: await_holding_lock is warn"],
        ),
        (
            "[workspace.lints.clippy]\nawait_holding_lock = \"deny\"\nawait_holding_refcell_ref = \"allow\"\n",
            &["Cargo.toml: await_holding_refcell_ref is allow"],
        ),
        (
            "[workspace.lints.clippy]\nawait_holding_lock = { level = \"warn\", priority = 1 }\nawait_holding_refcell_ref = \"deny\"\n",
            &["Cargo.toml: await_holding_lock is warn"],
        ),
        (
            "[workspace.lints.clippy]\n# await_holding_lock = \"deny\"\nawait_holding_refcell_ref = \"deny\"\n",
            &["Cargo.toml: await_holding_lock is missing"],
        ),
        (
            "[workspace.lints.clippy]\nawait_holding_lock = \"deny\"\nawait_holding_refcell_ref = { priority = 1 } # level = \"deny\"\n",
            &["Cargo.toml: await_holding_refcell_ref is missing"],
        ),
        (
            "[workspace.lints.rust]\nawait_holding_lock = \"deny\"\nawait_holding_refcell_ref = \"deny\"\n",
            &[
                "Cargo.toml: await_holding_lock is missing",
                "Cargo.toml: await_holding_refcell_ref is missing",
            ],
        ),
        (
            "[workspace.lints.clippy]\nsuspicious = \"deny\"\n",
            &[
                "Cargo.toml: await_holding_lock is missing",
                "Cargo.toml: await_holding_refcell_ref is missing",
            ],
        ),
        (
            "[workspace.lints.clippy]\nawait_holding_lock = \"deny\"\nawait_holding_refcell_ref = \"deny\"\n",
            &[],
        ),
        (
            "[workspace.lints.clippy]\nawait_holding_lock = { level = \"deny\", priority = 1 }\nawait_holding_refcell_ref = { level = \"deny\" }\n",
            &[],
        ),
        (
            "[workspace.lints.clippy]\nawait_holding_lock = \"forbid\"\nawait_holding_refcell_ref = \"forbid\"\n",
            &[],
        ),
    ];
    for (text, expected) in cases {
        assert_eq!(manifest_refusals(text), expected, "{text}");
    }
}

#[test]
fn a_lowering_of_an_await_lint_or_its_group_is_refused_wherever_it_is_planted() {
    let files = planted(&[
        (
            "Cargo.toml",
            "[workspace.lints.clippy]\nawait_holding_lock = \"deny\"\nawait_holding_refcell_ref = \"deny\"\n",
        ),
        (
            "crates/planted/Cargo.toml",
            "[lints.clippy]\nawait_holding_lock = \"allow\"\nawait_holding_refcell_ref = { level = \"warn\", priority = 1 }\nsuspicious = \"warn\"\n",
        ),
        (
            "crates/planted/src/outer.rs",
            "#[allow(clippy::await_holding_lock)]\nfn one() {}\n#[expect(clippy::await_holding_refcell_ref)]\nfn two() {}\n#[warn(clippy::suspicious)]\nfn three() {}\n",
        ),
        (
            "crates/planted/src/inner.rs",
            "#![allow(clippy::suspicious)]\n#![expect(clippy::await_holding_lock)]\n#![warn(clippy::await_holding_refcell_ref)]\n",
        ),
        (
            "crates/planted/src/cfg_attr.rs",
            "#[cfg_attr(test, allow(clippy::await_holding_refcell_ref))]\nfn one() {}\n#[cfg_attr(test, expect(clippy::suspicious))]\nfn two() {}\n#[cfg_attr(test, warn(clippy::await_holding_lock))]\nfn three() {}\n",
        ),
        (
            "crates/planted/src/quiet.rs",
            r##"// #[allow(clippy::await_holding_lock)]
/// #[allow(clippy::suspicious)]
fn quiet() {
    let text = "#[allow(clippy::await_holding_refcell_ref)]";
}

#[expect(clippy::await_holding_lock, reason = "listed")]
fn listed() {}
"##,
        ),
        (
            "scripts/planted.sh",
            "cargo clippy -- -A clippy::await_holding_lock --allow clippy::suspicious\ncargo clippy -- -W clippy::await_holding_refcell_ref --warn=clippy::await_holding_lock\ncargo clippy -- -Aclippy::suspicious -Wclippy::await_holding_lock\n",
        ),
        (
            ".github/workflows/planted.yml",
            "      - run: cargo clippy -- -A clippy::await_holding_refcell_ref\n",
        ),
        (
            ".cargo/planted.toml",
            "rustflags = [\"-W\", \"clippy::suspicious\"]\n",
        ),
    ]);
    let audit = audit(
        &files,
        &[("crates/planted/src/quiet.rs", "await_holding_lock")],
    )
    .expect("the audit reads the planted files");
    assert_eq!(
        audit.refused,
        [
            "crates/planted/Cargo.toml: await_holding_lock = allow",
            "crates/planted/Cargo.toml: await_holding_refcell_ref = warn",
            "crates/planted/Cargo.toml: suspicious = warn",
            "crates/planted/src/outer.rs: allow(clippy::await_holding_lock)",
            "crates/planted/src/outer.rs: expect(clippy::await_holding_refcell_ref)",
            "crates/planted/src/outer.rs: warn(clippy::suspicious)",
            "crates/planted/src/inner.rs: allow(clippy::suspicious)",
            "crates/planted/src/inner.rs: expect(clippy::await_holding_lock)",
            "crates/planted/src/inner.rs: warn(clippy::await_holding_refcell_ref)",
            "crates/planted/src/cfg_attr.rs: allow(clippy::await_holding_refcell_ref)",
            "crates/planted/src/cfg_attr.rs: expect(clippy::suspicious)",
            "crates/planted/src/cfg_attr.rs: warn(clippy::await_holding_lock)",
            "scripts/planted.sh: -A clippy::await_holding_lock",
            "scripts/planted.sh: --allow clippy::suspicious",
            "scripts/planted.sh: -W clippy::await_holding_refcell_ref",
            "scripts/planted.sh: --warn clippy::await_holding_lock",
            "scripts/planted.sh: -A clippy::suspicious",
            "scripts/planted.sh: -W clippy::await_holding_lock",
            ".github/workflows/planted.yml: -A clippy::await_holding_refcell_ref",
            ".cargo/planted.toml: -W clippy::suspicious",
        ]
    );
    assert_eq!(audit.manifests, ["Cargo.toml", "crates/planted/Cargo.toml"]);
    assert_eq!(audit.sources, 4);
    assert_eq!(
        audit.flags,
        [
            "scripts/planted.sh",
            ".github/workflows/planted.yml",
            ".cargo/planted.toml"
        ]
    );
}

#[test]
fn the_repository_lowers_no_await_lint_anywhere() {
    let audit = audit(&repository().expect("the tree reads"), SUPPRESSIONS)
        .expect("the audit reads the tree");
    let mut manifests = vec!["Cargo.toml".to_owned()];
    for (name, path) in listed("crates").expect("the crates list") {
        if path.join("Cargo.toml").is_file() {
            manifests.push(format!("{name}/Cargo.toml"));
        }
    }
    let mut examined = audit.manifests.clone();
    examined.sort();
    manifests.sort();
    assert_eq!(examined, manifests);
    for flag_file in [
        "scripts/check.sh",
        ".github/workflows/ci.yml",
        ".cargo/mutants.toml",
    ] {
        assert!(
            audit.flags.iter().any(|file| file == flag_file),
            "{flag_file}"
        );
    }
    assert_eq!(audit.refused, Vec::<String>::new());
    assert!(audit.sources > 0);
}
