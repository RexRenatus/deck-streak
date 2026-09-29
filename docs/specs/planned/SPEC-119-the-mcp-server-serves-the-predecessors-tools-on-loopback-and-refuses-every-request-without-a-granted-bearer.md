# SPEC-119: the MCP server serves the predecessor's tools on loopback and refuses every request without a granted bearer

- **Wave:** W6. **Issues:** #157 (the MCP server) and #158 (fail-closed bearer auth for the law and
  drill tools) (epic #7). **Context(s):** `deck-streak-mcp`, a new adapter (the server, its guard,
  its limiter and its roster); `deck-streak-daemon` (the `mcp` role); `docs/CONTEXT-MAP.md` (the
  adapter's line).
- **Decided by:** ADR-002 (one crate per context; the map is the crate graph), ADR-025 (a
  concurrency bound sheds instead of queueing), ADR-037 (owner triggers of the sync), ADR-054 (no-AI
  mode is the default), ADR-059 (public text describes DeckStreak only), ADR-067 (the loader refuses
  an empty credential by its id), ADR-119 (the server is Rust on `rmcp`, in its own adapter crate,
  on loopback) and ADR-121 (the guard fails closed on every request, compares digests in constant
  time, and keeps the predecessor's limiter).
- **Prerequisites:** SPEC-020, SPEC-021, SPEC-022, SPEC-066, SPEC-071, SPEC-072, SPEC-073,
  SPEC-075, SPEC-076, SPEC-077, SPEC-078, SPEC-079, SPEC-080, SPEC-083, SPEC-085, SPEC-086, SPEC-090,
  SPEC-091, SPEC-092 and SPEC-093 (planned, W4, on `docs/w4-plan`) and SPEC-110. **Mutation band:**
  `S11900-S11999`.
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-119.md` (ADR-016).

## 1. The problem, measured

- **The predecessor's tool server** (`server.py:create_server`, at `27ee2bc`) registers 35 tools on
  the streamable HTTP transport: 31 always, `get_law_track` behind a bearer scope, and `list_drills`
  and `get_drill` only when its drill flag is on. Read tools carry `readOnlyHint`, `idempotentHint`
  and a closed world (`server.py:_RO`); a writer's `{"ok": false}` becomes a tool error
  (`server.py:_raise_on_error`); three tools clamp their arguments; and thirteen chart resources
  answer images (`server.py:_CHART_RESOURCES`).
- **Its guard covers three tools.** `mcp_auth.py:DrillAuth.require` checks a token passed as a tool
  ARGUMENT against per-scope tokens (`law_track`, `drills`) with `hmac.compare_digest`, denies with
  the one message `unauthorized`, and counts failures per bucket: five in 60 seconds, 512 buckets,
  ten timestamps a bucket. The other 32 tools answer anyone who reaches the port.
- **DeckStreak has no such surface.** No crate speaks MCP, and the agent context cannot hold one:
  a tool calls a use case of `coordination`, and `coordination` depends on `agent`, so the edge would
  be a cycle (ADR-119). The Mini App's routes answer an owner session (SPEC-024), which a machine
  client does not have.
- **What changes, and why.** Every request needs a granted bearer, not only three tools; the bearer
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
R4. At most 8 requests are in flight; a ninth is shed with 503 at once, never queued (ADR-025's
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

| id | criterion | decided by |
|---|---|---|
| A1 | `127.0.0.1` and `::1` are accepted; `0.0.0.0`, `::`, a private-range address, unset and unparseable each refuse start naming `DECKSTREAK_MCP_LISTEN` | `the_listen_address_must_be_loopback` |
| A2 | a missing, empty, unreadable or non-text `mcp-core-token` each refuses start by its id | `the_core_credential_is_required` |
| A3 | a missing law-track or drills credential grants nothing, and the core token still starts the role | `a_missing_scope_credential_grants_nothing` |
| A4 | an empty, unreadable or non-text law-track or drills credential refuses start by its id | `a_broken_scope_credential_refuses_start` |
| A5 | a 31-character token refuses start by its id, and a 32-character one starts | `a_short_credential_refuses_start` |
| A6 | two credentials holding one value refuse start | `two_credentials_with_one_value_refuse_start` |
| A7 | with the drill tools off, the drills credential is never read: an empty one does not refuse start | `the_drills_credential_is_read_only_when_drills_are_on` |
| A8 | `DECKSTREAK_MCP_DRILLS` unset or `off` is off, `on` is on, and any other value refuses start | `the_drills_setting_is_off_unless_it_reads_on` |
| A9 | a request with no `Authorization` header, `initialize` among them, is answered 401 `unauthorized` with `WWW-Authenticate: Bearer`, and the server never sees it | `a_request_with_no_bearer_is_refused` |
| A10 | an empty header, `Bearer` alone, an empty token, another scheme, two headers, a token holding a space and a token holding a non-ASCII byte are each refused as A9 | `a_malformed_bearer_is_refused` |
| A11 | a token differing in its last character, a prefix of a granted token, and a granted token with one character added are each refused as A9 | `a_wrong_bearer_is_refused` |
| A12 | the absent, empty, malformed, wrong and rate-limited refusals are byte for byte the same response, the date header aside | `every_request_refusal_is_the_same_response` |
| A13 | the core token reaches every core tool and no other; the law-track token adds `get_law_track`; the drills token adds the drill tools; `bearer` in lower case is accepted | `each_grant_reaches_its_scopes_and_no_other` |
| A14 | a tool outside the token's scope answers a tool error whose whole text is `unauthorized`, and no data | `a_tool_outside_the_scope_answers_unauthorized` |
| A15 | a granted token in the query string, with no header, is refused as A9 | `a_token_in_the_query_string_is_ignored` |
| A16 | in `crates/mcp/src/`, a token, digest or header value is compared only by `ct_eq` over SHA-256 digests, in one fold with no early exit; a planted `==` on a token is refused | `the_guard_compares_only_digests_in_constant_time` |
| A17 | the limiter equals its golden: the fifth and sixth failures, 59.999 and 60.000 seconds after the first, a limited failure recording nothing, the eleventh failure's history, and the 513th bucket's eviction | `the_limiter_matches_its_golden` |
| A18 | the bucket equals its golden for a token, a malformed header's whole value, an empty value and no header | `the_bucket_matches_its_golden` |
| A19 | the limiter's constants and the denial word equal the golden | `the_guard_constants_match_the_golden` |
| A20 | after ten out-of-scope failures, the core token still reaches every core tool | `a_granted_token_never_limits_itself` |
| A21 | each refusal writes one `warn` event with its outcome, bucket and scope, and no event holds the token, the header's value or a full digest | `a_refusal_logs_the_bucket_and_never_the_token` |
| A22 | `/health`, `/metrics` and `/` answer 404 to a granted bearer and 401 to none | `only_the_mcp_path_is_served` |
| A23 | a body over 64 KiB is refused and runs no tool | `a_body_over_64_kib_is_refused` |
| A24 | a request whose Host is not a loopback name is refused and runs no tool | `a_foreign_host_is_refused` |
| A25 | with 8 requests held in a scripted use case, a ninth is answered 503 within 5 seconds | `the_ninth_request_in_flight_is_shed` |
| A26 | with the drill tools off, the roster equals the golden's 33 tools | `the_roster_matches_the_golden_with_drills_off` |
| A27 | with the drill tools on, the roster equals the golden's 35 tools | `the_roster_matches_the_golden_with_drills_on` |
| A28 | each clamp equals its golden, the cases 0, 1, 200, 201, 62, 63, 3650 and 3651 among them, and the exchange window ends on today's study day | `the_clamps_match_the_golden` |
| A29 | a writer whose use case fails answers a tool error, never a result carrying `ok: false` | `a_failed_write_is_a_tool_error` |
| A30 | `erase_all_data` with `confirm` empty, `erase` or `ERASE ` answers the golden's refusal and erases nothing; `ERASE` empties every table | `erase_runs_only_with_the_word_erase` |
| A31 | `export_data` answers the data-rights registry's export document | `export_answers_the_data_rights_document` |
| A32 | `force_sync` answers `full`, `skipped` and `read_failed` as its golden does, and another cycle error answers `sync failed` | `force_sync_reports_each_recompute_value` |
| A33 | `get_law_track` answers exactly the golden's numeric fields | `the_law_track_answers_numbers_only` |
| A34 | the drill list and view equal the golden: 25 of 26 drills, a 121-character title, bodies of 4000 and 4001 characters, a multibyte title | `the_drill_tools_match_the_golden` |
| A35 | the ids `a/b`, `a\b`, `..`, the empty id and an unknown id answer `drill not found`, and the audit event carries a 12-character reference and never the id | `an_unsafe_or_unknown_drill_is_not_found` |
| A36 | the drill reference equals its golden | `the_drill_ref_matches_the_golden` |
| A37 | the resources are exactly SPEC-085 R6's names as `charts://<name>`, each `application/json` with the route's payload, and an unknown name is a resource error | `each_chart_resource_answers_its_series` |
| A38 | `deckstreakd mcp` is a role, not refused as an unknown one | `the_mcp_role_is_a_known_role` |
| A39 | with the scripted loader answering `Missing` for `mcp-core-token`, a token-shaped value in the process environment, on the command line and in a file of the working directory still refuses start by its id | `the_role_reads_its_tokens_only_through_the_loader` |

```acceptance
A1: cargo test -p deck-streak-mcp --test settings -- --exact the_listen_address_must_be_loopback
A2: cargo test -p deck-streak-mcp --test settings -- --exact the_core_credential_is_required
A3: cargo test -p deck-streak-mcp --test settings -- --exact a_missing_scope_credential_grants_nothing
A4: cargo test -p deck-streak-mcp --test settings -- --exact a_broken_scope_credential_refuses_start
A5: cargo test -p deck-streak-mcp --test settings -- --exact a_short_credential_refuses_start
A6: cargo test -p deck-streak-mcp --test settings -- --exact two_credentials_with_one_value_refuse_start
A7: cargo test -p deck-streak-mcp --test settings -- --exact the_drills_credential_is_read_only_when_drills_are_on
A8: cargo test -p deck-streak-mcp --test settings -- --exact the_drills_setting_is_off_unless_it_reads_on
A9: cargo test -p deck-streak-mcp --test guard -- --exact a_request_with_no_bearer_is_refused
A10: cargo test -p deck-streak-mcp --test guard -- --exact a_malformed_bearer_is_refused
A11: cargo test -p deck-streak-mcp --test guard -- --exact a_wrong_bearer_is_refused
A12: cargo test -p deck-streak-mcp --test guard -- --exact every_request_refusal_is_the_same_response
A13: cargo test -p deck-streak-mcp --test guard -- --exact each_grant_reaches_its_scopes_and_no_other
A14: cargo test -p deck-streak-mcp --test guard -- --exact a_tool_outside_the_scope_answers_unauthorized
A15: cargo test -p deck-streak-mcp --test guard -- --exact a_token_in_the_query_string_is_ignored
A16: cargo test -p deck-streak-mcp --test guard_census -- --exact the_guard_compares_only_digests_in_constant_time
A17: cargo test -p deck-streak-mcp --test limiter -- --exact the_limiter_matches_its_golden
A18: cargo test -p deck-streak-mcp --test limiter -- --exact the_bucket_matches_its_golden
A19: cargo test -p deck-streak-mcp --test limiter -- --exact the_guard_constants_match_the_golden
A20: cargo test -p deck-streak-mcp --test limiter -- --exact a_granted_token_never_limits_itself
A21: cargo test -p deck-streak-mcp --test limiter -- --exact a_refusal_logs_the_bucket_and_never_the_token
A22: cargo test -p deck-streak-mcp --test server -- --exact only_the_mcp_path_is_served
A23: cargo test -p deck-streak-mcp --test server -- --exact a_body_over_64_kib_is_refused
A24: cargo test -p deck-streak-mcp --test server -- --exact a_foreign_host_is_refused
A25: cargo test -p deck-streak-mcp --test server -- --exact the_ninth_request_in_flight_is_shed
A26: cargo test -p deck-streak-mcp --test roster -- --exact the_roster_matches_the_golden_with_drills_off
A27: cargo test -p deck-streak-mcp --test roster -- --exact the_roster_matches_the_golden_with_drills_on
A28: cargo test -p deck-streak-mcp --test tools -- --exact the_clamps_match_the_golden
A29: cargo test -p deck-streak-mcp --test tools -- --exact a_failed_write_is_a_tool_error
A30: cargo test -p deck-streak-mcp --test tools -- --exact erase_runs_only_with_the_word_erase
A31: cargo test -p deck-streak-mcp --test tools -- --exact export_answers_the_data_rights_document
A32: cargo test -p deck-streak-mcp --test tools -- --exact force_sync_reports_each_recompute_value
A33: cargo test -p deck-streak-mcp --test tools -- --exact the_law_track_answers_numbers_only
A34: cargo test -p deck-streak-mcp --test drills -- --exact the_drill_tools_match_the_golden
A35: cargo test -p deck-streak-mcp --test drills -- --exact an_unsafe_or_unknown_drill_is_not_found
A36: cargo test -p deck-streak-mcp --test drills -- --exact the_drill_ref_matches_the_golden
A37: cargo test -p deck-streak-mcp --test resources -- --exact each_chart_resource_answers_its_series
A38: cargo test -p deck-streak-daemon --test roles -- --exact the_mcp_role_is_a_known_role
A39: cargo test -p deck-streak-mcp --test settings -- --exact the_role_reads_its_tokens_only_through_the_loader
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

- **A token guessed by varying it on every attempt.** Each guess gets a fresh bucket, as in the
  predecessor, so the limiter bounds memory rather than guessing; R7's 32-character floor makes a
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
