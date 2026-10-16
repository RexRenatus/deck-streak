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
    let _ = text;
    Arms::default()
}

/// The census of the three source sets under `root` (R14, R17).
#[expect(
    clippy::unnecessary_wraps,
    reason = "the red commit's stub: it lists and reads nothing, finds nothing and refuses nothing"
)]
fn census(root: &Path) -> Result<Census, String> {
    let _ = root;
    Ok(Census::default())
}

/// Holds `sites` equal to `register`, file by file and arm by arm (R16). A refusal names its
/// reason on each of its lines.
#[expect(
    clippy::unnecessary_wraps,
    reason = "the red commit's stub: it accepts whatever it is given"
)]
fn judge(sites: &[ArmSite], register: &[ArmRow]) -> Result<(), String> {
    let _ = (sites, register);
    Ok(())
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
