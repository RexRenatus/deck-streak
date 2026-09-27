---
requires_phxd_schema: phxd.pack.probe.v1
tip_floor: cac3ede067874a89b831ed4e544829291364ddd9
body_status: seeded
---

# packs/sdd

Spec-driven development in terms any repository can hold: the order of work, the shape of a SPEC
and an ADR, the kinds of schematic, and numbering that does not collide when several builders
write at once. Its probe judges any repository root (SPEC-V2-2186, ADR-V2-2186). Which seats
consume this pack is its catalog row's `consumes` (ADR-V2-1990), so this body names none.

```
phxd pack probe --pack sdd --root PATH --format json
python3 scripts/sdd-probe.py --root PATH check all
python3 scripts/sdd-probe.py --root PATH next spec --across-refs
```

`pack probe` runs each row in `PATH` and keeps only its exit, so read a red row by running the
probe itself: one line per finding, then one verdict line per class carrying `examined N`.

## The order of work

1. **SPEC.** What is wrong, measured; the requirements; acceptance criteria, each decided by a
   command; the file manifest; what the work does not do. No implementation without one.
2. **SCHEMATIC**, when a component, a data flow or a state machine is added or changed. It comes
   before the code, because a diagram drawn after the code describes what was built, not what was
   decided.
3. **ADR**, one per real decision, naming what it was chosen against. A decision with no rejected
   alternative is a preference; nobody can revisit it, because nobody recorded the trade.
4. **TESTS, red first.** Each acceptance command's test is written, run, and seen to fail for the
   reason the criterion states, before the code that makes it pass. The tdd pack holds the record.
5. **IMPLEMENTATION**, inside the manifest. A file the manifest does not name is a change the SPEC
   did not decide: amend the SPEC first.
6. **GATE.** The repository's own gate, whole, on the change. A gate that examined nothing is not
   a pass.

## The SPEC

`docs/specs/SPEC-<n>-<slug>.md`, from the repository's template. Six sections, each non-empty:

| section | holds | the probe's test |
|---|---|---|
| the problem, measured | numbers, with the command that produced each | a `## ` title holding `problem` |
| requirements | `R1.`, `R2.`: each true or false of the finished work | `requirement` |
| acceptance criteria | a table `\| A1 \| criterion \| decided by \|` and a fenced block of commands | `acceptance` |
| file manifest | every file added or changed, backticked, with its context | `manifest` |
| what this does NOT do | the boundary, each unit citing a tracked item | `what this does not`, `not covered`, `non-goals`, `out of scope` |
| risks | what could go wrong, and what would detect it | `risk` |

**Acceptance commands are fenced.** Every criterion the table states has a line in one block, and
every line names a stated criterion. A criterion two commands decide takes two lines:

    ```acceptance
    A1: cargo test -p deckstreak-cards --test deck -- --exact due_cards_come_oldest_first
    A2: cargo test -p deckstreak-review --test session
    A3: npx vitest run miniapp/src/review -t "shows the due count"
    ```

A command that selects no test cannot fail, so it decides nothing: the tdd pack's
`acceptance-has-a-test` resolves each line to a test that is in the tree.

**Every exclusion cites a tracked item**, an issue (`#12`) or an idea (`i7`), or whatever
`citation` in the config matches. An exclusion with no owner is a promise nobody holds. A
paragraph, a bullet and a table row are each a unit; a citation needs no letter or digit on
either side (`di7` and `i7a` are not citations).

**An amendment shares its parent's number**: `SPEC-12-amendment-<slug>.md`. It is counted for
numbering and not judged for shape.

## The ADR

`docs/decisions/ADR-<n>-<slug>.md`: Context, Decision, **What it was chosen AGAINST**,
Consequences, What would make this wrong. The alternatives section holds at least one real
alternative, as a table row `| alternative | why it lost |` or a bullet `- alternative: why it
lost`, and every one states its reason. A placeholder (`...`, `<...>`) is not an alternative.

Every SPEC is decided by an ADR: one carrying the SPEC's number, or one the SPEC names by id. A
named ADR that is not in the decisions directory is refused as a dangling reference.

## Schematic kinds

`docs/schematics/<slug>.md`, drawn in mermaid, naming the commit its `path:line` citations were
read at:

- **data flow**: what reads what, what writes what, and where a value crosses a boundary;
- **state machine**: every state, every transition, and the event or verdict that drives it;
- **component**: the units and the edges between them, which for a bounded context is the context
  map;
- **sequence**: the order of calls between actors, when an interleaving is the point.

## Numbers that do not collide

Concurrent builders each read only their own tree, so each derives the same "next free" number.
Measured in phoenix-v2 (the delivery-artifacts pack): two live deliveries both called themselves
d1652; two pull requests both went green holding 1905, because each compared itself to main alone.

- **One assigner.** A dispatcher hands out numbers and ranges; the builder's census is the net
  under it, never the source.
- **Census everything before writing and again before pushing.** `next spec --across-refs` and
  `check numbering-unique --across-refs` read the tree, every git ref (`refs/heads`,
  `refs/remotes`) and every worktree's files, uncommitted ones included. The same slug on two
  branches is one document; two slugs under one number is a collision, and the finding names
  where each was seen.
- **Namespaces are separate.** SPEC, ADR, and any other numbered series collide independently.
  Leading zeros name the same number: `SPEC-012` and `SPEC-12` collide.
- **The committed claim keeps the number.** An uncommitted one moves. A silent renumber leaves the
  assigner's records wrong: stop and ask instead.

## Check table

| id | scope | green when | on phoenix-v2 |
|---|---|---|---|
| `spec-sections` | tree | every judged SPEC has each configured section, none empty | binding |
| `exclusions-cited` | tree | every exclusion unit matches `citation` | advisory: the spec-idea-citation binary resolves each `iNNN` against the committed idea population |
| `acceptance-fenced` | tree | the `acceptance` fence and the criteria table name the same ids; every line parses | advisory: `scripts/spec_contracts.py` (packs/spec-contracts) proves each command selects a test |
| `manifest-present` | tree | the manifest names at least one path, none absolute or escaping the root | binding |
| `adr-alternatives` | tree | every judged ADR names an alternative and why it lost | binding |
| `adr-linked` | tree | every judged SPEC is decided by an ADR that exists | binding |
| `numbering-unique` | tree | no number is held by two documents unless one is an amendment; every document is judged | binding |

"Judged" means non-amendment and numbered at or above `adopted_from`.

## Configuration

`methodology.json` at the repository root; every key is optional, and a key the probe does not
know is refused by name (exit 2), so a typo cannot fall back to a default in silence.

| key (`sdd`) | default |
|---|---|
| `specs`, `decisions`, `schematics` | `docs/specs`, `docs/decisions`, `docs/schematics` |
| `spec_prefix`, `adr_prefix` | `SPEC-`, `ADR-` |
| `adopted_from` | `0`: every document is judged |
| `spec_sections` | all six keys: `problem`, `requirements`, `acceptance`, `manifest`, `exclusions`, `risks` |
| `citation` | `(?<![A-Za-z0-9])(?:i\d+\|#\d+)(?![A-Za-z0-9])` |
| `advisory` | `{}`: `{"<class>": "<the stronger gate that holds it here>"}` |

Exit codes: `0` OK, or ADVISORY (findings printed, the stronger gate named); `1` REFUSED; `3` VOID,
when a class examined nothing, because a check that examined nothing cannot fail; `2` a config or
usage error. `check all` exits with the worst: VOID over REFUSED over OK.

## Adopt in another repository

1. **Copy four files** from a phoenix-v2 checkout into your repository's `scripts/`, side by side:
   `scripts/methodology_probe.py`, `scripts/sdd-probe.py`, `scripts/ddd-probe.py` and
   `scripts/tdd-probe.py`. They need Python 3.11 and nothing else. The pack's rows name
   `scripts/<pack>-probe.py`, and a row runs inside the root it judges, so the copies must live
   there.
2. **Write `methodology.json`** at the root. For a Rust workspace, one crate per bounded context:

       {
         "vendored_from": "<git -C <phoenix-v2> rev-parse HEAD, at the copy>",
         "sdd": {},
         "ddd": {"dependency_prefix": "deckstreak-"},
         "tdd": {}
       }

   `vendored_from` is printed on every verdict line, so a stale copy names the commit it came
   from. An existing repository with documents written before it adopted the discipline sets
   `adopted_from` to its next free number; a greenfield one leaves it out.
3. **Add the gate line** to CI, one step:

       python3 scripts/sdd-probe.py --root . check all && python3 scripts/ddd-probe.py --root . check all && python3 scripts/tdd-probe.py --root . check all

A repository with no SPEC yet reads VOID on every class. That is the truth: it has not started.

## The reference implementation

phoenix-v2 holds itself to more than this pack, with machinery a greenfield repository does not
have: the delivery-artifacts pack (the artifact bar, idea citations resolved against the ledger's
population, the numbering census), the spec-contracts pack (every fenced command proven to select
a test by `phxd idea selects`), and the systems-architect seat's SPEC shape. This pack is their
portable subset. phoenix-v2's `methodology.json` adopts it from 2186, with the two advisory
classes above.

## What this pack does not do

- It runs no command a SPEC fences. Resolving a command to a test is the tdd pack's
  `acceptance-has-a-test`; running it is the repository's gate.
- It judges no document's prose beyond its shape: a SPEC can hold all six sections and still
  decide nothing.
- It assigns no number. `next` prints what the census reads; the assigner decides.
