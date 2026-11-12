---
status: accepted
date: "2026-09-29"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# A tool the mutation runner cannot run is a refusal, decided once before each spawn

## Context and Problem Statement

`scripts/mutation_rows.py prove` spawns `git`, the interpreter, `cargo`, `bash` and `sh`, and the
`retired` verb spawns `git`. When
`cargo` was not on `PATH`, the cargo killer's spawn raised an uncaught `FileNotFoundError`, and the
process ended with exit 1, which is `EXIT_SURVIVED` (#431). A check that could not start read as a
surviving mutant. The same fact, a tool that cannot be run, is met at every spawn under every verb
and in six shapes: absent from `PATH`, present but not executable, a directory at the name, a script
whose interpreter line names a missing program, an empty file with the execute bit, and a wrapper
whose program is missing (it starts and exits 127, or 126 behind a `PATH` entry that cannot be
searched).
Which exit code and which one place decide it for all of them, and does a missing shell parser,
which section 12 (A41) reads as VOID, belong to the same rule?

## Decision Drivers

- One class, one rule: a fix at the spawn the issue names would leave the other four spawns and two
  unrunnable modes reading as they did.
- A refusal must not be mistaken for a survivor, and must not be read as a pass by any reader of the
  exit code.
- A mutant that was installed must be restored by digest on every way out.
- No weakening: no test, row, timeout or verdict is removed or narrowed.

## Considered Options (the alternatives it was chosen against)

- Resolve in one place before each spawn, map to a named refusal, `main` prints it — chosen: one check covers every spawn, mode and verb.
  `main` alone prints the line and exits 2 (`EXIT_REFUSED`), and a census of the module's own
  source refuses a spawn that skips the check, by any name the standard library gives a spawner.
- Catch `FileNotFoundError` at the one cargo site only — lost: it fixes the reported spawn and
  leaves `git`, the interpreter, the build check and both parsers as tracebacks, and it misses a
  file without the execute bit (`PermissionError`) and a directory at the name (`PermissionError`
  too), which are not `FileNotFoundError`.
- A `shutil.which` pre-check at the top of `prove` only — lost: a tool that is found at the start
  can be gone when a later spawn runs, it says nothing about the interpreter of a script, and
  `retired` and any later verb would need their own copy; it is kept as the resolution, but inside
  the spawn helper, with the spawn's own failure mapped to the same refusal as a backstop.
- Map the crash to `EXIT_VOID` (3) — lost: VOID writes a report with a VOID row, which the verdict
  reads as a failed row (`judge_rows` reads a report entry that is not KILLED as `verdict.fail`),
  and the runner never examined the row, so the honest reading is the refusal it already gives an
  unreadable population.
- Keep VOID for a missing parser and refuse only the killer tools — lost: `parses` runs `bash -n`
  or `sh -n` so the runner can check a mutant, and a shell that cannot be run is the same fact as
  a missing `cargo`. Two rules for the same fact would need a reader to know which spawn a message
  came from. A41's other readings stay: a parse check that outlives its bound is VOID, and a mutant
  that does not parse is VOID.
- Enumerate the dynamic spellings the census will read (`getattr(m, "run")`, `__import__("os")`) — lost: each new spelling is a new way to skip the check, and a list is one behind the language. The census refuses the ways of reaching a spawner by a name built at run time instead (`getattr`, `__import__`, `importlib`, `vars`/`globals`/`locals`, `__dict__` and `sys.modules`, `eval`, `exec`), whatever they are given, so a spelling not yet invented is refused by default.
- Resolve a relative `PATH` candidate in the runner's own directory, as the first two rounds did — lost: the spawn reads the candidate in the directory the child runs in, so the file judged and the file run could differ. The resolver takes the child's `cwd` and reads each relative candidate there, and both spawn helpers pass the `cwd` they spawn with.
- Census only the spellings already seen, or only the runner's own imports — lost: a spawner reached by reference (an alias, `partial`, `attrgetter`, a subclass, a default argument, another module's `os`) or through an import the census has not read is a spawn no list of spellings covers. The census refuses the reference and the unread import, and derives the modules it reads from the runner's own imports.
- Assert only that a refusal happened, or that its line holds a word of the reason — lost: CI
  generated 14 surviving mutants of the refusal's text and branches (round 1). Every refusal is
  compared whole, over populations read from the operating system's own tables (every errno, every
  exit).

## Decision Outcome

Chosen option: "resolve in one place, refuse by name, exit 2", because it is the only option that
makes the rule a property of the runner rather than of a call site, and because exit 2 is the code
the runner already gives every other input it cannot examine.

- `run_tool` wraps every `subprocess.run`, and `run_in_own_group` resolves before its `Popen`. Both
  raise `ToolMissing(tool, why)` for an executable that is absent, not executable, or a directory.
  A candidate along `PATH` that cannot be looked at is passed over, as the spawn's own search passes
  over it. A spawn that still fails for its executable after resolution passed (an interpreter line
  naming a program that does not exist, an empty file the kernel will not execute) maps every
  `OSError` to the same refusal, unless the error names the working directory, and an exit of 126 or
  127 from the spawned tool is read as the refusal too (`cannot be run`, `is not found`).
- Resolution is against the child's `PATH`, and a relative candidate (an empty entry, `.`, a relative entry, a name holding a `/`) is read in the directory the child runs in: `resolve_tool(command, env, cwd)`, with `cwd` passed by `run_tool` and `run_in_own_group`. The tool judged is the tool run.
- `main` alone catches `ToolMissing` under `prove` and `retired` and prints one line,
  `<verb>: REFUSED: missing tool: <name as spawned>: <why>`, then returns `EXIT_REFUSED`.
  `prove_row`'s `finally` restores the target, so the digest is unchanged.
- `parses` no longer returns "is not installed"; a shell that cannot be run is a `ToolMissing`.
  This supersedes A41's missing-parser clause only. SPEC-039 stays insert-only: A41's text on dev
  is not edited, and the amendment (sections 30 and 31) says what it supersedes.

**Who reads exit 2, measured** (every consumer of the runner's exit code):

- `.github/workflows/ci.yml` runs `prove --rows-from ... || rc=$?` and prints the code; the report
  the verdict reads is written only at the end of a `prove`, so an exit 2 leaves none.
- `mutation-verdict.py` `judge_rows` reads a selected row set with no report as
  `verdict.void("... selected and no rows report")`, and the battery check reads a missing
  `rows.json` as "MISSING rows". A refusal is therefore a VOID at the verdict, never a pass.
- `.github/workflows/ci.yml` also runs `python3 scripts/mutation_rows.py retired --base HEAD^1` as a
  step of its own, with no `|| rc=$?`, so the refusal's exit 2 fails that step.
- `.github/workflows/mutation-weekly.yml` runs `prove --all` and `prove --row ...` as steps, so a
  non-zero exit fails the step.

No consumer reads 2 as a pass, a skip or a usage error to ignore.

### Consequences

- Good, because a missing tool can no longer read as a survivor, and it reads the same under every
  verb, every spawn and every mode.
- Good, because a later change that lets the rows leg skip its toolchain when no selected row
  needs cargo can rely on the refusal.
- Bad, because a leg whose `bash` is missing now refuses where it used to report a VOID row; the
  verdict reads both as not passing, and the refusal names the tool.

### Confirmation

`scripts/tests/test_mutation_rows_missing_tool.py` generates its population: the spawn sites are
read from the module's source with `ast` (a new spawn joins the population and fails the census
until a scenario covers it), the census reads every spawner name of the `subprocess`, `os`, `pty`
and `asyncio` documentation and every import that reaches one, and each member is one site, tool,
mode (absent, not executable, directory, a script whose interpreter line names a missing program,
an empty file, a wrapper whose program is missing), position in `PATH` and verb, run as a child
process in a temporary repository. It asserts exit 2, one line naming the tool and the reason, no
traceback, no verdict line, and the target's digest and the tree's state unchanged. Rows S03960
to S03982 pin the resolution, each spawn, the mapping, the exits and the census.

Round 2 adds `scripts/tests/test_mutation_rows_refusal.py`, whose members are every reason, every
errno of `errno.errorcode` and every exit from 0 to 255 at every spawn route, each asserting the
whole `ToolMissing`; it is the first module the mutation map runs for the runner. The one
mutant no test could tell from the original, `replace "." with "" in resolve_tool`
(`Path("") == Path(".")`), was chosen against recording it as equivalent: the runner reads
`Path(part)` and the mutant no longer exists. The census refuses dynamic reach (SPEC-039 A74).

Round 3 states the class as one rule: the tool the runner judges is the tool the spawn runs, and every refusal is one whole outcome, for every spawn route. Its tests generate 384 child-directory members and 768 whole-outcome members; the census reads 442 spawner-reference members and 568 unread-import members and each is refused, so its escape population of 1010 has 0 escapes as the builder reported; no independent verify measured this population (SPEC-039 A75 to A77). Every generated mutant of the runner's resolution and spawn helpers (136) is red by assertion except 1 equivalent, and of the census (88) all but 3 equivalent are red by assertion, with none red by an error alone.

Round 4 closes the last route by which the judged file and the run file could differ. The resolver
returns the judged file and `run_tool` and `run_in_own_group` spawn it with `executable=` that
path, so a candidate the kernel refuses (a bad interpreter line, an empty or unknown-format file, an
interpreter without the execute bit) is one whole refusal and never a run of a later copy on
`PATH`. It was chosen against keeping the spawn's own `PATH` search and mapping its errors: that
search continues past the refused candidate, so the error it reports belongs to a file other than
the one that ran. The census refuses a name that reaches what it has not read (a module, a private
name, a frame's tables, the dunders of the class graph) and reads an annotation that holds code as
code. The scope is stated: a `PATH` that changes between the judge and the spawn is not closed,
because the runner has no writer between the two statements (#431).

## More Information

Issue #431. SPEC-039 sections 30 and 31 (A66 to A69), which amend section 12 (A41). ADR-057,
ADR-073 and ADR-196 decide the runner's other exit codes and its process group.
