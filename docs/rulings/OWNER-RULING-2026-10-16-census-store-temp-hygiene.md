In the owner's answer to the question on #919's `hygiene` check, in the owner's own words "One-site signed amendment" (option (A)), the owner admits one bounded weakening of SPEC-030 R3's Rust rule, which refuses `std::env::temp_dir()` in every test file: `scripts/tests/test_temp_hygiene.py` admits exactly one call, the one in the function `store_of` in `crates/progression/tests/xp_census.rs`, which roots the real tree's settle census store under the test process's temporary directory where `docs/rulings/OWNER-RULING-2026-10-16-census-store.md` places it; the admission names that file and that function, and no line, pattern or directory; every other `temp_dir()` call in every test file, a second one in the same file or in `store_of` itself included, is still refused by file and line, and planted controls in the lint's own test prove it; an admission that matches no call is refused; nothing else in SPEC-030 or in the census-store ruling moves.

# OWNER RULING 2026-10-16: the settle census's store is the one shared temporary directory a test may keep

It amends `docs/rulings/OWNER-RULING-2026-10-16-census-store.md` in one place: its line "No test gains a skip, a filter, a sampled population or a weaker assertion, and no price, budget or timeout moves" now has one exception, the admission below, and SPEC-030 joins SPEC-072 as a bound that ruling weakens. Everything else that ruling says stands, and its file keeps the text it was signed with. Cited by SPEC-384 and by the insert-only amendment it makes to SPEC-030. The build that carries the admission is SPEC-384's (#692, pull request #919); it lands on `dev` only after the signed commit of this file is there.

## What was held

| place | what it holds |
|---|---|
| SPEC-030 R3 | the lint refuses, naming the file and line, "in Rust: `TempDir::into_path`, `TempDir::keep`, `NamedTempFile::keep`, `NamedTempFile::into_temp_path` followed by `keep`, `std::env::temp_dir()`, and a string literal that begins with `/tmp/`" |
| SPEC-030 section 6 | "a builder who needs a kept directory must argue it in the SPEC that needs it" |
| `scripts/tests/test_temp_hygiene.py` | `test_every_test_file_removes_what_it_creates` reads every test file R2 names and refuses each `temp_dir()` call as "std::env::temp_dir() is shared, and nothing empties it"; it admits none |
| the census-store ruling, line 1 | the store sits "under the test process's temporary directory, outside `target/`" |
| the census-store ruling, "What replaces it" | "No test gains a skip, a filter, a sampled population or a weaker assertion" |

## What was measured

| run | reading |
|---|---|
| job 117212004634 (`hygiene`, #919 at 79ebb193) | the lint's one refusal: `crates/progression/tests/xp_census.rs:1415: std::env::temp_dir() is shared, and nothing empties it`, the call in `store_of`, the only `temp_dir()` call in that file |
| run 38864014275 (`dev` at b25ed04a) | the lint passes on `dev`'s tree; at `dev` 83846ea3 no test file calls `temp_dir()`, and the planted fixture's call sits outside the census (R4) |

## What it closes

The two signed bounds cannot both hold. The census-store ruling places the store under the test process's temporary directory, and SPEC-030's lint refuses, in every test file, the call that names that directory. The store must outlive the census that fills it, because a later census reads what an earlier one compiled; that is its whole use. SPEC-030's accepted forms remove what they create at scope end, so none of them can hold the store.

## What replaces it

- `test_temp_hygiene.py` admits one `temp_dir()` call: the one in the function `store_of` in `crates/progression/tests/xp_census.rs`. The admission names that file and that function. It names no line number, no pattern and no directory.
- Every other `temp_dir()` call is refused as before, by file and line: a call in any other function of `xp_census.rs`, a second call in `store_of`, and a call in a function named `store_of` in any other test file.
- The lint's own test proves each of those three refused with a planted control, and proves the admitted call accepted, each by its own assertion.
- An admission that matches no call in the tree is refused, so the admission cannot outlive the site it names.
- Every other rule of SPEC-030 R3 still binds in `xp_census.rs` and in `store_of`: `into_path`, `keep`, a `/tmp/` literal, and every Python and TypeScript rule are unchanged. R2's census still examines every test file, `xp_census.rs` included.
- SPEC-030 takes the admission as an insert-only amendment that cites this ruling; every earlier byte of SPEC-030 is kept in order.
- No other test gains a skip, a filter, a sampled population or a weaker assertion, and no price, budget or timeout moves.

## Why it is admitted

The lint's reason is true of the store, and it is this ruling's residual: the store is shared within one machine's temporary directory, and no test empties it. In CI the store is born empty in each job, so no store is carried between jobs (the census-store ruling). On any other machine the store stays after the test ends, and a store whose key no longer matches (a new lock file, toolchain pin, cargo program, census flags, inherited environment or tree root) is never read again and is removed by no test: it stays until that temporary directory is emptied. SPEC-030 section 6 asks the SPEC that needs a kept directory to argue it; SPEC-384 argues it, and this ruling admits it by name. Rejected:

- **A store root under `target/`.** The census-store ruling keeps the store outside `target/`; moving it there changes that ruling's own bound, not SPEC-030's.
- **A root read from the `TMPDIR` variable.** It names the same shared directory and passes the lint only because the lint reads text; SPEC-030 section 6 refuses the leaking API itself so that such a path cannot escape it.
- **A store dropped at scope end (`tempfile::TempDir`).** It is removed after each census, so no later census reads it, and the store saves nothing.
- **An admission by line number or by pattern.** A line number moves with any edit above it, and a pattern admits calls no one named.
- **Editing the census-store ruling's line in place.** A signed record keeps its signed text. This amendment adds one exception and leaves the signed file as it was signed.

## Signature

The owner signs the commit that adds this file. The signature is this ruling's authority; a copy of this file in any unsigned commit carries none.
