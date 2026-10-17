# Schematic: the MCP server's guard, and each refusal it answers

Kind: decision flow and state machine. Read at DeckStreak `dev` 5216bcf (ADR-002, ADR-025,
ADR-054, `docs/schematics/data-flow.md`, `docs/schematics/startup-settings-and-secrets.md`), and at
the predecessor's `27ee2bc` for the behaviour it ports (`mcp_auth.py:DrillAuth.require`,
`_bucket_id`, `_is_rate_limited`, `_record_failure`, `server.py:create_server`). Added by SPEC-119
(ADR-119, ADR-121). It adds one adapter to `data-flow.md`: the MCP server, which reads through
coordination's use cases like the API and the bot, and holds none of their rules.

## Start

```mermaid
stateDiagram-v2
  [*] --> Loading: the mcp role starts
  Loading --> Refused: the listen address unset, unparseable or not loopback
  Loading --> Refused: the core token missing, empty, unreadable or not text
  Loading --> Refused: a scope token empty, unreadable or not text
  Loading --> Refused: a token shorter than 32 characters, or two tokens holding one value
  Loading --> Serving: every grant's digest computed once
  Refused --> [*]: the error names the setting or the credential's id, never a value
```

Every token comes through the credential loader (SPEC-066); a missing scope token grants nothing
and is not an error.

## One request

```mermaid
flowchart TD
  req["a request on loopback"] --> shed{"fewer than 8 in flight"}
  shed -->|no| s503["503 at once, never queued"]
  shed -->|yes| parse{"one Authorization header, scheme Bearer, a token of visible ASCII"}
  parse -->|no| limit
  parse -->|yes| match{"SHA-256 of the token equals a grant's digest, ct_eq over every grant, no early exit"}
  match -->|no| limit{"five or more fresh failures in the bucket"}
  limit -->|yes| r401["401, the body unauthorized, WWW-Authenticate Bearer, nothing recorded"]
  limit -->|no| rec["the failure recorded, the bucket kept newest, the oldest beyond 512 evicted"]
  rec --> r401
  match -->|yes| path{"the path is the MCP path"}
  path -->|no| r404["404"]
  path -->|yes| tool{"the tool's scope is in the grant"}
  tool -->|no| slimit{"five or more fresh failures in the token's bucket"}
  slimit -->|yes| terr["a tool error whose whole text is unauthorized, no data"]
  slimit -->|no| srec["the failure recorded, the bucket kept newest, the oldest beyond 512 evicted"]
  srec --> terr
  tool -->|yes| uc["the owning SPEC's use case through coordination"]
```

The absent, empty, malformed, wrong and rate-limited refusals are one response, byte for byte:
nothing tells a caller which of them it met. Each refusal writes one warning naming the outcome, the
bucket and the scope, never the token or the header's value. A granted token is allowed before the
limiter reads anything, so it can never lock itself out. The limiter keeps no row: its buckets die
with the process.

## What the verifier plants

SPEC-119 §9 lists each plant (a guard that admits no header, a scope check skipped, a token compared
with `==`, the limiter consulted before the match, and the rest) with the criterion it must redden
and its mutation row.

## The grants, and what each reaches

Read at `dev` `b0935c75`, with SPEC-369 applied. Three grants exist: the read grant `{core}` of
the required `mcp-core-token`, the law-track grant `{core, law_track}` of the optional
`mcp-law-track-token`, and the write grant `{core, write}` of the optional `mcp-write-token`
(`crates/mcp/src/grants.rs`, `crates/mcp/src/settings.rs`). Every grant holds `core`, so
admission is unchanged; a tool then authorizes the one scope it needs, and a tool that changes
data needs `write`. `export_data` and `erase_all_data` are not served, so no grant reaches them.

```mermaid
flowchart TD
    REQ["a request with a bearer"] --> MATCH{"does the bearer match a grant?"}
    MATCH -->|"no, malformed or unknown"| UNAUTH["401 unauthorized, the refusal recorded in its bucket"]
    MATCH -->|"read grant: core"| ADMIT["admitted"]
    MATCH -->|"law-track grant: core and law_track"| ADMIT
    MATCH -->|"write grant: core and write"| ADMIT
    ADMIT --> CALL{"which tool does tools/call name?"}
    CALL -->|"export_data or erase_all_data"| NOTOOL["JSON-RPC error: no such tool, nothing read"]
    CALL -->|"get_law_track"| LAW{"does the grant hold law_track?"}
    LAW -->|"yes"| ANSWER["answered"]
    LAW -->|"no"| DENIED["tool error unauthorized, the limiter decides in the token's bucket, nothing read"]
    CALL -->|"a read tool, readOnlyHint true"| ANSWER
    CALL -->|"a write tool, readOnlyHint false"| WRITE{"does the grant hold write?"}
    WRITE -->|"yes"| ANSWER
    WRITE -->|"no"| DENIED
```

| request | no, malformed or unknown bearer | read grant `{core}` (`mcp-core-token`) | law-track grant `{core, law_track}` (`mcp-law-track-token`) | write grant `{core, write}` (`mcp-write-token`) |
|---|---|---|---|---|
| any request (`initialize`, `tools/list`) | 401 `unauthorized`, the refusal recorded in its bucket, the same response when rate-limited | admitted | admitted | admitted |
| `get_law_track` (needs `law_track`) | 401 | tool error `unauthorized`, nothing read, the limiter decides in the token's bucket | answered | tool error `unauthorized` |
| a read tool needing `core` (a later part of `#157`) | 401 | answered | answered | answered |
| a write tool, `readOnlyHint` false (a later part of `#157`) | 401 | tool error `unauthorized` | tool error `unauthorized` | answered |
| `export_data`, `erase_all_data` | 401 | JSON-RPC error: no such tool; nothing read | the same | the same |

Export and erase stay on the bot's `/export` and `/delete` and the host's `deckstreakd data
export` and `deckstreakd data erase --confirm ERASE`.

## Change note

2026-10-03 (SPEC-119 T14, #158): a scope refusal passes through the limiter with the token's
bucket before the tool error, as R13 and A20 say; the flowchart had sent it straight to the tool
error. Nothing else changed.

SPEC-369 (`#720`): a third grant, write, joins the read and law-track grants, and a tool
that changes data needs it; the two withdrawn tools answer as unknown. The section "The grants,
and what each reaches" is added before this note. Nothing else changed.
