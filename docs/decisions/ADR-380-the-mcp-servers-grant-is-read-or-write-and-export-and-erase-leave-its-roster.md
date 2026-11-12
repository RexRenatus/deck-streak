---
status: proposed
decision-makers: "the owner, the DeckStreak architect"
---

# ADR-380: the MCP server's grant is read or write, and export and erase leave its roster

Decides SPEC-369 (issue `#720`, parent `#157`). The MCP server's planned roster carries the
predecessor's data-export and erase-everything tools, and the core token's one grant would reach
every core tool once served, the eight that change data among them. This record decides how the
grant splits into read and write, how the two tools leave the roster without editing the golden,
and what waits for the first write tool.

**Amends, by this record, with no line of theirs edited:**
- ADR-320 D5 (`:47`): the closed scope set becomes `{core, law_track, write}`. The reason its
  rejected option gave (`:48`: a scope no credential can grant, a constant no code reads) does not
  apply here: `mcp-write-token` grants `write`, and `grants.rs` and SPEC-369's A6 census read it.
- ADR-320's "What would make this wrong" (`:100`), a client that must hold a scope without
  `core`: still not the case, since the write grant holds `core` (D7).
- ADR-121 (`:50-53`): kept. Core is required, and each scope token grants `core` and its scope;
  `mcp-write-token` is one more scope token.
- ADR-119 (`:27`): the server no longer offers erase, and its loopback reasoning stands. (`:11`):
  its 35 tools become 33, counting `#158`'s two drill tools.
- ADR-329 (`:13`): the predecessor's 33 core and law tools become 31 served by the roster.
- ADR-332 (`:60`): the unit's two credentials are unchanged now (D3).

## Context and Problem Statement

At `dev` `b0935c75` the server serves one tool, `get_law_track`, which authorizes `law_track`
(`crates/mcp/src/tools.rs:130-152`). `Scope` is closed at `core` and `law_track`
(`crates/mcp/src/grants.rs:12-23`); the required core credential grants `{core}` (`:135`), and
the guard admits any grant holding `core`. The roster golden holds 33 tools, 8 of them annotated
`readOnlyHint` false, `erase_all_data` among them, and SPEC-119 plans `export_data` and
`erase_all_data` under `core` (`:141-142`, R21 `:169-173`). So the core token would reach every
write, and a leaked one could erase the ledger (SPEC-119 section 6). Export and erase already have
their paths: the bot's `/export` and `/delete` and the host's `deckstreakd data` commands, all
through `export_all` and `erase_all` (`crates/coordination/src/data_rights_registry.rs:96`,
`:106`). The golden cannot be edited without regenerating it on the predecessor's checkout
(`tools/parity-oracle/test_goldens.py:154-165`).

## Decision Drivers

- No tool that changes data is reached by a grant that only reads.
- Every covered span, every existing row's anchor and the owner's stored credential id stay where
  they are.
- Nothing the predecessor offered is dropped silently: each withdrawal is recorded where a SPEC
  named it (SPEC-119, SPEC-001), as CHARTER 1 requires.
- The golden stays byte-equal to what the predecessor generated.
- Export and erase keep the paths they have, and the server adds none.

## Considered Options (the alternatives each was chosen against)

### D1. The names

- Keep the scope `core` and the credential `mcp-core-token` as the read grant, and add the scope `write` and an optional `mcp-write-token` — chosen, because no covered span moves and the owner's stored credential keeps its id.
- Rename the scopes to read and write — rejected, because it changes `admit`'s text, which stales a BearerGuard cover, and moves the stored credential's id, the scope values in logs, about six test helpers, the unit, the README and ADR-332.
- A scope per tool — rejected, because it makes 31 scopes for one owner's clients, and a client still needs every read scope.

### D2. The existing core token

- The core token maps to the read grant only — chosen, because no write tool is served at the base, so it loses nothing.
- Refuse start until a new read token is provisioned — rejected, because it breaks a working deployment for no write tool.
- The core token maps to read and write — rejected, because that is the single grant this record removes.

### D3. The unit

- No `LoadCredential` line for `mcp-write-token` now: the part that serves the first write tool adds it with its `CREDENTIAL_SOURCES` and `ROLE_CREDENTIALS` entries — chosen, because no credential is provisioned before a tool needs it.
- Add the line now — rejected, because a credential id the rail does not hold can answer empty, an empty credential refuses start, and the line buys no write tool.

### D4. When the split lands

- In code now — chosen, because the first write tool cannot land under `core`, and a split landed beside it would land unproved by any earlier delivery.
- In text now and in code with the first write tool — rejected, because the split would stay unproved until a delivery that also adds a write tool.

### D5. How the two tools leave the roster

- A reader-side exception, `WITHDRAWN` in `crates/mcp/tests/support/mod.rs`, which `roster()` drops — chosen, because the golden and its registry stay byte-equal and A41 still refuses a withdrawn tool served later.
- Edit the golden — rejected, because it breaks the golden's provenance and needs the predecessor's checkout to regenerate.
- A hand mark that the roster diverges — rejected, because the roster is one case, so the mark would cover all 33 tools.
- A production constant naming the two tools — rejected, because no production code would read it, which is ADR-320 D5's own objection.

### D6. Which tools need write

- A tool needs `write` when its annotations say `readOnlyHint` false — chosen, because the golden already carries that annotation for every tool.
- A hand list of write tools — rejected, because it drifts from the annotations.
- `destructiveHint` alone — rejected, because only the erase tool carried it, so seven writes would stay under `core`.

### D7. What the write grant holds

- The write grant is `{core, write}` — chosen, because admission is unchanged.
- `write` without `core` — rejected, because it needs `admit` changed, and it is ADR-320's own "what would make this wrong".

### D8. The law-track grant

- The law-track grant stays `{core, law_track}` — chosen, because a law client reads.
- The law-track grant also holds `write` — rejected, because a law client would then write.

### D9. Model and mutation

- No model change, and seven hand rows in the band `S36900-S36999` — chosen, because BearerGuard's "a scope the grant lacks" already stands for `write`, no covered span changes, and each new name, bit, grant, id and comparison gets a row its test kills.
- Extend BearerGuard with a write scope — rejected, because a new constant would move no variable the model checks.

## Decision Outcome

D1 to D9 as chosen above. The core credential's grant is the read grant; an optional write
credential grants `core` and `write`, loaded through the same arm as the law-track credential and
refused when it shares a value with either; a tool annotated as changing data authorizes `write`;
`export_data` and `erase_all_data` are withdrawn by a reader-side exception, with the golden
unchanged; and the unit waits for the first write tool.

## Consequences

- Good: a read-only client can no longer reach a tool that changes data, and no client of the
  server can erase the ledger.
- Good: no covered span, stored credential id, unit line or golden byte moves.
- Bad: a client that needs a write tool needs a second credential, provisioned with that tool.
- Bad: the predecessor's export and erase tools are gone from the server; an agent that used them
  uses the bot or the host instead.
- Neutral: `Scope` holds a third value that no served tool authorizes until the first write tool
  lands; A1 to A5 exercise it.

### Confirmation

SPEC-369's A1 to A9, the census's plants, the rows `S36901` to `S36907`, and the staged anchor
audit of every row on `grants.rs` and `settings.rs`.

## What would make this wrong

- If a client must write without reading, the write grant would drop `core` and `admit` changes
  then, with a model of the new admission (ADR-320's case).
- If the owner wants export or erase reachable from an agent, the tool returns under the write
  grant by a new SPEC, and `WITHDRAWN` loses its name.
- If a tool's annotation says read-only while its handler writes, D6 serves it under the read
  grant; the part that serves each write tool reads its handler against its annotation.
- If the router answers an unknown tool with a result rather than an error, A8 fails and the
  withdrawal needs a refusal of its own.
