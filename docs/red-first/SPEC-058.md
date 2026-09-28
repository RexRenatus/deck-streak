# Red-first record: SPEC-058

The SPEC, the schematic, ADR-055's note and SPEC-038's amendment were committed alone (13ef8a9),
with the SPEC in `docs/specs/planned/`. Then came the tests of A1 to A5 (64517ff). They ran against
the base's own stage, `pnpm audit --prod`, with no stub, since every case drives `check.sh` itself.
Each was red for its own reason. The stage and its verdict (9f79d10) turned all five green. After
the green, the verdict read an advisory's fields directly (c4f16f9), and A2 and A3 gained the cases
the mutants below needed (4ad713c), each green on arrival. `dev` c3d769b, which holds #259, was
merged in (355bbb7) with no conflict. Each red was observed on the tree its commit holds.

```red-first
A1: red at 64517ff: AssertionError: Lists differ: ['audit', '--prod'] != ['audit', '--json', '--audit-level', 'low']
A1: green at 9f79d10
A2: red at 64517ff: AssertionError: Regex didn't match: '^FAILED +audit-web +\\d+s exit 3: audit-web: VOID: ' not found in 'ok     audit-web     0s' (a report that examined 0 packages passed, and each run that gave no report failed with pnpm's own last line, never VOID)
A2: green at 9f79d10
A3: red at 64517ff: AssertionError: Lists differ: ['  }', '}'] != ['audit-web: low planted-package 1.0.0 GHSA[184 chars]: 1'] (the stage's log ended with pnpm's raw report, naming no advisory; a report holding one while pnpm exited 0 passed)
A3: green at 9f79d10
A4: red at 64517ff: AssertionError: Lists differ: ['}'] != ['audit-web: examined 5 package(s) (depende[79 chars]: 0'] (the stage printed no examined count)
A4: green at 9f79d10
A5: red at 64517ff: AssertionError: Regex didn't match: '^FAILED +audit\\-web .*: missing tool: python3 \\(\\S.*\\)$' not found in 'ok     audit-web     0s'
A5: green at 9f79d10
```

A5 is SPEC-038's test, whose table `TOOLS` grew by `python3` for `audit-web` in the red commit
(SPEC-038 section 10); its other cases were green before and after.

## The stage on the real tree

Measured by `bash scripts/check.sh audit-web` (SPEC-058 section 7): at 4ad713c, on `dev` 16ed8e2's
lockfile, the stage failed, `examined 428 package(s) (dependencies 0, devDependencies 428,
optionalDependencies 87), advisories at or above low: 4`; at 355bbb7, with #259 merged in, it
passed, `examined 428 package(s) (dependencies 0, devDependencies 428, optionalDependencies 87),
advisories at or above low: 0`.

## Mutants of the changed code

No mutation runner generates mutants of this repository's Python or shell (#218), so the changed
code's mutants were proved by hand at 4ad713c, on the committed tree. Each mutant replaced an
anchor that occurs exactly once, and its killer, one acceptance test selected alone, was green on
the unmutated file and red on the mutant. The file was restored byte for byte after each, checked
by its sha256. All 38 were killed:

| requirement | mutant | killed by |
|---|---|---|
| R1 | the call narrowed by `--prod`, or by `--dev`; `--ignore-registry-errors` added | A1 |
| R2 | no level on the command line | A1 |
| R2 | the level raised to `moderate`; the verdict holding `low` whatever level it is given | A3 |
| R3 | a report that is not an object, or metadata that is not one, read without its guard; a boolean, a text or a negative count accepted; 0 examined not VOID; VOID exiting 0; a text that is not JSON escaping as a traceback; the advisories read without the report's guard; a report with no advisories object, or with no count, judged; a blank first line quoted; the quoted line unstripped; no output quoted as nothing | A2 |
| R3 | an advisory of no known grade passing; one at the level passing; versions joined by a space; an advisory with no findings escaping as a traceback; a clean report failing; the verdict line printed before the advisories; every advisory counted, not only the failing ones; a failed pnpm passing, its exit handed over as 0, or its exit lost; a level pnpm does not take accepted; the level, or pnpm's exit, made optional | A3 |
| R3 | pnpm's exit read as text; the classes' counts misnamed; the report never reaching the verdict | A4 |
| R4 | `python3` not checked; `python3` checked before `node` | A5 |

Seven of them survived the first cases, measured on an export of 4ad713c holding 9f79d10's test
file, and are killed by the cases 4ad713c added: the quoted line unstripped (A2's registry error
after a blank line), the space between versions (an advisory found at two versions), an advisory
with no findings, the verdict holding `low` whatever it is given (the run at each level), and a
level pnpm does not take accepted, or the level or pnpm's exit made optional (the usage errors).

One mutant was not proved, because it changes no run. Writing the level's own value, `low`, in
place of `$AUDIT_WEB_LEVEL` in either call gives the same commands; that the level is defined once
is a property of the text.

The invariants among them are also hand-proved rows in the band S05800 to S05899, which CI's
mutation-rows job proves on every pull request that touches their targets: VOID on 0 examined and
its exit, an advisory at the level and one of no known grade failing, a failed pnpm never passing,
the call naming no dependency class, the level `low`, pnpm's exit reaching the verdict, and
`python3` checked. `python3 scripts/mutation_rows.py prove --band S05800-S05899` killed all nine
at 8a03b03 (killed 9, survived 0, void 0).
