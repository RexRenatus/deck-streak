# SPEC-369: the MCP server offers no export or erase tool, and its grant is read or write

- **Issue:** `#720` (parent `#157`). The MCP server's planned roster carries the
  predecessor's data-export tool and its erase-everything tool, and one grant reaches every core
  tool. This SPEC withdraws both tools from the roster and splits the grant into read and write,
  before the first tool that changes data is served.
- **Context(s):** `mcp` (`crates/mcp/src/grants.rs`, `crates/mcp/src/settings.rs`,
  `crates/mcp/tests`), and the records section 4 names.
- **Decided by:** ADR-380 (this SPEC's own). It amends SPEC-119 by two insert-only sections, 17
  and 18, appended at its end, and SPEC-001 by one `Amended (§14):` line after its `mcp-server`
  row and one bullet appended to its section 14; no existing line of either changes. ADR-380
  amends ADR-320 D5, ADR-121, ADR-119, ADR-329 and ADR-332 by its own record, with no line of
  theirs edited. It works under ADR-121's guard (every request needs a granted bearer, compared by
  digest in constant time), which it keeps.
- **Schematic:** `docs/schematics/mcp-guard-refusals.md` gains one section, the grants and what
  each reaches (a flowchart and its table), and one change-note line. No other line changes.
- **Assumption:** `#158` stays parked. The roster counts this SPEC states (31 tools, 33 with the
  two drill tools) hold only while it is.
- **Status:** one delivery. **Mutation band:** `S36900-S36999`, rows `S36901` to `S36907`
  (section 8). **Model:** none (section 7).

## 1. The problem, measured

Each figure was read at DeckStreak `dev` `b0935c75` by `git grep -n`, `git grep -c` and
`git show`. Nothing was run: every figure is a read.

### 1a. What the server serves, and what it plans to serve

- One tool is served: `get_law_track` (`crates/mcp/src/tools.rs:130-152`). It is annotated
  read-only (`:134`) and authorizes `Scope::LawTrack` before it reads (`:140-144`).
- The roster golden (`tools/parity-oracle/goldens/mcp_roster.json`) holds the predecessor's 33
  core and law tools: 25 annotated `readOnlyHint` true and 8 false (`log_reading`, `log_writing`,
  `log_focus`, `remediate_leech`, `take_skip_day`, `undo_skip_day`, `erase_all_data`,
  `force_sync`). It names `export_data` at `:556` and `erase_all_data` at `:576`.
- SPEC-119 plans both tools on 8 lines, 8 occurrences: the roster rows `:141` (`export_data`,
  core, read) and `:142` (`erase_all_data`, core, write), R21 (`:169-173`), A30 and A31
  (`:274-275`) and the `mcp_erase_confirm` golden (`:385`).
- Neither tool is called or served anywhere in `crates/`: the one mention is a provenance comment
  at `crates/daemon/src/role_data.rs:27`, beside `CONFIRMATION = "ERASE"` (`:28`), which stays
  true and is not edited.

### 1b. One grant reaches every core tool

- `Scope` is closed at two, `core` and `law_track` (`crates/mcp/src/grants.rs:12-23`, ADR-320
  D5). The required core credential `mcp-core-token` (`crates/mcp/src/settings.rs:19`) grants
  `{core}` (`grants.rs:135`). The optional `mcp-law-track-token` (`settings.rs:23`) grants
  `{core, law_track}`; when it is missing it grants nothing (`grants.rs:143`), and two credentials
  holding one value refuse start (`:147`).
- The guard admits a request whose grant holds `core`, and a tool authorizes its own scope
  through the limiter, in the token's bucket.
- So every core tool a later part of `#157` serves, the eight that change data among them, is
  reached by the core token. SPEC-119 section 6 names the risk: a leaked core token erases the
  ledger.

### 1c. Where export and erase already live

- `coordination::data_rights_registry::export_all` (`crates/coordination/src/data_rights_registry.rs:96`)
  and `erase_all` (`:106`) are called by the bot's `/delete` (`crates/bot/src/commands.rs:698`)
  and `/export` (`:718`), and by the host's `deckstreakd data export`
  (`crates/daemon/src/role_data.rs:101`) and `deckstreakd data erase --confirm ERASE` (`:117`).
- SPEC-021 R1 ("`export_all` and `erase_all` are the one pair of use cases every surface calls",
  `:44-46`) stays true without the server.
- The web and iOS clients have no export or erase path: the API crate has no such route.

### 1d. What holds the roster and the grants today

- `crates/mcp/tests/tools.rs:134` (A41) asserts that the served names are `["get_law_track"]`,
  and reads the golden through `roster()` (`:26`).
- The golden cannot be edited: `tools/parity-oracle/test_goldens.py:154-165` checks its
  `generator_sha256` and `registry_sha256` against the committed files, so an edit forces a
  regeneration on the predecessor's checkout. ADR-329 set the precedent for a difference the
  reader applies (its `token` parameter, never edited into the golden).
- The mcp crate's tests: guard 8, guard_census 2, limiter 5, scopes_and_service 3, server 8,
  settings 9, tools 4.
- The unit loads two credentials (`deploy/systemd/deck-streak-mcp.service:24-25`).
- The model `formal/tla/BearerGuard/BearerGuard.tla` covers `guard.rs`'s `admit`, `authorize`
  and `call`, `limiter.rs`'s `decide` and `tools.rs`'s `get_law_track` (`:2-6`). Nothing covers
  `grants.rs` or `settings.rs`.

## 2. Requirements

R1. **Three scopes.** `Scope` holds `Core`, `LawTrack` and `Write`. `Write` is named `"write"`
    and takes bit 4, and `Scope::ALL` lists `Core`, `LawTrack`, `Write` in that order.

R2. **The read grant.** The core credential `mcp-core-token` stays required, and its grant is the
    read grant `{core}`. Every loader error on it still refuses start.

R3. **The write grant.** `WRITE_CREDENTIAL` is `mcp-write-token`, and it is optional. When it is
    missing it grants nothing. When it loads, it grants `{core, write}`. An empty, unreadable or
    non-text write credential, one shorter than 32 characters, or one holding a byte no request
    can present refuses start naming `mcp-write-token`.

R4. **No shared value.** The check that the law-track credential differs from the core credential
    stays byte-equal. A write credential equal to the core credential refuses start with
    `SharedCredential { first: "mcp-core-token", second: "mcp-write-token" }`; one equal to the
    law-track credential with `first: "mcp-law-track-token"`. Each comparison is in constant time.

R5. **One load path for the optional credentials.** The law-track and write credentials load
    through one associated function of `Grants`, so a missing one grants nothing and any other
    loader error refuses start, by the same arm. Grants keep the order core, law-track, write.

R6. **Write is the write grant's alone.** The law-track grant stays `{core, law_track}`, and the
    read grant `{core}`. No grant but the write grant holds `write`.

R7. **A write tool needs write.** A tool whose annotations say `read_only_hint = false` authorizes
    `Scope::Write` before it reads or writes anything; a read tool authorizes its own scope
    (`core`, or `law_track` for `get_law_track`). At the base no write tool is served; a census
    holds the rule for every tool that lands (A6).

R8. **Export and erase are withdrawn.** `export_data` and `erase_all_data` are never served, and
    no grant reaches either. The test side declares them in `WITHDRAWN`
    (`crates/mcp/tests/support/mod.rs`); `roster()` drops them, so the portable roster counts 31
    tools. The golden and its registry module are unchanged.

R9. **No rights use case in the server.** `crates/mcp/src` names none of `data_rights_registry`,
    `export_all` and `erase_all`.

R10. **The covered spans hold.** `admit`, `authorize`, `call` and `get_law_track` are not edited,
     and neither are `guard.rs`, `limiter.rs` and `tools.rs`.

R11. **No unit change and no new credential.** `deploy/systemd/deck-streak-mcp.service` keeps its
     two `LoadCredential` lines. `mcp-write-token` is provisioned, and the unit line added with its
     `CREDENTIAL_SOURCES` and `ROLE_CREDENTIALS` entries, by the part that serves the first write
     tool.

R12. **The records.** SPEC-119 gains sections 17 (amendments T17 to T30) and 18 (their
     acceptance), appended. SPEC-001 gains one `Amended (§14):` line after its `mcp-server` row
     (`:266`) and one bullet appended to section 14 (after `:427`), so the parity matrix records
     both withdrawals and nothing is dropped silently (CHARTER 1, `CHARTER.md:24-27`). The
     schematic gains its section. ADR-380 records what it amends.

## 3. Acceptance criteria

The base is the red commit: the tests below and stubs that compile (`Scope::Write` with its name
and bit, `WRITE_CREDENTIAL`, a load that ignores it, and `WITHDRAWN` empty). No test pins an
engine-computed literal.

| # | criterion | red at the base, for this reason | test |
|---|---|---|---|
| A1 | With the three credentials loaded, the grants' scope names read `[["core"], ["core", "law_track"], ["core", "write"]]`. The test writes the credential under the literal id `mcp-write-token`, never through the constant | the names read `[["core"], ["core", "law_track"]]`: the stub ignores the write credential | `crates/mcp/tests/settings.rs` `the_write_credential_grants_core_and_write` (added) |
| A2 | With no write credential, the core and law-track credentials load and the names read `[["core"], ["core", "law_track"]]` | not red: the stub's ignore is the missing answer, so this is a control | `settings.rs` `a_missing_write_credential_grants_nothing` (added) |
| A3 | An empty, unreadable, non-text, 31-character and carriage-return-holding write credential each refuses start naming `mcp-write-token` | each loads `Ok` | `settings.rs` `a_broken_write_credential_refuses_start` (added) |
| A4 | A write credential equal to the core credential refuses start with `SharedCredential` first `mcp-core-token`, second `mcp-write-token`; one equal to the law-track credential, first `mcp-law-track-token`, second `mcp-write-token` | both load `Ok` | `settings.rs` `the_write_credential_cannot_share_a_value` (added) |
| A5 | Through the request layer the write token is admitted with the grant `["core", "write"]`; across the three grants and three scopes, the read grant is allowed `core` alone, the law-track grant `core` and `law_track`, and the write grant `core` and `write`, each refusal `Outcome::Denied` in the token's own bucket | the write token is refused 401: the layer saw `[]`, not `[Some(["core", "write"])]` | `crates/mcp/tests/guard.rs` `the_write_grant_holds_core_and_write_and_no_other` (added) |
| A6 | Every tool annotated `read_only_hint = false` in `crates/mcp/src` authorizes `Scope::Write`. The census prints its examined tool count, refuses zero tools, and first refuses by name the planted `crates/mcp/tests/fixtures/planted_write_under_core.rs.fixture` (a write tool authorizing `Scope::Core`) | not red: a guard; at the base it examines 1 tool and 0 write tools | `crates/mcp/tests/guard_census.rs` `every_write_tool_authorizes_the_write_scope` (added) |
| A7 | The portable roster (the golden less `WITHDRAWN`) counts 31 tools and names neither `export_data` nor `erase_all_data`, and every `WITHDRAWN` name is in the golden | it counts 33 and holds both names: `WITHDRAWN` is empty | `crates/mcp/tests/tools.rs` `the_withdrawn_tools_leave_the_portable_roster` (added) |
| A8 | Under the read, law-track and write tokens, `tools/list` names neither withdrawn tool, and `tools/call` of each answers a JSON-RPC error with no `result`, carrying the exact unknown-tool code the pinned router answers | not red: neither tool is served at the base; the write leg's 401 at the stub is A1's reason, not this criterion's | `tools.rs` `no_grant_reaches_a_withdrawn_tool` (added) |
| A9 | `crates/mcp/src` names none of `data_rights_registry`, `export_all` and `erase_all`. The census prints its examined file count, refuses zero, and first refuses a planted line by name | not red: a guard; the server reaches no rights use case at the base | `guard_census.rs` `the_server_reaches_no_rights_use_case` (added) |

```acceptance
A1: cargo test -p deck-streak-mcp --test settings -- --exact the_write_credential_grants_core_and_write
A2: cargo test -p deck-streak-mcp --test settings -- --exact a_missing_write_credential_grants_nothing
A3: cargo test -p deck-streak-mcp --test settings -- --exact a_broken_write_credential_refuses_start
A4: cargo test -p deck-streak-mcp --test settings -- --exact the_write_credential_cannot_share_a_value
A5: cargo test -p deck-streak-mcp --test guard -- --exact the_write_grant_holds_core_and_write_and_no_other
A6: cargo test -p deck-streak-mcp --test guard_census -- --exact every_write_tool_authorizes_the_write_scope
A7: cargo test -p deck-streak-mcp --test tools -- --exact the_withdrawn_tools_leave_the_portable_roster
A8: cargo test -p deck-streak-mcp --test tools -- --exact no_grant_reaches_a_withdrawn_tool
A9: cargo test -p deck-streak-mcp --test guard_census -- --exact the_server_reaches_no_rights_use_case
```

The red-first record is `docs/red-first/SPEC-369.md`, one fence line per fact: `red at <sha>:
<failure>` and `green at <sha>` for A1, A3, A4, A5 and A7, and `not red: <why>` for A2, A6, A8
and A9.

A shipped test that keeps its name and whose helper changes: `tools.rs`
`each_served_tool_matches_its_roster_golden_entry` (A41, `:134`) reads `roster()`, which now drops
`WITHDRAWN`, so a withdrawn tool served later fails it as not in the roster golden.

Shipped requirements this delivery changes, by their own SPECs (their text above section 17 is
not edited; SPEC-119 section 17 and ADR-380 record each change, and their tests keep their names):

| shipped requirement | what changes |
|---|---|
| SPEC-119 R6 to R8 | a third, optional credential and the write grant (T17 to T19) |
| SPEC-119 R15 and R16's roster rows `:141` and `:142` | withdrawn (T20, T21) |
| SPEC-119 R17 | a tool that changes data needs `write` (T22) |
| SPEC-119 R21 (`:169-173`) | withdrawn; export and erase stay on the bot and the host (T23) |
| SPEC-119 A26 | 31 tools, 33 with the two parked drill tools (T24) |
| SPEC-119 A30 and A31 (`:274-275`) | withdrawn (T25, T26) |
| SPEC-119 section 6's risk, section 7's `mcp_erase_confirm` (`:385`), section 9's `S11924-ERASE-WORD`, section 13's list | no longer, withdrawn, withdrawn, two fewer (T27 to T30) |

## 4. File manifest

| file | context | change |
|---|---|---|
| `docs/specs/SPEC-369-the-mcp-server-offers-no-export-or-erase-tool-and-its-grant-is-read-or-write.md` | docs | added |
| `docs/decisions/ADR-380-the-mcp-servers-grant-is-read-or-write-and-export-and-erase-leave-its-roster.md` | docs | added |
| `docs/specs/SPEC-119-the-mcp-server-serves-the-predecessors-tools-on-loopback-and-refuses-every-request-without-a-granted-bearer.md` | docs | sections 17 and 18 appended, insert-only (R12) |
| `docs/specs/SPEC-001-campaign-prd-parity-and-waves.md` | docs | one `Amended (§14):` line after `:266` and one bullet after `:427`, insert-only (R12) |
| `docs/schematics/mcp-guard-refusals.md` | docs | one section before `## Change note` (`:61`), one change-note line at its end (R12) |
| `docs/red-first/SPEC-369.md` | docs | added |
| `changelog.d/mcp-write-grant-369.md` | docs | added |
| `crates/mcp/src/grants.rs` | mcp | `Scope::Write`, `ALL`, `name`, `bit`, the closed-set comment (`:12`), the associated `optional`, the write grant, its shared-value check and the grant order (R1 to R6) |
| `crates/mcp/src/settings.rs` | mcp | `WRITE_CREDENTIAL` (R3) |
| `crates/mcp/tests/settings.rs` | mcp | A1 to A4 (added) |
| `crates/mcp/tests/guard.rs` | mcp | A5 (added) |
| `crates/mcp/tests/guard_census.rs` | mcp | A6 and A9 (added) |
| `crates/mcp/tests/fixtures/planted_write_under_core.rs.fixture` | mcp | added (A6's plant) |
| `crates/mcp/tests/tools.rs` | mcp | A7 and A8 (added); `roster()` drops `WITHDRAWN` |
| `crates/mcp/tests/support/mod.rs` | mcp | `WITHDRAWN` and a write-token helper (R8) |
| `scripts/mutation-rows.d/S36900-S36999.json` | scripts | added (section 8) |

## 5. What this does NOT do

- It serves no tool that changes data: each lands in a later part of `#157`, which adds the
  unit's credential line and its `CREDENTIAL_SOURCES` and `ROLE_CREDENTIALS` entries with it
  (`#157`).
- It changes no unit and provisions no credential (`#720`).
- It adds no export or erase path to the web or iOS clients (`#721`).
- It leaves the bot's `/export` and `/delete` and the host's `deckstreakd data` commands as they
  are, and SPEC-021 R1 stays true (`#14`).
- It adds no drill scope, credential or tool (`#158`).
- It edits neither the roster golden nor its registry module (`#720`).
- It changes no formal model (`#720`).

## 6. Risks

- **A write tool annotated read-only.** ADR-380 D6 sends a tool to `write` by its annotation. A
  tool that writes while annotated `readOnlyHint` true would run under the read grant. A6 holds
  the authorize call to the annotation, not the annotation to the handler, so the part that
  serves each write tool reads its handler against its annotation.
- **A deployment's core token loses write.** At the base it loses nothing: no write tool is
  served. A client that needs a write tool later needs the write credential, provisioned with
  that tool's part.
- **Anchors on `grants.rs`.** The law-track arm moves into the associated `optional`. The row
  `S11903`'s find keeps its 12 spaces there, and each find of `S11901` to `S11905`, `S11929` and
  `S11930` must still occur exactly once. The build audits them staged and re-kills every row on
  the files it changes.
- **A withdrawn tool served later.** `roster()` drops `WITHDRAWN`, so a served withdrawn tool
  fails A41 as not in the roster golden, and A7 refuses a `WITHDRAWN` name the golden does not
  hold.
- **An unprovisioned write credential once the unit names it.** A credential id the rail does not
  hold can answer empty, and an empty credential refuses start (A3), so the unit line waits for
  the first write tool (R11, ADR-380 D3).

## 7. Formal model

None (ADR-380 D9). BearerGuard covers `admit`, `authorize`, `call`, `decide` and
`get_law_track`, and none is edited, so no cover's digest moves. The model's abstraction
(`BearerGuard.tla:49-50`) treats law as a scope the grant lacks and states that a second grant
adds nothing the limiter can see; `write` is one more scope a grant may lack, so the model stands
for it. The change lives in `grants.rs` and `settings.rs`, which nothing covers, and A1 to A5
test the load and the grants.

## 8. Mutation rows

Band `S36900-S36999`, fragment `scripts/mutation-rows.d/S36900-S36999.json`, shaped
`{"tables": {"MUTATIONS": [[stem, crate, file, find, replace, killer, description]]}}`, crate
`mcp`. Each find is the formatted line the build writes and occurs exactly once in its target.

| stem | file | find -> replace | killer |
|---|---|---|---|
| `S36901-WRITE-BIT` | `src/grants.rs` | `Self::Write => 4,` -> `Self::Write => 1,` | `guard::the_write_grant_holds_core_and_write_and_no_other` |
| `S36902-WRITE-NAME` | `src/grants.rs` | `Self::Write => "write",` -> `Self::Write => "law_track",` | `settings::the_write_credential_grants_core_and_write` |
| `S36903-ALL-SCOPES` | `src/grants.rs` | `[Self::Core, Self::LawTrack, Self::Write];` -> `[Self::Core, Self::LawTrack, Self::LawTrack];` | `settings::the_write_credential_grants_core_and_write` |
| `S36904-READ-GRANT` | `src/grants.rs` | `Scopes::NONE.with(Scope::Core),` -> `Scopes::NONE.with(Scope::Core).with(Scope::Write),` | `guard::the_write_grant_holds_core_and_write_and_no_other` |
| `S36905-WRITE-GRANT` | `src/grants.rs` | `Scopes::NONE.with(Scope::Core).with(Scope::Write),` -> `Scopes::NONE.with(Scope::Core),` | `settings::the_write_credential_grants_core_and_write` |
| `S36906-WRITE-ID` | `src/settings.rs` | `"mcp-write-token";` -> `"mcp-write-tokens";` | `settings::the_write_credential_grants_core_and_write` |
| `S36907-WRITE-SHARED` | `src/grants.rs` | `bool::from(write.digest.ct_eq(&other.digest))` -> `bool::from(write.digest.ct_ne(&other.digest))` | `settings::the_write_credential_cannot_share_a_value` |

`S36906` pins a literal: a mutation tool never changes a literal in a `const` initializer, so the
value owes a hand row, and its killer writes the credential under the literal id rather than
through the constant it mutates.

Existing rows the diff re-kills: `S11901` to `S11905`, `S11928`, `S11929` and `S11930`
(`scripts/mutation-rows.d/S11900-S11999.json`).

## 9. What only CI proves

| # | what | where |
|---|---|---|
| C1 | Every row `S36901` to `S36907`, and every existing row on `crates/mcp/src/grants.rs` and `crates/mcp/src/settings.rs`, is killed by its killer | the pull request's mutation job |
| C2 | BearerGuard's covers read unchanged | the builder's formal ratchet run, quoted in the PR body |
| C3 | The workspace builds, lints and tests whole | CI on the pull request |
