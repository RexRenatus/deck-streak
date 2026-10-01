# SPEC-129: a package dispatch is sharded by its projected weight, so a small package does not build thirty-two baselines

- **Wave:** W4. **Issue:** #368. **Context(s):** `repo` (`.github/workflows/mutation-weekly.yml`,
  `scripts/mutation-verdict.py`, their tests and `docs/`).
- **Decided by:** ADR-129 (this SPEC's own: size the dispatch from its listing, and what that was
  chosen against), SPEC-057 R14, R18 and R22 (the package dispatch, the projection and the listing;
  it takes a dated amendment) and ADR-057 D7.
- **Status:** delivered. It holds `docs/red-first/SPEC-129.md`.

## 1. The problem, measured

- **A dispatch that names a package still fans out to 32 legs.** `mutation-weekly.yml`'s `rust` job
  has a fixed matrix of shards 0 to 31 and passes `--shard <i>/32` to `cargo mutants`, with or
  without `--package`. Every leg checks out, installs the toolchain and the tools, restores the
  cache and builds and tests the unmutated baseline before its first mutant (SPEC-057 R18's
  `BASELINE_SECONDS`).
- **A small package leaves most legs with nothing to do but the baseline.** Shard `k` of a
  round-robin sweep holds a mutant only when the scope lists more than `k`, so a package of a few
  dozen mutants gives the rest of the 32 legs a baseline and no mutant. The battery already knows
  this: it owes a report only from the shards the scope's listing gave a mutant.
- **The per-pull-request plan sizes its legs and the dispatch does not.** `mutation-verdict.py
  shards` projects each shard's seconds from the listing (`projected`, `SECONDS_PER_MUTANT`,
  `BASELINE_SECONDS`) and writes the fewest round-robin shards whose slowest is within
  `SHARD_BOUND_SECONDS` as the matrix (SPEC-057 R18). The weekly dispatch has the listing and does
  not use it.

## 2. Requirements

R1. **A dispatch that names a package is sized before its legs run.** A `size` job, before `rust`,
lists that package's mutants with the command the `listing` job runs plus `--package` (the same
`--no-shuffle --list --json --in-place --timeout 300 --build-timeout 600`, the same pinned
cargo-mutants and test tool), and runs `mutation-verdict.py size` on the listing.

R2. **The count is the fewest round-robin shards whose slowest is projected within the bound, by
the plan's own function.** `size` and `shards` share one function that returns that count, over
the same `projected`, `SECONDS_PER_MUTANT` (a package the table does not name costs the table's
highest), `BASELINE_SECONDS` and `SHARD_BOUND_SECONDS`. `size` writes `shards=<n>` and
`matrix=<[0..n-1]>` to the step's outputs.

R3. **n is at least 1 and at most `MAX_SHARDS`, and a projection past the limit is refused, never
capped.** A package that lists no mutant is sized to 1. A listing that needs more than `MAX_SHARDS`
shards within the bound makes `size` exit 1 with `REFUSED`, the mutant count and the projected
serial seconds, and write no output, as `shards` does. A file that is not a cargo-mutants listing
is `VOID` (exit 3) and writes no output.

R4. **A scheduled run and a dispatch with no package keep 32.** `size` with no `--package` reads no
listing and writes `shards=32` and the matrix `[0..31]`. A dispatch naming `miniapp` runs no rust
leg and sizes as no package.

R5. **The legs, their `--shard` argument and the battery read the one count.** The `rust` job
`needs: size`, its matrix is `fromJSON(needs.size.outputs.matrix)`, its step `env:` sets `SHARD`
from `matrix.shard` and `SHARDS` from `needs.size.outputs.shards`, and its command passes
`--shard "$SHARD/$SHARDS"`. The survivors job, which needs `size`, sets `SHARDS` the same way and
runs `mutation-verdict.py battery --reports "$reports" --shards "$SHARDS" …`. No `${{ }}` expression is interpolated into a `run:` block, and
no `--shard` or `--shards` argument of a rust or survivors command is a literal number. The
`--timeout 300`, `--build-timeout 600`, `--in-place` and test tool of every leg are unchanged, and
every `cargo mutants` command line of every workflow file carries `--timeout 300 --build-timeout
600` literally: the bounds are read as whole values (`6000` is not `600`), a command continued
with `\` is read as one line, and a bare `cargo mutants` counts.

R6. **The battery refuses a report set whose shard count differs from n.** A missing report is
already `MISSING`; a report `mutants-shard-<k>` with `k` at least n is now `FOREIGN` and fails the
battery, so a set produced at another count is never read as n.

R7. **The examined total equals the package's listing at the same commit.** The sized shards take
every listed mutant exactly once, as any round-robin count does (mutant `i` in shard `i mod n`), so
the union over n shards is the union over 32. The survivors job's `table --package <name> --listed
"$reports/listing/whole.json"` (`table_rust`) already refuses the run when it does not hold: a
listed mutant no report tested is `VOID` (`never tested: <name>`), and one tested more or fewer
times than it is listed is a failure. The delivery adds no second check of it.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | a fixture listing of a small package sizes to 1, larger ones to the fewest shards within the bound, the count equals `shards`' on the same listing, a projection past the limit is refused with its projection and writes nothing, and an empty or unreadable listing is sized to 1 or VOID | `test_dispatch_shards.py` `TheDispatchIsSizedFromItsListing` |
| A2 | with no package `size` reads no listing and writes 32 and the matrix 0 to 31 | `test_dispatch_shards.py` `TheWholeTreeKeepsThirtyTwo` |
| A3 | the rust matrix, its `--shard` argument and the battery's `--shards` read the size job's one count, no fixed count remains, and a plant that puts 32 back into any of the three goes red | `test_dispatch_shards.py` `TheWorkflowReadsTheOneCount` |
| A4 | a report set beyond n fails the battery as `FOREIGN` and a set of exactly n passes | `test_dispatch_shards.py` `TheBatteryRefusesAForeignShardCount` |
| A5 | the sized shards' mutants are the same set, and the same count, as the 32 shards' | `test_dispatch_shards.py` `TheExaminedTotalIsTheListing` |
| A6 | every `cargo mutants` command line in every workflow file, at least one in each of `ci.yml` and `mutation-weekly.yml`, carries `--timeout 300 --build-timeout 600` literally, read as whole values, with a `\`-continued command read as one line and a bare `cargo mutants` counted | `test_dispatch_shards.py` `EveryMutationCommandKeepsTheGatesBounds` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_dispatch_shards.py -k test_a_small_package_takes_one_shard_and_a_large_one_the_fewest_within_the_bound
A1: python3 -m unittest discover -s scripts/tests -p test_dispatch_shards.py -k test_a_projection_past_the_limit_is_refused_with_its_projection_never_capped
A2: python3 -m unittest discover -s scripts/tests -p test_dispatch_shards.py -k test_no_package_is_thirty_two_shards_whatever_the_listing
A3: python3 -m unittest discover -s scripts/tests -p test_dispatch_shards.py -k test_the_matrix_the_argument_and_the_battery_read_the_sized_count
A3: python3 -m unittest discover -s scripts/tests -p test_dispatch_shards.py -k test_a_plant_that_puts_the_fixed_count_back_goes_red
A4: python3 -m unittest discover -s scripts/tests -p test_dispatch_shards.py -k test_a_report_beyond_the_count_fails_the_battery
A5: python3 -m unittest discover -s scripts/tests -p test_dispatch_shards.py -k test_the_union_of_the_sized_shards_equals_the_union_of_the_thirty_two
A6: python3 -m unittest discover -s scripts/tests -p test_dispatch_shards.py -k test_every_cargo_mutants_command_carries_the_gates_own_bounds
```

The live proof is one `workflow_dispatch` of this workflow at the delivery's branch naming the
smallest package the listing shows: the legs it fans out to, the battery's examined line, and the
survivors job, quoted in the pull request.

## 4. File manifest

| file | context | change |
|---|---|---|
| `.github/workflows/mutation-weekly.yml` | repo | changed: the `size` job, the rust matrix and `--shard`, the survivors job's `--shards` (R1, R4, R5) |
| `scripts/mutation-verdict.py` | repo | changed: the shared `fewest_shards`, the `size` verb, and the battery's `FOREIGN` check (R2, R3, R6) |
| `scripts/tests/test_dispatch_shards.py` | repo | added: A1 to A6 |
| `scripts/tests/test_mutation_workflows.py` | repo | changed: the three assertions that read the fixed matrix and the fixed divisor now read the sized count |
| `scripts/mutation-rows.d/S12900-S12999.json` | repo | added: six rows: S12901 to S12903 for the verdict's clauses the per-PR Python gate cannot reach, S12904 and S12905 for the rust leg's `--timeout 300` and `--build-timeout 600`, and S12906 for the same build timeout raised by an appended digit (`6000`), which only a whole-value read of the bound kills, by `EveryMutationCommandKeepsTheGatesBounds` |
| `docs/specs/SPEC-129-a-package-dispatch-is-sharded-by-its-projected-weight.md` | repo | added |
| `docs/decisions/ADR-129-a-package-dispatch-is-sized-from-its-own-listing.md` | repo | added |
| `docs/specs/planned/SPEC-057-every-surviving-mutant-is-killed-or-recorded-equivalent-before-the-first-mutation-gated-release.md` | repo | changed: a dated amendment at its end (insert-only) |
| `docs/red-first/SPEC-129.md` | repo | added |
| `changelog.d/ci-dispatch-shards-129.md` | repo | added |

No schematic: the change adds no component. The data flow is SPEC-057's (a listing, a plan, shards,
a battery), with a listing and a sizing step in front of the legs.

## 5. What this does NOT do

- It never lowers a gate: the mutants, `--timeout 300`, `--build-timeout 600` and the test tool are
  the 32-leg dispatch's, and only the number of legs changes (#368).
- It does not size a scheduled run or a dispatch with no package, which keep 32 legs; sizing the
  whole sweep is a separate finding (#368).
- It does not cache the baseline build across legs: that is a repository cache and storage
  decision the orchestrator proposes separately (#368).
- It does not change `SECONDS_PER_MUTANT`, `BASELINE_SECONDS` or `SHARD_BOUND_SECONDS`, so the
  projection is the per-pull-request plan's and no new calibration is claimed (#368).
- It changes the per-pull-request plan's behaviour in no way: `shards` keeps its outputs and its
  refusal (#368).

## 6. Risks

- **The package listing and the legs disagree.** Both use the same command and `--no-shuffle`
  order; R7's table check refuses any mutant a report set did not test, and A5 pins the
  arithmetic.
- **A projection is not a measurement.** A leg's time is the plan's projected time, whose table
  the shards of the weekly runs measured at 0.66 to 1.33 times its projection (SPEC-057 R18); the
  job's timeout is twice the bound.
- **The survivors job downloads one more artifact.** The `sizing` artifact joins the run's others
  and the survivors job reads its reports by name and by recursive search, so the extra directory
  changes no path it reads (the battery ignores every directory not named `mutants-shard-<k>`).

## 7. References

Issue #368; SPEC-057 R14, R18, R22; SPEC-039 R12; ADR-057 D7; ADR-129; cargo-mutants 27.1.0
(`--shard`, `--sharding round-robin`, `--list --json`).

## 8. Amendment, 2026-09-29: A6 finds every spelling of the command (#395)

Insert-only against dev under ruling (i) of SPEC-038 section 8: every byte of dev's file is kept in
order, and this amendment inserts this section and the next after it, and nothing else. Section 8
and the next are this PR's own text, and round 8 rewrites their earlier rounds' words. Issues #395 and #447.

- **The rule.** A6 found each `cargo mutants` command with a pattern that missed three spellings of
  the same command: `cargo +<toolchain> mutants`, the `cargo-mutants mutants` binary form, and a
  second command on the same line, which the greedy line pattern merged into the first. The guard
  now reads each workflow as YAML and bash read it, over a declared part of each grammar, and
  refuses what it does not read. It finds a command only where bash itself runs it, with the words
  bash passes it, and it refuses every other place in the text it reads where a `mutants` word could
  reach cargo. A refusal is reported as `refused: <why>`, which carries no bounds, so A6 fails on it
  (#395).
- **The YAML.** The guard reads a workflow's block mappings and sequences; literal and folded block
  scalars, with their chomping and indentation indicators; and single-quoted, double-quoted and
  plain scalars, with YAML's escapes (`\\` and `\x23` among them), its escaped line breaks, its
  folding and its comments. So a `#` that YAML keeps (in a block scalar, in quotes, or as `\x23`)
  reaches the shell's reading, and a comment that YAML cuts (a blank and a `#` in a plain scalar,
  inside shell quotes too) reaches it as nothing. It refuses a flow mapping, an anchor, an alias, a
  tag, a quoted or complex key, a directive or document marker, a tab indent, a carriage return, a
  scalar on the line after its key, a more-indented line in a folded block, a `shell:` other than
  `bash`, and any other form it does not read. A `${{ matrix.<key> }}` in a `run:` value takes each
  value of the one-line list that its job's matrix gives that key; a matrix with `include` or
  `exclude` gives none. Any other `${{ }}` expression in a `run:` value is refused: GitHub puts its
  value into the text before bash reads it, the value is not in the workflow, and it can end one
  command and start another. An expression outside every `run:` value is not in the text bash reads
  (#395, #447).
- **The shell.** Each `run:` value that still holds `cargo` or `mutants` once every quote mark,
  backslash, `$` and line break is taken out, or that holds a `$` and a `'` with only those marks
  between them, is read as bash's token recognition reads it: a comment from a `#` that starts a
  word; `\`-newline inside and outside words; a backslash by the parity of its run; single, double,
  `$'...'` and `$"..."` quotes; parameter expansions, command and process substitutions, backquotes,
  arithmetic and arrays; here-documents (expanded or not, as the delimiter's quoting says) and
  here-strings; redirections and their targets; and each operator that ends a command. Of the units
  that bash's token recognition reads as one word (bash 5.2, `read_token_word` in `parse.y`), it
  reads each of those above and refuses the rest: `$[ ]`; a pattern character glued to `(`, which
  bash reads as one extended pattern where extended globbing is on, as it always is after `==`,
  `!=` and `=` in `[[ ]]`; and a subscript, after a name or at the start of an array's element,
  that holds more than letters, digits, `_`, `$` and `+*/%-`. It refuses
  an unclosed quote or substitution, a continued line in an expanded here-document, a here-document
  opened inside a substitution's line, `=~`, a `case` inside a substitution, an expansion or a form
  it does not read, `shopt`, `enable` and `alias`, and a `set` whose option bash computes or that
  changes `-k`, `-H`, `posix`, `keyword` or `histexpand` (#395, #447).
- **The command and its bounds.** A command is found only where bash itself runs `cargo` or
  `cargo-mutants` (or a path to either) as the program of one simple command, in the script or in
  any substitution: after the command's assignments and the words that run what follows them in the
  same shell (`if`, `then`, `else`, `elif`, `do`, `while`, `until`, `{`, `!`, `time`, `command`,
  `builtin` and `exec`), past `+<toolchain>` words, flag words and a valued flag's separate word, to
  `mutants`. Its bounds are the four words `--timeout 300 --build-timeout 600`, whole and in order,
  before any `--`. So a bound's value that bash reads as longer (`600"0"`, `600$'0'`, `600{,0}`, or
  `600` continued by `\`-newline) bounds nothing, and neither do bounds in a comment, in a
  here-string or another redirection's target, or after an operator that ends the command, inside
  an expansion or outside one. A word that bash computes is never a bound. The guard refuses a
  computed word where cargo reads its subcommand; a `cargo` with no subcommand word (`xargs cargo`,
  `cargo --version`); `cargo mutants` that another program runs (`env`, `nice`, `timeout`, `xargs`
  or any other), since that program decides the arguments cargo gets; every literal `mutants` word
  that is not a found command's subcommand; and a text that bash computes and hands to `eval`, or
  to `bash` or `sh` after `-c`, to read as shell (#395, #447).
- **Texts handed on.** A text that bash hands to a program that may read it as shell is read again
  by the same grammar, and a `cargo mutants` command or any other `mutants` word found in it is
  refused, since the program that reads it decides the arguments. Such a text is the value of each
  word that bash reads as more than one plain word (one holding a blank, a quote, a backslash, an
  operator, an expansion, a brace, a glob or history character, `#`, `~` or `=`), which covers
  `bash -c '...'`, `eval '...'` and `echo '...' | bash`; the value of every assignment; the operand
  of every parameter expansion (`${X:-...}`); a here-string; and a here-document's body as its
  reader receives it (for an unquoted delimiter, with `\$`, `` \` `` and `\\` unescaped and each
  expansion taken as computed). A text given to `python3` or `python` is not read (#395, #447).
- **A computed word before the bounds (R5).** A word that bash computes before the bounds and that
  bash could expand to exactly `--` would hand the bounds to the test tool, so the guard refuses it.
  It refuses a word that holds an unquoted expansion, which bash splits, and a word that holds an
  expansion and no literal character besides `-` (`"$X"`, `"${X}"`, `"$@"`, `"$*"`, `-"$X"`). A word
  with any other literal character (`"$A/$B"`, `--package="$P"`, `x$X`) can never be `--` and is
  read as before. One test generates each expansion form the reading reads, crossed with quoting and
  with the word's literal content, has bash run each member with a stub, and asserts that every
  member that loses the bounds is refused and that the refusals equal the rule exactly. It prints
  and asserts each count (#395, #447).
- **The weekly sweep in literal words.** `mutation-weekly.yml` gave its two package-bearing `cargo
  mutants` commands the package as `${PACKAGE:+--package "$PACKAGE"}`, an expansion the rule above
  refuses. Each command is now an `if [ -n "$PACKAGE" ]; then` branch with `--package="$PACKAGE"`,
  an `else` branch with no package word, and `fi`; every other word is kept in its order. One test
  holds the head's two commands as literals, has bash run both spellings with a stub cargo over
  each package the workspace declares and over hostile values, and asserts that the two argvs are
  equal with `--package V` as the one word `--package=V`. A value that begins with `-` selects
  nothing under both spellings. The guard now finds 6 commands in `mutation-weekly.yml` where it
  found 4, because each branch is a command; each is found and bounded and none is refused (#395).
- **What it hides.** Each member of the generated classes below that bash runs without the bounds is
  found or refused, in both states: with no `${{ }}` expression in its `run:` value, and past one.
  The guard can refuse a value that bash runs bounded. On the real tree it finds the same commands
  as before (3 in `ci.yml`, 6 in `mutation-weekly.yml`) and refuses none (#395, #447).
- **A plant per shape.** Three tests write one workflow each into a temporary directory: one with
  a toolchain spelling, one with the binary form, and one with two commands on a line (the first
  bounded, the second not). Each asserts that every command is found and, for the last, that the
  bounded one and the unbounded one are told apart. On the real tree the guard finds the same
  commands as before (3 in `ci.yml`, 6 in `mutation-weekly.yml`).
- **Four plants more.** One workflow each with `cargo --config <value> mutants`; with the bounds
  written in a comment after an unbounded command (and a comment that holds a whole command, and a
  `#` inside shell quotes in a `run: |` block); with a bounded command followed on its line by
  `cargo -C <dir> mutants`; and with the bounds written in a comment that starts right after `;`
  (#395).
- **The class, generated.** One test generates the members of the class, where a `#` starts a
  comment as bash reads it: each fragment that carries a would-be comment or a would-be closer
  (`#`, `;#`, `)#`, `}#`, `"}"`, `')'`, `"#"`, `\#`, `a#`, `\'`, `;;`) in each context (bare, the
  substitutions, backquotes, `$(( ))`, `${ }`, the three quotes, a here-string, `(( ))`, `a=( )`,
  `[[ =~ ]]`, and a case, a brace group and a subshell, alone and inside each substitution),
  followed by a `#` and an unbounded command, by the bounds, or by a continued line; and each
  context left open at its line's end or continued inside it, with its closer and a `#` on the
  next line or the one after, then an unbounded command or the bounds. Bash reads
  every member, run without `-e`, with `cargo` a function that logs its words and no other
  command on the path, and the guard must find each unbounded command bash runs. The test asserts
  and prints the member count (#395).
- **The grammar class, generated.** One test generates the members of the class at the grammars:
  each shell text of an axis, crossed with each YAML spelling that reads back as that text. The axes
  are a run of 1 to 4 backslashes, bare and in each quote, followed by a line break, a blank, a `#`,
  a quote or the end of the value (on its first line or a later one); each spelling of each word of
  the command and its bounds, among them a value that bash reads as longer; here-documents,
  here-strings and a text that another program runs, among them an expansion glued to a `#` in an
  expanded here-document; the operators that end a command, inside and outside an expansion;
  redirections among the bounds; the programs and compound commands that run another command, and
  `xargs` running cargo with words from its input (the subcommand decoded by `printf`, an argument
  put before the bounds, the subcommand held in a variable); `set`, `shopt` and aliases; a comment
  after each operator; a `${{ }}` value before, inside and after the command; each unit that bash's
  token recognition reads as one word (the four quotes, `$( )`, `$(( ))`, `$[ ]`, `${ }`, `<( )`,
  `>( )`, backquotes, a subscript and the five extended patterns) holding `;#`, `)#` or ` #`, alone
  and glued to a word, among a command's words, after `==`, `!=` and `=` in `[[ ]]` and as an
  array's element, with and without a `#` glued after it;
  and a text that bash hands on (to a shell after `-c`, through a pipe, a here-string or a
  here-document, in a variable, or inside an expansion), with the command's words spelled by
  quoting, by an expansion or by another program; among them a variable that the text assigns a
  literal, handed to `eval` or to `bash` or `sh` after `-c` (with and without operands after it),
  quoted and not, spelled plainly and in braces. The spellings are a literal block, a folded block
  (each line kept, and each line's words split onto lines that YAML joins), a double-quoted scalar
  (plain, with `\x23` for `#`, and with escaped line breaks), a single-quoted scalar, and a plain
  scalar, alone and before a YAML comment that holds the bounds; a text handed on is spelled in a
  literal block. YAML's node forms around a `run:` value are members too. Bash runs each text as
  a script twice, with the stub's status 0 and 1, under a timeout, without `-e` and without a parse
  gate, with only stubs on the path: `cargo` and `cargo-mutants` log their words, and `env`, `nice`,
  `timeout`, `xargs`, `sh` and `bash` pass the command on. The test fails unless every text ran
  twice and a bounded and an unbounded control read as they must. The guard must find or refuse each
  member that bash runs without the bounds, and each member whose `run:` value holds an expression
  that the workflow does not state. The test asserts and prints the member count (#395, #447).
- **The reading, declared.** One test states the reading the guard declares. A bounded command after
  each word after which bash runs the next word as the program (`if`, `then`, `else`, `elif`, `do`,
  `while`, `until`, `{`, `!`, `time`, `command`, `exec`, and `builtin command`) is found, bounded,
  and bash runs each one bounded. A command that changes how bash reads what follows (`set -k`,
  `set -H`, `set -o keyword`, `set -o posix`, `set -o histexpand`, `shopt`, `enable` and `alias`,
  alone and after `builtin` or `command`), a continued line in an expanded here-document, and an
  expression whose value the workflow does not state are each refused for that (#395, #447).
- **The memory scope's wrapper (round 8, after the merge of `dev`).** `dev` runs each `cargo
  mutants` that runs tests in `ci.yml` and `mutation-weekly.yml` as `python3 scripts/memory_scope.py
  --report <dir> -- cargo mutants ...` (SPEC-196). The wrapper's parser declares one option that
  takes a value, `--report`. The wrapper takes one leading `--` off the words after its options and
  runs the rest in a subprocess, without a shell, each word unchanged. So a command whose program
  word is the literal `python3`, whose next word is the literal `scripts/memory_scope.py`, and whose
  next words are only options that the wrapper's parser declares (read from the parser, not from a
  list), each with one value as its next word or after `=`, then a literal `--` and at least one
  word, is read as the command that begins after that `--`, with its first word as the program;
  every rule above applies to that command, and a wrapper inside a wrapper is read the same way. A
  value is read only where bash hands it on as exactly one word: a literal word, or double quotes
  around literal text and named expansions (`"$out"`, `"${out}/x"`). Every other spelling is not the
  form, so a `cargo mutants` in it is refused as before: the value unquoted or from `"$@"`, no `--`
  or one that bash computes, a computed option or path, another path or program (`./scripts/...`, an
  absolute path, `python`, `python3.12`, `env python3`, `python3 -u`), an undeclared, abbreviated or
  help option, and a declared option after the `--`. A text given to `python3` or `python` is read
  as a text handed on when an argument of that command names `memory_scope` or is computed; a text
  given to any other `python3` or `python` command is still not read. The guard reads the wrapper as
  the file the tree holds: a step that rewrites `scripts/memory_scope.py`, or that runs the command
  from a directory holding another copy of it, is left out, as a script file that a step runs is
  (R2, #465). One test runs the wrapper with its scope's seams planted and a spy as its command,
  over 374 argvs (the declared option in both spellings and twice, with dash-led values, before
  commands of dash-led words, `--` again, empty words, blanks, a newline and non-ASCII), and asserts
  that the spy receives exactly the words after the `--`, or that a dash-led value ends at the
  parser's usage error and nothing runs; a wrapper planted to drop the last word, to drop
  `--timeout`, or to add `--in-place` turns it red. One test generates 1160 members (29 spellings of
  the wrapper, around 10 commands, in 4 contexts), has bash run each with the real wrapper and a
  stub cargo, and asserts that each member that bash runs without the bounds is found or refused;
  another asserts that each of the 8 spellings it reads through, around a bounded command, is found
  bounded. On the real tree the guard finds 3 commands in `ci.yml` and 6 in `mutation-weekly.yml`,
  the wrapped ones read through the wrapper, and refuses none. `scripts/tests/test_memory_scope.py`
  pins the weekly sweep's test run and its size listing in each literal branch, and makes every
  assertion it made before. The rows S12904 to S12906 of this SPEC's band and SPEC-196's S19616 name
  the branch with a package, since their anchors occur once per branch; S12907 to S12910 are their
  twins in the branch with no package, and S12911 and S12912 hold the guard's wrapper arms (#395,
  #447).
- **Out of scope, named.** The guard reads the workflow files in `.github/workflows`, so it does not
  read a script file that a step runs, a composite action, a cargo alias, an `env:` value (a step's,
  a job's or the workflow's), or a file that bash or the runner reads at start (`BASH_ENV`,
  `GITHUB_ENV`). It neither finds nor refuses a command whose program word bash computes (by a
  variable, a substitution, a brace expansion or word splitting) where no word the guard reads is
  `mutants`, or a text that another program decodes (`printf`, `base64`) and hands to a shell; a
  literal `cargo` whose subcommand bash computes, or that has no subcommand word, is refused. It
  does not read a text given to `python3` or `python`. Three properties are left out, and issue
  #465 holds them: a program word that bash computes where no word the guard reads is `mutants`
  (R1), text outside a `run:` value that a step runs (R2), and text given to `python3` or `python`
  (R3). The bounds are also checked before the run, and not while it runs: the run-time bound is
  issue #466.
- Files: `scripts/tests/test_dispatch_shards.py`, this SPEC, `docs/red-first/SPEC-129.md` and a
  changelog fragment (#395).
- It changes no Rust and no Python outside the test, and one workflow, `mutation-weekly.yml`, in the spelling of its package word only (#395).
- It adds no mutation-row band: the change is to a test file only (#395).

## 9. Acceptance criteria added by the section 8 amendment

| id | criterion | decided by |
|---|---|---|
| A7 | the guard finds `cargo +<toolchain> mutants`, `cargo-mutants mutants`, and each of two commands on one line, the second spelled with a valued flag too | `test_dispatch_shards.py` `EveryMutationCommandKeepsTheGatesBounds` |
| A8 | a cargo flag's separate value word is part of the command, a comment the guard cuts is no command and bounds nothing, and the guard finds every unbounded command bash runs in the generated members of the comment class | `test_dispatch_shards.py` `EveryMutationCommandKeepsTheGatesBounds` |
| A9 | the guard reads each `run:` value as YAML and bash read it, or refuses it: in the generated members of the grammar class it finds or refuses every command that bash runs without the bounds, and every value that holds an expression the workflow does not state | `test_dispatch_shards.py` `EveryMutationCommandKeepsTheGatesBounds` |
| A10 | a text that bash computes (by a substitution, or from a variable) and hands to `eval`, or to a shell after `-c`, is refused for that alone, and a literal text handed to a shell or to a builtin that reads it as shell is read, and refused for the command it holds | `test_dispatch_shards.py` `EveryMutationCommandKeepsTheGatesBounds` |
| A11 | the reading is declared: a bounded command after each word after which bash runs the next word as the program is found, bounded, as bash runs it; and a change to how bash reads what follows, a continued line in an expanded here-document, and an expression the workflow does not state are refused for that | `test_dispatch_shards.py` `EveryMutationCommandKeepsTheGatesBounds` |
| A12 | a computed word before the bounds that bash could expand to exactly `--` is refused: every generated member that loses the bounds is refused, and the refusals equal the rule | `test_dispatch_shards.py` `AComputedWordBeforeTheBoundsIsRefused` |
| A13 | the weekly sweep's two package-bearing commands, in literal words, hand cargo the words the head's spelling did, and the guard finds all 6 commands of `mutation-weekly.yml` bounded and refuses none | `test_dispatch_shards.py` `TheWeeklySweepNamesItsPackageInLiteralWords` |
| A14 | the memory scope's wrapper runs exactly the words after its `--`, and a wrapper that drops or adds a word turns the pin red; the guard reads the wrapper's declared form through to the command after its `--`, so it finds or refuses each generated wrapper member that bash runs without the bounds, finds each spelling it reads through around a bounded command bounded, and finds the real tree's wrapped commands bounded and refuses none | `test_dispatch_shards.py` `TheMemoryScopeRunsTheWordsAfterItsSeparator` |

```acceptance
A7: python3 -m unittest discover -s scripts/tests -p test_dispatch_shards.py -k test_a_command_spelled_with_a_toolchain_is_found
A7: python3 -m unittest discover -s scripts/tests -p test_dispatch_shards.py -k test_the_cargo_mutants_binary_form_is_found
A7: python3 -m unittest discover -s scripts/tests -p test_dispatch_shards.py -k test_a_second_command_on_one_line_is_its_own_command
A7: python3 -m unittest discover -s scripts/tests -p test_dispatch_shards.py -k test_a_valued_flag_spelling_after_a_bounded_command_is_its_own_command
A8: python3 -m unittest discover -s scripts/tests -p test_dispatch_shards.py -k test_a_cargo_flag_with_a_separate_value_is_part_of_the_command
A8: python3 -m unittest discover -s scripts/tests -p test_dispatch_shards.py -k test_a_comment_is_no_command_and_bounds_nothing
A8: python3 -m unittest discover -s scripts/tests -p test_dispatch_shards.py -k test_every_unbounded_command_bash_runs_past_a_hash_is_found
A9: python3 -m unittest discover -s scripts/tests -p test_dispatch_shards.py -k test_every_command_bash_runs_from_a_run_value_is_found_or_refused
A10: python3 -m unittest discover -s scripts/tests -p test_dispatch_shards.py -k test_a_text_bash_computes_for_a_shell_to_read_is_refused_as_computed
A11: python3 -m unittest discover -s scripts/tests -p test_dispatch_shards.py -k test_the_reading_is_declared_what_it_reads_is_found_and_what_it_does_not_is_refused
A12: python3 -m unittest discover -s scripts/tests -p test_dispatch_shards.py -k test_every_word_bash_can_expand_to_dashes_before_the_bounds_is_refused
A12: python3 -m unittest discover -s scripts/tests -p test_dispatch_shards.py -k test_the_rule_refuses_exactly_an_unquoted_expansion_or_one_with_no_literal_but_dash
A12: python3 -m unittest discover -s scripts/tests -p test_dispatch_shards.py -k test_the_designs_three_members_are_refused
A12: python3 -m unittest discover -s scripts/tests -p test_dispatch_shards.py -k test_the_real_tree_commands_are_found_bounded_and_not_refused
A13: python3 -m unittest discover -s scripts/tests -p test_dispatch_shards.py -k test_the_rewrite_hands_cargo_exactly_the_words_the_head_did
A13: python3 -m unittest discover -s scripts/tests -p test_dispatch_shards.py -k test_a_dash_led_value_selects_nothing_in_the_step_after_the_listing
A13: python3 -m unittest discover -s scripts/tests -p test_dispatch_shards.py -k test_the_rewritten_blocks_are_the_only_package_words_in_the_two_cargo_commands
A14: python3 -m unittest discover -s scripts/tests -p test_dispatch_shards.py -k test_the_wrapper_runs_exactly_the_words_after_its_separator
A14: python3 -m unittest discover -s scripts/tests -p test_dispatch_shards.py -k test_a_wrapper_that_drops_or_adds_a_word_goes_red
A14: python3 -m unittest discover -s scripts/tests -p test_dispatch_shards.py -k test_every_wrapped_command_bash_runs_without_the_bounds_is_found_or_refused
A14: python3 -m unittest discover -s scripts/tests -p test_dispatch_shards.py -k test_the_declared_form_around_a_bounded_command_is_found_bounded
A14: python3 -m unittest discover -s scripts/tests -p test_dispatch_shards.py -k test_the_real_tree_commands_are_found_bounded_and_not_refused
```

## 10. Amendments, 2026-10-01: both workflow scans find every spelling through one finder, and a stand-in that cannot plant its wrapper fails closed (#418, #497)

Insert-only against dev under ruling (i) of SPEC-038 section 8: every byte of dev's file is kept in
order, and this amendment inserts this section and the next after it, and nothing else. It is
decided by ADR-306 (one shared finder, chosen against a second copy and against importing the
guard's own module; and a stand-in that fails closed, chosen against falling back to the real
program). Issues #418 and #497.

- **The rule, #418.** `test_mutation_workflows.py` found `cargo mutants` commands with two patterns
  of its own: a line pattern that needs a blank after `mutants`, and a job filter that needs the
  literal text `cargo mutants`. Both miss spellings the guard of section 8 finds:
  `cargo +nightly mutants --in-place`, `cargo-mutants mutants --in-place`, `cargo  mutants
  --in-place` with two blanks, and `cargo --config net.retry=2 mutants --in-place`; the line pattern
  also misses `cargo mutants` at the end of a line. Both scans now read each workflow through ONE
  finder, `scripts/tests/_mutants_finder.py`, which holds the guard's reader and its command finder
  moved out of `test_dispatch_shards.py` unchanged. Both modules import it and neither keeps a copy.
- **The seam.** What a wrapper is differs between the two importers, so the finder takes it as a
  parameter, `wrapped`, which defaults to a recognizer that sees no wrapper. It is never a module
  attribute, so what one importer passes cannot reach the other when both load in one process.
  `test_dispatch_shards.py` passes its own `wrapped`, which reads the wrapper's file;
  `test_mutation_workflows.py` passes a recognizer that names no wrapper and reads no file: for a
  command whose program is not cargo, the command is the words after its first standalone `--`.
- **The weight of the move.** The finder's text keeps its one literal mention of the wrapper's
  file name, in a string match and a comment, so a search for that name lists the support module.
  It imports nothing and reaches no path, and `test_mutation_workflows.py` imports nothing that
  does, so it stays runnable on a machine where the wrapper's own tests are not.
- **The rule, #497.** The stand-in `test_dispatch_shards.py` runs in place of the interpreter
  planted the wrapper and, when planting raised, fell through to running the real program with the
  words unchanged. It now prints the failure to standard error and exits non-zero, and runs nothing
  after it. A census that reads the text of `scripts/tests/` lists each stand-in that falls back to
  a real program on a failed plant, with its file, line and arm; it lists none at this head.
- **A refusal is a find.** A workflow the reader refuses is reported by the command scan as
  `refused: <why>`, which carries no bounds, and counted by the job scan as running the command, so
  the assertions over them fail on it and never pass over it.
- **Not measured on this machine.** `test_dispatch_shards.py` and `test_memory_scope.py` run in CI
  only. The test of #497 is therefore a CI-only red and a CI-only green, and its record cites the
  run.
- **Files:** `scripts/tests/_mutants_finder.py`, `scripts/tests/test_mutation_workflows.py`,
  `scripts/tests/test_dispatch_shards.py`, `scripts/tests/_stand_in_census.py`,
  `scripts/tests/test_stand_in_census.py`, `docs/decisions/ADR-306-one-finder-for-the-mutants-scans-and-a-stand-in-that-fails-closed.md`,
  `docs/red-first/SPEC-129.md` and a changelog fragment (#418, #497).
- It changes no Rust, no workflow and no production Python, and adds no mutation-row band: every
  changed file is a test or its support (#418, #497).
- It leaves the survivors step, the step runner's shell and the VOID and held-twice lines of the
  workflow tests to their own issues (#482, #487, #509).

## 11. Acceptance criteria of the section 10 amendments

| id | criterion | decided by |
|---|---|---|
| A15 | the command scan of `test_mutation_workflows.py` finds a planted workflow in each of five spellings, and one run by a wrapper after its `--`, as the guard's finder does | `test_mutation_workflows.py` `EveryMutantsSpellingIsFound` |
| A16 | the job scan finds the job of each planted spelling, and neither scan finds a job that holds no such command | `test_mutation_workflows.py` `EveryMutantsSpellingIsFound` |
| A17 | the finder is defined once, in the support module, and a planted copy of it is caught by the census of definitions | `test_mutation_workflows.py` `EveryMutantsSpellingIsFound` |
| A18 | when planting the wrapper raises, the stand-in exits non-zero, names the failure and runs none of the words (CI only) | `test_dispatch_shards.py` `TheMemoryScopeRunsTheWordsAfterItsSeparator` |
| A19 | the census lists no stand-in under `scripts/tests/` that falls back to a real program on a failed plant, and lists a planted one | `test_stand_in_census.py` `TheCensusOfStandIns` |

```acceptance
A15: python3 -m unittest discover -s scripts/tests -p test_mutation_workflows.py -k test_each_spelling_of_the_command_is_found_by_the_command_scan
A16: python3 -m unittest discover -s scripts/tests -p test_mutation_workflows.py -k test_each_spelling_of_the_command_is_found_by_the_job_scan
A16: python3 -m unittest discover -s scripts/tests -p test_mutation_workflows.py -k test_a_job_without_the_command_is_found_by_neither_scan
A17: python3 -m unittest discover -s scripts/tests -p test_mutation_workflows.py -k test_the_finder_is_defined_once_and_a_planted_copy_is_caught
A18: python3 -m unittest discover -s scripts/tests -p test_dispatch_shards.py -k test_a_failed_plant_exits_non_zero_names_the_failure_and_runs_no_words
A19: python3 -m unittest discover -s scripts/tests -p test_stand_in_census.py -k test_no_stand_in_falls_back_to_a_real_program_on_a_failed_plant
A19: python3 -m unittest discover -s scripts/tests -p test_stand_in_census.py -k test_a_planted_fallback_is_listed
```
