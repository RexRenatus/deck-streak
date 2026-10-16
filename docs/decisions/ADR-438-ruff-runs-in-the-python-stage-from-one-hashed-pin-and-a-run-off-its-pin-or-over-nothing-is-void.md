---
status: accepted
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# ruff runs in the python stage from one hashed pin, a run off its pin or over nothing reads VOID, and every computed repository root is proved by its syntax tree

## Context and Problem Statement

Read at DeckStreak `dev` `ac7ebd75`.

- `ruff.toml:6-15` sets the line length, the target version, `force-exclude`, two excludes (the
  parity oracle's generator and registry, whose bytes are pinned) and `unfixable`. It selects no
  rule, so a hand run of ruff checks only ruff's default set, and nothing runs it.
- `scripts/check.sh:31` lists the gate's stages. None lints Python. `stage_python`
  (`scripts/check.sh:179-196`) runs three unittest suites and checks `python3` and `cargo` first
  (`:181`). `git grep -i ruff` at `ac7ebd75` finds the word only in `ruff.toml`, `CHANGELOG.md` and
  two documents: no workflow installs ruff, and no stage calls it.
- The tree holds no requirements file of any kind (`git ls-tree -r --name-only ac7ebd75`), and the
  `hygiene` job (`.github/workflows/ci.yml:298-358`) installs `cargo-deny`, gitleaks and protoc,
  each pinned, and no Python tool.
- Under the rule set D2 names, the tree has these sites, each a tree fact a reader can open:
  - S324 at `scripts/ios_lane.py:312` (`hashlib.sha1(der)`, the certificate fingerprint the lane
    matches against the signing tool's identities) and at `scripts/tests/test_ios_lane.py:488`
    (the test's own fingerprint of its fixture certificate);
  - TID251 (`os.chdir`) at `scripts/tests/test_mutation_rows_refusal.py:189`, `:190`, `:195`,
    `:246`, `:278`, `:287`, `:365` and `:369`, where the test changes its working directory as its
    subject and restores it;
  - PLE2515 at `crates/agent/tests/fixtures/fake-probe.py:17`, whose string of invisible
    characters is the fake probe's `output-invisible` class;
  - S104 at `scripts/tests/test_deploy_scripts.py:2029` (an assertion's expected prefix) and S604
    at `scripts/tests/test_dispatch_shards.py:1056` (a `shell=` keyword of a workflow builder,
    which starts no process);
  - and one file the formatter would change, `scripts/tests/test_sync_server_launcher.py:279-281`,
    a call it joins onto one line.
- Eleven assignments compute the repository root from `Path(__file__)` by a count of parents, and
  a count that is off by one reads a sibling directory without any error:
  `scripts/create-issues.py:31`, `scripts/dynamic-imports-check.py:27`,
  `scripts/public-scrub.py:59`, `scripts/threat_model.py:17`, `scripts/tests/_support.py:7`,
  `scripts/tests/test_bot_messages.py:19`,
  `scripts/tests/test_mutation_rows.py:52` and `:467`, `scripts/tests/test_ship_profile.py:14`,
  `scripts/tests/test_threat_model.py:21` and `deploy/host-scrub/inventory.py:226`. Each resolves to
  the repository root today.
- DeckStreak's history holds no commit that corrected such a count. Searched at `ac7ebd75`:
  `git log --grep` over the root-finding words (parents, `.parent.parent`, wrong root, repository
  root, computed root), `-S 'Path(__file__).resolve().parents['`, `-S '.parents['` and
  `-S '.parent.parent'` over fix commits, and `-G` over every `ROOT` or `REPO` assignment from
  `Path(__file__)`. Every ROOT or REPO line the `-G` search returns was added, or removed with its
  file, and never rewritten to another depth. The guard is therefore preventive: it holds a count
  that is right today and that nothing else checks.
- At `a3a052b2`, the `hygiene` job's python stage took 853 s in job 116733923723 of run 38826135989.
  In the same job, the checksum-verified gitleaks and protoc install steps took about 0.7 s and
  0.3 s.

## Decision Drivers

- A lint gate is worth only what it examined: a ruff that is missing, off its pin, refusing its
  configuration or examining no file must never read as a pass.
- The rule set is settled. Widening or narrowing it is a separate decision, and a narrowing is a
  weakening.
- CI and a maintainer's machine must run the same ruff, installed from bytes the tree names.
- No new stage and no new job: the gate's layout per job is SPEC-038 R3's.
- Every site the set finds today is cured in this delivery, by a code change that changes no
  behaviour or by a per-file ignore the set names.

## Decisions, and the alternatives each was chosen against

### D1. One hashed pin, installed the same way in CI and on a maintainer's machine, and every other reading VOID

`scripts/ruff-requirements.txt` holds one requirement, `ruff==<version>`, followed by one
`--hash=sha256:<digest>` line for every wheel that release publishes, and no source archive.
`ruff.toml` sets `required-version = "==<version>"` with the same version, so a ruff off its pin
refuses the configuration itself, wherever it is run. A test holds the two equal.

The `hygiene` job installs it before its gate step, into a new virtual environment under the
runner's temporary directory, by `pip install --require-hashes --only-binary :all: -r
scripts/ruff-requirements.txt`, and appends to the job's path a directory that holds that ruff
alone. The stage's `python3` stays the runner's own. A maintainer runs the same install and puts
that ruff on `PATH`.

`stage_python` checks `ruff` after `python3` and `cargo`. Each other reading is VOID, which fails
the stage and is never a pass, and the stage's last line names it as `ruff VOID: <reason>`:

1. A missing ruff fails the stage by name, `missing tool: ruff (<install hint>)`, as every missing
   tool of the gate does.
2. A pin file with no `ruff==` line, or a `ruff --version` that is not `ruff <pin>`, is VOID
   before ruff checks anything.
3. `ruff check` or `ruff format --check` exiting 2 (a refused configuration, a refused option or
   an internal error) is VOID.
4. A file listing with no file in it is VOID (`examined 0 files`).
5. A format summary the stage cannot read for its count, or a format count that differs from the
   check listing's, is VOID.

Chosen against:

- An unpinned install (`pip install ruff`): rejected because each release changes which rules
  exist, what its default set holds and what its formatter writes, so the verdict would move with
  no change to the tree.
- The runner image's ruff: rejected because the image is not the repository's to pin, one image
  update would change the gate, and a maintainer's machine has no copy of that image.
- A git hook or an editor integration outside the gate: rejected because a hook is optional and
  local, so nothing would refuse a change that skipped it.
- A release archive checked by `sha256sum`: rejected because, as the gitleaks and protoc steps
  do, it pins one platform's archive inside the workflow and leaves a maintainer's machine with no
  checked install, where one requirements file hashes every platform's wheel.
- An install into the runner's own interpreter (`--user` or `--target`): rejected because a
  virtual environment isolates the install without a flag that switches the interpreter's own
  protection off.
- The virtual environment's whole `bin` directory on the path: rejected because it holds a
  `python3` too, and the stage's three suites would then run under another interpreter than today.
- A missing ruff skipped with a warning: rejected because a stage that skips its check reports a
  green it did not earn, which `scripts/check.sh:11-13` already refuses for every tool.

### D2. The rule set, as settled

`ruff.toml`'s `[lint]` table sets `extend-select` to the families `S`, `B`, `ASYNC`, `PGH`, `PLE`,
`FIX`, `T10` and `DTZ`, with the single rules `RUF034` and `TID251`. ruff's own default set stays
enabled beside them, at the pinned version. `ignore` holds the permanent ignores `S603` and `S607`
(an argument-list subprocess is the house shape) and `S105`, `S106` and `S107` (gitleaks is the
authority on secrets), each reason written in a comment beside it, and the rules not admitted:
`S101`, `S102`, `S108`, `S311`, `B904`, `B905`, `B023`, `B007`, `B008` and `DTZ001`.
`per-file-ignores` holds `S104` and `S604` for `scripts/tests/**`, `TID251` for
`scripts/tests/test_mutation_rows_refusal.py` and `PLE2515` for
`crates/agent/tests/fixtures/fake-probe.py`. `[lint.flake8-tidy-imports.banned-api]` bans
`os.system`, `os.popen`, `subprocess.getoutput`, `subprocess.getstatusoutput`, `pickle`,
`marshal`, `shelve`, `tempfile.mktemp`, `telnetlib`, `ftplib` and `os.chdir`, each with a message
naming what to use instead. `unfixable` keeps `F401` and `F841`, and nothing turns on preview.

Chosen against:

- `select` restating ruff's default set beside the families: rejected because it would name the
  default rules as if this delivery chose them, where `extend-select` names exactly what it adds
  and the pin holds the default set still.
- A wider or a narrower set: rejected because the set is settled, so a change to it is its own
  decision, and a narrowing is a weakening that only a signed ruling admits.
- A `# noqa` comment at each site instead of the per-file ignores: rejected because the per-file
  table is the settled form, and it states each exception once, where a reader of `ruff.toml` finds
  and counts it.
- Rewriting the eight `os.chdir` sites: rejected because that test's subject is how the rows runner
  reads an empty or `.` path entry, which is the working directory, so changing the directory is
  the behaviour under test, and each call restores it.

### D3. Every site cured in this delivery, without a change of behaviour

The two S324 sites take `usedforsecurity=False`: the hash is the certificate's fingerprint, which
the signing tool also reports by SHA-1 and the lane matches against it
(`scripts/ios_lane.py:309-313`), not a secret's digest, and the keyword changes no byte of it. Each
line is then written in the formatter's own wrapping. The TID251, PLE2515, S104 and S604 sites
stay as they are, under the per-file ignores of D2. The formatter's one change joins
`scripts/tests/test_sync_server_launcher.py:279-281`. A cure that changed behaviour, or a site the
set finds beyond these, is not this delivery's to decide.

Chosen against:

- A per-file S324 ignore for `scripts/ios_lane.py`: rejected because it would also hide a later
  insecure hash in a production module, where the keyword states at the call what the hash is for.
- Another digest for the fingerprint: rejected because the lane compares it with the signing
  tool's SHA-1 identities, so any other digest would change what the lane accepts.
- Escaping the fixture's invisible characters as `\u` sequences: rejected because the per-file
  ignore is the settled cure and the delivery has no need to change a fixture's bytes.

### D4. What the gate lints: every tracked Python file under five roots, under the existing excludes

Both arms run from the repository root over `scripts tools deploy agent crates`, named once in
`scripts/check.sh` as `RUFF_ROOTS`, under `ruff.toml`'s excludes (with `force-exclude`), ruff's
default excludes and `.gitignore`. Every tracked Python file of the repository sits under these
roots at `ac7ebd75`. Both arms pass `--no-cache`, so the stage writes no cache directory into the
checkout that the `scrub` and `secrets` stages after it in the same job would read, and `ruff check`
prints its findings in the concise form. `ruff.toml` excludes neither
`scripts/tests/test_deploy_standin.py` nor `scripts/tests/_standin_checks.py`, so the gate lints
both, as it lints every other module of `scripts/tests`.

Chosen against:

- The whole repository (`ruff check .`): rejected because naming the roots makes the population a
  decision a diff shows, where `.` would take in any directory a later change adds.
- Only the files a change touches: rejected because a rule held only on changed lines lets the rest
  of the tree drift unseen.
- A cache kept between runs: rejected because the stage's run takes seconds without one, and a
  cache directory in the checkout is a file the next two stages would scan.

### D5. The computed-root guard reads the syntax tree and proves each count

`scripts/tests/test_computed_roots.py` walks every `.py` file under the five roots, parses each
with `ast`, and finds every assignment, at any scope, plain or annotated, whose one target is a
name holding `ROOT` or `REPO` in any case, and whose value is `Path(__file__)` or
`pathlib.Path(__file__)`, optionally `.resolve()`, followed by `.parents[N]` with an integer
literal or by `.parent` repeated. It proves that each one resolves to the repository root, or to
the directory `LISTED_ROOTS` names for that site (`<path>::<name>` to a repository-relative
directory and its reason). A site on such a name with any other chain fails as one the guard cannot
resolve, and a file that does not parse fails by name. The guard prints how many files and sites it
examined and fails at zero. `LISTED_ROOTS` is empty, and the guard lands with no violation: the
eleven sites of the context all resolve. Its walk skips the names `FENCED` lists before any open
(the two deploy stand-in test modules and the `drill` and `readings_tree` named files), whose review
is held apart.

Chosen against:

- A regular-expression grep over the text: rejected because it counts the two `ROOT` lines inside
  string literals at `scripts/tests/test_mutation_python.py:151` and `:577`, which are a fixture's
  source and not this repository's roots, and it misses an assignment split across lines.
- A runtime check that imports each module and compares the value: rejected because an import runs
  each module's top level, the function-scoped site at `deploy/host-scrub/inventory.py:226` is never
  evaluated by an import, and a module loaded by `importlib` in `scripts/tests` is held to the
  dynamic-import register.
- Upper-case `ROOT` and `REPO` only: rejected because the local `repository` at
  `deploy/host-scrub/inventory.py:226` computes the same root and would go unchecked.
- A list of exempt sites: rejected because an exempt site is checked by nothing, where a listed root
  still proves its count, against the directory it names.

### D6. Placement and cost: inside the python stage, after its suites

The ruff arm runs inside `stage_python`, after the three suites, whatever they found; the suites
run whatever ruff finds, and the arm's verdict is appended to the stage's last line, whose
SPEC-054 R7 prefix stays as it is. The guard is a module of `scripts/tests`, which the stage's
first suite already discovers. No stage, job or stage list changes. The stage's measured base is
853 s (job 116733923723, run 38826135989). The install step and both arms add an amount this
delivery's own `hygiene` job measures and its red-first record names; it is expected to be well
under a minute, against a stage that takes minutes.

Chosen against:

- A new `lint` stage in `STAGES_ALL`: rejected because the stages per job are the gate's layout
  (SPEC-038 R3, held by `OWNER_LAYOUT` at `scripts/tests/test_ci_workflows.py:62-68`), and the
  python stage already holds the repository's Python guards.
- A new CI job: rejected because a job pays its own checkout and setup for a check that takes
  seconds, and the aggregate `ci` would have to need it as well.
- ruff before the suites: rejected because the stage ends only after its slowest part either way,
  so the order buys no time, and keeping the suites first leaves every existing reading of the
  stage log where it is.

### D7. Kill criteria

Each window reads "the last 30 merged pull requests", never a span of days.

1. **Live precision.** For each enabled rule, fixes over fixes plus new suppressions or
   listed-root entries, over the last 30 merged pull requests. Below 0.8 with at least 5 hits,
   propose a demotion.
2. **Rules at zero hits today.** If 2 or more of a rule's first 5 hits are false, propose a
   demotion.
3. **A demotion is a weakening.** It goes to the owner as a signed ruling, never into a commit
   alone.
4. **Tooling.** `examined 0`, a missing ruff, or a ruff off its pin is VOID, and VOID blocks. It is
   never a pass.
5. **The computed-root guard.** More than 2 `LISTED_ROOTS` additions over the last 30 merged pull
   requests means the guard's premise, that a computed root is the repository root, has stopped
   holding: propose its redesign.

Chosen against:

- A window of days: rejected because the rate of merges varies, so a fixed span measures the
  calendar rather than the rule.
- Demotion by the builder who meets a false hit: rejected because a demotion narrows the gate,
  which only a signed ruling admits.

### D8. Tests, rows, FORMAL and the pushes

Every criterion of SPEC-424 section 3 has a test written first and seen red in CI for its own
reason, or a stated `not red` with its reason. No test asserts a wall-clock duration. The rows are
in `scripts/mutation-rows.d/S42400-S42499.json`, from S42400. FORMAL is not applicable, by
surface: the install step and the gate step run one after the other in one job; the arm reads the
tree and prints one verdict; the guard reads committed files; no two actors share state, and no
`@phx covers` line names a file this delivery changes. The delivery takes three pushes: the red
commit alone, so CI reads each red; the green and rows commits; and the red-first record, whose
green lines quote the second push's run.

Chosen against:

- A TLA+ model of the install step and the gate step: rejected because they are sequential steps
  of one job, so no interleaving exists for a model to hold.
- A Lean proof of the guard's depth arithmetic: rejected because it is one comparison of a count
  with a path's depth, which the planted table of A10 holds case by case.
- One push holding red and green together: rejected because the criteria whose tests run only in CI
  would then have no red reading at all.
- Two pushes, with the record in the second: rejected because the record's green lines would then
  quote a run that had not happened yet.

### D9. One delivery, and insert-only amendments where a rule's text becomes false

SPEC-424 and this ADR carry the whole delivery. SPEC-038 R4 lists each stage's tools, and its
`python` entry stops at `cargo`, so SPEC-038 gains an insert-only last section that adds `ruff`.
SPEC-054 R7 stays true word for word, since the stage's line keeps its prefix and its two failure
rules, so it is not amended. `docs/schematics/ci-jobs-and-caches.md`, which draws one stage in
whichever job runs it, gains an insert-only last section for this delivery's stage.

Chosen against:

- A new schematic file: rejected because the stage's diagram already lives in
  `ci-jobs-and-caches.md`, and later deliveries to the gate append their sections there.
- An amendment of SPEC-054: rejected because none of R7's clauses becomes false.
- The guard in a delivery of its own: rejected because the guard and the lint are one review of the
  python stage's Python checks, decided together, and they share the stage's population.

## Decision Outcome

ruff runs in `stage_python` from one hashed pin, over five named roots, with the settled set.
Every reading that examined nothing or ran the wrong ruff is VOID and blocks. Every current site is
cured or named in the per-file table. Every computed repository root is proved by its syntax tree.

### Consequences

- A maintainer needs the pinned ruff on `PATH` to run the python stage; without it the stage fails
  by name with its install hint.
- A ruff upgrade is a delivery: the pin file, `required-version` and the test holding them equal
  change together, and the rule set's effect is re-read then.
- A new computed root at the wrong depth fails the guard by place; a root that is meant to be
  another directory needs a `LISTED_ROOTS` entry with its reason.

### Confirmation

SPEC-424 section 3's tests, run by the python stage in the `hygiene` job, its rows in
`scripts/mutation-rows.d/S42400-S42499.json`, and the stage's own last line at the delivery's head,
which names both arms' examined counts and exits.

## What would make this wrong

- The pinned release publishes no wheel for a maintainer's platform: the install then refuses, and
  the pin needs a release that does.
- Live precision falls below D7's bar, or a zero-hit rule's first hits are false: D7 proposes the
  demotion.
- A legitimate root other than the repository's keeps appearing: D7's guard criterion fires.
- The runner's Python cannot make a virtual environment: the install step fails on the delivery's
  first run, and the install's shape is decided again.

## More Information

SPEC-424 builds this decision. SPEC-038 R3 and R4 (the gate's jobs and each stage's tools), SPEC-054
R7 (the python stage's last line) and ADR-055 (the hygiene job's tools) are the decisions it
extends.
