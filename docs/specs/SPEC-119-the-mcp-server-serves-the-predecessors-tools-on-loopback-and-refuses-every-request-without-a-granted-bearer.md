# SPEC-119: the MCP server serves the predecessor's tools on loopback and refuses every request without a granted bearer

- **Wave:** W6. **Issues:** #157 (the MCP server) and #158 (fail-closed bearer auth for the law and
  drill tools) (epic #7). **Context(s):** `deck-streak-mcp`, a new adapter (the server, its guard,
  its limiter and its roster); `deck-streak-daemon` (the `mcp` role); `docs/CONTEXT-MAP.md` (the
  adapter's line).
- **Decided by:** ADR-002 (one crate per context; the map is the crate graph), ADR-025 (a
  concurrency bound sheds instead of queueing), ADR-037 (owner triggers of the sync), ADR-054 (no-AI
  mode is the default), ADR-059 (public text describes DeckStreak only), ADR-067 (the loader refuses
  an empty credential by its id), ADR-119 (the server is Rust on `rmcp`, in its own adapter crate,
  on loopback), ADR-121 (the guard fails closed on every request, compares digests in constant
  time, and keeps the predecessor's limiter) and ADR-320 (the guard lands first, as a library of
  the adapter crate with the kernel edge only; the server and its tools follow).
- **Prerequisites:** SPEC-020, SPEC-021, SPEC-022, SPEC-066, SPEC-071, SPEC-072, SPEC-073,
  SPEC-075, SPEC-076, SPEC-077, SPEC-078, SPEC-079, SPEC-080, SPEC-083, SPEC-085, SPEC-086, SPEC-090,
  SPEC-091, SPEC-092 and SPEC-093 (planned, W4) and SPEC-110. **Mutation band:**
  `S11900-S11999`.
- **Status:** delivered in part (moved from `docs/specs/planned/` with its tests and
  `docs/red-first/SPEC-119.md`, ADR-016): #158's pull request delivers the guard (section 3,
  ADR-320); #157 delivers the server and its tools, and the drill tools follow R19 (section 3c).

## 1. The problem, measured

- **The predecessor's tool server** (`server.py:create_server`, at `27ee2bc`) registers 35 tools on
  the streamable HTTP transport: 31 always, `get_law_track` behind a bearer scope, and `list_drills`
  and `get_drill` only when its drill flag is on. Read tools carry `readOnlyHint`, `idempotentHint`
  and a closed world (`server.py:_RO`); a writer's `{"ok": false}` becomes a tool error
  (`server.py:_raise_on_error`); three tools clamp their arguments; and thirteen chart resources
  answer images (`server.py:_CHART_RESOURCES`).
- **Its guard.** `mcp_auth.py:DrillAuth.require` checks per-scope tokens (`law_track`, `drills`), denies with the one message `unauthorized`, and counts failures per bucket: five in 60 seconds, 512 buckets, ten timestamps a bucket (#158).
- **DeckStreak has no such surface.** No crate speaks MCP, and the agent context cannot hold one:
  a tool calls a use case of `coordination`, and `coordination` depends on `agent`, so the edge would
  be a cycle (ADR-119). The Mini App's routes answer an owner session (SPEC-024), which a machine
  client does not have.
- **What changes, and why.** Every request needs a granted bearer; the bearer
  travels in the `Authorization` header, never in a tool argument that a client writes into its
  transcript; the comparison is over digests, so a token's length leaks nothing; and the chart
  resources answer the JSON series SPEC-085 serves, never an image (SPEC-085 R1, R11). Each change
  is ADR-121's or ADR-119's, and each keeps the predecessor's numbers.

## 2. Requirements

The server

R1. `deck-streak-mcp` is an adapter crate: it depends on `deck-streak-kernel` and
    `deck-streak-coordination`, and `deck-streak-daemon` depends on it. `docs/CONTEXT-MAP.md` gains
    its line in the map's fence, and nothing else gains an edge (ADR-002, ADR-119).
R2. `deckstreakd mcp` is a role (`crates/daemon/src/role_mcp.rs`), run by
    `deploy/systemd/deck-streak-mcp.service`. It serves MCP's streamable HTTP transport through
    `rmcp` at the one path `/mcp`: JSON responses, no session kept between requests, the allowed
    hosts the loopback names, and a request body of at most 64 KiB. Any other path answers 404.
R3. The listen address is the setting `DECKSTREAK_MCP_LISTEN`, which must be a loopback address and
    port. Unset, unparseable or not loopback, the role refuses start and names the setting
    (`McpError::Listen`), the shape of the API's own setting (SPEC-025 R6).
R4. At most 8 requests are in flight (the plan's bound: one owner's agent needs few, and a small bound sheds a runaway client before it holds the listener); a ninth is shed with 503 at once, never queued (ADR-025's
    pattern: a global concurrency limit inside a load shed).
R5. No tool runs a model, so every tool answers the same with the AI route absent (ADR-054), and no
    W6 duty is given an MCP tool (SPEC-043 R5's `--strict-mcp-config`).

The credentials (#158)

R6. The role reads its tokens only through `CredentialLoader::load` (SPEC-066), never from the
    environment, an argument, a query string, a row or a file in the repository:
    - `mcp-core-token` is required: every loader error refuses start by its id.
    - `mcp-law-track-token` is optional: `Missing` grants nothing, and `Empty`, `Unreadable` and
      `NotText` refuse start by its id.
    - `mcp-drills-token` follows the same rule, and is read only when the drill tools are on (R19).
R7. A loaded token shorter than 32 characters refuses start by its id (`McpError::WeakCredential`),
    and two credentials that hold one value refuse start (`McpError::SharedCredential`), because a
    shared value would give one token both scopes.
R8. The grants: the core token grants `{core}`; the law-track token `{core, law_track}`; the drills
    token `{core, drills}`. The scope names `law_track` and `drills` equal the predecessor's
    (`mcp_auth.py:SCOPE_LAW_TRACK`, `SCOPE_DRILLS`; `goldens/mcp_auth.constants.json`).

The guard (#158)

R9. **The bearer.** A request presents a token only as one `Authorization` header whose value is
    the scheme `Bearer` (any case), one space, and a token of visible ASCII characters (0x21 to
    0x7E) only. An absent header, an empty value, another scheme, a second header, an empty token,
    a space inside the token or any other byte presents no token.
R10. **The match.** The presented token's SHA-256 digest is compared with each grant's digest,
    computed once at start, by `subtle::ConstantTimeEq::ct_eq`, folding over every grant with no
    early exit. No other comparison of a token, a digest or the header exists in the crate.
R11. **The request refusal.** A request that presents no token matching a grant is answered before
    `rmcp` sees it, `initialize` and `tools/list` included: HTTP 401, the body `unauthorized`
    (`mcp_auth.py:DENIED_MESSAGE`), `WWW-Authenticate: Bearer`, and nothing else. An absent, empty,
    malformed, wrong or rate-limited bearer is answered byte for byte the same.
R12. **The scope refusal.** A tool call whose matched grant lacks the tool's scope (R17) is answered
    with a tool error whose whole text is `unauthorized`, and no data. It differs from R11's refusal
    only in its layer: #158 asks that a wrong scope and a wrong token read the same, and here both
    carry the one word, while a request that reached a tool has already proved its token grants
    `core` (ADR-121).
R13. **The limiter** (`mcp_auth.py:DrillAuth.require`, `_bucket_id`, `_is_rate_limited`,
     `_record_failure`; `goldens/mcp_auth_limiter.json`, `goldens/mcp_auth_bucket.json`). Its
     outcome set is exactly {`allowed`, `denied`, `rate_limited`}:
     - a presented token whose grant holds the scope is `allowed` before the limiter reads anything,
       so a token can never lock itself out of its own scope;
     - otherwise the bucket is the first 16 hexadecimal characters of the SHA-256 of the presented
       value: the token when one parses, else the header's whole value, else `\x00absent`;
     - a failure is fresh while less than 60 seconds old; with 5 or more fresh failures the outcome
       is `rate_limited` and nothing is recorded;
     - otherwise the outcome is `denied`, the failure is recorded (a bucket keeps its newest 10),
       the bucket becomes the newest, and the oldest bucket beyond 512 is evicted.
     The limiter's clock is injected, and it keeps no row: its buckets die with the process.
R14. **The log.** Each refusal writes one `warn` event naming the outcome, the bucket and the scope
     (`core` for R11, the tool's scope for R12), and never the token, the header's value or a digest
     longer than the bucket. The redactor already holds every token (SPEC-066).

The tools (#157)

R15. **The roster.** The tools are exactly the predecessor's (`server.py:create_server`,
     `goldens/mcp_roster.json`): their names, the four annotations, the parameter names and
     defaults, and the output fields, with the predecessor's `token` parameter removed from
     `get_law_track`, `list_drills` and `get_drill` (R9 replaces it). A stable shape declares its
     output schema and answers structured content.
R16. **The owners.** Each tool calls the use case of the SPEC that owns its data, through
     `coordination`, and holds none of its rules; a write keeps its SPEC's idempotency guard and, for
     XP, its grant through the grant port (SPEC-040).

| tools | scope | kind | owner |
|---|---|---|---|
| `get_stats` | core | read | SPEC-086 (the today snapshot) |
| `get_score`, `get_weekly_report` | core | read | SPEC-071 |
| `get_level` | core | read | SPEC-072 |
| `get_badges` | core | read | SPEC-073 |
| `get_leaderboard`, `get_xp_exchange_rates` | core | read | SPEC-075 |
| `get_streak` | core | read | SPEC-076 |
| `get_progress` | core | read | SPEC-077 |
| `get_habits`, `get_reading_analytics`, `get_reading_retention_correlation` | core | read | SPEC-078 |
| `log_reading`, `log_writing` | core | write | SPEC-078 |
| `get_focus`, `get_focus_analytics` | core | read | SPEC-079 |
| `log_focus` | core | write | SPEC-079 |
| `get_quests` | core | read | SPEC-080 |
| `get_skip_history` | core | read | SPEC-083 |
| `take_skip_day`, `undo_skip_day` | core | write | SPEC-083 |
| `get_forecast`, `get_goal`, `get_balance` | core | read | SPEC-090 |
| `get_memory` | core | read | SPEC-091 |
| `get_strands`, `get_weakspots` | core | read | SPEC-092 |
| `get_leeches` | core | read | SPEC-093 |
| `remediate_leech` | core | write | SPEC-093 |
| `export_data` | core | read | SPEC-021 |
| `erase_all_data` | core | write | SPEC-021 |
| `force_sync` | core | write | SPEC-022, SPEC-071 |
| `get_law_track` | `law_track` | read | SPEC-077 (the law track's numbers) |
| `list_drills`, `get_drill` | `drills` | read | SPEC-110 |

R17. **The scopes.** `get_law_track` needs `law_track`, the drill tools need `drills`, and every
     other tool needs `core`.
R18. **The clamps** (`server.py:create_server`'s `get_badges`, `get_weekly_report` and
     `get_xp_exchange_rates`; `goldens/mcp_clamps.json`): the badges' `limit` defaults to 50 and is
     clamped to 1..200; the weekly report's `days` defaults to 7 and is clamped to 1..62; the
     exchange rates' `days` of 0 or less reads the whole ledger, and any other value reads
     `min(3650, days)` study days ending today (the 04:00 rollover, SPEC-020). The reading tools'
     `window_days` (28) and `weeks` (12) pass to SPEC-078's read model unchanged, as the
     predecessor's do.
R19. **The drill tools** are registered only when `DECKSTREAK_MCP_DRILLS` reads `on`; unset or `off`
     they are absent from `tools/list`, and any other value refuses start (`McpError::Drills`).
     `list_drills` answers at most 25 of SPEC-110's active drills with titles cut to 120 characters;
     `get_drill` answers one drill's body cut to 4000 characters with `truncated`, and
     `content_is_untrusted: true` with `content_kind: vault-authored-drill`. An id SPEC-110's safe
     rule refuses, or no active drill, answers the tool error `drill not found`. Its audit event
     carries `drill_ref`, the first 12 hexadecimal characters of the id's SHA-256, never the id or
     the body (`server.py:DRILL_LIST_MAX`, `DRILL_TITLE_MAX_CHARS`, `DRILL_BODY_MAX_CHARS`,
     `DRILL_REF_LEN`, `_drill_ref`; `goldens/mcp_drills.constants.json`,
     `goldens/mcp_drill_ref.json`, `goldens/mcp_drill_view.json`). Every cut counts characters,
     never bytes.
R20. **The writers.** A writer's failure is a tool error, never a result carrying `ok: false`
     (`server.py:_raise_on_error`).
R21. **Export and erase** (`goldens/mcp_erase_confirm.json`). `export_data` answers
     `coordination::data_rights_registry::export_all`'s document (SPEC-021). `erase_all_data` runs
     `erase_all` only when `confirm` is exactly `ERASE`, the word the `data` role takes; any other
     value answers the tool error `pass confirm='ERASE' to permanently delete all data` and erases
     nothing (`server.py:create_server`'s `erase_all_data`).
R22. **The sync** (`goldens/mcp_sync_out.json`). `force_sync` runs `coordination::sync_cycle` with
     the owner's trigger (SPEC-022 R17: debounced as `/sync` is) and answers the predecessor's
     `SyncOut` fields. Its `recompute` outcome set is exactly {`full`, `skipped`, `read_failed`}
     (`pipeline.py:GamifyPipeline._run_sync_cycle_impl` and `_maybe_skip_recompute`): `full` for a
     recompute that ran, `skipped` for the gate's skip, and `read_failed` for a cycle whose window
     could not be read. Any other cycle error answers the tool error `sync failed`, with no text of
     the error.
R23. **The chart resources.** Each name of SPEC-085 R6's closed set is a resource at
     `charts://<name>`, `application/json`, answering the payload `GET /api/charts/{name}` answers,
     under the `core` scope. An unknown name is a resource error. No resource is an image.

## 3. Acceptance criteria

This pull request (#158) delivers the guard: the criteria of this table, and A40 (section 11). The
rest of the SPEC is #157's and the drill tools' (section 3c), and moves back here when they
deliver it.

| id | criterion | decided by |
|---|---|---|
| A2 | a missing, empty, unreadable or non-text `mcp-core-token` each refuses start by its id | `the_core_credential_is_required` |
| A3 | a missing law-track or drills credential grants nothing, and the core token still starts the role | `a_missing_scope_credential_grants_nothing` |
| A4 | an empty, unreadable or non-text law-track or drills credential refuses start by its id | `a_broken_scope_credential_refuses_start` |
| A5 | a 31-character token refuses start by its id, and a 32-character one starts | `a_short_credential_refuses_start` |
| A6 | two credentials holding one value refuse start | `two_credentials_with_one_value_refuse_start` |
| A9 | a request with no `Authorization` header, `initialize` among them, is answered 401 `unauthorized` with `WWW-Authenticate: Bearer`, and the server never sees it | `a_request_with_no_bearer_is_refused` |
| A10 | an empty header, `Bearer` alone, an empty token, another scheme, two headers, a token holding a space and a token holding a non-ASCII byte are each refused as A9 | `a_malformed_bearer_is_refused` |
| A11 | a token differing in its last character, a prefix of a granted token, and a granted token with one character added are each refused as A9 | `a_wrong_bearer_is_refused` |
| A12 | the absent, empty, malformed, wrong and rate-limited refusals are byte for byte the same response, the date header aside | `every_request_refusal_is_the_same_response` |
| A15 | a granted token in the query string, with no header, is refused as A9 | `a_token_in_the_query_string_is_ignored` |
| A16 | in `crates/mcp/src/`, a token, digest or header value is compared only by `ct_eq` over SHA-256 digests, in one fold with no early exit; a planted `==` on a token is refused | `the_guard_compares_only_digests_in_constant_time` |
| A17 | the limiter equals its golden: the fifth and sixth failures, 59.999 and 60.000 seconds after the first, a limited failure recording nothing, the eleventh failure's history, and the 513th bucket's eviction | `the_limiter_matches_its_golden` |
| A18 | the bucket equals its golden for a token, a malformed header's whole value, an empty value and no header | `the_bucket_matches_its_golden` |
| A19 | the limiter's constants and the denial word equal the golden | `the_guard_constants_match_the_golden` |
| A20 | after ten out-of-scope failures, the core token still reaches every core tool | `a_granted_token_never_limits_itself` |
| A21 | each refusal writes one `warn` event with its outcome, bucket and scope, and no event holds the token, the header's value or a full digest | `a_refusal_logs_the_bucket_and_never_the_token` |

```acceptance
A2: cargo test -p deck-streak-mcp --test settings -- --exact the_core_credential_is_required
A3: cargo test -p deck-streak-mcp --test settings -- --exact a_missing_scope_credential_grants_nothing
A4: cargo test -p deck-streak-mcp --test settings -- --exact a_broken_scope_credential_refuses_start
A5: cargo test -p deck-streak-mcp --test settings -- --exact a_short_credential_refuses_start
A6: cargo test -p deck-streak-mcp --test settings -- --exact two_credentials_with_one_value_refuse_start
A9: cargo test -p deck-streak-mcp --test guard -- --exact a_request_with_no_bearer_is_refused
A10: cargo test -p deck-streak-mcp --test guard -- --exact a_malformed_bearer_is_refused
A11: cargo test -p deck-streak-mcp --test guard -- --exact a_wrong_bearer_is_refused
A12: cargo test -p deck-streak-mcp --test guard -- --exact every_request_refusal_is_the_same_response
A15: cargo test -p deck-streak-mcp --test guard -- --exact a_token_in_the_query_string_is_ignored
A16: cargo test -p deck-streak-mcp --test guard_census -- --exact the_guard_compares_only_digests_in_constant_time
A17: cargo test -p deck-streak-mcp --test limiter -- --exact the_limiter_matches_its_golden
A18: cargo test -p deck-streak-mcp --test limiter -- --exact the_bucket_matches_its_golden
A19: cargo test -p deck-streak-mcp --test limiter -- --exact the_guard_constants_match_the_golden
A20: cargo test -p deck-streak-mcp --test limiter -- --exact a_granted_token_never_limits_itself
A21: cargo test -p deck-streak-mcp --test limiter -- --exact a_refusal_logs_the_bucket_and_never_the_token
```

## 3a. What the box run judges

The box run (`scripts/box-packs.sh`, ADR-069) judges these over the committed tree and posts its
verdict on the pull request as the `box/packs` status. They have no line in the acceptance fence,
because no public test can run a pack's row. The rust-service, observability and durable-services
packs are already enforced, and this delivery changes no pack's state, so the private wiring does
not change when it merges.

| id | criterion | decided by |
|---|---|---|
| B1 | over every binary of the workspace that serves HTTP, `deckstreakd` with its `mcp` role among them: no secret is read from the environment, every request body read is bounded, and a concurrency bound holds | the rust-service pack |
| B2 | over the same binaries: request headers are marked sensitive before the trace layer logs them, so no `Authorization` value reaches the journal, and no log field carries a secret | the observability pack |
| B3 | over the units under `deploy/systemd/`, `deck-streak-mcp.service` among them: the service's type, restart, sandbox and memory settings, its credentials passed as credentials and never in an `Environment=` line, and its memory within the budget the other units leave | the durable-services pack |

B1 to B3 are judged when #157 adds the role, its binary path and its unit; #158's pull request
adds no binary and no unit.

## 3c. Delivered by the next pull requests

This SPEC lands in parts. This one (#158) delivers the guard as a library of `deck-streak-mcp`:
R1's crate with its kernel edge only, R6 to R11, R13 and R14 for the core and law-track
credentials, R12's scope decision, A2 to A6, A9 to A12, A15 to A21, and A40 (section 11). #157
delivers the server: R1's `coordination` edge and `deck-streak-daemon`'s line, R2 to R5, the tool
error that carries R12's decision, R15 to R18 and R20 to R23. The drill tools follow R19, with
#157 and #158. The table below holds the criteria those parts deliver, each row naming its part,
and the lines under it are their fence lines, each prefixed `SERVER: ` or `DRILLS: `. Each part
moves its criteria back verbatim: the row into section 3's table, without the `delivered by`
column, and the fence line into the acceptance fence, without the prefix. B1 to B3 (section 3a)
are judged when #157 adds their files.

| id | criterion | decided by | delivered by |
|---|---|---|---|
| A1 | `127.0.0.1` and `::1` are accepted; `0.0.0.0`, `::`, a private-range address, unset and unparseable each refuse start naming `DECKSTREAK_MCP_LISTEN` | `the_listen_address_must_be_loopback` | #157 |
| A7 | with the drill tools off, the drills credential is never read: an empty one does not refuse start | `the_drills_credential_is_read_only_when_drills_are_on` | #157 and #158, with the drill tools (R19) |
| A8 | `DECKSTREAK_MCP_DRILLS` unset or `off` is off, `on` is on, and any other value refuses start | `the_drills_setting_is_off_unless_it_reads_on` | #157 and #158, with the drill tools (R19) |
| A13 | the core token reaches every core tool and no other; the law-track token adds `get_law_track`; the drills token adds the drill tools; `bearer` in lower case is accepted | `each_grant_reaches_its_scopes_and_no_other` | #157 |
| A14 | a tool outside the token's scope answers a tool error whose whole text is `unauthorized`, and no data | `a_tool_outside_the_scope_answers_unauthorized` | #157 |
| A22 | `/health`, `/metrics` and `/` answer 404 to a granted bearer and 401 to none | `only_the_mcp_path_is_served` | #157 |
| A23 | a body over 64 KiB is refused and runs no tool | `a_body_over_64_kib_is_refused` | #157 |
| A24 | a request whose Host is not a loopback name is refused and runs no tool | `a_foreign_host_is_refused` | #157 |
| A25 | with 8 requests held in a scripted use case, a ninth is answered 503 within 5 seconds | `the_ninth_request_in_flight_is_shed` | #157 |
| A26 | with the drill tools off, the roster equals the golden's 33 tools | `the_roster_matches_the_golden_with_drills_off` | #157 |
| A27 | with the drill tools on, the roster equals the golden's 35 tools | `the_roster_matches_the_golden_with_drills_on` | #157 and #158, with the drill tools (R19) |
| A28 | each clamp equals its golden, the cases 0, 1, 200, 201, 62, 63, 3650 and 3651 among them, and the exchange window ends on today's study day | `the_clamps_match_the_golden` | #157 |
| A29 | a writer whose use case fails answers a tool error, never a result carrying `ok: false` | `a_failed_write_is_a_tool_error` | #157 |
| A30 | `erase_all_data` with `confirm` empty, `erase` or `ERASE ` answers the golden's refusal and erases nothing; `ERASE` empties every table | `erase_runs_only_with_the_word_erase` | #157 |
| A31 | `export_data` answers the data-rights registry's export document | `export_answers_the_data_rights_document` | #157 |
| A32 | `force_sync` answers `full`, `skipped` and `read_failed` as its golden does, and another cycle error answers `sync failed` | `force_sync_reports_each_recompute_value` | #157 |
| A33 | `get_law_track` answers exactly the golden's numeric fields | `the_law_track_answers_numbers_only` | #157 |
| A34 | the drill list and view equal the golden: 25 of 26 drills, a 121-character title, bodies of 4000 and 4001 characters, a multibyte title | `the_drill_tools_match_the_golden` | #157 and #158, with the drill tools (R19) |
| A35 | the ids `a/b`, `a\b`, `..`, the empty id and an unknown id answer `drill not found`, and the audit event carries a 12-character reference and never the id | `an_unsafe_or_unknown_drill_is_not_found` | #157 and #158, with the drill tools (R19) |
| A36 | the drill reference equals its golden | `the_drill_ref_matches_the_golden` | #157 and #158, with the drill tools (R19) |
| A37 | the resources are exactly SPEC-085 R6's names as `charts://<name>`, each `application/json` with the route's payload, and an unknown name is a resource error | `each_chart_resource_answers_its_series` | #157 |
| A38 | `deckstreakd mcp` is a role, not refused as an unknown one | `the_mcp_role_is_a_known_role` | #157 |
| A39 | with the scripted loader answering `Missing` for `mcp-core-token`, a token-shaped value in the process environment, on the command line and in a file of the working directory still refuses start by its id | `the_role_reads_its_tokens_only_through_the_loader` | #157 |

SERVER: A1: cargo test -p deck-streak-mcp --test settings -- --exact the_listen_address_must_be_loopback
DRILLS: A7: cargo test -p deck-streak-mcp --test settings -- --exact the_drills_credential_is_read_only_when_drills_are_on
DRILLS: A8: cargo test -p deck-streak-mcp --test settings -- --exact the_drills_setting_is_off_unless_it_reads_on
SERVER: A13: cargo test -p deck-streak-mcp --test guard -- --exact each_grant_reaches_its_scopes_and_no_other
SERVER: A14: cargo test -p deck-streak-mcp --test guard -- --exact a_tool_outside_the_scope_answers_unauthorized
SERVER: A22: cargo test -p deck-streak-mcp --test server -- --exact only_the_mcp_path_is_served
SERVER: A23: cargo test -p deck-streak-mcp --test server -- --exact a_body_over_64_kib_is_refused
SERVER: A24: cargo test -p deck-streak-mcp --test server -- --exact a_foreign_host_is_refused
SERVER: A25: cargo test -p deck-streak-mcp --test server -- --exact the_ninth_request_in_flight_is_shed
SERVER: A26: cargo test -p deck-streak-mcp --test roster -- --exact the_roster_matches_the_golden_with_drills_off
DRILLS: A27: cargo test -p deck-streak-mcp --test roster -- --exact the_roster_matches_the_golden_with_drills_on
SERVER: A28: cargo test -p deck-streak-mcp --test tools -- --exact the_clamps_match_the_golden
SERVER: A29: cargo test -p deck-streak-mcp --test tools -- --exact a_failed_write_is_a_tool_error
SERVER: A30: cargo test -p deck-streak-mcp --test tools -- --exact erase_runs_only_with_the_word_erase
SERVER: A31: cargo test -p deck-streak-mcp --test tools -- --exact export_answers_the_data_rights_document
SERVER: A32: cargo test -p deck-streak-mcp --test tools -- --exact force_sync_reports_each_recompute_value
SERVER: A33: cargo test -p deck-streak-mcp --test tools -- --exact the_law_track_answers_numbers_only
DRILLS: A34: cargo test -p deck-streak-mcp --test drills -- --exact the_drill_tools_match_the_golden
DRILLS: A35: cargo test -p deck-streak-mcp --test drills -- --exact an_unsafe_or_unknown_drill_is_not_found
DRILLS: A36: cargo test -p deck-streak-mcp --test drills -- --exact the_drill_ref_matches_the_golden
SERVER: A37: cargo test -p deck-streak-mcp --test resources -- --exact each_chart_resource_answers_its_series
SERVER: A38: cargo test -p deck-streak-daemon --test roles -- --exact the_mcp_role_is_a_known_role
SERVER: A39: cargo test -p deck-streak-mcp --test settings -- --exact the_role_reads_its_tokens_only_through_the_loader

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/mcp/Cargo.toml` | `deck-streak-mcp` | added: `rmcp` (server, macros, the streamable HTTP server transport), `sha2`, `subtle`, `tower` (limit, load-shed), `axum`, `schemars` |
| `crates/mcp/src/lib.rs` | `deck-streak-mcp` | added: the modules, `#![forbid(unsafe_code)]` |
| `crates/mcp/src/settings.rs` | `deck-streak-mcp` | added: the listen address, the drill setting, the credential ids and `McpError` |
| `crates/mcp/src/grants.rs` | `deck-streak-mcp` | added: loading, the length and sharing rules, the digests |
| `crates/mcp/src/guard.rs` | `deck-streak-mcp` | added: the bearer, the match, the request refusal, the tower layer |
| `crates/mcp/src/limiter.rs` | `deck-streak-mcp` | added: the buckets, the window, the eviction |
| `crates/mcp/src/server.rs` | `deck-streak-mcp` | added: the service, the path, the body cap, the shed |
| `crates/mcp/src/tools.rs` | `deck-streak-mcp` | added: the roster, the scopes, the clamps, the writers, export, erase and the sync |
| `crates/mcp/src/drills.rs` | `deck-streak-mcp` | added: the drill tools, their caps and reference |
| `crates/mcp/src/resources.rs` | `deck-streak-mcp` | added: the chart resources |
| `crates/mcp/tests/settings.rs` | `deck-streak-mcp` | added: A1 to A8, A39 |
| `crates/mcp/tests/guard.rs` | `deck-streak-mcp` | added: A9 to A15 |
| `crates/mcp/tests/guard_census.rs` | `deck-streak-mcp` | added: A16 |
| `crates/mcp/tests/limiter.rs` | `deck-streak-mcp` | added: A17 to A21 |
| `crates/mcp/tests/server.rs` | `deck-streak-mcp` | added: A22 to A25 |
| `crates/mcp/tests/roster.rs` | `deck-streak-mcp` | added: A26, A27 |
| `crates/mcp/tests/tools.rs` | `deck-streak-mcp` | added: A28 to A33 |
| `crates/mcp/tests/drills.rs` | `deck-streak-mcp` | added: A34 to A36 |
| `crates/mcp/tests/resources.rs` | `deck-streak-mcp` | added: A37 |
| `crates/daemon/Cargo.toml` | `deck-streak-daemon` | changed: depends on `deck-streak-mcp` |
| `crates/daemon/src/role_mcp.rs` | `deck-streak-daemon` | added: the role, its lifecycle and its credentials |
| `crates/daemon/src/main.rs`, `crates/daemon/src/lib.rs` | `deck-streak-daemon` | changed: the role `mcp` |
| `crates/daemon/src/wiring.rs` | `deck-streak-daemon` | changed: each tool's use case |
| `crates/daemon/tests/roles.rs` | `deck-streak-daemon` | changed: A38 |
| `deploy/systemd/deck-streak-mcp.service` | deploy | added: the role, the three credentials |
| `docs/CONTEXT-MAP.md` | docs | changed: `deck-streak-mcp` in the map's fence, and `deck-streak-daemon`'s line names it (ADR-119) |
| `Cargo.toml`, `Cargo.lock`, `deny.toml`, `stack.json` | workspace | changed: the member and `rmcp` |
| `tools/parity-oracle/registry/spec_119.py` | repo | added: the goldens of §7 |
| `tools/parity-oracle/goldens/mcp_roster.json`, `mcp_clamps.json`, `mcp_auth.constants.json`, `mcp_auth_bucket.json`, `mcp_auth_limiter.json`, `mcp_drills.constants.json`, `mcp_drill_ref.json`, `mcp_drill_view.json`, `mcp_erase_confirm.json`, `mcp_sync_out.json` | repo | added: generated by §7 |
| `scripts/mutation-rows.d/S11900-S11999.json` | repo | added: the rows of §9 |
| `docs/specs/SPEC-119-the-mcp-server-serves-the-predecessors-tools-on-loopback-and-refuses-every-request-without-a-granted-bearer.md` | docs | moved from `docs/specs/planned/` |
| `docs/schematics/mcp-guard-refusals.md` | docs | added by the W6 architect turn; this delivery corrects it only where the code proves it wrong |
| `docs/red-first/SPEC-119.md` | docs | added |
| `changelog.d/` fragment | repo | added |

## 5. What this does NOT do

- It serves no health, readiness, metrics or dashboard route on the MCP listener; the service's
  health is the API's (#18).
- It gives no W6 duty an MCP tool; which tools a duty may hold is the agent core's (#29).
- It configures nothing that carries the loopback port off the host; how the owner's client reaches
  it is the private rail's (#157).
- It takes no token as a tool argument, the predecessor's shape; the header replaces it (#158).
- It serves no badges gallery or leech breakdown resource (#74, #133), and no collection atlas
  (#282, SPEC-120).
- It builds no settings screen for the drill flag or the tokens (#57).
- It changes no Mini App route and no bot command (#157).

## 6. Risks

- **A token guessed by varying it on every attempt.** Each guess gets a fresh bucket, so the limiter bounds memory rather than guessing; R7's 32-character floor makes a
  guess hopeless. Detected by A5 and A17.
- **A leaked core token erases the ledger.** The word `ERASE`, the loopback listener and the
  32-character floor all stand between; detected by A1, A5 and A30.
- **A token in the journal.** R14 and the redactor; detected by A21 and B2.
- **A long sync holding the listener.** R4 sheds past 8 in flight; detected by A25.

## 7. Parity goldens

Registered in `tools/parity-oracle/registry/spec_119.py` and generated on the owner's checkout of the
predecessor at `27ee2bc` (SPEC-029). Every case is synthetic, and every token is built from parts.

| golden | the predecessor's function | kind | the adapter builds |
|---|---|---|---|
| `mcp_roster` | `server.py:create_server` | adapter | the server over a scripted pipeline and synthetic settings, with the drill flag off and on; from `list_tools`, each tool's name, annotations, parameter names and defaults, and output fields |
| `mcp_clamps` | `server.py:create_server` (`get_badges`, `get_weekly_report`, `get_xp_exchange_rates`) | adapter | a scripted pipeline recording the argument each tool passes on, over `limit` -5, 0, 1, 50, 200, 201; `days` -1, 0, 1, 62, 63; exchange `days` -1, 0, 1, 3650, 3651 |
| `mcp_auth.constants` | `mcp_auth.py:_RATE_LIMIT_WINDOW_SECS`, `_RATE_LIMIT_MAX_FAILURES`, `_RATE_LIMIT_MAX_BUCKETS`, `_RATE_LIMIT_BUCKET_HISTORY_CAP`, `DENIED_MESSAGE`, `SCOPE_LAW_TRACK`, `SCOPE_DRILLS` | constants | none |
| `mcp_auth_bucket` | `mcp_auth.py:DrillAuth._bucket_id` | function | synthetic tokens, the empty token among them |
| `mcp_auth_limiter` | `mcp_auth.py:DrillAuth.require`, `_is_rate_limited`, `_record_failure` | adapter | a `DrillAuth` over synthetic grants and a scripted clock: five failures and the sixth, checks at 59.999 and 60.000 seconds after the first, a limited failure, the eleventh failure's history, 513 buckets, and an in-scope call after ten failures |
| `mcp_drills.constants` | `server.py:DRILL_LIST_MAX`, `DRILL_TITLE_MAX_CHARS`, `DRILL_BODY_MAX_CHARS`, `DRILL_REF_LEN` | constants | none |
| `mcp_drill_ref` | `server.py:_drill_ref` | function | synthetic ids, a multibyte one among them |
| `mcp_drill_view` | `server.py:create_server` (`list_drills`, `get_drill`) | adapter | the drill flag on, a scripted vault reader serving 26 synthetic drills, a 121-character title, bodies of 4000 and 4001 characters, a multibyte title, an unsafe id and an unknown one |
| `mcp_erase_confirm` | `server.py:create_server` (`erase_all_data`) | adapter | a scripted pipeline recording whether erase ran, for `confirm` empty, `erase`, `ERASE ` and `ERASE` |
| `mcp_sync_out` | `server.py:create_server` (`force_sync`), `pipeline.py:GamifyPipeline._run_sync_cycle_impl`, `_maybe_skip_recompute` | adapter | a scripted pipeline whose cycle reports each `recompute` value |

The `token` parameters are removed from `mcp_roster` by the adapter (R15), and the predecessor's
image resources are left out on purpose (R23).

## 8. Tables and the v9 import

No table. The limiter's buckets live in memory and die with the process, as the predecessor's did,
so W8's import maps nothing for this SPEC.

## 9. Mutation rows

| row | target | what it guards | killer |
|---|---|---|---|
| `S11901-LOOPBACK` | `crates/mcp/src/settings.rs` | only a loopback address is accepted | `settings::the_listen_address_must_be_loopback` |
| `S11902-CORE-REQUIRED` | `crates/mcp/src/grants.rs` | a missing core token refuses start | `settings::the_core_credential_is_required` |
| `S11903-ONLY-MISSING-GRANTS-NOTHING` | `crates/mcp/src/grants.rs` | only `Missing` is read as no grant | `settings::a_broken_scope_credential_refuses_start` |
| `S11904-MIN-LENGTH` | `crates/mcp/src/grants.rs` | 32 characters; the test names 31 and 32 | `settings::a_short_credential_refuses_start` |
| `S11905-SHARED-VALUE` | `crates/mcp/src/grants.rs` | two credentials with one value refuse start | `settings::two_credentials_with_one_value_refuse_start` |
| `S11906-DRILLS-OFF` | `crates/mcp/src/settings.rs` | the drill tools are off unless the setting reads `on` | `settings::the_drills_setting_is_off_unless_it_reads_on` |
| `S11907-ABSENT-REFUSED` | `crates/mcp/src/guard.rs` | no header presents no token | `guard::a_request_with_no_bearer_is_refused` |
| `S11908-SCHEME` | `crates/mcp/src/guard.rs` | the scheme must be `Bearer` | `guard::a_malformed_bearer_is_refused` |
| `S11909-EMPTY-TOKEN` | `crates/mcp/src/guard.rs` | an empty token presents none | `guard::a_malformed_bearer_is_refused` |
| `S11910-DIGEST-MATCH` | `crates/mcp/src/guard.rs` | the match compares digests by `ct_eq` | `guard_census::the_guard_compares_only_digests_in_constant_time` |
| `S11911-SCOPE-CHECK` | `crates/mcp/src/tools.rs` | a tool checks its scope before it runs | `guard::a_tool_outside_the_scope_answers_unauthorized` |
| `S11912-ONE-WORD` | `crates/mcp/src/guard.rs` | a rate-limited refusal is the same response | `guard::every_request_refusal_is_the_same_response` |
| `S11913-SIXTH-FAILURE` | `crates/mcp/src/limiter.rs` | 5 or more fresh failures limit; the golden names the fifth and sixth | `limiter::the_limiter_matches_its_golden` |
| `S11914-WINDOW-EDGE` | `crates/mcp/src/limiter.rs` | fresh is strictly less than 60 seconds; the golden names 59.999 and 60.000 | `limiter::the_limiter_matches_its_golden` |
| `S11915-LIMITED-UNRECORDED` | `crates/mcp/src/limiter.rs` | a limited failure records nothing | `limiter::the_limiter_matches_its_golden` |
| `S11916-EVICTION` | `crates/mcp/src/limiter.rs` | at most 512 buckets; the golden names the 513th | `limiter::the_limiter_matches_its_golden` |
| `S11917-MATCH-FIRST` | `crates/mcp/src/guard.rs` | an in-scope match is allowed before the limiter | `limiter::a_granted_token_never_limits_itself` |
| `S11918-NO-TOKEN-LOGGED` | `crates/mcp/src/guard.rs` | the refusal's event names the bucket only | `limiter::a_refusal_logs_the_bucket_and_never_the_token` |
| `S11919-BODY-CAP` | `crates/mcp/src/server.rs` | 64 KiB | `server::a_body_over_64_kib_is_refused` |
| `S11920-SHED` | `crates/mcp/src/server.rs` | past 8 in flight a request is shed; the test's wait is bounded at 5 seconds, so a queued ninth fails it rather than hangs | `server::the_ninth_request_in_flight_is_shed` |
| `S11921-BADGES-CLAMP` | `crates/mcp/src/tools.rs` | 1..200; the golden names 200 and 201 | `tools::the_clamps_match_the_golden` |
| `S11922-REPORT-CLAMP` | `crates/mcp/src/tools.rs` | 1..62; the golden names 62 and 63 | `tools::the_clamps_match_the_golden` |
| `S11923-WHOLE-LEDGER` | `crates/mcp/src/tools.rs` | 0 or less reads the whole ledger; the golden names 0 and 1 | `tools::the_clamps_match_the_golden` |
| `S11924-ERASE-WORD` | `crates/mcp/src/tools.rs` | only `ERASE` erases | `tools::erase_runs_only_with_the_word_erase` |
| `S11925-RECOMPUTE-LABEL` | `crates/mcp/src/tools.rs` | a window read failure is `read_failed` | `tools::force_sync_reports_each_recompute_value` |
| `S11926-DRILL-BODY-CAP` | `crates/mcp/src/drills.rs` | 4000 characters; the golden names 4000 and 4001 | `drills::the_drill_tools_match_the_golden` |
| `S11927-DRILL-SAFE-ID` | `crates/mcp/src/drills.rs` | an unsafe id is not found before the audit event | `drills::an_unsafe_or_unknown_drill_is_not_found` |

### Plant list

Each plant is a change the verifier applies by hand, on a committed tree, and restores; the named
criterion must fail. A plant with a row is also that row's mutant.

| plant | criterion that fails | row |
|---|---|---|
| the guard admits a request with no `Authorization` header | A9 | `S11907-ABSENT-REFUSED` |
| an `Empty` scope credential is read as an empty token that an empty bearer matches | A4, A10 | `S11903-ONLY-MISSING-GRANTS-NOTHING` |
| an `Unreadable` law-track credential is read as missing | A4 | `S11903-ONLY-MISSING-GRANTS-NOTHING` |
| the core credential is made optional | A2 | `S11902-CORE-REQUIRED` |
| a tool's scope check is skipped | A14 | `S11911-SCOPE-CHECK` |
| the sixth failure is allowed (`>` for `>=`) | A17 | `S11913-SIXTH-FAILURE` |
| the limiter is consulted before the match | A20 | `S11917-MATCH-FIRST` |
| a rate-limited refusal says `rate limited` | A12 | `S11912-ONE-WORD` |
| the presented token is written into the refusal's event | A21 | `S11918-NO-TOKEN-LOGGED` |
| a non-loopback listen address is accepted | A1 | `S11901-LOOPBACK` |
| the drill tools are registered with the setting unset | A26 | `S11906-DRILLS-OFF` |
| the bearer is read from a `token` query parameter | A15 | none (an addition, not a mutation) |
| the bucket map grows without eviction | A17 | `S11916-EVICTION` |
| the token is compared with `==` | A16 | `S11910-DIGEST-MATCH` |

## 10. Amendments, 2026-10-03: the guard lands first (#158), and what #157 and the drill tools deliver

Section 3's table now holds only the criteria this pull request delivers; every other row moved,
verbatim, to section 3c with its fence line, and A40 is added in section 11. The body above is
otherwise as planned. The amendments below are recorded here and are not applied to it; each Old
is the body's text and each New is what it reads from this pull request on. A line number below
names the line of the SPEC as planned, before this pull request moved it.

- **T1** (R1, lines 43-44; the Old is one sentence across a line break, quoted with its break
  folded to one space). Old: `it depends on `deck-streak-kernel` and `deck-streak-coordination`, and `deck-streak-daemon` depends on it.`
  New: `from #158's pull request it depends on `deck-streak-kernel` only, and the map's fence gains
  its line with that one edge; #157's pull request adds `deck-streak-coordination` and names the
  crate in `deck-streak-daemon`'s line, each edge in the change that first uses it (ADR-002,
  ADR-320).`
- **T2** (R10, line 81). Old: `No other comparison of a token, a digest or the header exists in the crate.`
  New: `Every other comparison of a token or a digest in the crate is a `ct_eq` of two SHA-256
  digests (R7's sharing rule is the one), and the header is read only by R9's parser, which
  compares its scheme and never its token. A bucket (R13) is a map key, not a digest, under this
  rule.`
- **T3** (R13, line 97). Old: `the token when one parses, else the header's whole value, else `\x00absent`;`
  New: `the token when one parses, else the first `Authorization` header's whole value as bytes,
  and `\x00absent` when that value is empty or no header is present (`_bucket_id` maps an empty
  value so);`
- **T4** (R13, lines 98-99, its break folded). Old: `with 5 or more fresh failures the outcome is `rate_limited` and nothing is recorded;`
  New: `with 5 or more fresh failures the outcome is `rate_limited` and nothing is recorded (the
  bucket's stale failures are dropped and its place among the buckets is kept, as
  `_is_rate_limited` does); a bucket leaves only by eviction, never by age;`
- **T5** (A3, line 189). Old: `a missing law-track or drills credential grants nothing` New: `a
  missing law-track credential grants nothing (the drills credential's half is delivered with
  R19)`.
- **T6** (A4, line 190). Old: `an empty, unreadable or non-text law-track or drills credential refuses start by its id`
  New: `an empty, unreadable or non-text law-track credential refuses start by its id (the drills
  credential's half is delivered with R19)`.
- **T7** (A17 at line 203, and the same words in section 7 at line 355). Old: `the eleventh failure's history`
  New: `the eleventh failure recorded through the recorder itself keeping the newest ten (through
  the guard a bucket never holds more than five, so `_record_failure` is called directly)`.
- **T8** (A20, line 206). Old: `after ten out-of-scope failures, the core token still reaches every core tool`
  New: `after ten out-of-scope failures through the guard's scope check, the core token is still
  admitted by the request layer and allowed `core` (every core tool is A13's, #157)`.
- **T9** (section 7, `mcp_auth_bucket`, line 354). Old: `| function | synthetic tokens, the empty token among them |`
  New: `| adapter | a `DrillAuth` over no grants, calling `_bucket_id` on a synthetic token, a
  malformed header's whole value, a multibyte value and the empty value |`.
- **T10** (section 7, `mcp_auth_limiter`, line 355). Old: `a `DrillAuth` over synthetic grants and a scripted clock:`
  New: `a `DrillAuth` over synthetic grants and synthetic scope names, with a clock the case sets
  (one instant for every read inside one call; times in whole milliseconds), each call's outcome
  read from the predecessor's own warning and its buckets from its failure map:`.
- **T11** (section 7, `mcp_auth.constants`, line 353). Old: `` `SCOPE_LAW_TRACK`, `SCOPE_DRILLS` ``
  New: `` `SCOPE_LAW_TRACK` (`SCOPE_DRILLS` is registered with R19) ``.
- **T12** (manifest). This pull request also adds
  `docs/decisions/ADR-320-the-mcp-guard-lands-first-as-a-library-with-the-kernel-edge-only.md` and
  `formal/tla/BearerGuard/` (the model, `MCBearerGuard.cfg`, four `witness/*.cfg`);
  `crates/mcp/Cargo.toml` adds `sha2`, `subtle`, `axum`, `tower`, `thiserror` and `tracing` now,
  and `rmcp`, `schemars` and tower's `limit` and `load-shed` with #157; `Cargo.toml` gains the
  member and its workspace line only.
- **T13** (section 9). Row `S11928-GRANT-SCOPES` (`crates/mcp/src/guard.rs`, a grant is allowed
  only its own scopes, `guard::each_grant_holds_its_scopes_and_no_other`) is added; S11911's
  tool-level row stays #157's.
- **T14** (the schematic, R13 and A20). `docs/schematics/mcp-guard-refusals.md`'s flowchart sent a
  scope refusal (`tool -->|no|`) straight to the tool error, past the limiter, while R13 and A20
  send it through the limiter with the token's bucket. This pull request routes it through the
  limiter's decision before the tool error, with the schematic's own change note. ADR-121 is not
  edited: ADR-320 records that A7, A8, A13, A14, S11906 and S11911 of its Confirmation are
  delivered later.
- **T15** (R7, line 66, and A5, line 191). Old: `A loaded token shorter than 32 characters refuses start by its id (`McpError::WeakCredential`),`
  New: `A loaded token holding any byte outside 0x21 to 0x7E, which R9 lets no request present (a
  credential file written with a carriage return before its newline keeps one), refuses start by
  its id (`McpError::UnpresentableCredential`, the id and never the value), before its length is
  read; a loaded token shorter than 32 characters refuses start by its id
  (`McpError::WeakCredential`),`. And A5's Old: `a 31-character token refuses start by its id, and a 32-character one starts`
  New: `a 31-character token refuses start by its id, a 32-character one starts, and a token
  ending in a carriage return refuses start by its id as unpresentable`.
- **T16** (manifest, the capture census). This pull request also edits one line under the kernel
  crate: `crates/kernel/tests/log_capture_class.rs` (line 1588, the routed-capture count 19 becomes
  20). A21's capture in `crates/mcp/tests/limiter.rs` goes through the helper, so the census still
  demands every capture routed and its count equals the measured population (ruling 72).

Decided also by ADR-320 (the part split, the kernel edge only, the wall clock, an unpresentable
credential refused at load).

This pull request's rows in band `S11900-S11999` are S11902 to S11905, S11907 to S11910, S11912
to S11918, `S11928-GRANT-SCOPES` (T13) and `S11929-UNPRESENTABLE` (T15: `crates/mcp/src/grants.rs`,
a loaded token holding a byte outside 0x21 to 0x7E refuses start, killer
`settings::a_short_credential_refuses_start`). S11901, S11911 and S11919 to S11925 stay #157's;
S11906, S11926 and S11927 follow with the drill tools (R19).

Files this pull request adds that section 4 and T12 do not name:
`crates/mcp/tests/fixtures/planted_token_compare.rs.fixture` (added: A16's planted comparison,
never compiled).

Section 4's paths this pull request does not touch:

- `crates/mcp/src/server.rs`: unchanged in this part; delivered by #157
- `crates/mcp/src/tools.rs`: unchanged in this part; delivered by #157
- `crates/mcp/src/drills.rs`: unchanged in this part; delivered with the drill tools (R19)
- `crates/mcp/src/resources.rs`: unchanged in this part; delivered by #157
- `crates/mcp/tests/server.rs`: unchanged in this part; delivered by #157
- `crates/mcp/tests/roster.rs`: unchanged in this part; delivered by #157
- `crates/mcp/tests/tools.rs`: unchanged in this part; delivered by #157
- `crates/mcp/tests/drills.rs`: unchanged in this part; delivered with the drill tools (R19)
- `crates/mcp/tests/resources.rs`: unchanged in this part; delivered by #157
- `crates/daemon/Cargo.toml`: unchanged in this part; delivered by #157
- `crates/daemon/src/role_mcp.rs`: unchanged in this part; delivered by #157
- `crates/daemon/src/main.rs`: unchanged in this part; delivered by #157
- `crates/daemon/src/lib.rs`: unchanged in this part; delivered by #157
- `crates/daemon/src/wiring.rs`: unchanged in this part; delivered by #157
- `crates/daemon/tests/roles.rs`: unchanged in this part; delivered by #157
- `deploy/systemd/deck-streak-mcp.service`: unchanged in this part; delivered by #157
- `deny.toml` and `stack.json`: unchanged in this part (no new external crate); `rmcp` is admitted by #157
- `tools/parity-oracle/goldens/mcp_roster.json`: unchanged in this part; delivered by #157, with
  `mcp_clamps.json`, `mcp_erase_confirm.json` and `mcp_sync_out.json`; the drill goldens follow
  R19

## 11. Acceptance criteria of the 2026-10-03 amendment

| id | criterion | decided by |
|---|---|---|
| A40 | through the guard's layer, the core token's request reaches the inner service once carrying `{core}` and the law-track token's carrying `{core, law_track}`, `bearer` in lower case among them; and the guard's scope check allows each grant exactly its scopes | `each_grant_holds_its_scopes_and_no_other` |

```acceptance
A40: cargo test -p deck-streak-mcp --test guard -- --exact each_grant_holds_its_scopes_and_no_other
```

## 12. Mutation round 1

Each test below was red first against its own hand plant, and the file was restored byte-equal.

- `grants::tests::or_if_keeps_a_scope_already_held`: `Scopes::or_if` with `|` replaced by `^` fails it.
- `grants::tests::a_grant_debug_names_its_type_and_never_its_token`: `Grant`'s `Debug` replaced by
  `Ok(Default::default())` fails it. Both live in a `#[cfg(test)]` module appended to
  `crates/mcp/src/grants.rs`, because `or_if` and `Grant` are `pub(crate)`.
- `guard_census::the_crate_root_forbids_unsafe_code`: row `S11931-FORBID-UNSAFE` replaces the crate
  root's `#![forbid(unsafe_code)]` with `#![deny(unsafe_code)]`, and the test reads `src/lib.rs`.

## 13. Amendments, 2026-10-03: the server's first slice (#157) and two named rows (#158)

#157 lands in parts (ADR-329). This pull request, its first part, serves the `mcp` role with one
tool, `get_law_track`; part 157b adds the unit, and the other tools follow as their owner SPECs
land, with A26 last. The body above and section 3c are not edited: the criteria this part delivers
are named below and stated for this part in section 14, whose fence holds them.

What this part delivers:
- R1's last two edges: `deck-streak-mcp` depends on `deck-streak-coordination` (for
  `law::law_block`), and `deck-streak-daemon` depends on `deck-streak-mcp`; the map's two lines
  change in the same commit as the manifests, and no other crate gains an edge (T1 of section 10
  named both for #157).
- R2 without its unit: `deckstreakd mcp` (`crates/daemon/src/role_mcp.rs`) serves MCP's streamable
  HTTP transport through `rmcp` at the one path `/mcp`, stateless (no session is kept, and no
  session id is issued), answering `application/json`, with the allowed hosts the loopback names
  and a request body of at most 64 KiB; any other path answers 404, and 401 without a granted
  bearer. `deploy/systemd/deck-streak-mcp.service` is part 157b's.
- R3: the listen setting `DECKSTREAK_MCP_LISTEN`, refused by `McpError::Listen` naming it when it is
  unset, unparseable or not a loopback address. R4: at most 8 requests in flight, a ninth shed with
  503 at once.
- R12's tool error: a tool call whose grant lacks the tool's scope answers the tool error whose
  whole text is `unauthorized`, and no data.
- R15 to R17 for `get_law_track` alone: its name, its annotations and its output fields equal its
  entry of `goldens/mcp_roster.json`; it takes no parameter (R15 removes the predecessor's
  `token`); it needs `law_track` (R17); and it reads the law track through `coordination`'s
  `law::law_block` for the kernel's study day with no leech count, as the API's law route reads it
  (R16).
- The pending-null rule (ADR-329): while the leech port is not wired (#133), `law_block` answers no
  active leech count and no mastery, and the tool answers `leech_total` and `mastery` as `null`,
  never 0, where the predecessor coerces them to 0 and 0.0 (`server.py:get_law_track`, `_law_int`,
  `_law_float`). The output schema declares `dues`, `leech_total` and `mastery` nullable and
  required, in the golden's order. A present mastery is rounded to 2 places as `_law_float` rounds
  it, by the kernel's port of the predecessor's rounding.
- The serving order, outermost first: the request id, the sensitive request headers
  (`Authorization`, `Cookie`, `Set-Cookie`), the trace with the matched path, the sensitive
  response headers, the request id's propagation, the panic catch, a timeout answering 408, the
  guard's layer, the shed's error handler (503), the load shed, the global concurrency limit of 8,
  the request body limit of 64 KiB, and then the `rmcp` service at `/mcp`. The guard sits inside
  the trace, so its `Authorization` header is redacted before the trace logs it, and outside the
  shed, so a refused request takes no slot (A44). The body limit is `tower-http`'s request body
  limit, because axum's own limit bounds only its extractors and the transport reads its own body;
  the transport's own body cap is set to the same 64 KiB.
- The guard's new callers: the serving stack calls `Guard::admit` for every request, and the tool
  calls `Guard::authorize` before it reads anything. `formal/tla/BearerGuard` is re-read for them:
  for a granted token `admit` reads and writes no limiter variable, so the gap between a request's
  admission and its tool's scope check is a stuttering step of the model, and a refused scope check
  is the model's `Decide`. The entry's covers gain both call sites, and it is checked again (#158).

The rows. This part's rows in band `S11900-S11999` are:

| row | target | what it guards | killer |
|---|---|---|---|
| `S11901-LOOPBACK` | `crates/mcp/src/settings.rs` | only a loopback address is accepted | `settings::the_listen_address_must_be_loopback` |
| `S11911-SCOPE-CHECK` | `crates/mcp/src/tools.rs` | the tool checks its scope before it reads | `guard::a_tool_outside_the_scope_answers_unauthorized` |
| `S11919-BODY-CAP` | `crates/mcp/src/server.rs` | 64 KiB | `server::a_body_over_64_kib_is_refused` |
| `S11920-SHED` | `crates/mcp/src/server.rs` | past 8 in flight a request is shed | `server::the_ninth_request_in_flight_is_shed` |
| `S11932-LOOPBACK-HOSTS` | `crates/mcp/src/server.rs` | the allowed hosts are the loopback names | `server::a_foreign_host_is_refused` |
| `S11933-STATELESS-JSON` | `crates/mcp/src/server.rs` | no session is kept and the answer is JSON | `server::initialize_answers_json_and_no_session_id` |
| `S11934-LAW-PENDING` | `crates/mcp/src/tools.rs` | a pending count answers null, never 0 | `tools::the_pending_law_numbers_answer_null_never_zero` |
| `S11935-MASTERY-ROUND` | `crates/mcp/src/tools.rs` | a present mastery is rounded to 2 places | `tools::the_law_track_answers_numbers_only` |
| `S11936-GUARD-WIRED` | `crates/mcp/src/server.rs` | the guard's layer is in the served stack | `server::the_served_stack_refuses_initialize_without_a_bearer` |
| `S11937-MCP-ROLE` | `crates/daemon/src/main.rs` | the role's name in the binary's roles | `roles::the_mcp_role_is_a_known_role` |

The band also holds `S11930-MIN-CREDENTIAL-CHARS` (`crates/mcp/src/settings.rs`) and
`S11931-FORBID-UNSAFE` (`crates/mcp/src/lib.rs`), which #578 delivered and section 10's rows
paragraph did not name. S11921 to S11925 stay with the tools a later part of #157 delivers, and
S11906, S11926 and S11927 follow with the drill tools (R19).

The criteria. Section 14 states A1, A13, A14, A22, A23, A24, A25, A33, A38 and A39 for this part,
which moves them from section 3c, and adds A41 to A45. Section 3c's A13 also names every other
core tool and the drill tools, and its A26 the whole roster: those halves stay in section 3c with
the parts that deliver their tools. Section 3c's fence keeps every `SERVER: ` line, and the line of
a criterion section 14 states is read from section 14.

The manifest, as this part leaves each path of section 4 and each path it adds:
- `Cargo.toml`: changed: the workspace's `rmcp` with its server, macros and streamable HTTP server
  features only
- `Cargo.lock`: changed: `rmcp` and the packages it brings
- `crates/mcp/Cargo.toml`: changed: `deck-streak-coordination`, `rmcp`, `serde`, `tokio`'s
  listener, `tower`'s limit and load shed, and `tower-http`'s layers; `schemars` is reached through
  `rmcp`
- `crates/mcp/src/lib.rs`: changed: the `server` and `tools` modules
- `crates/mcp/src/settings.rs`: changed: the listen setting and `McpError::Listen`
- `crates/mcp/src/server.rs`: added: the transport's configuration, the serving stack, the bind and
  the serve
- `crates/mcp/src/tools.rs`: added: the server's tool router, `get_law_track`, its scope check and
  its law-track port
- `crates/mcp/src/grants.rs`: unchanged in this part (the text `formal/tla/BearerGuard` covers)
- `crates/mcp/src/guard.rs`: unchanged in this part (the text `formal/tla/BearerGuard` covers)
- `crates/mcp/src/limiter.rs`: unchanged in this part (the text `formal/tla/BearerGuard` covers)
- `crates/mcp/src/resources.rs`: unchanged in this part; delivered by a later part of #157
- `crates/mcp/src/drills.rs`: unchanged in this part; delivered with the drill tools (R19, #158)
- `crates/mcp/tests/settings.rs`: changed: A1, A39
- `crates/mcp/tests/guard.rs`: changed: A13, A14
- `crates/mcp/tests/server.rs`: added: A22 to A25, A43 to A45
- `crates/mcp/tests/tools.rs`: added: A33, A41, A42
- `crates/mcp/tests/support/mod.rs`: added (section 4 does not name it): the served stack's test
  client over a loopback socket, the scripted law track and the tokens built from parts, shared by
  the three test files above
- `crates/mcp/tests/guard_census.rs`: unchanged in this part; its census now also reads
  `server.rs` and `tools.rs`
- `crates/mcp/tests/limiter.rs`: unchanged in this part
- `crates/mcp/tests/roster.rs`: unchanged in this part; delivered with A26 by the last part of #157
- `crates/mcp/tests/drills.rs`: unchanged in this part; delivered with the drill tools (R19, #158)
- `crates/mcp/tests/resources.rs`: unchanged in this part; delivered by a later part of #157
- `crates/daemon/Cargo.toml`: changed: depends on `deck-streak-mcp`
- `crates/daemon/src/role_mcp.rs`: added: the role, its lifecycle and its credentials
- `crates/daemon/src/main.rs`: changed: the role `mcp`
- `crates/daemon/src/lib.rs`: changed: the role's module
- `crates/daemon/src/wiring.rs`: unchanged in this part: the role opens its database through
  `wiring::open_database`, and no tool needs a use case wired there yet
- `crates/daemon/tests/roles.rs`: changed: A38, and the usage line's expected roles name the fifth
- `deploy/systemd/deck-streak-mcp.service`: unchanged in this part; delivered by part 157b of #157
- `deny.toml`: unchanged in this part: every licence the new packages carry is already allowed
- `stack.json`: unchanged in this part: the stack radar has no item for an MCP server
- `docs/CONTEXT-MAP.md`: changed: the adapter's line names the server and its tools and the
  `coordination` edge, and `deck-streak-daemon`'s line names `mcp`
- `tools/parity-oracle/registry/spec_119.py`: changed: the `mcp_roster` golden
- `tools/parity-oracle/goldens/mcp_roster.json`: added: the predecessor's 33 tools at its defaults,
  each tool's name, annotations, input schema and output fields, and no description text
- `tools/parity-oracle/goldens/mcp_auth.constants.json`: changed: its `registry_sha256` only,
  because the registry file it names changed
- `tools/parity-oracle/goldens/mcp_auth_bucket.json`: changed: its `registry_sha256` only
- `tools/parity-oracle/goldens/mcp_auth_limiter.json`: changed: its `registry_sha256` only
- `tools/parity-oracle/goldens/mcp_clamps.json`: unchanged in this part; delivered by a later part
  of #157
- `tools/parity-oracle/goldens/mcp_erase_confirm.json`: unchanged in this part; delivered by a
  later part of #157
- `tools/parity-oracle/goldens/mcp_sync_out.json`: unchanged in this part; delivered by a later
  part of #157
- `tools/parity-oracle/goldens/mcp_drills.constants.json`: unchanged in this part; delivered with
  the drill tools (R19, #158)
- `tools/parity-oracle/goldens/mcp_drill_ref.json`: unchanged in this part; delivered with the
  drill tools (R19, #158)
- `tools/parity-oracle/goldens/mcp_drill_view.json`: unchanged in this part; delivered with the
  drill tools (R19, #158)
- `scripts/mutation-rows.d/S11900-S11999.json`: changed: the ten rows above
- `formal/tla/BearerGuard/BearerGuard.tla`: changed (section 4 does not name it): the re-read in
  its comments and the two new covers
- `docs/decisions/ADR-329-the-mcp-server-lands-as-a-role-with-one-tool-before-its-unit-and-the-law-tracks-pending-numbers-answer-null.md`:
  added (section 4 does not name it)
- `docs/decisions/ADR-320-the-mcp-guard-lands-first-as-a-library-with-the-kernel-edge-only.md`:
  changed (section 4 does not name it): an appended amendment
- `docs/schematics/mcp-guard-refusals.md`: unchanged in this part
- `docs/red-first/SPEC-119.md`: changed: this part's criteria
- `changelog.d/mcp-server-157a.md`: added

What this part does not do:
- It serves none of the 13 other core tools whose owners exist (the scores, the report, the level,
  the badges, the streak, the progress, the skip history and its two writers, memory, export, erase
  and the forced sync), their goldens `mcp_clamps`, `mcp_erase_confirm` and `mcp_sync_out`, A28 to
  A32, or rows S11921 to S11925: a later part of #157.
- It serves none of the 19 core tools whose owner SPECs are planned, and no chart resource (R23,
  A37), and it does not decide A26, the whole roster: later parts of #157, after their owners land.
- It adds no unit, no host budget entry and no deploy rail credential, and B3 is not judged here:
  part 157b of #157.
- It registers no drill tool, names no drill scope and adds none of their criteria or rows: they
  are parked with #158.
- It answers no leech count and no mastery while the leech port is not wired: #133.
- It serves none of the predecessor's image chart resources, which R23 leaves out on purpose:
  #157.

## 14. Acceptance criteria of the 2026-10-03 amendment

| id | criterion | decided by |
|---|---|---|
| A1 | `DECKSTREAK_MCP_LISTEN` unset, unparseable or not loopback refuses start naming the setting | `the_listen_address_must_be_loopback` |
| A13 | the core token reaches no `law_track` tool and the law-track token reaches `get_law_track` | `each_grant_reaches_its_scopes_and_no_other` |
| A14 | a core-token call of `get_law_track` answers the tool error `unauthorized` and no data | `a_tool_outside_the_scope_answers_unauthorized` |
| A22 | only `/mcp` is served; any other path answers 404 | `only_the_mcp_path_is_served` |
| A23 | a body over 64 KiB is refused | `a_body_over_64_kib_is_refused` |
| A24 | a request whose Host is not a loopback name is refused | `a_foreign_host_is_refused` |
| A25 | with 8 granted requests held in flight, a ninth answers 503 at once | `the_ninth_request_in_flight_is_shed` |
| A33 | `get_law_track` answers exactly the golden's fields, numbers only, mastery rounded to 2 places | `the_law_track_answers_numbers_only` |
| A38 | `deckstreakd mcp` is a known role | `the_mcp_role_is_a_known_role` |
| A39 | the role's token reads go only through the loader | `the_role_reads_its_tokens_only_through_the_loader` |
| A41 | each served tool's name, annotations, parameters and output fields equal its roster golden entry | `each_served_tool_matches_its_roster_golden_entry` |
| A42 | pending leeches and mastery answer null, never 0 | `the_pending_law_numbers_answer_null_never_zero` |
| A43 | through the served stack, an `initialize` with no bearer answers 401 `unauthorized` | `the_served_stack_refuses_initialize_without_a_bearer` |
| A44 | with 8 granted requests held in flight, a request with no bearer answers 401, not 503 | `a_refused_request_holds_no_slot` |
| A45 | `initialize` answers `application/json` and no session id header | `initialize_answers_json_and_no_session_id` |

```acceptance
A1: cargo test -p deck-streak-mcp --test settings -- --exact the_listen_address_must_be_loopback
A13: cargo test -p deck-streak-mcp --test guard -- --exact each_grant_reaches_its_scopes_and_no_other
A14: cargo test -p deck-streak-mcp --test guard -- --exact a_tool_outside_the_scope_answers_unauthorized
A22: cargo test -p deck-streak-mcp --test server -- --exact only_the_mcp_path_is_served
A23: cargo test -p deck-streak-mcp --test server -- --exact a_body_over_64_kib_is_refused
A24: cargo test -p deck-streak-mcp --test server -- --exact a_foreign_host_is_refused
A25: cargo test -p deck-streak-mcp --test server -- --exact the_ninth_request_in_flight_is_shed
A33: cargo test -p deck-streak-mcp --test tools -- --exact the_law_track_answers_numbers_only
A38: cargo test -p deck-streak-daemon --test roles -- --exact the_mcp_role_is_a_known_role
A39: cargo test -p deck-streak-mcp --test settings -- --exact the_role_reads_its_tokens_only_through_the_loader
A41: cargo test -p deck-streak-mcp --test tools -- --exact each_served_tool_matches_its_roster_golden_entry
A42: cargo test -p deck-streak-mcp --test tools -- --exact the_pending_law_numbers_answer_null_never_zero
A43: cargo test -p deck-streak-mcp --test server -- --exact the_served_stack_refuses_initialize_without_a_bearer
A44: cargo test -p deck-streak-mcp --test server -- --exact a_refused_request_holds_no_slot
A45: cargo test -p deck-streak-mcp --test server -- --exact initialize_answers_json_and_no_session_id
```

## 15. Amendments, 2026-10-04: the server's unit (#157)

Part 157b of #157 (ADR-332) delivers R2's unit clause and the manifest's unit line, and B3 is
judged over the unit. The body above and sections 3c, 13 and 14 are not edited: the criteria this
part delivers are stated in section 16, whose fence holds them.

What this part delivers:
- R2's unit clause: `deploy/systemd/deck-streak-mcp.service` runs `deckstreakd mcp` as a
  `Type=notify` daemon with the watchdog, the daemons' lifecycle, `OnFailure=` the alert template,
  the full hardening set, the shared state directory and settings file, and two credentials from
  the private rail's socket, `mcp-core-token` and `mcp-law-track-token`. No `Environment=` line
  carries a token. The manifest's third credential waits on #158.
- The unit's place in the share: its entry in `deploy/host-budget.json` and its row in ADR-032's
  note of this date, with a CPU quota taken from the API's; ADR-064's note of this date records
  that the share's memory is now full (ADR-332 D2).
- The unit's rail entry: its neutral `ExecStart=` and `EnvironmentFile=` in
  `deploy/rail-contract.json`.
- R3's setting in the committed example: `DECKSTREAK_MCP_LISTEN`, a loopback address, in
  `deploy/deck-streak.env.example`, and among the settings the template tests require.
- The owner-started first run: a deploy installs the unit and never starts it, and
  `deploy/README.md` names the owner's two steps (ADR-332 D3; #167).
- B3 (section 3a), judged on the box over the units under `deploy/systemd/`, this unit among them.
- Three text corrections ride with this part: a dated amendment of one sentence of ADR-329's
  Consequences; the module doc of `crates/mcp/src/server.rs`, which now says what the guard's
  place outside the bound gives a refused request, an answer without waiting for a slot and no
  slot held past its refusal; and a correction in `docs/red-first/SPEC-119.md` that quotes A1's
  observed failure.

The files this part changes:
- `deploy/systemd/deck-streak-mcp.service`: added: the role, its lifecycle, its two credentials and
  its budget
- `deploy/systemd/deck-streak-api.service`: changed (section 4 does not name it): its CPU quota
  gives the MCP server its share (ADR-332 D2)
- `deploy/host-budget.json`: changed (section 4 does not name it): the unit's entry
- `deploy/rail-contract.json`: changed (section 4 does not name it): the unit's two neutral values
- `deploy/deck-streak.env.example`: changed (section 4 does not name it): `DECKSTREAK_MCP_LISTEN`
- `deploy/README.md`: changed (section 4 does not name it): the unit's rows and the owner's first
  start
- `scripts/tests/test_deploy_templates.py`: changed (section 4 does not name it): the unit in each
  table keyed by unit, the share's worst case, the API's quota, and the role and per-service tables
  held in both directions: A46 to A51
- `scripts/tests/test_rail_contract.py`: unchanged in this part; it decides A52 over the unit's
  rail entry
- `docs/decisions/ADR-032-deploy-templates-and-the-host-budget.md`: changed (section 4 does not
  name it): a dated note with the unit's row
- `docs/decisions/ADR-064-deckstreak-backs-up-with-its-own-units-and-never-the-collection.md`:
  changed (section 4 does not name it): a dated note on the share
- `docs/decisions/ADR-332-the-mcp-servers-unit-takes-its-place-inside-the-share-and-its-first-start-is-the-owners.md`:
  added (section 4 does not name it)
- `docs/decisions/ADR-329-the-mcp-server-lands-as-a-role-with-one-tool-before-its-unit-and-the-law-tracks-pending-numbers-answer-null.md`:
  changed (section 4 does not name it): a dated amendment of one Consequences sentence
- `crates/mcp/src/server.rs`: changed: one sentence of its module doc, and no code
- `docs/red-first/SPEC-119.md`: changed: A46 to A52, and a correction
- `changelog.d/mcp-unit-157b.md`: added (section 4 does not name it)

What this part does not do:
- It stores no credential in the private rail and starts no unit: the owner's first start, #167.
- It adds the unit to no restart list: `deploy/deploy.sh` is unchanged (ADR-332 D3), #167.
- It adds no Caddy route: the server listens on a loopback address alone (R3), #157.
- It serves no further tool or resource: later parts of #157.
- It loads no third credential and registers no parked tool: #158.
- It declares no service level objective for the server in `deploy/slo.json`: its traffic is
  the owner's agent alone, and an objective for it waits on that traffic measured, #157.

## 16. Acceptance criteria of the 2026-10-04 amendment

| id | criterion | decided by |
|---|---|---|
| A46 | every service's `MemoryHigh=` and `MemoryMax=` equal its host budget entry and its ADR row, `deck-streak-mcp.service` among them, and `MemoryHigh=` is below `MemoryMax=` | `test_every_unit_ceiling_matches_the_host_budget_and_high_is_below_max` |
| A47 | the long-running units, the MCP server among them, and the largest job fit the share, and the daemons' CPU quotas fit its CPUs; a daemon with no `MemoryMax=` or `CPUQuota=` is refused by name | `test_the_daemons_and_the_largest_job_fit_the_stack_share` |
| A48 | every service runs its role with R1's lifecycle, the MCP server as a daemon with its caps, and every role and cap the tables name has a unit | `test_every_service_runs_its_role_with_the_lifecycle_r1_names` |
| A49 | every unit that loads a credential, the MCP server among them, fails and pages on a refusal | `test_every_unit_that_loads_a_credential_fails_and_pages_on_a_refusal` |
| A50 | no unit passes a secret through its environment: the MCP server's tokens arrive as credentials, and the committed example names `DECKSTREAK_MCP_LISTEN` | `test_no_unit_passes_a_secret_through_its_environment` |
| A51 | every service carries R2's hardening and its per-service values, and the per-service table names every service and no other | `test_every_service_carries_the_hardening_r2_names` |
| A52 | the rail contract names every neutral value the templates carry, the MCP server's among them | `test_the_rail_contract_names_every_neutral_value` |

```acceptance
A46: python3 -m unittest discover -s scripts/tests -p test_deploy_templates.py -k every_unit_ceiling_matches_the_host_budget_and_high_is_below_max
A47: python3 -m unittest discover -s scripts/tests -p test_deploy_templates.py -k the_daemons_and_the_largest_job_fit_the_stack_share
A48: python3 -m unittest discover -s scripts/tests -p test_deploy_templates.py -k every_service_runs_its_role_with_the_lifecycle_r1_names
A49: python3 -m unittest discover -s scripts/tests -p test_deploy_templates.py -k every_unit_that_loads_a_credential_fails_and_pages_on_a_refusal
A50: python3 -m unittest discover -s scripts/tests -p test_deploy_templates.py -k no_unit_passes_a_secret_through_its_environment
A51: python3 -m unittest discover -s scripts/tests -p test_deploy_templates.py -k every_service_carries_the_hardening_r2_names
A52: python3 -m unittest discover -s scripts/tests -p test_rail_contract.py -k the_rail_contract_names_every_neutral_value
```

## 17. Amendments: the server offers no export or erase tool, and its grant is read or write (SPEC-369, #720)

This section amends the items it names, by SPEC-369 and ADR-380. No line above this section
changes; where an item above and this section differ, this section holds. `#158` stays parked, and
the counts below hold while it is.

T17. **R6, three credentials.** `mcp-core-token` is required. `mcp-law-track-token` and
     `mcp-write-token` each grant nothing when missing and refuse start, by their own id, on any
     other loader error.

T18. **R7, the write credential.** The write credential is held to the rules the others are: at
     least 32 characters, every byte one a request can present. No two of the three credentials
     share a value: a write credential equal to the core or the law-track credential refuses start
     naming that credential first and `mcp-write-token` second.

T19. **R8, three scopes.** The scope set is `core`, `law_track` and `write`. The core credential's
     grant is the read grant `{core}`, the law-track credential's `{core, law_track}`, and the
     write credential's `{core, write}`. No other grant holds `write`.

T20. **R15 and R16's roster row `export_data` (`:141`)** is withdrawn: the server serves no
     data-export tool.

T21. **R15 and R16's roster row `erase_all_data` (`:142`)** is withdrawn: the server serves no
     erase tool.

T22. **R17, writes need `write`.** A tool whose annotations say `readOnlyHint` false authorizes
     `write` before it reads or writes anything. A read tool authorizes its own scope: `core`, or
     `law_track` for `get_law_track`.

T23. **R21 (`:169-173`)** is withdrawn. Export and erase stay on the bot's `/export` and `/delete`
     and the host's `deckstreakd data export` and `deckstreakd data erase --confirm ERASE`, which
     call `export_all` and `erase_all` (SPEC-021 R1, unchanged).

T24. **A26.** The portable roster is the golden less the two withdrawn names: 31 tools, 33 when
     `#158`'s two drill tools land. The golden is unchanged; the reader declares the withdrawn
     names (SPEC-369 R8).

T25. **A30 (`:274`)** is withdrawn.

T26. **A31 (`:275`)** is withdrawn.

T27. **Section 6's risk "A leaked core token erases the ledger"** no longer holds: the core
     token's grant is read-only, and no erase tool exists. A leaked write token reaches the write
     tools once they are served, and the write credential is provisioned only then.

T28. **Section 7's `mcp_erase_confirm` golden (`:385`)** is withdrawn: it was never generated.

T29. **Section 9's row `S11924-ERASE-WORD`** is withdrawn: it was never written.

T30. **Section 13's list of what the first slice does not do.** No later part serves
     `export_data` or `erase_all_data`, so neither is counted among the core tools a later part of
     `#157` serves.

## 18. Acceptance criteria of the section 17 amendment

SPEC-369's criteria decide this amendment, each by the command SPEC-369's acceptance fence gives
it:

- T17 and T18: SPEC-369 A1 to A4.
- T19: SPEC-369 A1 and A5.
- T20, T21, T23, T25 and T26: SPEC-369 A8 and A9.
- T22: SPEC-369 A6.
- T24: SPEC-369 A7.
- T27 to T30 change text only, and no test decides them.
