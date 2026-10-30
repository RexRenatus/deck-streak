---
status: "proposed"
date: "2026-09-28"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The quest reintroduction draw is BLAKE2b in the shared kernel

## Context and Problem Statement

The daily quests suppress a quest key that shows no evidence of being reachable, and give it back
on 5 percent of days so a suppression never becomes a silent deletion (SPEC-080). The predecessor
decides this with a deterministic draw: BLAKE2b with an 8-byte digest over the UTF-8 bytes of
`seed|kind|reference`, read as a big-endian integer, modulo 10000
(`database.py:nudge_draw_bp`, predecessor `27ee2bc`). The quests use it with a fixed seed and a
reference that carries the study day's ISO date, so every past decision can be derived again from
what is stored. The predecessor's notification holdout draws from the same primitive, and
DeckStreak ports that holdout later (#132). The workspace admits `sha2` and `hmac` but no BLAKE2
implementation. What implements the draw, and which crate holds it?

## Decision Drivers

- Parity: the goldens hold the predecessor's draws, and any other hash or byte order changes every
  decision the predecessor made (CHARTER 8, ADR-012).
- Two contexts that may not depend on each other need the same primitive: quests now, and the
  notifications' holdout later. The shared kernel admits exactly what two such contexts both need
  (docs/CONTEXT-MAP.md).
- No new dependency without an ADR, and no cryptography written by hand in a public repository.
- A draw must stay derivable from stored state alone: the seed, the kind and the reference.

## Considered Options (the alternatives it was chosen against)

- RustCrypto's `blake2` in the kernel, with the 8-byte digest length set in BLAKE2b's parameter block (`Blake2b<U8>`), behind one function `draw_bp(seed, kind, reference)` — chosen: it reproduces the predecessor's draws byte for byte, it is a reviewed implementation of the same family as the admitted `sha2`, and one kernel function serves both contexts.
- `sha2`, which the workspace already admits — rejected because every draw, and so every suppression and reintroduction the predecessor decided, would differ from the golden.
- A BLAKE2b written by hand — rejected because unreviewed cryptography in a public repository is a liability when a reviewed crate exists.
- The first 8 bytes of a 64-byte BLAKE2b digest — rejected because BLAKE2 mixes the digest length into its parameter block, so a shortened long digest differs from Python's `digest_size=8` in every byte.
- One copy of the draw in quests and another in notifications — rejected because two copies drift, and neither context may own a primitive the other needs.
- The `rand` crate seeded from a hash — rejected because it adds a dependency and does not reproduce the predecessor's draw.

## Decision Outcome

Chosen option: "RustCrypto's `blake2` in the kernel, with the 8-byte digest length set in BLAKE2b's
parameter block, behind one function `draw_bp(seed, kind, reference)`", because it keeps the
predecessor's decisions exactly, needs no hand-written cryptography, and gives the two contexts one
primitive in the one crate they may both depend on.

- **The function.** `crates/kernel/src/draw.rs` defines `draw_bp(seed, kind, reference)`: the
  UTF-8 bytes of `seed`, `|`, `kind`, `|` and `reference`, hashed by BLAKE2b with an 8-byte digest,
  read as a big-endian 64-bit integer, modulo 10000. It returns a number from 0 to 9999, stores
  nothing and reads no clock. Confirm the conversion against the predecessor before building: its
  `database.py:nudge_draw_bp` returns `int.from_bytes(digest, "big") % 10_000`, and
  `goldens/draw_bp.json` holds that reading (SPEC-080 A1).
- **The dependency.** `blake2` joins `[workspace.dependencies]`, in the release built on the same
  `digest` generation as the admitted `sha2` 0.11, so the lock carries one `digest`; only the
  kernel's manifest names it. The digest length is the type's (`Blake2b<U8>`, Context7
  `/rustcrypto/hashes`); the crate's variable-length form is not used, so the length cannot be
  chosen at run time.
- **The callers.** The quests' reintroduction draws with the kind `quest_reintro` and the reference
  `<key>:<ISO date>`, the ISO date being the kernel's zero-padded rendering of the study day, or of
  the week's first study day for a weekly key (SPEC-020 R5); a key returns when the draw is below
  500. The holdout (#132) will draw with its own kind. A draw's inputs never leave the process.
- **The context map.** The kernel's line names the basis-point draw beside its other shared
  pieces; no edge changes.

### Consequences

- Good, because the port's decisions equal the predecessor's for every seed, key and study day,
  which the goldens prove.
- Good, because the holdout reuses a proved primitive rather than a second implementation.
- Bad, because every crate compiles one more dependency through the kernel, which all contexts
  depend on.
- Bad, because the kernel holds a primitive that is not itself a domain concept; the context map
  records why it is there.

### Confirmation

SPEC-080's A1 (`goldens/draw_bp.json`, references without dates) and A2
(`goldens/quest_reintro_draw_bp.json` and `goldens/quest_should_reintroduce.json`, the study day
written as its ISO date by the adapter), and the hand-proved row `S08008-DRAW-DIGEST-EIGHT-BYTES`,
which changes the digest length and is killed by A1.

## What would make this wrong

- The holdout (#132) is never built, or draws another way: the primitive would then serve one
  context and belong in it.
- The predecessor's stored decisions stop mattering after cutover and a new seed is chosen: a hash
  already admitted would then do, at the cost of every golden.
- A later `blake2` release changes the digest generation it builds on: the pin moves with `sha2`,
  in one change.

## More Information

SPEC-080 (the daily and weekly quests); #132 (the holdout that will reuse the draw); the
predecessor's `database.py:nudge_draw_bp`, `quest_reintro_draw_bp` and `quest_should_reintroduce`;
docs/CONTEXT-MAP.md (the shared kernel); ADR-012 (the parity oracle); Context7 `/rustcrypto/hashes`
(BLAKE2 with the output size fixed in the type).
