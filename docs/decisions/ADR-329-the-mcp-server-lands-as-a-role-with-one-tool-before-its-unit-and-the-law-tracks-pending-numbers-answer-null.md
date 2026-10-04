---
status: accepted
date: "2026-10-03"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The MCP server lands as a role with one tool before its unit, and the law track's pending numbers answer null

## Context and Problem Statement

SPEC-119's guard landed as a library of `deck-streak-mcp` (#158, ADR-320), and nothing serves it.
#157 asks for the server: the `mcp` role, its transport, the guard in the serving stack and the
predecessor's 33 core and law tools (`server.py:create_server`, at `27ee2bc`). The owners of 14 of
those tools exist; the owners of the other 19 and of the chart resources are planned SPECs. The
service's unit needs a memory answer of its own (#157).

The one tool whose owner exists and whose scope is not `core` is `get_law_track`, which reads
SPEC-077's law block. Its owner, `coordination`'s `law::law_block`, answers no active leech count
and no mastery while the leech port is not wired (#133), and the predecessor coerces both to 0 and
0.0 (`server.py:get_law_track`, `_law_int`, `_law_float`).

What does #157's first pull request serve, how is the serving stack ordered around the guard, how
does the role read its tokens, and what does `get_law_track` answer for a pending number?

## Decision Drivers

- ADR-320 rejected, on the record, a role and a unit with no tool to serve.
- A request the guard refuses must never compete with the owner's agent for the bound (R4).
- A pending number is never 0 (SPEC-077 R12; the law module: "pending, never 0").
- The role reads a token only through the credential loader (R6, SPEC-066).
- Every number the tool answers is proved against the predecessor's roster golden, never re-derived
  by hand.

## Considered Options (the alternatives it was chosen against)

- D1, the role with one tool, `get_law_track`, first, and its unit in a second part: chosen, because the role then answers a real tool behind the guard, and the unit's memory answer holds nothing else back (#157).
- D1, a first part with the server and no tool: rejected, because it is the role that answers nothing which ADR-320 already rejected, so B1 and B2 would judge a role with no tool (#157).
- D1, one part with the role and its unit: rejected, because the unit reaches the deploy templates' per-unit tables, the host budget and the rail contract, and it waits on a memory answer the server does not need (#157).
- D1, the 14 tools whose owners exist, in this part: rejected, because it adds three goldens, five criteria, five rows and four writers over covered tables, which is larger than the guard's own pull request and its two fix rounds (#157).
- D4, the official Rust MCP SDK's streamable HTTP service, stateless, answering JSON, with the loopback names as its allowed hosts: chosen, because ADR-119 chose it, and stateless JSON keeps no session id that would be a second credential to guard (#157).
- D4, a hand-written axum JSON-RPC handler: rejected, because it re-implements `initialize`, version negotiation, the tool schemas and the error split by hand and would drift from the protocol, to save nine packages (#157).
- D4, the stdio transport: rejected, because the owner's agent runs across a process boundary and #157 names streamable HTTP (#157).
- D4, a stateful session store: rejected, because R2 forbids sessions and a session id would be a second credential (#157).
- D5, the guard's layer inside the trace and outside the shed: chosen, because a refused request is answered before it takes one of the 8 slots, and its `Authorization` header is already marked sensitive when the trace logs it (#157).
- D5, the guard inside the concurrency bound: rejected, because a local client sending no token would hold slots the owner's agent needs and push its calls into the shed (#157).
- D5, axum's `DefaultBodyLimit` as the body cap: rejected, because it bounds only axum's own extractors, the transport reads its own body, and a limit that binds nothing would read as one (#157).
- D6, the tokens read only through the kernel's credential loader, by `Grants::load`: chosen, because the loader reads systemd's credentials directory and registers each value with the redactor (#157).
- D6, a token in an environment variable: rejected, because the environment is no place for a secret (systemd.exec(5)) and the rust-service pack refuses it (#157).
- D6, a token file path setting: rejected, because it is a second reading path beside the loader, which A39 exists to refuse (#157).
- D7, `leech_total` and `mastery` answer `null` while they are pending: chosen, because a pending number is never 0, and the field set and its order stay the golden's (#133).
- D7, 0 and 0.0 as the predecessor coerces them: rejected, because a 0 tells the agent the owner has no leeches, which nothing measured (#133).
- D7, the API's `*_pending` flags beside the numbers: rejected, because the roster golden has no such fields, so the tool's output would stop matching the predecessor's field set (#157).
- D7, refusing the tool until the leech port is wired: rejected, because the agent would lose five real numbers for two pending ones (#133).

## Decision Outcome

Chosen option: the `mcp` role with `get_law_track` alone, behind the guard, with these rulings.

1. **The slice (D1).** This part is SPEC-119 section 13: R1's last two edges, R2 without its unit,
   R3, R4, R12's tool error, and R15 to R17 for `get_law_track`. Part 157b adds the unit, its
   budget and its rail entry, and judges B3; the other tools follow as their owners land, with A26
   last; the drill tools stay parked with #158.
2. **The transport (D4).** The SDK's streamable HTTP service at `/mcp`, with no session kept and no
   session id issued, answering `application/json`, its allowed hosts the loopback names and its
   own body cap equal to the stack's.
3. **The stack (D5).** Outermost first: the request id, the sensitive headers, the trace, the panic
   catch and the timeout, then the guard's layer, then the shed's handler, the load shed, the
   global concurrency limit of 8 and `tower-http`'s request body limit of 64 KiB.
4. **The tokens (D6).** The role reads them through `Grants::load` over the kernel's credential
   loader, and through nothing else.
5. **The pending numbers (D7).** `leech_total` and `mastery` answer `null`, never 0, and `mastery`
   is rounded to 2 places as `_law_float` rounds it when it is present. The divergence from the
   predecessor's 0 and 0.0 ends when #133 wires the leech port and `law_block` answers both.
6. **The input schema.** The tool takes no parameter. The predecessor's `token` parameter is the
   one measured difference from its roster entry (R15 removes it), recorded here and never edited
   into the golden.

### Consequences

- Good, because the guard now answers real requests, and the role answers a tool B1 and B2 can
  judge.
- Good, because a refused request never takes a slot, so a client without a token cannot shed the
  owner's agent.
- Good, because the agent reads five real law numbers now, and two honest nulls.
- Bad, because the role has no unit until part 157b, so nothing starts it on the host yet.
- Bad, because the tool's answer differs from the predecessor's while #133 is open: a client that
  reads `leech_total` as a number must accept `null`.

### Confirmation

SPEC-119's A33 and A41 to A45 (section 14), the rows S11932 to S11937, and `formal/tla/BearerGuard`,
re-read for the guard's two new callers and checked again with each property clean and each
witness caught.

## What would make this wrong

- An owner of another tool landing before part 157b: the next part would then add it beside the
  unit, and this slice's order would not matter.
- The leech port wired by #133: `leech_total` and `mastery` then answer numbers, and the null rule
  has nothing left to decide.
- An MCP client that refuses `null` for a field it reads as a number: the field would then need a
  declared pending flag, which would differ from the roster golden.

## More Information

Issues #157, #158 and #133; SPEC-119 sections 13 and 14; SPEC-077 R12; ADR-002 (the map is the
crate graph), ADR-025 (a bound sheds instead of queueing), ADR-119 (the server is Rust on the SDK),
ADR-121 (the guard) and ADR-320 (the guard first). The predecessor is cited as
`server.py:create_server`, `server.py:get_law_track`, `_law_int` and `_law_float` at `27ee2bc`.

## Amendment, 2026-10-04: one Consequences sentence

Insert-only; the bullet above is not rewritten, and every decision stands. The Consequences
sentence "Good, because the agent reads five real law numbers now, and two honest nulls." claims
more than the law block answers: `dues` is present only once the first recompute has run, and the
two nulls hold only while #133 is open. It reads instead: "Good, because the agent reads four
numbers that are always present, `dues` once the first recompute has run, and two honest nulls
while #133 is open."
