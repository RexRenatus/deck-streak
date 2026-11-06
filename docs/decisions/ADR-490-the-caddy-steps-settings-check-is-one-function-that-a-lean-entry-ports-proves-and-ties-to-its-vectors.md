---
status: accepted
decision-makers: "@RexRenatus (owner), the DeckStreak orchestrator"
---

# The Caddy steps' settings check is one function that a Lean entry ports, proves over every environment block and ties to its vectors

## Context and Problem Statement

Each Caddy step of `deploy/deploy.sh` (`caddy-install` and `caddy-remove`) checks its deploy
settings before it reads or writes anything (ADR-198, SPEC-127 A39). The check reads the
environment the step received as the kernel keeps it, a block of NUL-terminated entries, cuts each
entry at its first `=`, and refuses the run when an entry with the `DECKSTREAK_DEPLOY_` prefix is
not a listed setting, when a listed setting has no `=`, or when one is given twice; it also
refuses an unlisted deploy variable of the step's own shell and a run that received no environment.
A39 samples that claim with a generated population. Issue #484 asks for a proof over every block:

> For every environment block, the step proceeds past the check if and only if the block holds at
> least one entry, every entry whose bytes before its first `=` begin with the deploy settings
> prefix has a key that is one of the listed settings, contains `=`, and no such key occurs twice,
> and every prefixed variable of the step's own shell is a listed setting. The check reads no
> setting and writes nothing before that verdict.

At dev the check is top-level code inside the subcommand `case` (`deploy/deploy.sh:35-58`), and no
`@phx covers` line names `deploy/deploy.sh`. A Lean entry's cover names one item, and in a shell
file that item is a function, so the check as it stands cannot be covered.

## Decision Drivers

- The proof is about the code the step runs: a port that no cover ties to its span proves
  something about code nobody reads again after it changes.
- Every verdict and every refusal line the step prints today stays as it is; the existing pins
  (A39 and rows S12739 to S12746) keep their meaning and their counts.
- `test_deploy_scripts.py` and `deploy/deploy.sh` run in CI only, so every red and green of a test
  that executes the step is read there by name.
- The repository's formal checker reads the entry; CI has no formal job.

## Decision Outcome

Six decisions, D1 to D6, each with what it was chosen against in the section after it.

## D1. The mechanism: what the check does, read at dev

Read at dev `36c4783a`, `deploy/deploy.sh`:

- **Which steps run it.** The check is the arm `caddy-install | caddy-remove)` of the first
  `case "${1:-}" in` (`:35-36`); `deploy.sh TAG` and `deploy.sh --rollback TAG` match no arm and
  run no check. `rollback.sh caddy-remove` only finds `deploy.sh` and execs it (SPEC-127).
  Before the `case`, the script runs `set -euo pipefail` (`:24`) and assigns `SETTINGS` (`:29-34`),
  and nothing else; both read no setting.
- **The listed settings.** `SETTINGS` (`:29-34`) is one quoted text: a newline, then thirteen names
  over four lines, space-separated, each line ending in a newline. Each loop reads it as
  `for setting in $SETTINGS`, an unquoted expansion: bash splits it into words at spaces, tabs and
  newlines and drops the empty fields; no name holds `*`, `?` or `[`, so pathname expansion leaves
  every word as it is. Each test `[ "$x" != "$setting" ] || named=1` is a byte comparison (the
  `!=` of `[` compares strings, never patterns), and `named` is set when any word equals the name.
- **The shell-variable loop** (`:37-41`). `${!DECKSTREAK_DEPLOY_@}` expands to the names of the
  shell's variables that begin with the prefix, names only, no value. A name no word equals prints
  `deploy: <name> is not a setting of deploy.sh` on stderr and exits 1. These names are the ones
  bash made variables of: an entry it received whose key is a shell name and holds `=`, and any
  variable a start-up file made before the script ran.
- **The read** (`:44-45`). `mapfile -t -d '' received </proc/self/environ` reads the block as the
  kernel keeps it into an array, one element per NUL-terminated entry, the NUL removed; an empty
  entry is an empty element. When the read fails, or yields no element, the step prints
  `deploy: the environment this step received could not be read` and exits 1. The read uses a
  builtin and a redirection, so no command of the script's own runs first.
- **Each entry** (`:46-56`). `given` starts as one space (`:46`). For each element in order:
  `key=${entry%%=*}` removes the longest suffix that begins with `=`, so the key is the bytes before
  the first `=`, or the whole entry when it holds none (`:48`). `case $key in DECKSTREAK_DEPLOY_*)`
  skips a key that does not begin with the prefix (`:49`); the key is the subject, never the
  pattern, so its own `*`, `?`, `[`, spaces or newlines are bytes. A prefixed key no word equals
  prints `deploy: <key> is not a setting of deploy.sh` and exits 1 (`:51-52`); the listed test
  comes first, so an unlisted key without `=` reads as not a setting. A listed key equal to its
  whole entry, an entry with no `=`, prints `deploy: <key> is given without a value` and exits 1
  (`:53`); `KEY=` holds `=` and passes, so an empty value is a value. A listed key already in the
  record prints `deploy: <key> is given twice` and exits 1 (`:54`): the record is the given keys,
  each followed by one space, after the first space, and the test is the pattern `*" $key "*`
  whose quoted middle matches literally. Every key that reaches the record is a listed name, and
  no listed name holds a space, so ` <key> ` occurs in the record exactly when the key was given
  before. Then the key joins the record (`:55`).
- **What runs before the verdict.** Lines `:24-56` read no setting's value and write nothing: the
  names loop reads names; the entry loop reads each entry's bytes only to cut its key and to compare
  the key with the whole entry; the refusals write one line to stderr. The first read of a setting
  is `:61` (`ROOT=${DECKSTREAK_DEPLOY_ROOT:-...}`), after the check.
- **Four kinds of entry.** An entry with no `=` has itself as its key: skipped when unprefixed,
  refused as not a setting when unlisted, refused as given without a value when listed. An empty
  entry has an empty key and is skipped, and it counts toward the block's size. A key with bytes no
  shell name can hold (a hyphen, a space, a byte above 0x7F, a control byte) gets no variable from
  bash, so only the entry loop sees it, and it is refused unless it equals a listed name, which no
  such key does. A key that holds a pattern character is compared byte for byte by `[` and matched
  as the subject of the prefix pattern, so it is refused as not a setting.
- **Not measured by these reads, and so stated, never assumed:** how this bash build's `mapfile`
  splits a block that ends without a NUL (the kernel always terminates each entry, so no vector can
  hold one); the order of `${!DECKSTREAK_DEPLOY_@}` (the proof holds for every order, and the
  vectors give the shell at most one unlisted name); how pattern removal and matching treat bytes
  that are not valid in the current locale (the block carries no locale entry unless a vector run
  adds one, and the vectors run under three locales); whether bash's DEBUG trap records a function
  definition (the builder measures it with a scratch probe before A39's declared opening is
  written); and what start-up code does before the check: a name outside the prefix that the shell
  reads as code when it starts can run any code before the check, the first limit ADR-198 and
  SPEC-127 leave open (#423). The proof and its vectors are stated for the check's commands as bash
  runs them; entries that change those commands are outside both.

No block was found on which the check, so read, departs from the issue's property.

## D1 chosen against

- Reading the check from its comments and SPEC-127's prose alone: the comments do not say that the listed test precedes the no-value test, nor that an empty entry counts toward the block's size, and both decide verdicts the vectors pin.
- Treating the shell-variable loop as redundant and leaving it out of the reading: a start-up file can make a prefixed variable that no entry holds (SPEC-127 A39's member made by `BASH_ENV`), and only that loop refuses it.

## D2. The proof

A Lean entry, `formal/lean/Formal/DeployEntryRefusal.lean`, with its vectors writer
`formal/lean/Formal/DeployEntryRefusalVectors.lean`, one import and one arm in
`formal/lean/Formal/Vectors.lean`, and its vectors `formal/vectors/deploy-entry-refusal.jsonl`.

**What it ports**, branch for branch, from the function D3 makes (`refuse_unlisted_settings`, the
one `covers` anchor), over bytes:
- the read: the area as `mapfile -t -d ''` splits it at each NUL, and the read that fails;
- the shell-variable loop over the prefixed members of the shell's variables, a list given as input;
- the word split of the `SETTINGS` text into its fields, and the `!=` loop over them;
- the cut at the first `=`, the prefix test, the listed test, the no-value test as the script
  writes it (the key compared with its whole entry), and the record of keys given as bytes, with
  its `*" $key "*` test as an infix test;
- each refusal's stderr line.

**The claim**, one `Prop` over a step function, in the issue's words, with the block given as the
kernel lays it out (each entry followed by its NUL, no entry holding NUL), over every list of the
shell's variable names:
1. the step proceeds exactly when the block holds at least one entry, every entry whose bytes
   before its first `=` begin with the prefix has a key that is one of the thirteen listed names
   and contains `=`, no such key occurs twice, and every prefixed variable name is a listed name;
2. a read that fails never proceeds;
3. the check reads no value: two blocks whose entries pair off with the same key and the same
   presence of `=` get the same verdict.

The claim names the keys with library functions (the bytes before the first `=`, a list prefix,
list membership, no duplicates), so every bridge from the script's mechanisms to them (the pattern
removal, the `case` pattern, the word split and the `!=` loop, the key-equals-entry test, the
space-separated record) is a lemma the proof needs, never a definition the claim borrows.

**The witness** is the recorded counterexample: the check as it stood at dev `dafbbbbb`, the
shell-variable loop alone with no read of the received block. The red-first record of SPEC-127
(sixth round) measured that step running with the entry whose key is the prefix followed by the
byte 0x01, a key no shell name can hold (`docs/red-first/SPEC-127.md:265`). Its port proceeds on
that block with no shell variable, while the claim refuses it, so the claim is false of it. One
theorem and one witness; the theorem enters at `ramp=report`.

**The vectors** tie the port to the step. Each line is a block (its entries, as hex), the names
a start-up file makes as variables, and the port's answer: `proceed`, or the refusal's whole
stderr line. The inputs cover the empty block, an empty entry, unprefixed entries and near misses
of the prefix, each listed name alone and all of them once in two orders, an empty value, a value
holding `=`, values and keys with bytes above 0x7F and above U+00FF, entries with no `=` before
and after a valued one, every listed name given twice, the bare prefix, keys a shell cannot name,
the recorded counterexample, pattern characters, a space and a newline in a key, an unlisted
variable made before the step with and without a block, and blocks whose first refusal depends on
the shell-variable loop running first. The writer computes each answer from the port with the
shell's variables modelled as the block's entries bash can make variables of, followed by the
made names; a lemma over the inputs shows that no input gives the shell more than one unlisted
name, so the answer does not depend on the order bash lists them in.

**How a test reads them against the step**, in CI only: `test_deploy_scripts.py` runs
`deploy.sh caddy-install` with no tag on each block exactly, through `execve`, so the step's
`/proc/self/environ` holds the vector's entries and nothing else. The step is sourced by a
`bash -c` lead that first makes the vector's variables and the elevation setting the harness
requires (a listed name, so it moves no verdict). A step that proceeds past the check reaches the
caddy-install usage refusal, `deploy: usage: deploy.sh caddy-install TAG`, before anything reads
a setting or touches a file; a refusal prints the check's own line. The test compares the exit
status and the whole stderr, under the C locale and again with `LC_ALL` set to `C.UTF-8` and
`en_US.UTF-8` (an unprefixed entry appended to a non-empty block, which moves no verdict).

**The ramp:** `ramp=report`; a flip to `ramp=block` is earned by the streak and is its own delivery.

## D2 chosen against

- A claim over the block as a list of entries with no read: it leaves the NUL split and the empty-block refusal outside the proof, and the issue states the property over the bytes the step reads.
- A witness that ports a mutant of this delivery's own rows (a cut at the last `=`): a recorded counterexample of the same check exists and was measured red, and a mutant port proves only that the claim excludes a program nobody ran.
- The vectors read against the port alone, with no test that runs the step: the port would then agree with itself, and nothing would show that the step does what the port says.
- Each part of the claim as its own theorem: the old check satisfies the no-value-read part, so that part alone has no counterexample and no witness, and it is one conjunct of the one claim instead.
- Modelling the record of given keys as a list membership test: the port would be tidier than the code, and the space-separated record and its pattern test, which the code runs, would be unproved.

## D3. Production and test

**Production, one hunk of `deploy/deploy.sh`.** The check's lines `:37-56` become the body of one
function, `refuse_unlisted_settings`, defined after `SETTINGS` and before the `case`, its body
byte for byte the lines as they stand; the `caddy-install | caddy-remove)` arm keeps its line and
calls the function as its only command. No verdict and no refusal line changes on any block: the
function runs in the step's own shell, its `exit 1` ends the step, and its variables stay global
as they were. Every row on `deploy/deploy.sh` keeps its `find`, each still occurring once. This is
not a cure: D1 found no block on which the step departs from the property. It exists because a
cover names a function.

**Test, two changes in `scripts/tests/test_deploy_scripts.py`:**
- A new test, `test_a_caddy_step_agrees_with_every_vector_of_its_lean_proof`, in
  `TheCaddyInstall`, reads every vector whose block holds no empty entry (60 of 63; it names the other 3, not measured) against the step as D2 states, checks the header's entry,
  cover, anchor and count (never its digest), checks that the names of the single-entry blocks
  that proceed are exactly `SETTINGS`' words, checks that the runs wrote nothing in their working
  directory, and last checks that `deploy.sh` defines `refuse_unlisted_settings` once and that the
  Caddy arm's only command calls it. It pins the step's verdict on each of those 60 vectors and the tie between
  the cover and the code the steps run. Rows S47301 to S47303 guard it.
- A39's declared opening (`:1770`) gains the call, between the case line and the refusal's first
  command, which bash's DEBUG trap under `set -T` records twice before the function's first
  command. The opening still names every command run before the refusal, exactly; it now names one
  more, the call, which the trap records twice. Its examined count of script runs does not move.

No existing pin's examined figure moves, and no pin is weakened.

## D3 chosen against

- No function, and an entry with no cover: the formal checker would never see the check change, and a proof of a stale port would stay green.
- Moving `SETTINGS` into the function too: A39 reads `SETTINGS` from the file's first column and declares its assignment as the second command the step runs, so both pins would move.
- A cure of the start-up limit in this delivery (an interpreter that ignores inherited functions and start-up files): A39's member made by `BASH_ENV` would stop being refused, which moves an existing pin, and ADR-198 leaves that limit open by design.
- A separate new test module for the vectors: whether CI runs a new module of `scripts/tests` is not measured here, and `test_deploy_scripts.py` already runs in CI and holds the harness the test needs.

## D4. Shape

One delivery under SPEC-473, this record and band S47300-S47399, sized for one builder. SPEC-127
and ADR-198 each gain one insert-only section, appended after their last lines, because the
delivery changes what they state about the check's place and A39's declared opening. A new
schematic, `docs/schematics/the-caddy-steps-settings-check-and-its-lean-proof.md`, draws the
check from the read to the verdict with the port's functions beside each step, since no schematic
owns the settings check.

## D4 chosen against

- Two deliveries, the function first and the Lean entry after it: the function alone has no reason to exist, and the cover would land a delivery after the code it covers.
- An insert into `docs/schematics/both-caddyfile-layouts-serve-the-site.md`: that schematic draws the two Caddyfile layouts, not the settings check.

## D5. Tests and rows

- **A1** (the Lean entry): the formal checker's check of the entry reads the theorem clean, the
  witness built, no hole, the vectors byte-equal to the writer's output, and no `STALE`. Red at the
  red commit, where the cover's anchor does not resolve (the function does not exist yet).
- **A2** (the vectors test): red at the red commit by its last assertion (`deploy.sh` defines no
  `refuse_unlisted_settings`), after every vector has run against the step as it stands; read in
  CI by name.
- **A3** (A39 with the call declared): red at the red commit, where the step's recorded commands
  hold the refusal's first command where the call is declared; read in CI by name.
- Each red fails every time: the anchor, the definition and the call are absent at the base, and
  no input of the run varies.
- Rows S47301 (the Caddy arm's call removed), S47302 (the key cut at the last `=`) and S47303 (a
  setting without a value judged by an empty value), each killed by A2's test. CI proves them;
  nothing runs a mutation on the builder's machine.
- Two pushes, the deploy-class form: the red commit alone, then the fix and record commits.
- The builder is opus: the proof is an equivalence over a loop with a byte record, a split and a
  round trip, and every test red and green is read in CI with no local run.

## D5 chosen against

- One push: a red that only CI can run would never be seen red before the fix.
- A1 decided by a test that reads the Lean module's text: a text check cannot tell a proof from a statement, and the checker can.
- Rows on the vectors file instead of the script: the delivery changes the script, and rows on its lines show that the vectors kill a wrong step.

## D6. Not in this delivery

The steps' writes, the install's undo and the Caddy reload (SPEC-127's ground, #423, #424, #674);
the subcommands that run no settings check (#423); start-up code read before the check (#423);
the read's failure to open the file, stated by the claim and run by no vector (#423); the entry's
flip to `ramp=block` (#468).

## D6 chosen against

- Proving the steps' writes in the same entry: they are file writes and a reload on the host, an interleaving with the host's own processes, not a pure function of the block.
- Folding the deploy subcommands without a check into the claim: they run no check, so the claim would be false of them by construction.

## Consequences

- Good, because the step's settings check is proved over every block of entries the kernel can
  hold, and every vector whose block holds no empty entry (60 of 63) is run against the step itself.
- Good, because the cover makes the entry `STALE` whenever the function's lines change.
- Bad, because `SETTINGS` lies outside the covered function: a change to it moves no digest, and
  only the vectors test's comparison of `SETTINGS`' words with the vectors' single-name blocks
  reads it.
- Bad, because A39's declared opening names one more command, the call.
- Neutral, because the check's verdicts and its lines are unchanged on every block.

## Confirmation

SPEC-473 criteria A1 to A3, rows S47301 to S47303, and the red-first record `docs/red-first/SPEC-473.md`.

## What would make this wrong

- A bash in CI whose `mapfile`, word split or pattern matching treats a vector's bytes otherwise:
  the vectors test reads it red, and the port is re-read against that bash.
- A start-up entry the delivery did not foresee being the reason a vector passes: the vectors
  carry no such entry, and the test refuses a vector whose unprefixed keys are outside its set.
- A DEBUG trap that records the function's definition: A39's declared opening must then name it,
  and the builder's probe shows it before the opening is written.

## More Information

#484, #423, #428, #468, SPEC-473, SPEC-127, ADR-198, SPEC-295.
