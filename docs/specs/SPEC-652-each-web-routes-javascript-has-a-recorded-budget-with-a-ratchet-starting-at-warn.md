# SPEC-652: each web route's JavaScript has a recorded budget, with a ratchet, and the check starts at warn

Path:
`docs/specs/SPEC-652-each-web-routes-javascript-has-a-recorded-budget-with-a-ratchet-starting-at-warn.md`.

- **Issue:** #1371, which this SPEC closes.
- **Context(s):** one new script and its tests, one new budget file, the web job of the CI
  workflow, the Python mutation map, the rows band and the documents. No product source, no file of
  the web app, no `scripts/check.sh` and no test-runner configuration changes.
- **Decided by:** ADR-674 (each web route's JavaScript is held to a recorded budget with a ratchet,
  and the check starts at warn). It cites ADR-450, whose promotion rule the check follows, ADR-336
  and ADR-349 (the web engine's budget and its arithmetic), and SPEC-338 and SPEC-511 (the two size
  budgets the tree holds), and amends none of them.
- **Schematic:** none: the change is one reader over one build output, which ADR-674 D1 and D2
  state in words.
- **Status:** delivered by the pull request that adds this file, with its tests and
  `docs/red-first/SPEC-652.md`. Read at dev `b9a08e60`.

## 1. The problem, measured

- **Two budgets hold the web engine, and none holds the app's routes.** `scripts/web-engine-size.py`
  holds the module a browser loads and its bindings, and with `--owner-file` the companion's module
  and weights, to ADR-336's 8000000 bytes `gzip -9 -n`, summed (`scripts/web-engine-size.py:38`;
  SPEC-338 R8, SPEC-511 R11). The figure is a constant of the script, restated apart in
  `scripts/tests/test_web_engine_size.py:23` and `scripts/tests/test_owner_bundle_size.py:21`, and
  pinned by rows S33810 and S33811. Neither reads a file of the Mini App's own build.
- **The app has 23 routes.** `git ls-tree -r --name-only b9a08e60 -- web/app/src/routes` names 25
  route files: 23 `+page.svelte`, and one `+layout.svelte` and one `+layout.ts` at the root; no
  `+page.ts`, no layout below the root and no group's folder.
- **The app is a single-page app that Vite builds.** `web/app/src/routes/+layout.ts:8-9` turns off
  server rendering and prerendering, the static adapter writes one fallback page
  (`web/app/sveltekit.config.js:16`), and `web/app/package.json:9` builds with `vite build`.
- **CI builds the app and reads nothing it built.** The web job's step `web (pnpm check with
  svelte-check --fail-on-warnings, vitest, vite build, playwright) and audit-web (pnpm audit)` runs
  `bash scripts/check.sh web audit-web` (`.github/workflows/ci.yml:278-281`), whose web stage runs
  `pnpm -r build` and then the browser tests, whose server runs `vite build` again
  (`scripts/check.sh:157-164`, `web/app/playwright.config.ts:51`). The build stays in the job's
  checkout, and no step reads it or keeps it.
- **Today's figures are owed to this delivery's own run.** No check before this one measured the
  app, so each route's figure is first printed by this delivery's first CI run, in its web job's
  `web-route-size` step, and the budget file's entries are written from that job (R11).

## 2. Requirements

R1. **One check, after the web build.** `scripts/web-route-size.py --app DIR --budgets FILE` reads
the client build under `DIR` (Vite's manifest at `.svelte-kit/output/client/.vite/manifest.json`,
the files it names under `.svelte-kit/output/client`, and the generated node each of its entries
names) and the budget file `FILE`.

R2. **The population is every page.** Each manifest entry whose source is a generated node is read
for the route file it re-exports. A node that re-exports a `+page.svelte` is a route, named by a
slash and the page's folder under `src/routes`, a group's folder included. A page under
`src/routes` with no node, two nodes for one route, and no route at all are each VOID.

R3. **A route's figure is its own static closure.** Its page's node and the node of every layout
between it and the root layout, with every chunk each imports statically, transitively. No dynamic
import is followed and no stylesheet is counted.

R4. **The shell is one entry.** Every other entry of the manifest (the client's start and app, the
root layout and the error page), with its static closure, is the entry `(shell)`, and no route's
figure counts a file of the shell.

R5. **The unit is the web engine's.** Each file is compressed by `gzip -9 -n`, measured by gzip
itself, and a figure is the sum of its files.

R6. **The budget file.** `config/web-route-budgets.json` holds `routes`, one entry per route and one
for `(shell)`, each `{"bytes": N, "job": J}`: whole bytes above zero and the CI job whose run
measured them. A key that names no route, or an entry of any other form, is VOID.

R7. **Four findings.** A route over its entry (`OVER BUDGET`), a route with no entry
(`NO BUDGET`), an entry for a route that is gone (`GONE`), and a route more than 1024 bytes under
its entry (`RATCHET`, which names the figure to lower the entry to).

R8. **The level.** `LEVEL` is `warn`: each finding prints a `WARN` line and the check exits 0. At
`fail` each prints a `FAIL` line and the check exits 1. The check moves to `fail` only by its own
delivery (ADR-674 D6).

R9. **The count, and VOID.** The check prints one line for each route and for the shell, then
`examined N route(s) and the shell`. A manifest or budget file that does not read, a manifest
entry that is not an object, a node or chunk the manifest names and does not hold, a file outside
the client folder, a missing or empty file, and a gzip that is absent or fails are each VOID, exit
2, at both levels, and never a pass.

R10. **CI runs it.** The web job runs the check right after its `check.sh web audit-web` step, over
`web/app` and `config/web-route-budgets.json`, with no `continue-on-error`.

R11. **The first figures.** This delivery's first push lands the budget file with no entry, so its
web job prints each route's figure in a `NO BUDGET` line. The second push writes one entry for each
route and the shell from that job, naming its id.

R12. **The map and the rows.** `scripts/mutation-python.json` maps the script to its test module,
and the rows of `scripts/mutation-rows.d/S65200-S65299.json` pin each arm.

## 3. Acceptance criteria of SPEC-652

| id | criterion | decided by |
|---|---|---|
| A1 | R7, R8: a route one byte over its entry prints its exact `WARN OVER BUDGET` line and the check exits 0; the same route at its entry passes | `test_web_route_size.EachRouteIsHeldToItsEntry.test_a_route_over_its_entry_is_reported_and_one_at_its_entry_is_not` |
| A2 | R7: a route with no entry prints `budget none` and its exact `WARN NO BUDGET` line | `test_web_route_size.EachRouteIsHeldToItsEntry.test_a_route_with_no_entry_is_reported` |
| A3 | R7: an entry for no route prints its exact `WARN GONE` line | `test_web_route_size.EachRouteIsHeldToItsEntry.test_an_entry_for_a_route_that_is_gone_is_reported` |
| A4 | R7: a route one byte more than the margin under its entry prints its exact `WARN RATCHET` line naming its figure; a route the margin under it passes | `test_web_route_size.EachRouteIsHeldToItsEntry.test_a_route_past_the_margin_under_its_entry_asks_the_entry_lowered` |
| A5 | R2 to R5: over a planted build with a root page, a layout below the root, a group's folder, an import cycle, a dynamic import and a stylesheet, each route's and the shell's file count and figure equal the rule's, summed in the test from gzip's own sizes, and the examined count prints | `test_web_route_size.EachRouteIsHeldToItsEntry.test_a_routes_figure_is_its_static_closure_beyond_the_shell` |
| A6 | R2, R6, R9: each planted build and budget file of the test's VOID cases prints its exact VOID reason, exits 2 and prints no `PASS` | `test_web_route_size.EachRouteIsHeldToItsEntry.test_a_build_or_table_that_does_not_read_or_holds_no_route_is_void` |
| A7 | R8: `LEVEL` is `warn`; a finding fails at `fail` with its `FAIL` line and passes at `warn`; no finding passes at `fail` | `test_web_route_size.TheLevelDecidesWhetherAFindingFails.test_a_finding_fails_at_the_fail_level_and_passes_at_warn` |
| A8 | R1: the help names the check and its two options, each option is required, and loading the check turns bytecode writing off | `test_web_route_size.TheCommandLineStatesItself.test_the_help_names_the_check_and_both_options_are_required`, `test_web_route_size.TheCommandLineStatesItself.test_loading_the_check_turns_bytecode_writing_off` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_web_route_size.py -k test_a_route_over_its_entry_is_reported_and_one_at_its_entry_is_not
A2: python3 -m unittest discover -s scripts/tests -p test_web_route_size.py -k test_a_route_with_no_entry_is_reported
A3: python3 -m unittest discover -s scripts/tests -p test_web_route_size.py -k test_an_entry_for_a_route_that_is_gone_is_reported
A4: python3 -m unittest discover -s scripts/tests -p test_web_route_size.py -k test_a_route_past_the_margin_under_its_entry_asks_the_entry_lowered
A5: python3 -m unittest discover -s scripts/tests -p test_web_route_size.py -k test_a_routes_figure_is_its_static_closure_beyond_the_shell
A6: python3 -m unittest discover -s scripts/tests -p test_web_route_size.py -k test_a_build_or_table_that_does_not_read_or_holds_no_route_is_void
A7: python3 -m unittest discover -s scripts/tests -p test_web_route_size.py -k test_a_finding_fails_at_the_fail_level_and_passes_at_warn
A8: python3 -m unittest discover -s scripts/tests -p test_web_route_size.py -k test_the_help_names_the_check_and_both_options_are_required
A8: python3 -m unittest discover -s scripts/tests -p test_web_route_size.py -k test_loading_the_check_turns_bytecode_writing_off
```

Every planted build, budget file and expected line is text the test spells literally, and every
expected figure is summed in the test from gzip's own sizes by the rule the test writes, never read
from the check. Each test asserts its positive artifact (the finding's line, the route's line, the
VOID reason) before it reads an exit code.

## 4. File manifest

| path | part | change |
|---|---|---|
| `scripts/web-route-size.py` | the check | added: R1 to R9 |
| `scripts/tests/test_web_route_size.py` | its tests | added: A1 to A8 |
| `config/web-route-budgets.json` | the budget file | added: R6, its `routes` empty until the second push (R11) |
| `.github/workflows/ci.yml` | the web job | changed: R10's step, insert-only |
| `scripts/mutation-python.json` | the Python mutation map | changed: R12's entry, insert-only |
| `scripts/mutation-rows.d/S65200-S65299.json` | the rows | added: S65200 to S65220 (section 9) |
| `docs/specs/SPEC-652-each-web-routes-javascript-has-a-recorded-budget-with-a-ratchet-starting-at-warn.md` | the spec | added |
| `docs/decisions/ADR-674-each-web-routes-javascript-is-held-to-a-recorded-budget-with-a-ratchet-and-starts-at-warn.md` | the decision | added |
| `docs/red-first/SPEC-652.md` | the record | added |
| `changelog.d/web-route-budgets-652.md` | the fragment | added |

## 5. What this does NOT cover

- It does not fail CI on a finding: the check moves to `fail` only by its own delivery, under
  ADR-674 D6, #1399.
- It does not pin the web job's step in a workflow test; that pin comes with the move to `fail`,
  #1399.
- It does not count a chunk a route loads by a dynamic import, or the engine Worker's own bundle,
  which no static import in the manifest reaches, #1400.
- It does not count stylesheets, #1401.
- It changes no budget in force: the web engine's budget, its tests and its rows stay as they are,
  and no rule is lowered, #913.

## 6. Risks

- **The client build's form differs from the form this SPEC reads.** Vite's documentation at the
  pinned version gives the manifest's form: one entry per chunk, keyed by its source path, naming
  its `file`, its static `imports` and `dynamicImports` as manifest keys, and its `css`. Where the
  pinned SvelteKit leaves the manifest and its generated nodes was not read from its documentation.
  A different form reads VOID by name in this delivery's first CI run, never a silent pass, and the
  build corrects `CLIENT`, `MANIFEST` or `NODE` before its second push.
- **A tree-scanning guard reads the new files.** `scripts/tests/test_mutation_python.py` holds every
  `scripts/*.py` in the map (R12); `scripts/tests/test_temp_hygiene.py` reads every Python test, and
  these use `tempfile.TemporaryDirectory` alone; `scripts/tests/test_ci_workflows.py`'s census of
  dynamic imports reads every test module, and these load the check through
  `test_mutation_python_judge_kills.load`, as `scripts/tests/test_companion_site_census.py` does,
  so they add no site of their own; the format bar reads every Python file; the scrub and secrets
  stages read every added line. Each runs in the pull request's CI, in the `hygiene` job.
- **A mutant loops the closure's walk.** The planted build holds an import cycle, so a mutant that
  drops the walk's seen set never ends; every command the tests start carries a timeout, so that
  mutant fails its killer by the timeout.
- **gzip differs between machines.** Every entry's figure is the CI runner's, named by its job, and
  no figure from another machine is written.
- **A sibling change moves a route's figure.** Detected by the check itself at the merge, at warn.

## 7. What only CI proves

- The check over the real build: the web job's `web-route-size` step prints one line for each route
  and the shell and `examined 23 route(s) and the shell` at the first push, read by the job's name,
  `web`, and its id; the second push's run of the same step reads `PASS`.
- The python stage's run of the new module and of every tree-scanning guard (`hygiene`), the rows
  (`mutation-rows`), the diff's Python mutants (`mutation-python`), the verdict
  (`mutation-verdict`), the fragment (`fragment`) and `ci`.

## 8. Formal model

No state, actor or write path is added, and no covered span is edited: of the `@phx covers` lines
at `b9a08e60` that name a path under `scripts/`, `.github/` or `config/`, none names a file this
delivery edits. The check is a pure reader of a build's files; its classification is closed by the
planted builds of A5 and A6, whose expected lines are spelt literally, and by section 9's rows. No
model checker runs.

## 9. Mutation rows (S65200-S65299)

Rows S65200 to S65220, in `scripts/mutation-rows.d/S65200-S65299.json`, table
`SCRIPT_MUTATIONS`, each on `scripts/web-route-size.py` and killed by a test of
`scripts/tests/test_web_route_size.py` named by its full id. Each find occurs exactly once in the
file.

| rows | what each mutant removes or breaks | killer |
|---|---|---|
| S65200, S65201 | the over-budget arm, and its edge at the entry | A1 |
| S65202 | a route with no entry, read as an entry of zero bytes | A2 |
| S65203 | the finding for an entry whose route is gone | A3 |
| S65204, S65205 | the ratchet's edge at the margin, and the margin | A4 |
| S65206 to S65211 | the transitive walk; a dynamic import followed; a stylesheet counted; the shell's files counted in a route; a layout below the root left out; the root layout read as a route's layout | A5 |
| S65212 to S65218 | the empty population; a page with no node; a missing file; an empty file; a file outside the client folder; an absent gzip; an entry measured by job 0 | A6 |
| S65219, S65220 | the warn level; the fail level's exit | A7 |
