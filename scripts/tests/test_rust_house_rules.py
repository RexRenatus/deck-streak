"""One guard holds four Rust house rules at zero over the shipped sources (SPEC-427, ADR-441).

Four rules of the house hold in the shipped Rust sources, and this module refuses the first break
of each, naming the file, the line and the rule:

- `sqlite-one-place` (R4): SQLite is opened only in `crates/kernel/src/db.rs`, the kernel's
  database base (SPEC-020 R17).
- `no-format-into-query` (R5): no `format!` reaches a query, directly, through `.leak()`, or
  through a name bound to one earlier in the same function.
- `no-secret-in-log` (R6, R7): no secret-named name is passed to a `tracing` or `log` macro.
- `randomness-through-getrandom` (R8, R9): randomness is drawn only through `getrandom`.

The population (R2) is every `crates/*/src/**/*.rs` file, sorted, less each file an out-of-line
`#[cfg(test)] mod` declares, its `#[path]` honoured, and less the span of every `#[cfg(test)]`
item. The lexer (R3) blanks comments and the contents of literals and keeps every offset and line,
so a rule reads only code and a refused site's line is the source's own. No site is admitted by
name and no file is left out by name.

Each rule returns one line per refused site, `<rule>: <path>:<line>: <what>`, with what it
examined, and fails when it examined nothing (R10). A1 to A7 plant fixture trees in a temporary
directory at test time; A8 reads the repository's own tree and prints each rule's count through
`_support.examined`. The walk reads every shipped source, so the verdict is read in CI.

At this commit the behaviour is stubbed, keeping every input: the walk returns every
`crates/*/src/**/*.rs` file with no test module removed, the lexer returns the text unblanked,
each rule refuses nothing, and no rule refuses an empty population (SPEC-427 section 3).
"""

import tempfile
import unittest
from pathlib import Path

from _support import REPO, examined

# R2: the shipped sources under each crate; R4: the one file that opens SQLite.
SOURCES = "*/src/**/*.rs"
DATABASE_BASE = "crates/kernel/src/db.rs"


def lexed(text):
    """R3, stubbed: the text unblanked, and no string literal read."""
    return text, {}


class Source:
    """One population file (R2): its path from the root, its code with comments, the contents of
    literals and its `#[cfg(test)]` items blanked, and its string literals' contents by the offset
    of each opening quote."""

    def __init__(self, path, code, literals):
        self.path, self.code, self.literals = path, code, literals

    def line(self, offset):
        """The source's own line at an offset."""
        return self.code.count("\n", 0, offset) + 1


def population(root):
    """R2, stubbed: every `crates/*/src/**/*.rs` file under `root`, sorted, with no test module
    removed and no `#[cfg(test)]` item blanked."""
    root, sources = Path(root), []
    for file in sorted((root / "crates").glob(SOURCES)):
        if file.is_file():
            code, literals = lexed(file.read_text(encoding="utf-8"))
            sources.append(Source(file.relative_to(root).as_posix(), code, literals))
    return sources


def judged(unit, items):
    """R10, stubbed: what a rule examined, with no refusal when it is nothing."""
    return list(items)


def sqlite_one_place(files):
    """R4, stubbed: (no refused line, the files it examined, no opener found in the base)."""
    read = [source.path for source in files if source.path != DATABASE_BASE]
    return [], judged("shipped source files", read), []


def no_format_into_query(files):
    """R5, stubbed: (no refused line, no site examined)."""
    return [], judged("query sites", [])


def no_secret_in_log(files):
    """R6, R7, stubbed: (no refused line, no call examined)."""
    return [], judged("log macro calls", [])


def randomness_through_getrandom(files, root):
    """R8, R9, stubbed: (no refused line, the files it examined)."""
    return [], judged("shipped source files and manifests", [source.path for source in files])


def planted(root, files):
    """Write each fixture file under `root`, its directories made on the way."""
    for path, text in files.items():
        target = Path(root) / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(text, encoding="utf-8")


def sites(lines):
    """The `<path>:<line>` each refused line names."""
    return {line.split(": ", 2)[1] for line in lines}


# A1: the database base, whose openers are its own, and a crate that opens SQLite every way R4
# names; a `SqlitePool` used as a type, and a call that connects nothing, are not refused.
DATABASE_PLANT = """\
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
pub async fn open(url: &str) {
    let options = SqliteConnectOptions::new();
    let pool = SqlitePoolOptions::new().connect_with(options).await;
    let other = sqlx::SqlitePool::connect(url).await;
}
"""
SQLITE_PLANT = """\
pub fn opens(url: &str, options: Options, pool: SqlitePool) {
    let a = SqliteConnectOptions::from_str(url);
    let b = SqlitePoolOptions::new();
    let c = PoolOptions::<Sqlite>::new();
    let d = rusqlite::Connection::open(url);
    let e = libsqlite3_sys::sqlite3_open(url);
    let f = SqlitePool::connect(url);
    let g = sqlx::Pool::<Sqlite>::connect_with(options);
    let h = SqliteConnection::connect_lazy(url);
    let i = <sqlx::SqliteConnection as sqlx::Connection>::connect_lazy_with(options);
    let j = options.connect_lazy_with();
    let k: SqlitePool = shared();
    let m = SqlitePool::clone(&pool);
}
"""

# A2: each family function and an imported bare name with a direct `format!`, `AssertSqlSafe`, a
# `.leak()`, a `let`-bound and an assigned `format!`; then a name bound in another function, a
# literal and a literal-only macro, none refused.
QUERY_PLANT = """\
use sqlx::{query_scalar, Row};
fn direct(t: &str, args: Args, built: String) {
    sqlx::query(&format!("SELECT {t}"));
    sqlx::query_as::<_, Row>(&format!("SELECT {t}"));
    sqlx::query_scalar(&format!("SELECT {t}"));
    sqlx::query_with(&format!("SELECT {t}"), args);
    sqlx::query_as_with(&format!("SELECT {t}"), args);
    sqlx::query_scalar_with(&format!("SELECT {t}"), args);
    sqlx::raw_sql(&format!("SELECT {t}"));
    query_scalar(&format!("SELECT {t}"));
    let asserted = AssertSqlSafe(format!("SELECT {t}"));
    sqlx::query(built.leak());
}
fn bound(t: &str) {
    let sql: String = format!("SELECT {t}");
    sqlx::query(&sql);
    let mut set = String::new();
    set = format!("SELECT {t}");
    sqlx::query(&set);
}
fn elsewhere() {
    sqlx::query(&sql);
    sqlx::query("SELECT 1");
    sqlx::query!("SELECT {}", format!("x"));
}
"""

# A3: each macro form and each passed-name form, carrying each part and each pair, refused; each
# about-name, a `target:` and a `Level::` argument, and the skipped parameters, not refused.
LOG_PLANT = """\
use tracing::{info, Level};
use log::warn as logged;
fn logs(cfg: &Config, session: &Session, tokens: &[u8], name: &str) {
    tracing::info!(bot_token = 1, "starting");
    tracing::warn!("webhook.secret" = 1);
    tracing::error!(value = cfg.sync_password.expose());
    tracing::debug!(%passkey_credential);
    tracing::trace!(?session_cookie);
    tracing::info!(bearer);
    tracing::info!("refreshing {}", session.authorization);
    tracing::info!("the {apikey} is read");
    log::info!(user = name; "sent {initdata}");
    info!(api_key = 1);
    logged!(private_key = 1);
    tracing::info_span!("sign", signing_key = 1);
    tracing::event!(Level::INFO, init_data = 1);
    tracing::info!(session_id = 1);
    tracing::info!(target: AUTH_TOKEN_TARGET, "fine");
    tracing::info!(token_count = 1, count = tokens.len(), "counted");
    tracing::info!(secret_length = 1, password_size = 1, token_total = 1);
    tracing::info!(secret_name = name, credential_path = 1, cookie_file = 1);
    tracing::info!(token_kind = 1, password_hash = 1, credential_id = 1);
    tracing::info!(token_expires_at = 1);
    tracing::event!(tracing::Level::INFO, count = 1);
}
#[instrument]
fn instrumented(secret: &str) {}
#[tracing::instrument(skip(token), fields(user = %name))]
fn skipped(token: &str, name: &str) {}
#[instrument(skip_all)]
fn all_skipped(password: &str) {}
"""

# A4: each refused path, identifier, import and `PyRandom::new`, and a uuid constructor under a
# lock whose uuid depends on rand; the getrandom draws and `RandomState` are not refused.
RANDOM_PLANT = """\
use getrandom::fill;
extern crate rand;
use fastrand;
fn draws(source: &mut Source, bytes: &mut [u8]) {
    let a = rand::random::<u64>();
    let b = rand_core::RngCore::next_u64(source);
    let c = rand_chacha::ChaCha20Rng::from_entropy();
    let d = fastrand::u64(..);
    let e = ring::rand::SystemRandom::new();
    let f = OsRng;
    let g: ThreadRng = made();
    let h = thread_rng();
    let i = StdRng::from_entropy();
    let j = SmallRng::from_entropy();
    let k = <Seeded as SeedableRng>::seed_from_u64(1);
    let m = PyRandom::new(42);
    let n = Uuid::new_v4();
    let o = Uuid::now_v7();
    getrandom::fill(bytes);
    let p = getrandom::u64();
    let q = getrandom::u32();
    getrandom::fill_uninit(bytes);
    let r = RandomState::new();
}
"""
RANDOM_MANIFEST = """\
[package]
name = "draw"

[dependencies]
getrandom = "0.3"
rand = "0.8"

[build-dependencies]
seeded = { package = "fastrand", version = "2" }

[target.'cfg(unix)'.dependencies]
rand_core = { workspace = true }
"""
RAND_LOCK = """\
version = 4

[[package]]
name = "uuid"
version = "1.0.0"
dependencies = [
 "getrandom",
 "rand 0.8.5",
]
"""
# A4: the same constructors under a lock whose uuid depends on getrandom alone, admitted.
UUID_PLANT = """\
fn handle(stamp: Timestamp) {
    let a = Uuid::new_v4();
    let b = Uuid::new_v7(stamp);
}
"""
UUID_MANIFEST = """\
[package]
name = "handle"

[dependencies]
uuid = "1"
"""
GETRANDOM_LOCK = """\
version = 4

[[package]]
name = "uuid"
version = "1.0.0"
dependencies = [
 "getrandom 0.4.3",
 "js-sys",
]
"""

# A5: a crate whose shipped sources hold an inline test module and an inline test function, an
# out-of-line test module beside its file and one by `#[path]`, a nested source, an integration
# test, an example and a build script, each of the last three outside the glob.
POPULATION_PLANT = {
    "crates/app/src/lib.rs": """\
mod deep;
mod other;
mod skip_write;
pub fn shipped() {
    sqlx::query("SELECT 1");
    tracing::info!(count = 1);
}
#[cfg(test)]
mod tests {
    fn opens() {
        let pool = sqlx::SqlitePool::connect(url);
        tracing::info!(bot_token = 1);
        let draw = rand::random::<u64>();
        sqlx::query(&format!("x"));
    }
}
#[cfg(test)]
#[allow(dead_code)]
fn helper() {
    let options = SqliteConnectOptions::new();
}
""",
    "crates/app/src/deep/mod.rs": "pub fn deep() {}\n",
    "crates/app/src/other.rs": """\
pub fn other() {}
#[cfg(test)]
#[path = "other_tests.rs"]
mod tests;
""",
    "crates/app/src/other_tests.rs": "fn opens() {\n    let o = SqliteConnectOptions::new();\n}\n",
    "crates/app/src/skip_write.rs": "pub fn write() {}\n#[cfg(test)]\nmod tests;\n",
    "crates/app/src/skip_write/tests.rs": "fn logs() {\n    tracing::info!(bot_token = 1);\n}\n",
    "crates/app/tests/integration.rs": "fn opens() {\n    let o = SqliteConnectOptions::new();\n}\n",
    "crates/app/examples/demo.rs": "fn main() {\n    let draw = rand::random::<u64>();\n}\n",
    "crates/app/build.rs": "fn main() {\n    let o = SqliteConnectOptions::new();\n}\n",
}

# A6: a violation of each rule in a line comment, a nested block comment, a string and a raw
# string, none refused; and a format string's inline capture, refused.
LEXED_PLANT = """\
pub fn lexed() {
    // let pool = SqlitePool::connect(url);
    // sqlx::query(&format!(sql));
    // tracing::info!(bot_token = 1);
    // let draw = rand::random::<u64>();
    /* outer /* inner */ let pool = SqlitePool::connect(url); */
    /* outer /* inner */ sqlx::query(&format!(sql)); */
    /* outer /* inner */ tracing::info!(bot_token = 1); */
    /* outer /* inner */ let draw = rand::random::<u64>(); */
    let quoted = "let pool = SqlitePool::connect(url);";
    let quoted = "sqlx::query(&format!(sql));";
    let quoted = "tracing::info!(bot_token = 1);";
    let quoted = "let draw = rand::random::<u64>();";
    let raw = r#"a" let pool = SqlitePool::connect(url); "b"#;
    let raw = r#"a" sqlx::query(&format!(sql)); "b"#;
    let raw = r#"a" tracing::info!(bot_token = 1); "b"#;
    let raw = r#"a" let draw = rand::random::<u64>(); "b"#;
    sqlx::query("SELECT 1");
    tracing::info!("refreshing {bot_token}");
}
"""


class RustHouseRules(unittest.TestCase):
    def test_sqlite_is_opened_only_in_the_kernels_database_base(self):
        with tempfile.TemporaryDirectory() as scratch:
            planted(
                scratch,
                {
                    "crates/kernel/src/db.rs": DATABASE_PLANT,
                    "crates/store/src/lib.rs": SQLITE_PLANT,
                },
            )
            refused, read, _openers = sqlite_one_place(population(scratch))
        self.assertEqual(
            sites(refused),
            {
                "crates/store/src/lib.rs:2",
                "crates/store/src/lib.rs:3",
                "crates/store/src/lib.rs:4",
                "crates/store/src/lib.rs:5",
                "crates/store/src/lib.rs:6",
                "crates/store/src/lib.rs:7",
                "crates/store/src/lib.rs:8",
                "crates/store/src/lib.rs:9",
                "crates/store/src/lib.rs:10",
                "crates/store/src/lib.rs:11",
            },
        )
        examined("fixture files outside the database base", read)

    def test_no_format_reaches_a_query(self):
        with tempfile.TemporaryDirectory() as scratch:
            planted(scratch, {"crates/query/src/lib.rs": QUERY_PLANT})
            refused, read = no_format_into_query(population(scratch))
        self.assertEqual(
            sites(refused),
            {
                "crates/query/src/lib.rs:3",
                "crates/query/src/lib.rs:4",
                "crates/query/src/lib.rs:5",
                "crates/query/src/lib.rs:6",
                "crates/query/src/lib.rs:7",
                "crates/query/src/lib.rs:8",
                "crates/query/src/lib.rs:9",
                "crates/query/src/lib.rs:10",
                "crates/query/src/lib.rs:11",
                "crates/query/src/lib.rs:12",
                "crates/query/src/lib.rs:16",
                "crates/query/src/lib.rs:19",
            },
        )
        examined("planted calls of the family and AssertSqlSafe sites", read)

    def test_no_secret_named_name_reaches_a_log_macro(self):
        with tempfile.TemporaryDirectory() as scratch:
            planted(scratch, {"crates/log/src/lib.rs": LOG_PLANT})
            refused, read = no_secret_in_log(population(scratch))
        self.assertEqual(
            sites(refused),
            {
                "crates/log/src/lib.rs:4",
                "crates/log/src/lib.rs:5",
                "crates/log/src/lib.rs:6",
                "crates/log/src/lib.rs:7",
                "crates/log/src/lib.rs:8",
                "crates/log/src/lib.rs:9",
                "crates/log/src/lib.rs:10",
                "crates/log/src/lib.rs:11",
                "crates/log/src/lib.rs:12",
                "crates/log/src/lib.rs:13",
                "crates/log/src/lib.rs:14",
                "crates/log/src/lib.rs:15",
                "crates/log/src/lib.rs:16",
                "crates/log/src/lib.rs:17",
                "crates/log/src/lib.rs:26",
            },
        )
        examined("planted log calls and instrument attributes", read)

    def test_randomness_is_drawn_only_through_getrandom(self):
        with tempfile.TemporaryDirectory() as scratch:
            planted(
                scratch,
                {
                    "crates/draw/src/lib.rs": RANDOM_PLANT,
                    "crates/draw/Cargo.toml": RANDOM_MANIFEST,
                    "Cargo.lock": RAND_LOCK,
                },
            )
            drawn, read = randomness_through_getrandom(population(scratch), scratch)
        with tempfile.TemporaryDirectory() as scratch:
            planted(
                scratch,
                {
                    "crates/id/src/lib.rs": UUID_PLANT,
                    "crates/id/Cargo.toml": UUID_MANIFEST,
                    "Cargo.lock": GETRANDOM_LOCK,
                },
            )
            admitted, read_too = randomness_through_getrandom(population(scratch), scratch)
        self.assertEqual(
            sites(drawn),
            {
                "crates/draw/src/lib.rs:2",
                "crates/draw/src/lib.rs:3",
                "crates/draw/src/lib.rs:5",
                "crates/draw/src/lib.rs:6",
                "crates/draw/src/lib.rs:7",
                "crates/draw/src/lib.rs:8",
                "crates/draw/src/lib.rs:9",
                "crates/draw/src/lib.rs:10",
                "crates/draw/src/lib.rs:11",
                "crates/draw/src/lib.rs:12",
                "crates/draw/src/lib.rs:13",
                "crates/draw/src/lib.rs:14",
                "crates/draw/src/lib.rs:15",
                "crates/draw/src/lib.rs:16",
                "crates/draw/src/lib.rs:17",
                "crates/draw/src/lib.rs:18",
                "crates/draw/Cargo.toml:6",
                "crates/draw/Cargo.toml:9",
                "crates/draw/Cargo.toml:12",
            },
        )
        self.assertEqual(admitted, [])
        examined("planted files and manifests", read + read_too)

    def test_the_population_is_the_shipped_sources_without_test_code(self):
        with tempfile.TemporaryDirectory() as scratch:
            planted(scratch, POPULATION_PLANT)
            files = population(scratch)
            refused = (
                sqlite_one_place(files)[0]
                + no_format_into_query(files)[0]
                + no_secret_in_log(files)[0]
                + randomness_through_getrandom(files, scratch)[0]
            )
        self.assertEqual(
            {source.path for source in files},
            {
                "crates/app/src/deep/mod.rs",
                "crates/app/src/lib.rs",
                "crates/app/src/other.rs",
                "crates/app/src/skip_write.rs",
            },
        )
        self.assertEqual(refused, [])
        examined("fixture files in the population", files)

    def test_comments_and_literals_are_not_code_but_a_format_capture_is_read(self):
        with tempfile.TemporaryDirectory() as scratch:
            planted(scratch, {"crates/lexed/src/lib.rs": LEXED_PLANT})
            files = population(scratch)
            logged_lines = no_secret_in_log(files)[0]
            others = (
                sqlite_one_place(files)[0]
                + no_format_into_query(files)[0]
                + randomness_through_getrandom(files, scratch)[0]
            )
        self.assertEqual(sites(logged_lines), {"crates/lexed/src/lib.rs:19"})
        self.assertEqual(others, [])
        examined("fixture files lexed", files)

    def test_each_rule_refuses_an_empty_population(self):
        with tempfile.TemporaryDirectory() as scratch:
            files = population(scratch)
            cases = (
                ("shipped source files", sqlite_one_place, (files,)),
                ("query sites", no_format_into_query, (files,)),
                ("log macro calls", no_secret_in_log, (files,)),
                (
                    "shipped source files and manifests",
                    randomness_through_getrandom,
                    (files, scratch),
                ),
            )
            for unit, rule, given in cases:
                with self.subTest(unit=unit):
                    with self.assertRaisesRegex(AssertionError, "examined 0 " + unit):
                        rule(*given)
        examined("rules over an empty population", cases)

    def test_the_repository_holds_every_rule_at_zero(self):
        files = population(REPO)
        opened, shipped, openers = sqlite_one_place(files)
        self.assertTrue(openers, "sqlite-one-place found no opener in crates/kernel/src/db.rs")
        queried, query_sites_read = no_format_into_query(files)
        logged_lines, calls = no_secret_in_log(files)
        drawn, read = randomness_through_getrandom(files, REPO)
        refused = opened + queried + logged_lines + drawn
        for line in refused:
            print(line)
        self.assertEqual(refused, [])
        examined("shipped source files", shipped)
        examined("query sites", query_sites_read)
        examined("log macro calls", calls)
        examined("shipped source files and manifests", read)


if __name__ == "__main__":
    unittest.main()
