# Schematic: the equivalence record, from a claim to the verdict, and the campaign that fills it

Kind: data flow (a record from its fragment to the verdict), a state machine (one record across
runs) and a delivery flow (the campaign, crate by crate, then the release rehearsal). Read at
DeckStreak `dev` 16ed8e2 (`scripts/mutation-verdict.py`, `.github/workflows/ci.yml`,
`.github/workflows/mutation-weekly.yml`), with cargo-mutants 27.1.0 and StrykerJS 10.0.0. Decided by
ADR-070 (proposed); planned by SPEC-057, whose vault delivery builds it. The pull request's own
mutation jobs, which this record joins, are drawn in `docs/schematics/mutation-testing.md`.

## 1. A record, from its fragment to the verdict

```mermaid
flowchart TD
  frag[(scripts/mutation-equivalent.d: one fragment per package, miniapp.json for the Mini App)] --> census[the census, in the gate's python stage: every field, a one-line reason, evidence, an issue, the anchor once in its file, the file inside its package, reached_by resolving to one test of that package, no record twice]
  census -->|a refusal| gatefail([the gate fails, naming the record])
  census -->|every record whole| bindable[records the verdict can bind]

  subgraph listings [the listings: mutation-plan and every battery]
    whole[cargo mutants --list --json: the whole tree, nothing built]
    diff[cargo mutants --list --json --in-diff git.diff: the diff's mutants, sharded]
  end

  bindable --> bind{each Rust record: listed mutants of its file and description whose span starts inside its anchor, and equals its span when it names one}
  whole --> bind
  bind -->|none| stale([STALE: fails, named])
  bind -->|two or more| ambiguous([AMBIGUOUS: fails, named])
  bind -->|exactly one| bound[the record's mutant]

  shardreports[(each shard's outcomes.json)] --> outcome{the bound mutant's outcome, when this run tested it}
  bound --> outcome
  outcome -->|missed| equivalent([EQUIVALENT: counted apart, named])
  outcome -->|caught or timed out| refuted([REFUTED: fails, named])
  outcome -->|unviable| unneeded([UNNEEDED: fails, named])
  outcome -->|outside this run's diff| idle([nothing to judge in this run; the battery tests it])

  shardreports --> missed{each missed mutant}
  missed -->|one record binds it| equivalent
  missed -->|no record binds it| unexplained([unexplained: fails, named])

  stryker[(mutation.json, each mutated Mini App file whole)] --> web{each miniapp.json record of a mutated file}
  bindable --> web
  web -->|binds a survived mutant| equivalent
  web -->|binds a killed or timed-out one| refuted
  web -->|binds an uncovered one| uncovered([UNCOVERED: fails, named])
  web -->|binds a compile or runtime error| unneeded
  web -->|binds no mutant of that file| stale
  stryker --> ignored{any Ignored mutant}
  ignored -->|yes| exclusionfail([fails: no disable comment is allowed])
```

The mutant a record excuses is never taken out of the listing: cargo-mutants and StrykerJS list,
shard, run and report it like any other, so each run that reaches it tests the claim again. Every
failing state names the record and fails its job; the census and the verdict print `examined N`.

## 2. What a record binds, and who checks it

| field | holds | checked by |
|---|---|---|
| `file` | the mutated file, from the repository's root | the census: inside its fragment's package |
| `mutant` | the tool's own description without its location: cargo-mutants' name after `<file>:<line>:<column>: `, or StrykerJS's `<mutatorName>: <replacement>` | the verdict, against the listing and the reports |
| `anchor` | a text that occurs exactly once in `file`, inside whose occurrence the mutant's span starts | the census (once in its file); the verdict (binds one mutant) |
| `span` | the mutated text, only when two mutants of that description start at one position | the verdict |
| `reason` | one line: why no test can tell the mutant apart | the census (one line); the reviewer |
| `evidence` | the code fact that makes it so, where a reviewer checks it | the census (present, not the reason again); the reviewer |
| `reached_by` | Rust only: a test of the mutant's own package that runs the mutated code, `<target>::<test path>` | the census (resolves to exactly one test); the reviewer (it runs the code) |
| `issue` | `#N`, the issue its delivery closes | the census |

An anchor is a whole line where the line holds one mutant of that description, and a narrower
window around the span's start where it holds two. Lines and columns are 1-based in both tools'
reports.

## 3. One record across runs

```mermaid
stateDiagram-v2
  [*] --> Written: a delivery records a mutant its sweep reported missed or survived
  Written --> Refused: the census refuses a field, the anchor, the file or a duplicate
  Refused --> Written: the delivery repairs the record
  Written --> Bound: exactly one listed mutant binds
  Bound --> Equivalent: the run tests it and it is missed or survived again
  Equivalent --> Bound: the next run that reaches it
  Bound --> Refuted: a test caught it, or it timed out
  Bound --> Stale: its anchored code changed, or its mutant left the listing
  Bound --> Ambiguous: a second mutant of its description starts inside the anchor
  Bound --> Unneeded: the mutant stopped building
  Refuted --> [*]: the record is removed; a test now kills the mutant
  Stale --> Written: the claim is examined again and bound anew
  Stale --> [*]: the mutant is gone, and the record with it
  Ambiguous --> Written: the anchor is narrowed, or span is added
  Unneeded --> [*]: the record is removed
```

A record never ends silently: each transition out of Bound fails the run that finds it, so the
record changes in a pull request that a reviewer reads. Lines that move above an anchor change
nothing: the anchor still occurs once and the mutant still starts inside it.

## 4. The campaign, crate by crate

```mermaid
flowchart TD
  start[a delivery, cut from dev] --> relist[cargo mutants --list --json --package the crate, at its base]
  relist --> opening[dispatch mutation-weekly.yml with package set to the crate]
  opening --> openrow[table: the opening row, unexplained above 0; committed with the row's test, red]
  openrow --> each{each unexplained mutant of the crate}
  each -->|a test can tell it apart| kill[a test in the crate's own tests that asserts the behaviour: red first when the behaviour is new]
  each -->|no test can| record[a record in the crate's fragment, with reason, evidence and reached_by]
  each -->|a test shows the code wrong| fix[a red-first fix, named in the pull request]
  kill --> closing
  record --> closing
  fix --> closing
  closing[dispatch the scoped battery again, at the head] --> closerow[table: the closing row, unexplained 0; the row's test green]
  closerow --> pr[a draft pull request into dev: the row, both runs, the red-first lines]
  pr --> next{the owner's order: vault, ingest, kernel, identity, daemon, coordination, api, then privacy, then the Mini App}
  next -->|another crate| start
  next -->|the Mini App, last| release
```

The first delivery, the vault's, builds sections 1 to 3 of this schematic before its first kill,
with the scoped dispatch and `table`, and carries the squash-subject match and the canonical-lock
guard (SPEC-057 R17, R18).

## 5. The release rehearsal

```mermaid
flowchart LR
  release[the last delivery's head] --> full[dispatch the battery with no scope]
  release --> merge[a local synthetic merge of that head into main, never pushed]
  merge --> plan[mutation-verdict.py plan --event pull_request --base-ref main]
  plan --> listing[cargo mutants --list --json --in-diff of the merge diff]
  full --> reports[(every shard's outcomes.json, mutation.json, rows.json)]
  listing --> table[mutation-verdict.py table --listed]
  reports --> table
  table -->|a listed mutant no report tested| void([VOID, named])
  table -->|an unexplained mutant, or a record that fails| fail([FAIL, named])
  table -->|every package unexplained 0, every listed mutant tested| ready([every row refilled; section 8 records the run; SPEC-057 moves to docs/specs])
```

The first release's merge diff holds every mutant of the tree (2,207 at `dev` 16ed8e2, the whole
tree's listing), so a scoped sweep of one crate at a head runs exactly the mutants the release
would run for that crate's files, with the same flags; the rehearsal ties every crate's row to one
head.
