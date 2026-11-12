# SPEC-657: a process waiter never matches its own command line

Path: `docs/specs/SPEC-657-a-process-waiter-never-matches-its-own-command-line.md`.

- **Issue:** #1378.
- **Context(s):** `scripts/tests` (one census module and its planted fixtures), the rows band and
  the documents. No workflow, `scripts/check.sh`, shell script, product source or test-runner
  configuration changes, and no existing test is edited.
- **Decided by:** ADR-679 (a process waiter is a census at warn over scripts, workflow steps and
  Python). It cites ADR-450 (D6, the kill criteria and the promotion rule of a census at warn) and
  SPEC-436 (R21, the `WARN` and `STALE` verdict), and amends neither.
- **Schematic:** none; the census is one module, and section 2 states its flow (population, member
  reader, lookup classifier, keys, verdict).
- **Status:** delivered by the pull request that adds this file, with its tests and
  `docs/red-first/SPEC-657.md`. Read at dev `b9a08e60`.

## 1. The problem, measured

`pgrep -f` and `pkill -f` match a pattern against each process's whole command line, and the
running `pgrep` or `pkill` never reports itself (procps, `pgrep(1)`). Nothing in that page leaves
out the shell, the workflow step or the program that runs the lookup, so a waiter whose pattern
occurs in that command line can find itself and stop waiting at once. A `ps` output piped to `grep`
has the same fault: the grep's own command line holds the pattern, so its own line matches unless
the pattern is written so it cannot match itself, as `[c]ron` is (GNU grep manual, Usage).

Measured at dev `b9a08e60` by reading the tracked files' text, every count a lower bound:

- The population read: 17 shell scripts (15 `.sh` files and 2 files with no extension and a
  `bash` shebang), 18 workflow files with about 287 `run:` texts, and 235 Python files. Some
  files of the population were not read by this measurement; they stay in the census's population,
  so a site there prints at warn, unlisted, and this SPEC claims no count over the whole
  population.
- Lookups: no `pgrep`, `pkill`, `pidof` or psutil listing call in the files read, and one `ps`
  call by argument list (`deploy/host-scrub/inventory.py`, line 100), which names no pattern and
  pipes to no `grep`. Sites: none in the files read.
- Every file read decodes as UTF-8, every Python file read parses, and no `run:` text read holds
  an expression other than a matrix value, so no member read is unread today.

So the listing at the cut holds only the planted fixtures' three sites, and the repository test
(A8) prints a `WARN` line for any site in a file this measurement did not read.

## 2. Requirements

- **R1. The population.** Every file `git ls-files` lists that is a `.sh` file, a file with no
  extension whose first bytes are a `sh` or `bash` shebang, a `.yml` or `.yaml` file directly
  under `.github/workflows`, or a `.py` file. The census prints `examined N file(s)` and refuses
  an empty population (VOID), never passing it.
- **R2. Reading shell text.** A logical line joins a quoted newline and a backslash-newline and
  drops a comment; each pipeline is split by `shlex`; a command's program is the first of its
  words that names a lookup (`pgrep`, `pkill`, `pidof`, `ps`) or a shell (`bash`, `sh`), so a
  wrapper and a keyword are read through; a `$( )` substitution is read as a command; a shell's
  `-c` script is read as shell text. A workflow's `run:` texts are read by the shared run-text
  reader of `scripts/tests/_mutants_finder.py`.
- **R3. The command line that runs a lookup.** A script's is `bash <path>` with its own
  arguments; a `run:` text's is the runner's `bash --noprofile --norc -eo pipefail`, whose
  arguments are not the text's own; a `-c` script's is its shell's whole command, its own
  arguments included; a Python command string handed to a process call is read as shell text run
  by `sh -c <text>`; a Python argument list's is `python3 <path>` with its own arguments
  (`sys.argv`).
- **R4. Reading Python.** A call named `run`, `Popen`, `call`, `check_call`, `check_output`,
  `getoutput` or `system` whose first argument is a string (read as R2) or a list or tuple whose
  first item names a lookup; and `psutil.process_iter` and `psutil.pids`, by that receiver only,
  recorded as lookups and never as sites.
- **R5. The lookup's form.** `pgrep` and `pkill` options are read as procps defines them: `-f`
  and `--full`, `-x` and `--exact`, `-F` and `--pidfile`; an option that takes a value skips it,
  a signal option is skipped, and `--` ends the options. The first other word is the pattern.
- **R6. A site.** (a) A `pgrep -f` or `pkill -f` whose pattern is a positional parameter of a
  command line whose arguments are its own (`${1}`, `"$@"`, `sys.argv[...]`); (b) one whose literal
  pattern, read as an extended regular expression, matches the command line that runs it, or with
  `-x` the whole of it; (c) a `ps` piped to a `grep` that selects by a literal pattern matching the
  grep's own command line or the command line that runs it. A pattern Python's engine refuses is
  read as fixed text.
- **R7. Not a site.** A pattern that cannot match its own command line (the bracket form, or one
  absent from it); a lookup by process name (`pgrep` or `pkill` without `-f`, with or without
  `-x`); `pidof`; a `ps` with no selecting `grep`; a pid file read by `-F`; a psutil listing; and a
  pattern that holds any other expansion (`$NAME`), which is recorded and not judged.
- **R8. An unread member.** A workflow the shared reader refuses, a Python file that does not
  parse, or a file that is not UTF-8 is one `unread` site, reported and never passed.
- **R9. Keys.** A site's key is `<path> <form>-own-command-line #<n>`, `n` its ordinal among that
  path's sites of that form; an unread member's is `<path> unread #1`.
- **R10. Verdict and level.** `LEVEL` is `warn`. Each unlisted site prints `WARN process-waiter
  <key>` and fails only at `fail`; each listed entry no site matches prints `STALE process-waiter
  <key>` and fails at both levels, as SPEC-436 R21 does.
- **R11. The listing.** `LISTED` holds the three planted sites, each with its reason. Moving
  `LEVEL` to `fail` is its own delivery, under ADR-450 D6's rule.
- **R12. Where it runs.** The planted tests read text only: they run no process, no shell and no
  lookup. The repository test lists the tracked files with `git ls-files`, reads the whole
  population and runs in CI's `hygiene` job.

## 3. Acceptance criteria of SPEC-657

| id | criterion | decided by |
|---|---|---|
| A1 | R1: the planted tree's population is its workflow, its six shell scripts (one by its shebang) and its Python file, and not its prose file | `test_a_planted_tree_counts_each_script_step_and_python_file` |
| A2 | R2, R3, R5, R6, R9: the planted sites (a `bash -c` in a script and in a workflow step, an own argument, a `ps` piped to a plain `grep`) are found with their keys | `test_a_planted_lookup_that_can_match_its_own_command_line_is_a_site` |
| A3 | R2, R5, R7: the planted clean lookups (the bracket form, `-x`, a name, `pidof`, `-F`, a pattern not in the command line, a substitution, an expansion, a comment) are found and are not sites | `test_a_planted_lookup_that_cannot_match_itself_is_clean` |
| A4 | R3, R4, R6, R7: a planted Python text's argument lists, command strings and psutil listings are read, and another receiver's `pids()` is not | `test_the_planted_python_lookups_are_read_by_their_argument_list_and_command_text` |
| A5 | R10: the level is `warn`; an unlisted site prints its `WARN` line, passing at warn and failing at fail; a stale entry prints its `STALE` line and fails | `test_the_planted_level_warns_on_an_unlisted_site_and_fails_on_a_stale_entry` |
| A6 | R1: an empty planted population fails naming `examined 0 file(s)` | `test_a_planted_empty_population_is_void` |
| A7 | R8, R9: a workflow the reader refuses, Python that does not parse and bytes that are not UTF-8 are each one `unread` site with its key | `test_a_planted_member_the_census_cannot_read_is_an_unread_site` |
| A8 | R1, R11, R12: over the repository, no listed entry is stale, the level is `warn`, and each unlisted site prints its `WARN` line | `test_the_repository_holds_exactly_the_listed_sites_at_warn` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_process_waiter_census.py -k test_a_planted_tree_counts_each_script_step_and_python_file
A2: python3 -m unittest discover -s scripts/tests -p test_process_waiter_census.py -k test_a_planted_lookup_that_can_match_its_own_command_line_is_a_site
A3: python3 -m unittest discover -s scripts/tests -p test_process_waiter_census.py -k test_a_planted_lookup_that_cannot_match_itself_is_clean
A4: python3 -m unittest discover -s scripts/tests -p test_process_waiter_census.py -k test_the_planted_python_lookups_are_read_by_their_argument_list_and_command_text
A5: python3 -m unittest discover -s scripts/tests -p test_process_waiter_census.py -k test_the_planted_level_warns_on_an_unlisted_site_and_fails_on_a_stale_entry
A6: python3 -m unittest discover -s scripts/tests -p test_process_waiter_census.py -k test_a_planted_empty_population_is_void
A7: python3 -m unittest discover -s scripts/tests -p test_process_waiter_census.py -k test_a_planted_member_the_census_cannot_read_is_an_unread_site
A8: python3 -m unittest discover -s scripts/tests -p test_process_waiter_census.py -k test_the_repository_holds_exactly_the_listed_sites_at_warn
```

Every planted case is text the test or a fixture spells literally, and every expected lookup, key
and line is a literal the test writes, never a value the census computes.

## 4. File manifest

| path | part | change |
|---|---|---|
| `scripts/tests/test_process_waiter_census.py` | the census | added: R1 to R12, A1 to A8 |
| `scripts/tests/fixtures/process-waiters/tree/` | the planted tree | added: one workflow, five `.sh` scripts, one script with a shebang and no extension, one Python file and one prose file |
| `scripts/tests/fixtures/process-waiters/empty/notes/README` | the empty population | added |
| `scripts/mutation-rows.d/S65700-S65799.json` | the rows | added (section 9) |
| `docs/specs/SPEC-657-a-process-waiter-never-matches-its-own-command-line.md` | the spec | added |
| `docs/decisions/ADR-679-a-process-waiter-is-a-census-at-warn-over-scripts-steps-and-python.md` | the decision | added |
| `docs/red-first/SPEC-657.md` | the record | added |
| `changelog.d/process-waiter-census.md` | the fragment | added |

## 5. What this does NOT cover

- It does not cure a site: a listed site stays until its own change, #1407.
- It does not follow a pattern held in a variable, a function argument or a file, or a lookup run
  through `eval`, a shell other than `sh` and `bash`, or a helper (R7's lower bound), #1407.
- It does not read a `.bash` file or a script with an `sh` shebang spelt another way, #1407.
- It does not count `grep -v grep` after a `ps` pipeline as a cure, #1407.
- It does not move `LEVEL` to `fail`.

## 6. Risks

- **A site this SPEC's measurement missed.** The repository test reads the whole population and
  prints each unlisted site; a site the build's base adds is listed with its reason or cured,
  never silently passed.
- **A tree-scanning guard reads the new fixtures.** The planted scripts are tracked shell
  scripts; a census over every tracked shell script reads them, and the delivery that lands
  second re-reads them at its base.
- **The shared reader refuses a workflow.** The member is one `unread` site, printed at warn.
- **A mutant hangs a reader.** Each loop advances by its own index over a finite text.

## 7. What only CI proves

- A8 reads the tracked tree, so its result is read in the pull request's `hygiene` job.
- The rows are read by the `mutation-plan`, `mutation-python`, `mutation-rows` and
  `mutation-verdict` jobs.

## 8. Formal model

No state, actor or write path is added: the census is a pure reader of committed text, closed by
the seven planted tests' cases, whose expected values are spelt literally, and by section 9's rows.
No model checker runs.

## 9. Mutation rows (S65700-S65799)

Rows S65700 to S65734, in `scripts/mutation-rows.d/S65700-S65799.json`, each on
`scripts/tests/test_process_waiter_census.py`. Each find is taken from the green commit's text and
occurs exactly once in the file, and each mutant still parses.

| rows | what each mutant removes or breaks | killer |
|---|---|---|
| S65700, S65707, S65708, S65714, S65725, S65726, S65733, S65704, S65705 | `-f`, an own argument, a `-c` script's command line, a workflow's run texts, a pipe, the site ordinal, a script's own arguments, `-fx` as a whole match, the grep's own line | A2 |
| S65701, S65702, S65706, S65715, S65720, S65721, S65723, S65724 | `-x`, `-F`, `grep -v`, a substitution, `pidof`, a lookup by name, a comment, a long option's value | A3 |
| S65703, S65713, S65718, S65719, S65722 | the fixed-text reading, the psutil receiver, an argument list, a command string's shell, an expansion | A4 |
| S65711, S65712, S65727, S65728, S65729 | failing at fail, a stale entry, a `WARN` line, a `STALE` line, the level | A5 |
| S65709, S65730, S65731, S65732 | a shebang, a `.sh` file, a Python file, a workflow file | A1 |
| S65710 | the empty population's refusal | A6 |
| S65716, S65717, S65734 | an unread member, its key, bytes that are not UTF-8 | A7 |
