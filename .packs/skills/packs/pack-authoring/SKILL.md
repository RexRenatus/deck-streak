---
name: pack-authoring
description: >-
  Teaches building a new skill pack, or growing an existing one, by the method every phoenix-v2
  pack was made with: R0 research with WebSearch and Context7 from primary sources, a coverage
  matrix, severity by evidence, wiring, red-first tests, proved rows and the hand-back. Lints any
  repository's packs through scripts/pack-lint.py --root. Use when creating, extending, splitting
  or reviewing a pack, its checks.json, its SPEC or its catalog row, in phoenix-v2 or in another
  repository that keeps the same skills layout, such as DeckStreak.
requires_phxd_schema: phxd.pack.probe.v1
---

# packs/pack-authoring

How a skill pack is made, and the check that keeps every pack honest (SPEC-V2-2210 /
ADR-V2-2210). The owner's words, 2026-09-27: "dev a skill pack for skill pack creations",
following "the same skill pack creation that we used to enhance the others with contrxt7 and
websearch".

The pack has two halves:

- **the method**: the procedure the first two pack waves followed, with the traps each of them
  hit, written where the next builder will read it;
- **the lint**: `scripts/pack-lint.py`, standard-library Python and vendorable. It judges every
  pack under a repository's `skills/` through `--root`, and each of this pack's rows runs one of
  its classes.

```
phxd pack probe --pack pack-authoring --root PATH --format json
```

Which seats consume this pack is its catalog row's `consumes`, the one record of that edge, so
this body names none.

## Why the method is written down

26 packs and 622 rows were built by one procedure that lived in two orchestrator briefs outside
the repository. Nothing checked several facts a pack states about itself. cyber-pipeline's body
still says web-security has "12 rows"; it has 43. A pack that states its own counts, links and
commands must have them checked, or they drift. The checks that did exist were phoenix-only:
`lint_tree` needs a cargo build, and `phxd pack probe` refuses every catalog but its own.

## The method, end to end

1. **Take your identity, never claim it.** The delivery number, the SPEC and ADR number, the
   row band and the worktree are allocated before you start. Verify your working directory with
   `&&` before every git write.
2. **R0: research first.** Use WebSearch AND Context7 (`resolve-library-id`, then `query-docs`)
   against primary sources: the standard bodies, the official docs and the specs.
   - Record every URL with its access date, and every Context7 library id that answered.
   - Note each deprecation and the date it took effect.
   - WebSearch shares one session cap across every agent. When it is spent, fetch the known
     primary URLs directly and use Context7, and say so in `## References`.
   - Send the orchestrator a checkpoint: the inventory size, the stages and the rows. Then keep
     working.
3. **R1: the coverage matrix.** In the SPEC, map every practice you found to a row id with its
   severity (`b` or `a`), or to an exclusion. An exclusion names its reason (a live fetch, a
   browser, judgement, a deprecation, another pack's subject) and a real ledger idea `iNNN` at or
   below the population frontier. An idea filed after the frontier needs a one-line fragment,
   `docs/records/idea-population.d/d<N>.txt`.
4. **Severity by evidence.**
   - `block` only for an invariant the repository already enforces, or one you measured green on
     every pack at your base.
   - `advisory` for heuristics and judgement. An advisory row reports and never refuses.
   - Run each candidate block rule over every pack before you write it. A rule that is red
     anywhere today is advisory, or waits on a named cleanup idea.
   - Send the orchestrator your block rules BEFORE you implement them, so that sibling builders
     can satisfy them.
5. **The body, `checks.json`.** The body is the one model's shape, `phx.skills.checks.v1`, and
   its keys are closed.
   - Its `pack` is the catalog id, `packs/<slug>`.
   - Every row carries an `id`, a `severity` from the card's vocabulary, a `probe`, a `stage`, a
     `reason` slug and a `scope` (`tree` or `live`).
   - A command probe names `{root}` for the judged tree and `{skills}` for the skills directory
     the pack ships from. Its wall is 1 to 900 seconds.
   - A pack whose rows read the box declares `live_rows_need_the_box`.
   - A `refuse-on-red` pack with tree rows lists the tree it judges in `paths`. If it has none
     here, `paths_pending_because` or a cited SPEC exclusion bullet naming the slug and the word
     "paths" says why.
6. **The probe's contract.** A class prints one line per finding and ends with `examined N`. It
   exits 0 green, 1 on a finding, 2 on a usage error, and 3 VOID when it examined nothing. VOID is
   never a pass. A class with no subject in a tree says `not-applicable` and counts what it read
   to decide that.
7. **`SKILL.md`.**
   - The frontmatter declares the card in `requires_phxd_schema`, equal to the catalog row's, and
     the Agent Skills `name` (the directory name) and `description` (what and when, third person,
     at most 1024 characters).
   - The body is the exhaustive catalog of the rows. It states each stage's count in a form the
     lint reads: `` The `<stage>` stage: <N> rows (<B> block, <A> advisory). ``
   - Keep it at 500 lines or under, and move reference material into files the body links.
8. **Wiring.**
   - Append ONE catalog row at the end of `skills/catalog.json`. Its leaf must be unique, its gate
     one of `none`, `advisory` or `refuse-on-red`, and its `consumes` the seats that apply it.
   - Run `python3 scripts/seat-pack-recipes.py --write`, then `--check`.
   - Run `python3 scripts/pack-census.py --repo .`.
9. **Tests, red first.**
   - Give every class a case it must refuse and a case it must pass, and make the passing case
     the one a naive reader gets wrong: a second statement, a longer id, a folded scalar.
   - Build fixtures inside the test. A Rust test compiles at the base and fails there by
     assertion.
10. **Rows.** Mutation rows go in your band's fragment, `scripts/mutation-rows.d/S<lo>-S<hi>.json`.
    Prove each one killed:
    - apply the mutant;
    - run its killer under a fresh `PYTHONPYCACHEPREFIX`;
    - restore the file byte-exact, checked by sha256.

    A mutant that does not compile is UNVIABLE, not caught.
11. **Documents.** The SPEC (with the sections in
    [spec-sections.template.md](templates/spec-sections.template.md)), an ADR that names what it
    was chosen against, a schematic, and `changelog.d/d<N>.md`.
12. **Verify with a script, not the prose list,** then hand back: the PR, its head, the red-first
    class, the evidence, the coverage numbers, and a short "how a repository uses this" note.

## Growing an existing pack

- Existing row ids and reasons keep their meaning.
- A parser defect you find is fixed red first. Look for the class d2190 found nine of: a
  substring, a prefix or a first match standing in for reading the value or every occurrence.
- A row your change makes wrong is re-anchored under its own id, never retired. Retiring a row
  whose target still exists is an owner ruling.
- Edit only your own pack. A defect in a sibling's pack goes to the orchestrator.

## Splitting a pack from a sibling

- **One bar, one owner (#641).** When two packs would judge the same thing, one pack owns the
  probe. The other either extends it through the catalog's `extends` and concatenates its rows, or
  judges something different under a different name.
- **Distinct names everywhere:** row ids unique across the catalog, a unique leaf, a probe
  script of your own, and your own band fragment, changelog fragment and SPEC number. A Rust
  constant takes a per-pack prefix, such as `GREENFIELD_PROBES`.
- **Never assert position.** A test that asserts your pack is the catalog's last entry breaks
  the moment a sibling appends after you.
- **Composition is measured, not assumed.** The union runs every sibling's tree-scanning guard
  over every sibling's files.

## The traps, each with its measurement

| # | trap | measured | do this |
|---|---|---|---|
| 1 | one constant, one home | three siblings each declared a file-scope `PROBES`, and `one_name_one_home` refused the union | prefix per pack; `grep -rn "const NAME" crates/` first |
| 2 | vocabulary locks read ANY text in `crates/*/src` | "evaluate" counts as `eval`; a union comment tripped `speculat` | say "repository", not "project", in Rust source |
| 3 | the deny read-set census reads string literals and comments | three pattern literals in a scanner spelled denied needles | spell them in pieces, with a comment saying why |
| 4 | catalog order | a test asserting a pack was last broke when two siblings appended | assert the property you mean |
| 5 | idea citations stop at the population frontier | an exclusion citing an idea one past it refused | add `docs/records/idea-population.d/d<N>.txt` |
| 6 | red-first carries only `.rs` files from `crates/*/tests/` | a committed fixture tree is invisible to it | build fixtures inside the test |
| 7 | compile at the base, fail by assertion | a test importing a new module's symbols is compile-arm, and a train carries one | drive the CLI or an existing public API |
| 8 | a new `src` module needs a judged crate home | its inline `#[cfg(test)]` span VOIDs red-first | tests in `crates/<crate>/tests/*.rs`; run the crate-map test |
| 9 | no literal work-left markers | a check that looks for them must not carry them | build the words from fragments |
| 10 | documented `phxd pack run` lines are executed | `every_documented_probe_command_runs` runs each one | keep flags inline in prose; a line names its own pack |
| 11 | a white-box walk concatenates the pack it extends | five blocking web-security rows read red on a sibling's bare Caddyfile | a fixture satisfies both packs |
| 12 | rows are re-anchored, never retired | ROW-REMOVED with its target present needs an owner ruling | re-anchor and re-prove |
| 13 | one red fixture and one green fixture prove little | d2190's nine parser defects were one class | a false-red and a false-green case per check |
| 14 | verify with a script | the first d2190 build ran a subset and missed four gate reds | one script running every stage |
| 15 | a file named `SKILL.md` anywhere under `skills/` is a skill | `lint_tree` refuses a directory holding one that no row catalogues | templates are `*.template.*` |
| 16 | a skill never waits for a landing | `test_skill_landing_conditionals.py` refuses a "once", "until", "when" or "after" clause that waits for a SPEC or delivery | state the current fact |
| 17 | a pack body names no seat | `lint_tree` refuses a seat id in a pack body | write "the consuming seats"; `consumes` carries the edge |
| 18 | a deny token in any `SKILL.md` or `checks.json` | the lint compares without case | read `skills/deny/DENY.md`, never restate it |
| 19 | no cargo inside a probe row | a pack probe inside a cargo test would start a second cargo on the box's two-slot limiter | cargo guards go in the verify script |
| 20 | `phxd pack probe` reads only the catalog it was built with | a foreign repository's packs cannot reach it | lint them with `pack-lint.py --root` |
| 21 | a consumed pack loads into its seat's prompt | the context-economy census read 334,401 characters against a 160,000 budget at the base | consume only where the seat applies the pack |

The cargo-built guards that rows 1 to 3, 5 and 8 name stay out of this pack's rows (trap 19). Run
them in your verify script:

- `bash scripts/skills-lint.sh`;
- `cargo test -p phxd --test one_name_one_home`, and each `*_vocabulary` test;
- `cargo test -p phxd --test spec_idea_citation`;
- `cargo test -p phxd --test forge_exec` (the read-set census);
- `python3 scripts/tests/test_phxd_crate_map.py`.

## The rows

Twenty-seven rows, all `tree`-scoped, one per class of `pack-lint.py`. Each runs
`python3 {skills}/../scripts/pack-lint.py --root {root} check <class>` under a 120-second wall, and
lints every pack in the judged tree's `skills/`. The block rows state invariants measured green on
all 26 packs at `e9fded051`, most of them enforced already by `lint_tree`, the census or the leaf
test.

The `body` stage: 8 rows (5 block, 3 advisory). They read each pack's `checks.json`.

| row | severity | reason | refuses when |
|---|---|---|---|
| `body-schema` | block | `body-malformed` | the body is not JSON, its schema is not `phx.skills.checks.v1`, its `pack` is not the catalog id, it has no rows, or it carries a key the one model does not declare |
| `row-fields` | block | `row-incomplete` | an id is not `[a-z0-9][a-z0-9._-]*`, a severity is off its card's vocabulary, a stage or reason is empty, a scope or schedule is unknown, or a live row's pack declares no `live_rows_need_the_box` |
| `row-ids-unique` | block | `row-id-duplicated` | two rows in one body share an id |
| `walk-agrees` | block | `walk-disagrees` | a declared `walk` names a stage no row uses, or a row's stage is missing from it |
| `probe-runnable` | block | `probe-unrunnable` | a wall is outside 1 to 900 seconds; a program, a `{skills}` path or a tree row's `{root}` path names nothing; a bare program is not on PATH; a word has a backslash; the reserved marker is used; or a string probe sits outside `phxd.pack.run.v1` |
| `reason-slug` | advisory | `reason-not-a-slug` | a reason is prose rather than a stable slug |
| `row-ids-catalog-unique` | advisory | `row-id-shared` | another pack uses the same row id |
| `bar-owned-once` | advisory | `bar-shared` | two packs run the same probe and neither extends the other (#641) |

The `skill` stage: 11 rows (7 block, 4 advisory). They read each pack's documents.

| row | severity | reason | refuses when |
|---|---|---|---|
| `frontmatter` | block | `frontmatter-invalid` | `SKILL.md` does not open with a `---` fence, has no closing one, or declares a card set other than its catalog row's |
| `stage-counts` | block | `stage-count-false` | a stated stage count, or a stated block-and-advisory split, differs from `checks.json` |
| `doc-lines` | block | `doc-line-foreign` | a line beginning `phxd pack probe`, `run`, `quality` or `release` names no `--pack` or another pack, or a `phxd pack check --id` line names no row |
| `links-resolve` | block | `link-dangling` | a relative markdown link outside code resolves to nothing |
| `deny-clean` | block | `deny-token` | `SKILL.md` or `checks.json` contains a token of `skills/deny/DENY.md`, compared without case; not-applicable with no table |
| `no-work-markers` | block | `work-marker` | any file of the pack carries a work-left marker word |
| `names-no-seat` | block | `body-names-seat` | `SKILL.md` names a catalogued seat on an id boundary |
| `rows-named` | advisory | `row-unnamed` | `SKILL.md` never names a row |
| `stage-counts-stated` | advisory | `stage-count-unstated` | `SKILL.md` states no count for a stage |
| `agent-skill-fields` | advisory | `agent-skill-fields-invalid` | the Agent Skills `name` or `description` is missing or breaks the specification's rules |
| `body-length` | advisory | `body-too-long` | `SKILL.md` is over 500 lines |

The `spec` stage: 3 rows (0 block, 3 advisory). They read the SPEC the body's `spec` names.

| row | severity | reason | refuses when |
|---|---|---|---|
| `spec-named` | advisory | `spec-unnamed` | the body names no SPEC, or no single file under `docs/specs/` is that SPEC |
| `spec-coverage` | advisory | `coverage-incomplete` | the SPEC has no coverage-matrix section, or the section never names one of the pack's rows |
| `spec-references` | advisory | `references-incomplete` | the SPEC has no References section, or it names no URL, no access date, or no Context7 id and never says none answered |

The `catalog` stage: 5 rows (4 block, 1 advisory). They read the wiring.

| row | severity | reason | refuses when |
|---|---|---|---|
| `catalog-row` | block | `catalog-row-invalid` | a directory under `skills/packs/` holds a `SKILL.md` no row catalogues, or a pack row has no directory, a path off its id, a bad gate, an empty or foreign `consumes`, a dangling `extends`, or no card |
| `leaf-unique` | block | `leaf-collides` | two catalog rows share a leaf |
| `seat-recipes` | block | `recipe-drift` | a seat's pack recipes are not what `seat-pack-recipes.py --write` makes; not-applicable with no seat |
| `census` | block | `census-gap` | `pack-census.py` reports a gap; not-applicable where the root carries no `ops/pack-probe.py` |
| `skills-readme` | advisory | `skills-readme-missing` | `skills/README.md` is missing, or has no install or no ratchet section (#1293) |

`seat-recipes` imports `seat-pack-recipes.py`'s own functions, and `census` runs `pack-census.py`,
so neither is a second copy. Several classes restate a `lint_tree` rule in Python, so a
repository without cargo can run it: `body-schema`, `row-fields`, `deny-clean`, `names-no-seat`,
`frontmatter` and `catalog-row`. A change to one of those rules is made in both places.

**Reading a verdict.** The card shows each row's exit. `pack-lint.py ... check <class>` prints the
findings, and `lint` prints every row at once. In SARIF's words:

| verdict | SARIF |
|---|---|
| a red block row | `kind: fail`, `level: error` |
| a red advisory row | `kind: fail`, `level: warning` |
| exit 3 (VOID) | `kind: open` |
| `not-applicable` | `kind: notApplicable` |
| green | `kind: pass` |

## The templates

Copy them, rename `example-pack` to your slug and `example` to your stage, and replace the
example rows with yours. Each template is green on every row as it stands, which the pack's tests
prove.

- [SKILL.template.md](templates/SKILL.template.md) becomes `skills/packs/<slug>/SKILL.md`. It
  carries the frontmatter, the stage statement, the rows table and the adoption section.
- [checks.template.json](templates/checks.template.json) becomes `skills/packs/<slug>/checks.json`.
  It holds one block row and one advisory row, each a command probe that runs a script of the
  pack's own.
- [spec-sections.template.md](templates/spec-sections.template.md) goes into your SPEC. It holds
  the coverage matrix, the exclusion section with its idea citations, and `## References`.

## How a repository adopts this

A repository such as DeckStreak keeps its packs in the same layout: `skills/catalog.json`,
`skills/packs/<slug>/SKILL.md` and `checks.json`, and its seats under `skills/roles/`.

1. Copy the three templates into a new pack directory, as above, and add its catalog row.
2. Lint it from phoenix-v2, against the other repository's tree:

   ```
   phxd pack probe --pack pack-authoring --root PATH --format json
   ```

   Or vendor `scripts/pack-lint.py` with `seat-pack-recipes.py` and `pack-census.py` beside it,
   plus a copy of this pack's `checks.json`, which `lint` reads for each row's severity. Then run
   `python3 scripts/pack-lint.py --root . --pack <slug> lint --checks <that copy>` in that
   repository's CI. It exits 1 on a red block row and prints every advisory.
3. Read what refuses: a lying stage count, a probe script that does not exist, a row id used
   twice, a link to nothing, a seat named in a pack body, a template named `SKILL.md`, or a seat
   whose recipe drifted. `census` is phoenix-only and reports `not-applicable` elsewhere.

## Filing an idea for an exclusion (#1701)

An exclusion cites an idea. When none exists, mint one with `phxd idea propose`. Give it a
`dedup_key` that starts with `docs-pack:` for agent documentation, or `cai:` for continuous
improvement. Write ledger ids as `iNNNN` and GitHub numbers as `#N`, never as bare numbers. An
idea filed after the population frontier also needs its fragment (trap 5).

## References

Accessed 2026-09-27. The full inventory, 78 practices mapped to rows or exclusions, is
SPEC-V2-2210 §4.

- Agent Skills: https://agentskills.io/specification ·
  https://platform.claude.com/docs/en/agents-and-tools/agent-skills/best-practices ·
  https://code.claude.com/docs/en/skills · https://github.com/anthropics/skills
- Rule design: https://semgrep.dev/docs/writing-rules/testing-rules ·
  https://eslint.org/docs/latest/extend/custom-rules ·
  https://doc.rust-lang.org/clippy/lints.html · https://doc.rust-lang.org/rustc/lints/levels.html ·
  https://github.com/ossf/scorecard/blob/main/checks/write.md ·
  https://docs.oasis-open.org/sarif/sarif/v2.1.0/sarif-v2.1.0.html
- Documentation: https://diataxis.fr/ ·
  https://platform.claude.com/docs/en/build-with-claude/prompt-engineering/claude-prompting-best-practices
- Context7 ids: `/anthropics/skills`, `/websites/code_claude`,
  `/websites/platform_claude_en_agents-and-tools_agent-skills`, `/semgrep/semgrep-docs`,
  `/eslint/eslint`, `/rust-lang/rust-clippy`, `/ossf/scorecard`, `/websites/diataxis_fr`
