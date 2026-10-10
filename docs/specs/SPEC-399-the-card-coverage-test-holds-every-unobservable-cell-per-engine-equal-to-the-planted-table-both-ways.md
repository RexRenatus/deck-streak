# SPEC-399: the card coverage test holds every unobservable cell, per engine, equal to the planted table both ways

- **Issue:** #661, raised as an advisory by the verify of #655 (SPEC-341). **Context(s):** `miniapp`
  (`web/app/src`, its unit tests).
- **Decided by:** ADR-413 (this SPEC's own: the population, the parse and the equality, the
  red-first proof, no formal model, the order against open work), under ADR-352 D7 (the planted
  suite with reference controls).
- **Schematic:** `docs/schematics/card-frame-channels.md`, which owns section 3. This delivery adds
  its last section, "How section 3's unobservable cells are held", with the data flow of this check.
- **Status:** this delivery builds it, with its tests and `docs/red-first/SPEC-399.md`. **Mutation
  band:** S39900-S39999, claimed and left unwritten: no row (ADR-413 D3).

## 1. The problem, measured

Every figure is read at `dev` cfa7279991a9, with `git show cfa7279991a9:<path>`.

- Section 3 of `docs/schematics/card-frame-channels.md` (lines 80-275) has 29 channel rows (lines
  97-125). Seven of them carry an `UNOBSERVABLE` note in their last cell, nine tokens in all. Two
  hold for every engine, spelt `UNOBSERVABLE:` (`dns-prefetch` line 107, `external-scheme` line
  118). Five hold for named engines, spelt `UNOBSERVABLE in <Engine>,`: `prefetch` in WebKit (line
  105), `preconnect` in Chromium and in Firefox (line 106), `shadow-link` in Chromium and in
  Firefox (line 108), `ping` in Firefox (line 115) and `webrtc` in Firefox (line 124). The command
  `git show cfa7279991a9:docs/schematics/card-frame-channels.md | sed -n '80,275p' | grep -o -E 'UNOBSERVABLE( in [A-Za-z]+|:)' | sort | uniq -c`
  prints 2 `UNOBSERVABLE in Chromium`, 4 `UNOBSERVABLE in Firefox`, 1 `UNOBSERVABLE in WebKit` and 2
  `UNOBSERVABLE:`: nine tokens in seven cells, where three cells hold one engine's token, two hold
  two engines' tokens, and two hold the every-engine token.
- `web/app/tests-card/planted.ts` declares thirteen `(engine, id)` pairs in its `UNOBSERVABLE`
  table (lines 207-219): `dns-prefetch` and `external-scheme` in all three engines, `prefetch` in
  `webkit`, `preconnect` and `shadow-link` in `chromium` and `firefox`, `ping` and `webrtc` in
  `firefox`.
- `web/app/src/lib/card/planted-coverage.test.ts`, in "every channel in the schematic has a planted
  card" (lines 101-150), selects the rows whose last cell holds `UNOBSERVABLE:` (line 128). It
  reads the two every-engine cells and none of the five named-engine cells, then requires each of
  the two ids in each engine's declared list (lines 130-133): six inclusions, one way only. A
  declared pair that no cell names, and a named-engine cell with no declared pair, both pass it.
  Its engines are a list kept by hand (line 19), beside the card suite's own configuration
  (`web/app/playwright.card.config.ts` lines 17-21: three projects, `chromium`, `webkit` and
  `firefox`).
- `web/app/tests-card/card.spec.ts` holds the declared table against the engines (the blind check
  from line 44, used at lines 114, 136 and 159), and it reads the engine as the project's name
  (line 40). The table therefore equals what each engine measures, and no test holds it against
  section 3's named-engine cells.
- Expanded over the three configured engines, the seven cells name thirteen pairs, and they are the
  thirteen declared: the set difference is empty in both directions at this commit. The work is the
  hold, not a correction of either side.

## 2. Requirements

R1. The coverage test reads every `UNOBSERVABLE` token in every cell of each section-3 channel row
    (a row whose first cell is a backticked id), counted case-insensitively, and reads each in one
    of two forms only: `UNOBSERVABLE:`, which names every engine the card suite runs, or
    `UNOBSERVABLE in <Engine>` followed by a comma or a closing parenthesis, which names the one
    engine whose project name equals `<Engine>` case-folded. A token in any other form, or an
    `<Engine>` that no configured project carries, fails the test and names the row.
R2. The engines are the project names of `web/app/playwright.card.config.ts`, read from that file's
    text: one literal `name` for each project object in its `projects` array, in order. A file with
    no `projects` array, a project with no literal name, or a name given twice fails the test. The
    test keeps no second engine list: every check in the file that ranges over engines ranges over
    this one.
R3. The pairs section 3 names (each every-engine cell expanded to one pair per engine, each
    named-engine token one pair) equal the pairs `planted.ts` declares in `UNOBSERVABLE`, exactly
    and both ways: no named pair is undeclared, no declared pair is unnamed, and neither side holds
    a pair twice. A failure names each missing and each extra pair.
R4. The test prints the counts it examined (the engines, the unobservable cells, the pairs section
    3 names and the pairs `planted.ts` declares) and refuses a zero count, after its judgment.
R5. The test is seen red first: its tests are committed over a planted copy of the parse they
    replace (every-engine cells only, one way, engines kept by hand), where the planted mismatches
    in test-local fixture text (cells copied from section 3, a copied table) read red by assertion.
    No shipped file is edited to make a red.
R6. The schematic gains an insert-only last section that states how section 3's unobservable cells
    are held, with the data flow of this check and the commit it was read at.
R7. No shipped file changes. `web/app/tests-card/planted.ts`, `web/app/tests-card/card.spec.ts`,
    `web/app/playwright.card.config.ts`, `.github/workflows/ci.yml`, the card frame and its policy
    are read and never edited.

## 3. Acceptance criteria of this delivery

| id | criterion | decided by |
|---|---|---|
| A1 | section 3's unobservable cells, expanded over the configured engines, equal the pairs `planted.ts` declares, both ways, every token is read in one of the two forms, and the test prints the counts it examined | `web/app/src/lib/card/planted-coverage.test.ts` "the schematic's unobservable cells equal the declared unobservable pairs both ways" |
| A2 | a planted mismatch is refused by name: a named-engine cell with no declared pair, a declared pair with no cell, an every-engine cell declared in one engine, an engine no project carries, a note in another spelling, and a further configured engine expanded from the every-engine cells | `planted-coverage.test.ts` "a planted mismatch between the cells and the declared pairs is refused" |
| A3 | the engines are the configuration's project names, read from its text: an altered configuration's projects follow, and a project with no literal name, a name given twice or no projects array is refused | `planted-coverage.test.ts` "the engines are the card configuration's projects" |

```acceptance
A1: pnpm exec vitest run web/app/src/lib/card/planted-coverage.test.ts -t "the schematic's unobservable cells equal the declared unobservable pairs both ways"
A2: pnpm exec vitest run web/app/src/lib/card/planted-coverage.test.ts -t "a planted mismatch between the cells and the declared pairs is refused"
A3: pnpm exec vitest run web/app/src/lib/card/planted-coverage.test.ts -t "the engines are the card configuration's projects"
```

## 4. File manifest

| file | context | change |
|---|---|---|
| `web/app/src/lib/card/planted-coverage.test.ts` | `miniapp` | changed: the engines read from the card configuration's text (R2); A1 to A3 and their helpers |
| `docs/schematics/card-frame-channels.md` | docs | changed, insert-only: a new last section, how section 3's unobservable cells are held (R6) |
| `docs/specs/SPEC-399-the-card-coverage-test-holds-every-unobservable-cell-per-engine-equal-to-the-planted-table-both-ways.md` | docs | added |
| `docs/decisions/ADR-413-each-unobservable-cell-expands-over-the-card-suites-configured-engines-and-must-equal-the-planted-table-exactly.md` | docs | added |
| `docs/red-first/SPEC-399.md` | docs | added |
| `changelog.d/card-unobservable-cells-399.md` | docs | added |

## 5. What this does NOT cover

- It changes no shipped file and no measured fact: section 3 and the declared table agree at the
  base, so neither is corrected (#661).
- It adds no engine to the card suite. Firefox is already its third project (SPEC-398, #652); a
  further configured project is expanded from each every-engine cell by this test, which then
  requires that project's pairs with no edit here (#652).
- It does not hold the iPhone and iPad planted table against the schematic's iOS sections (4 and
  6): the issue's scope is section 3, the web table (#661).
- It does not change what the planted suite measures in each engine, nor what SPEC-341 A8 to A11
  already hold: each channel's planted card, its layer alone, the scripts-on cards (#619).
- It adds no mutation row: no row table takes this runner's killers, and the web mutation tool
  skips test files, so the test's own planted mismatches are its counterexamples (#661).
- It does not touch the native card view (#664, #677).
- No CI job changes: the `web` job already runs the unit tests on every pull request (#661).

## 6. Risks

- **A further engine lands first.** Its project makes every every-engine cell name one more pair,
  so `planted.ts` must declare them. Detected by A1 at the cut: the build re-measures section 3,
  `planted.ts` and the configuration there, and states the cut's counts.
- **A note in a new spelling** (two engines in one note, a lower-case token) is refused rather than
  read. Detected by A1, which names the row; the writer splits the note into one token per engine,
  or an ADR widens the grammar.
- **The `Engine` type in `planted.ts` is a closed union.** A configured engine it lacks cannot be
  declared, so A1 stays red until both widen together, which is the intended failure.
- **A configuration that builds its projects in code** (a mapped list, a spread) gives the text read
  no literal names; R2 refuses it instead of dropping engines, and the read is then reconsidered by
  an ADR.
- **The counts above are this commit's.** The test reads them live, so a later count is never
  compared with a number kept here.

## 7. Requirements no test runner decides

- R6: insert-only, by `cmp -n <the cut's size> <the cut's copy> <the head's copy>`, silent with
  rc 0, and the new section is the file's last.
- R7: the file manifest against the delivery's diff, which holds no shipped file.
