# ADR-679: a process waiter is a census at warn over scripts, workflow steps and Python

- **Status:** accepted with SPEC-657, by the pull request that adds both.
- **Decides:** SPEC-657 R1 to R12. It cites ADR-450 (D6, the kill criteria and the promotion rule
  of a census at warn) and SPEC-436 (R21, the `WARN` and `STALE` verdict), and amends neither.

## Context

A waiter that polls for a process by `pgrep -f` or `pkill -f` matches its pattern against every
process's whole command line. procps documents that the running `pgrep` or `pkill` never reports
itself, and documents nothing that leaves out the shell, the workflow step or the program that runs
it. A `bash -c 'until pgrep -f deck-api; do sleep 1; done'` holds `deck-api` in its own command
line, so it can find itself and stop waiting before the process it waits for exists. A `ps` piped
to `grep deck-api` has the same fault, which the GNU grep manual answers with the bracket form,
`grep '[d]eck-api'`, whose own command line the pattern cannot match.

Measured at dev `b9a08e60` (SPEC-657 section 1): the files read hold no `pgrep`, `pkill`,
`pidof` or psutil listing call and one `ps` call with no pattern, so the files read hold no site.

## Decision Drivers

- A rule with no site today is cheapest to hold before its first site lands.
- A census reads its whole population, prints what it examined and refuses an empty one.
- A new rule starts at warn and is promoted by ADR-450 D6's rule.
- No existing test, reader or workflow is changed.

## Decisions

**D1. One census module under `scripts/tests` (SPEC-657 R1, R12).** The census lists the tracked
files with `git ls-files` and reads every shell script, every workflow file and every Python file,
whether or not it holds a lookup today, so a lookup a later change adds is read the day it lands.

**D2. A site is a lookup whose pattern can match the command line that runs it (R3, R6, R7).**
The census reads the command line from where the lookup sits: a script's own path and arguments, a
`-c` script's shell, a step's runner shell, a Python program's path and arguments, and the grep's
own command after a `ps`. A pattern held in another expansion is recorded and not judged, so every
count is a lower bound. A lookup by process name, `pidof`, a pid file read and the bracket form are
not sites.

**D3. One shared reader for `run:` texts (R2, R8).** Workflow `run:` texts are read by
`_mutants_finder.run_texts`, the reader the mutation guards use, and the census does not import the
workflow loader of `test_ci_workflows.py`, so it adds no register entry there. A member the reader
refuses, Python that does not parse or bytes that are not UTF-8 are one `unread` site each.

**D4. Warn first, with a listing (R9 to R11).** `LEVEL` is `warn`; an unlisted site prints `WARN`
and a stale entry fails, as SPEC-436 R21. `LISTED` holds the three planted sites. Promotion to
`fail` is its own delivery under ADR-450 D6.

**D5. Planted text on the box, the tree in CI (R12).** The planted tests read fixtures and literal
texts only; no planted case is run as a process or a shell. The repository test runs in CI's
`hygiene` job.

**D6. Kill criteria.** ADR-450 D6 binds this census: live precision below 0.8 with at least five
hits, or two of its first five hits false, proposes a demotion, which is a signed ruling;
`examined 0` is VOID and blocks.

## What it was chosen against

- **A text grep for `pgrep -f`.** It cannot tell a pattern in its own command line from one that is
  not, and reads no `ps` pipeline or Python call.
- **Running each waiter.** A census that runs lookups or shells cannot run on every machine that
  runs the tests, and proves only the processes alive at that moment.
- **Importing the workflow loader.** It would add the module to the loader's read census and to the
  register of `test_ci_workflows.py` for no reading the shared run-text reader lacks.
- **Starting at fail.** A new rule starts at warn.

## Consequences

- A waiter that can match itself prints a `WARN` line in the `hygiene` job from the day it lands.
- The planted fixtures are tracked shell scripts that other censuses of shell scripts read.
- The rows band gains S65700 to S65734.

## What would make this wrong

- procps documenting that `pgrep` leaves out its parent shell: R6 (b) would then over-report a
  `-c` script's own pattern.
- A runner that runs a `run:` text with the text itself on the command line: R3's step command
  line would then under-report.
- Two of the first five hits false: D6 proposes a demotion.

## More Information

- procps `pgrep(1)`: `-f`, `-x`, `-F`, and the note that the running process never reports itself.
- GNU grep manual, Usage: the bracket form after `ps`.
