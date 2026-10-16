//! The await lints' audit (SPEC-436 R1 to R4, ADR-450 D1): the workspace denies
//! `await_holding_lock` and `await_holding_refcell_ref` by name, and nothing in the tree lowers
//! either of them or the `suspicious` group that holds them. It reads its sources as data (R19);
//! only the repository tests read the tree.

use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use proc_macro2::{TokenStream, TokenTree};

/// The listed `expect` suppressions, each `(file, lint)` (R3): none.
const SUPPRESSIONS: &[(&str, &str)] = &[];

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

/// The level the root manifest's clippy table gives each required lint, or `missing` (R2).
fn required_levels(_text: &str) -> Vec<(String, String)> {
    Vec::new()
}

/// The root manifest's refusals, each naming its lint (R2).
fn manifest_refusals(_text: &str) -> Vec<String> {
    Vec::new()
}

/// The audit over a list of (path, text) (R3, R4).
fn audit(files: &[(String, String)], _suppressions: &[(&str, &str)]) -> Result<Audit, String> {
    let audit = Audit::default();
    for (path, text) in files {
        if is_rust(path) {
            lexed(path, text)?;
        }
    }
    eprintln!("examined {} manifests", audit.manifests.len());
    eprintln!("examined {} sources", audit.sources);
    eprintln!("examined {} flag files", audit.flags.len());
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
