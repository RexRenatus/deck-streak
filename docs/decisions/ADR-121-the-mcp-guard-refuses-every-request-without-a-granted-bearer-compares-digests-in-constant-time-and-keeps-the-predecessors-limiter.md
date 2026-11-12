---
status: "proposed"
date: "2026-09-29"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The MCP guard refuses every request without a granted bearer, compares digests in constant time, and keeps the predecessor's limiter

## Context and Problem Statement

#158 asks for fail-closed bearer auth on the law-track and drill tools: per-scope tokens from the
secret store, a constant-time comparison, one denial word, and the predecessor's limiter of five
failures a minute over at most 512 buckets (`mcp_auth.py:DrillAuth.require`, at `27ee2bc`). #157's server also carries erase and every read (SPEC-119 R15). DeckStreak loads secrets only through the credential loader
(SPEC-066). What does the guard cover, where does the token travel, how is it compared, and what
does a refusal look like?

## Decision Drivers

- Fail closed: an absent, empty, malformed or wrong bearer reaches nothing.
- A refusal tells a caller nothing it did not already know.
- A token is never written where a transcript, a log or a process listing keeps it.
- The predecessor's numbers are the parity: the window, the count, the buckets and the word.

## Considered Options (the alternatives it was chosen against)

- A granted bearer on every request: chosen, because nothing is open and every refusal carries the
  one word. The bearer travels in the `Authorization` header and matches a grant (core, law track
  or drills); SHA-256 digests are compared with `subtle`'s `ct_eq`; a 401 answers before the
  server sees the request; a tool error answers a scope the token lacks; and the limiter is the
  predecessor's.
- A token argument on the law-track and drill tools only: rejected because erase and every read would answer without a bearer, and an argument is written into the client's transcript.
- Compare the tokens themselves with `ct_eq`: rejected because `subtle` answers a length difference
  at once, so a token's length would leak; every digest has one length.
- A 401 for a missing scope too, read from the body: rejected because it puts a second protocol
  parser in front of `rmcp` for one word, and a caller whose request reached a tool has already
  proved its token is granted.
- Answer a limited bucket with 429: rejected because a distinct status tells a guesser which bucket
  is limited, and the predecessor's limited and ordinary denials read the same.
- Refuse start when a scope's credential is missing: rejected because the owner may run with the
  core token alone, and a missing scope granting nothing is already closed.
- The protocol's OAuth 2.1 authorization: rejected because it needs an authorization server and a
  browser flow for one owner on a loopback listener.

## Decision Outcome

Chosen option: "a granted bearer on every request, digests compared in constant time, a 401 before
the server, a tool error for a scope, and the predecessor's limiter", because it is the only option
that closes every tool while keeping each of #158's numbers.

- **The credentials.** `mcp-core-token` is required; `mcp-law-track-token` and `mcp-drills-token`
  grant nothing when missing and refuse start on any other loader error; each is at least 32
  characters, and no two share a value.
- **The grants.** Core grants `core`; each scope token grants `core` and its scope, so one client
  holds one token.
- **The refusals.** A request with no granted bearer: HTTP 401, `unauthorized`,
  `WWW-Authenticate: Bearer`, byte for byte the same for every cause. A granted token outside its
  scope: the tool error `unauthorized`. This departs from #158's "a wrong scope and a wrong token
  are identical" in the layer only, and the word is the same.
- **The limiter.** Match first, then the bucket; five fresh failures in 60 seconds limit a bucket;
  a limited failure records nothing; 10 timestamps a bucket; 512 buckets, oldest evicted. Its effect
  is the log's and the memory bound's, because the response never changes.

### Consequences

- Good, because a leaked law-track token cannot read drills, and no tool answers without a token.
- Good, because the journal holds a 16-character bucket, never a token.
- Bad, because a client must be configured with a header rather than an argument. The owner's
  client configuration changes once, at the cutover (#62).
- Bad, because a guesser who varies the token gets a fresh bucket each time.
  The 32-character floor is the plan's choice, because 32 characters of a generated token carry 128 bits when hex-encoded, and it is what makes guessing hopeless; the limiter only bounds memory.

### Confirmation

SPEC-119's A2 to A21, its rows S11902 to S11918, and its plant list.

## What would make this wrong

- A second client that must hold `core` without a scope and must not see which tools exist: the
  401 would have to cover `tools/list` per scope, and the roster would be filtered by grant.
- A limiter that must slow a guesser: it would need an identity other than the token, which this
  transport does not carry, and a delay or a lockout of its own decision.

## More Information

SPEC-119, ADR-119 (the server), SPEC-066 and ADR-067 (the loader), ADR-059, and the W6 schematic
`docs/schematics/mcp-guard-refusals.md`. Context7 answered for `rmcp` (the tower service, its
configuration and a tool's request parts) and for `subtle` (`ConstantTimeEq` on slices).
