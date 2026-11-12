---
status: accepted
date: "2026-10-03"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The MCP guard lands first, as a library of the adapter crate with the kernel edge only

## Context and Problem Statement

SPEC-119 (planned) covers two issues: #157, the MCP server with its role, its listener, its shed
and its 33 tools, and #158, the fail-closed bearer guard with its grants and the predecessor's
limiter (`mcp_auth.py:DrillAuth.require`, at `27ee2bc`). The tools call use cases of SPEC-071 to
SPEC-093 through `coordination`, and most of those SPECs are still planned. The guard reads only
the kernel: the credential loader (SPEC-066), the clock (SPEC-020 R6) and the redactor. ADR-119
puts both in one adapter crate, `deck-streak-mcp`, and ADR-002 adds a dependency edge in the change
that first uses it.

Which part of SPEC-119 can land now, which edges does the crate declare when it does, which clock
does the limiter read, what does the loader do with a credential no request can present, and how
is the limiter's check-then-record made safe under concurrent requests?

## Decision Drivers

- #158's numbers (the window, the count, the buckets, the history and the word) are proved against
  the predecessor's goldens, never re-derived by hand.
- The context map is binding: an edge no code uses is a line the map cannot verify.
- A credential that refuses nothing and grants nothing fails silently; start is where a fault is
  cheapest to read.
- Up to eight requests at once share one limiter (R4), so its read and its write race unless they
  are one step.

## Considered Options (the alternatives it was chosen against)

- D1, the guard alone as a library of `deck-streak-mcp`, with the kernel edge only: chosen, because every #158 number lands proved by its golden now and nothing in it waits on a use case (#158).
- D1, one pull request for the whole SPEC: rejected, because the guard would wait unproved behind the planned use cases its 33 tools call (#157).
- D1, the guard with the server shell and `rmcp` now: rejected, because it admits `rmcp` and adds a role and a unit with no tool to serve, so B1 to B3 would judge a role that answers nothing (#157).
- D1, declaring the `coordination` edge now, unused: rejected, because ADR-002 adds an edge in the change that first uses it, and an unused edge is a map line no code holds (#157).
- D2, the kernel's `Clock`, read once per decision: chosen, because SPEC-020 R6 makes it the workspace's one reader of the time and a test drives it with `ManualClock` (#158).
- D2, a monotonic instant read inside the limiter, as the predecessor's `time.monotonic`: rejected, because it is a second reader of the time outside the kernel and no test can set it (#158).
- D3, a loaded token holding a byte outside 0x21 to 0x7E refuses start by its id (`McpError::UnpresentableCredential`): chosen, because R9 lets no request present such a token, so its grant would be dead and silent (#158).
- D3, loading it as it is: rejected, because a credential file written with a carriage return would grant nothing and nothing would say why (#158).
- D3, trimming a trailing carriage return in the adapter: rejected, because the loader trims one newline only (ADR-067), and a second trimming rule would make the granted value differ from the one the redactor registered (#158).
- D4, one lock around the whole decision (read the fresh failures, then refuse as limited or record): chosen, because the count and the record are then one step, which `formal/tla/BearerGuard`'s `FreshFailuresAtMostMax` checks (#158).
- D4, counting and recording under two locks: rejected, because two refusals that each read four fresh failures record a fifth and a sixth, and the model's witness catches it (#158).
- D4, a concurrent map crate for the buckets: rejected, because it is a new external crate for at most 512 entries, which a linear scan bounds (#158).
- D5, the closed scope set `{core, law_track}`, with no drill scope in this part: chosen, because the drill surface is parked and its part adds its scope, credential, setting and constant together (#158).
- D5, the set `{core, law_track, drills}` now, with no grant able to hold the third: rejected, because it names a scope no credential can grant and pins a constant no code reads (#158).

## Decision Outcome

Chosen option: the guard alone, as a library of `deck-streak-mcp` whose only edge is the kernel,
with these rulings.

1. **The slice (D1).** This delivery is SPEC-119's R1 in part (the crate, its member and its map
   line `depends on: kernel`), R6 to R11, R13 and R14 for the core and law-track credentials, R12's
   scope decision, A2 to A6, A9 to A12, A15 to A21 and the new A40, and the goldens
   `mcp_auth.constants`, `mcp_auth_bucket` and `mcp_auth_limiter`. #157 adds the `coordination`
   edge, `deck-streak-daemon`'s line, the role, the listener, the shed, `rmcp` behind the guard's
   layer and the tool error that carries the scope decision. The drill tools follow R19. SPEC-119
   section 3c holds the rest and section 10 records the amendments.
2. **The request layer.** A tower `Layer` answers R11's refusal before its inner service is called,
   and inserts the matched grant into the request's extensions, where #157's tools read it from the
   request parts `rmcp` hands them. The refusal is built in one function from constants.
3. **The clock (D2).** The limiter reads the kernel's `Clock` once per decision. Where the
   predecessor's clock was monotonic, a wall clock that steps back keeps failures fresh longer; that
   changes which log line a refusal writes, never the response.
4. **An unpresentable credential (D3).** R7 refuses it by its id, before its length is read (SPEC-119
   T15).
5. **The lock (D4).** One `std::sync::Mutex` holds the buckets; it is never held across an await, and
   a poisoned lock is recovered, because every statement leaves the buckets consistent.
6. **The scopes (D5).** `core` and `law_track`; the core credential grants `{core}` and the
   law-track credential `{core, law_track}`.

ADR-121 is not edited. Its Confirmation names A7, A8, A13, A14, S11906 and S11911, which this
delivery does not hold: A13, A14 and S11911 are #157's, and A7, A8 and S11906 come with the drill
tools.

### Consequences

- Good, because #158's numbers land proved while #157 waits on its use cases.
- Good, because the crate's one edge is one the map can verify today.
- Good, because a credential file with a carriage return refuses start by its id instead of
  granting nothing.
- Bad, because the guard is a library no binary serves yet: until #157, no request reaches it in
  production.
- Bad, because a step back of the wall clock keeps failures fresh longer than the predecessor's
  monotonic clock would.

### Confirmation

SPEC-119's A2 to A6, A9 to A12, A15 to A21 and A40; its rows S11902 to S11905, S11907 to S11910,
S11912 to S11918, S11928 and S11929; and `formal/tla/BearerGuard`, whose four properties each catch
their witness.

## What would make this wrong

- A use case #157 needs that the guard must call: the guard would then need `coordination`, and
  the edge would land with the guard after all.
- A second client that must hold a scope without `core`: the grants would no longer each hold
  `core`, and the request layer's admission would have to read a scope.
- A limiter whose window must survive a wall-clock step: it would need a monotonic reader in the
  kernel, which is a new ADR.

## More Information

Issues #157 and #158; SPEC-119's section 3c and section 10; ADR-002 (the map is the crate graph),
ADR-016 (a delivered SPEC moves with its tests), ADR-067 (the loader refuses an empty credential),
ADR-119 (the server) and ADR-121 (the guard). The predecessor's guard is cited as
`mcp_auth.py:DrillAuth.require`, `_bucket_id`, `_is_rate_limited` and `_record_failure` at
`27ee2bc`.

## Amendments, 2026-10-03: the Confirmation names two more rows (ruling 85 (b))

The Confirmation's rows also include `S11930-MIN-CREDENTIAL-CHARS` and `S11931-FORBID-UNSAFE`,
which this decision's pull request delivered (#158). This amendment records no new decision, so it
names no rejected option.
