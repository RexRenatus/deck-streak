---
status: "proposed"
date: "2026-09-29"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The MCP server is Rust on `rmcp`, in its own adapter crate, listening on loopback

## Context and Problem Statement

#157 ports the predecessor's tool server: 35 tools and the chart resources on MCP's streamable HTTP
transport, written in Python on FastMCP (`server.py:create_server`, at `27ee2bc`). #157 and #158
name the `agent` context as its home. The house stack is Rust-first (ADR-003), every other surface
is an adapter over `coordination` (the CONTEXT-MAP's third layer), and SPEC-001's parity row still
names FastMCP. Which language and library serve the protocol, which crate holds the server, and
where does it listen?

## Decision Drivers

- One language and one dependency audit for everything the host runs (ADR-003, the web and cargo
  audits).
- A tool is a use case: it must reach each context's rules through `coordination`, never around
  them.
- The crate graph is the context map (ADR-002): an edge that makes a cycle is refused by the
  compiler.
- The protocol changes by dated revisions; whoever speaks it must follow them.
- The server holds the owner's whole ledger, erase among it, so it listens where only the host can
  reach it.

## Considered Options (the alternatives it was chosen against)

- `rmcp` in a new adapter crate, on loopback only: chosen, because it keeps one language, follows
  the protocol's revisions, and sits in the graph exactly where `api` and `bot` sit. `rmcp` is the
  protocol's official Rust SDK, and the crate `deck-streak-mcp` depends on `kernel` and
  `coordination`.
- The predecessor's Python FastMCP, run beside DeckStreak: rejected because a second runtime and
  dependency tree would need its own packaging, audit and credential path, and it would read the
  ledger around `coordination`'s rules.
- A JSON-RPC server written by hand on `axum`: rejected because the protocol's negotiation, its
  transport headers and its schema declarations would be DeckStreak's to keep current with every
  revision, which the official SDK already tracks.
- The server inside `deck-streak-agent`, as the issues name: rejected because a tool calls
  `coordination`, and `coordination` depends on `agent`, so the edge would be a cycle.
- The server inside `deck-streak-api`: rejected because the API answers an owner session on the
  listener the public reverse proxy reaches (ADR-007), and a machine bearer's routes beside it
  would put the tool surface behind that proxy.
- The stdio transport, started by the client: rejected because a stdio server is its client's child
  process, and the client is not a DeckStreak process, so something outside DeckStreak would have
  to start the server.

## Decision Outcome

Chosen option: "`rmcp` in a new adapter crate `deck-streak-mcp`, loopback only", because it is the
only option that keeps the stack in one language, reaches every rule through `coordination`, and
leaves the crate graph acyclic.

- **The crate.** `deck-streak-mcp` depends on `kernel` and `coordination`; `daemon` depends on it.
  The delivery that builds SPEC-119 adds its line to `docs/CONTEXT-MAP.md`'s fence and names it in
  `daemon`'s line, in that same change (ADR-002's rule for a new edge).
- **The library.** `rmcp` with its server, macros and streamable HTTP server features, pinned at
  one version in `Cargo.lock`, admitted in `stack.json` and `deny.toml`.
- **The transport.** `/mcp` only, JSON responses, no session kept between requests, the loopback
  host names allowed, a 64 KiB body, and ADR-025's shed past a bound.
- **The listener.** A loopback address and port from `DECKSTREAK_MCP_LISTEN`; anything else refuses
  start. How a client reaches that port is outside the product (ADR-059).

### Consequences

- Good, because the tools are the same use cases the Mini App calls, so a rule changes in one place.
- Good, because the protocol's revisions arrive as a version bump of one crate.
- Bad, because the workspace gains a dependency tree the audits must cover. It is admitted once, in
  `stack.json` and `deny.toml`.
- Bad, because SPEC-001's parity row names FastMCP. The row's library is the predecessor's; the
  parity it asks for is the roster, the annotations and the numbers, which SPEC-119's goldens prove.

### Confirmation

SPEC-119's A1, A22 to A27 and A38, and the ddd probe's map in the box run, which holds every
manifest equal to the fence.

## What would make this wrong

- A client that must reach the tools over the network: the listener would need TLS and a public
  address, and this decision would be revisited with a threat model, not widened.
- `rmcp` falling behind the protocol's revisions: a hand-written transport would become the cheaper
  option, and the tools, which are use cases, would move unchanged.

## More Information

SPEC-119, ADR-121 (the guard), ADR-002, ADR-003, ADR-007, ADR-025, ADR-054, ADR-059, SPEC-001's
`mcp-server` row, and the W6 schematic `docs/schematics/mcp-guard-refusals.md`. Context7 answered
for `rmcp` and for the predecessor's Python SDK.
