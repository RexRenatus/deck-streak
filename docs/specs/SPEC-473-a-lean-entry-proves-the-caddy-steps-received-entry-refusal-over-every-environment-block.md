# SPEC-473: a Lean entry proves the Caddy steps' received-entry refusal over every environment block

- **Status:** in delivery: one pull request into dev, which closes #484.
- **Issue:** #484 (refs #423, #428, #468). **Decided by:** ADR-490. **Amends:** SPEC-127 and
  ADR-198, each by one section appended after its last line.
- **Context(s):** `deploy` (`deploy/deploy.sh`), `formal` (`formal/lean`, `formal/vectors`) and
  `repo` (`scripts/tests`, `scripts/mutation-rows.d`, `docs`, `changelog.d`).
- **Numbers used:** SPEC-473, ADR-490, band `S47300-S47399` (rows S47301 to S47303), Lean entry
  `lean/DeployEntryRefusal`.

## 1. The problem, measured

Read at dev `36c4783a`:

- Each Caddy step's settings check is top-level code inside the subcommand `case` of
  `deploy/deploy.sh` (`:35-58`), and no function holds it:
  `git show 36c4783a:deploy/deploy.sh | grep -c '^refuse_unlisted_settings() {$'` prints `0`.
- No Lean or TLA+ entry covers the script: `git grep -n '@phx covers deploy/deploy.sh' 36c4783a -- formal`
  prints nothing. A cover names one item, and in a shell file that item is a function.
- `SETTINGS` lists 13 names:
  `git show 36c4783a:deploy/deploy.sh | sed -n '29,34p' | tr -s ' \n' '\n' | grep -c '^DECKSTREAK_DEPLOY_'`
  prints `13`.
- Eight rows pin the check, S12739 to S12746:
  `git show 36c4783a:scripts/mutation-rows.d/S12700-S12799.json | grep -c -E '"S127(39|4[0-6])-'`
  prints `8`. Their killer is SPEC-127's A39, which runs the step over a generated population of
  names and a few odd environments: it samples the claim and cannot prove it (#484).
- The check has a recorded counterexample in its own history: at dev `dafbbbbb` it was the
  shell-variable loop alone, and the step ran with the entry whose key is the prefix followed by
  the byte 0x01 (`docs/red-first/SPEC-127.md:265`, A39's red line of the sixth round).

## 2. Requirements

- **R1.** `deploy/deploy.sh` defines one function, `refuse_unlisted_settings`, after `SETTINGS`
  and before the subcommand `case`. Its body is dev's lines 37 to 56, byte for byte. The arm
  `caddy-install | caddy-remove)` keeps its line and calls the function as its only command.
- **R2.** On every block and every set of the step's own deploy variables, each Caddy step's exit
  status and stderr line from the check are what they were at dev: the function is the check, moved.
- **R3.** A Lean entry, `formal/lean/Formal/DeployEntryRefusal.lean`, covers the function with one
  `covers` line (anchor `refuse_unlisted_settings`) and ports it branch for branch over bytes: the
  read of the area as `mapfile -t -d ''` splits it at each NUL, and the read that fails; the loop
  over the prefixed names of the shell's variables; the word split of `SETTINGS` and the `!=` loop
  over its words; the cut at the first `=`; the prefix test; the listed test; the no-value test as
  the key compared with its entry; the space-separated record of keys given with its `*" $key "*`
  test; and each refusal's stderr line.
- **R4.** The entry's one theorem, at `ramp=report`, is the claim of the port, over every list of
  the shell's variable names and every block laid out as the kernel keeps it (each entry followed
  by its NUL, no entry holding NUL): the step proceeds exactly when the block holds at least one
  entry, every entry whose bytes before its first `=` begin with `DECKSTREAK_DEPLOY_` has a key that
  is one of the 13 listed names and contains `=`, no such key occurs twice, and every prefixed
  variable name is a listed name; a read that fails never proceeds; and two blocks whose entries
  pair off with the same key and the same presence of `=` get the same verdict, so the check reads
  no value. It is proved with no hole and only the axioms `config/formal.json` allows.
- **R5.** The entry's witness ports the check as it stood at dev `dafbbbbb` (the shell-variable
  loop alone) and proves the claim false of it, at the block whose one entry's key is the prefix
  followed by the byte 0x01, with no shell variable.
- **R6.** The vectors, `formal/vectors/deploy-entry-refusal.jsonl`, are the writer's output, never
  edited by hand: a header line, then one line per input with its entries as hex, the names a
  start-up file makes as variables, and the port's answer (`proceed`, or the refusal's whole stderr
  line as hex). The inputs are the classes ADR-490 D2 lists, the empty block, an entry with no `=`,
  every listed name given twice and keys no shell name can hold among them.
- **R7.** A test runs `deploy.sh caddy-install` with no tag on each vector's block exactly, under
  the C locale and again with `LC_ALL` set to `C.UTF-8` and `en_US.UTF-8` for every non-empty
  block, and compares the exit status and the whole stderr with the vector: the caddy-install usage
  line when the vector proceeds, the vector's line when it refuses. It reads the header's entry,
  cover, anchor and count, never its digest; it finds the single-entry blocks that proceed name
  exactly `SETTINGS`' words; it finds the runs wrote nothing in their working directory; and it
  finds the function of R1 defined once and called by the Caddy arm.
- **R8.** A39's declared opening names the function's call between the `case` line and the
  refusal's first command, and nothing else changes in A39.
- **R9.** Rows S47301 to S47303 in `scripts/mutation-rows.d/S47300-S47399.json` pin the call and
  two lines of the function, each killed by A2's test.

## 3. Acceptance criteria of SPEC-473

| id | criterion | decided by |
|---|---|---|
| A2 | every vector's block, run through `deploy.sh caddy-install` with no tag under each locale R7 names, exits 1 with exactly the vector's stderr; the header, the `SETTINGS` tie and the empty working directory hold; `deploy.sh` defines `refuse_unlisted_settings` once and the Caddy arm calls it (R1, R2, R6, R7) | `test_deploy_scripts.TheCaddyInstall.test_a_caddy_step_agrees_with_every_vector_of_its_lean_proof`, in CI only |
| A3 | the commands bash records before each Caddy step's refusal are the declared opening, the call included, exactly (R8) | `test_deploy_scripts.TheCaddyInstall.test_a_caddy_step_reads_only_the_settings_it_names_and_refuses_any_other`, in CI only |

A2 and A3 run in CI only, because `test_deploy_scripts.py` executes the deploy scripts. The
result A1, in section 3a, is decided by the formal checker over the entry
`lean/DeployEntryRefusal`, so this fence names no command for it.

```acceptance
A2: python3 -m unittest discover -s scripts/tests -p test_deploy_scripts.py -k a_caddy_step_agrees_with_every_vector_of_its_lean_proof
A3: python3 -m unittest discover -s scripts/tests -p test_deploy_scripts.py -k a_caddy_step_reads_only_the_settings_it_names_and_refuses_any_other
```

## 3a. Results the formal checker decides

| id | result | decided by |
|---|---|---|
| A1 | the formal checker's check of `lean/DeployEntryRefusal` reads the theorem clean, its witness built, no hole, the vectors byte-equal to the writer's output and no `STALE` (R3 to R6) | the formal checker, run on the committed tree (CI has no formal job) |

A1 is read alone on the committed tree, since CI has no formal job: the checker reads the
theorem clean with its witness built, no hole, the vectors byte-equal to the writer's output
and no `STALE`. The red-first record `docs/red-first/SPEC-473.md` carries A1's red and green
lines beside those of A2 and A3.

## 4. File manifest

| path | part | change |
|---|---|---|
| `deploy/deploy.sh` | 1 | the check's lines become the body of `refuse_unlisted_settings`, called as the Caddy arm's only command (R1) |
| `scripts/tests/test_deploy_scripts.py` | 1 | the vectors test of A2; A39's declared opening names the call (A3) |
| `formal/lean/Formal/DeployEntryRefusal.lean` | 1 | new: the entry, its port, claim, theorem and witness |
| `formal/lean/Formal/DeployEntryRefusalVectors.lean` | 1 | new: the entry's vectors writer |
| `formal/lean/Formal/Vectors.lean` | 1 | one import and one arm, each in sorted place |
| `formal/vectors/deploy-entry-refusal.jsonl` | 1 | new: the writer's vectors |
| `scripts/mutation-rows.d/S47300-S47399.json` | 1 | new: rows S47301 to S47303 |
| `docs/specs/SPEC-473-a-lean-entry-proves-the-caddy-steps-received-entry-refusal-over-every-environment-block.md` | 1 | new: this SPEC |
| `docs/decisions/ADR-490-the-caddy-steps-settings-check-is-one-function-that-a-lean-entry-ports-proves-and-ties-to-its-vectors.md` | 1 | new: its decision record |
| `docs/red-first/SPEC-473.md` | 1 | new: the red-first record |
| `docs/schematics/the-caddy-steps-settings-check-and-its-lean-proof.md` | 1 | new: the check from the read to the verdict, with the port beside it |
| `docs/specs/SPEC-127-a-failed-caddy-reload-restores-the-previous-site-file.md` | 1 | one section appended after its last line |
| `docs/decisions/ADR-198-the-install-undoes-every-write-and-a-linked-candidate-is-refused.md` | 1 | one section appended after its last line |
| `changelog.d/deploy-entry-refusal-473.md` | 1 | new: the changelog fragment |

## 5. What this does NOT cover

- The steps' writes, the install's undo and the Caddy reload, which are SPEC-127's ground (#423,
  #424, #674).
- The deploy subcommands that run no settings check, `deploy.sh TAG` and `deploy.sh --rollback
  TAG` (#423).
- Start-up code read before the check: a name outside the prefix that the shell reads as code when
  it starts can run before the check, the limit SPEC-127 and ADR-198 leave open; the proof and the
  vectors hold for the check's commands as bash runs them (#423).
- A read that fails to open the file: the claim's second part states its refusal, and no vector
  runs it, since the kernel always provides the file to the step (#423).
- The entry's flip to `ramp=block`, earned by its streak in a delivery of its own (#468).
- What a deploy setting's value may hold before a Caddy step uses it: the check reads each entry's
  key and whether it holds `=`, never its value, so no value is checked here (#961).
- Bash builds other than the one CI runs: the vectors test runs the check under that build only, so
  how another build splits, cuts or matches a vector's bytes is not measured (#962).

## 6. Risks

- CI's bash splits, cuts or matches a vector's bytes otherwise than the port: A2 reads that vector
  red, and the port is re-read against the step before anything is stamped again.
- Bash's DEBUG trap records the function's definition as a command: A3 would then miss it. The
  builder measures the record with a scratch probe before writing the declared opening.
- `SETTINGS` lies outside the covered function, so a change to it moves no digest: A2's comparison
  of `SETTINGS`' words with the single-name blocks that proceed reads it.
- A runner without the `en_US.UTF-8` locale: A2 fails by its own assertion, as the existing locale
  test of this module does, and never skips.
- `formal/lean/Formal/Vectors.lean` gains an import and an arm beside other entries' arms: each goes
  in sorted place, so a merge keeps both sides.

## 7. Mutation rows

| row | line | mutant | killer |
|---|---|---|---|
| S47301 | the Caddy arm's call | the call replaced by `:` | A2 |
| S47302 | `key=${entry%%=*}` | `key=${entry%=*}`, the key cut at the last `=` | A2 |
| S47303 | `[ "$key" != "$entry" ] \|\|` | `[ "$key=" != "$entry" ] \|\|`, a value judged by being empty | A2 |

## References

#484, #423, #428, #468; ADR-490; SPEC-127 and its A39; ADR-198; SPEC-295.
