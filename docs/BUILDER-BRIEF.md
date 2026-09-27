# Builder brief (template)

The orchestrator copies this template for each builder, fills every `<...>` field, and adds the
private environment notes the public repository must not hold. A builder works ONE issue (or a
tight bundle the orchestrator names) under ONE SPEC, in ONE locked worktree, and hands back a pull
request into `dev`.

## Your assignment

- **Issue:** #<N> (epic #<E>, wave W<w>).
- **SPEC:** `docs/specs/planned/SPEC-<NNN>-<slug>.md`. Your delivery moves it to
  `docs/specs/SPEC-<NNN>-<slug>.md` (ADR-016) and adds `docs/red-first/SPEC-<NNN>.md`.
- **Numbers you may use:** SPEC-<NNN>, ADR-<NNN> (and only those). Never take "the next free
  number": siblings are building at the same time.
- **Worktree:** `<path>` on branch `<type>/<slug>-<NNN>`, cut from `dev` at `<sha>`. It is LOCKED;
  do not unlock or remove it. Verify your working directory with `&&` before every git write.
- **Verify command:** `bash scripts/check.sh` (all stages), plus the SPEC's acceptance commands.
- **Refusals:** the files outside the SPEC's manifest; any new dependency edge the context map does
  not declare; any external crate or npm package the SPEC and its ADR do not name.

## Read before you write

1. `CLAUDE.md`, `CHARTER.md`, `docs/CONTEXT-MAP.md`, `docs/LEXICON.md`.
2. Your SPEC, and every ADR it names.
3. The packs your SPEC names, in `.packs/skills/packs/<pack>/SKILL.md`. Load a pack when your work
   touches its subject, and run its rows over your change with `python3 scripts/pack-rows.py --pack <pack>`.
4. Context7 (`resolve-library-id`, then `query-docs`) for every library API you call. Record the
   library ids that answered in your hand-back.

## The order of work

1. The SPEC is decided; amend it (in the same pull request) only when the code proves it wrong, and
   say why in the pull request.
2. A schematic before the code when you add a component, a data flow or a state machine.
3. An ADR for each real decision your SPEC leaves open, naming what it was chosen against.
4. **Tests red first.** Write each acceptance test, run it at your base, and record WHY it failed:
   `A1: red at <sha>: <the failure>`. A test that fails to compile, or selects nothing, is not red
   for its criterion's reason; give the code under test a stub that compiles and fails by
   assertion. Commit the tests before the implementation, so the red sha exists.
5. Implement inside the manifest. Then `A1: green at <sha>`.
6. `bash scripts/check.sh` whole and green. Read each stage's line; a stage that examined nothing
   is not a pass.

## Rules

- The crate graph is the context map: never add an edge to make code compile.
- Assert positive values; enumerating tests report `examined N` and refuse zero; inject the clock.
- Game math is proved against `tools/parity-oracle/goldens/`; never hand-derive a golden.
- No secret, address, host path, cloud id, chat id or personal data in any file, commit or pull
  request: this repository is public. Test data is synthetic.
- No `TODO`, `FIXME`, `HACK` or `XXX`.
- Conventional commits (subject at most 72 characters), no attribution trailers, never `--no-verify`.

## Land it

1. Add a changelog fragment under `changelog.d/` (see its README).
2. Push your branch fast-forward, then open a DRAFT pull request into `dev`:
   `gh pr create --base dev --draft --title "<type>(<scope>): <description>" --body-file <file>`.
   The body names the issue (`Closes #<N>`, never in bold), the SPEC, the red-first record, and the
   gate's output.
3. The orchestrator verifies by measurement (the gate on your head, the red-first record, the
   examined counts, the pack rows) and marks the pull request ready and merges it. Do not merge.
4. `dev` requires an up-to-date head (ADR-034). When `dev` moves while you work, bring it into your
   branch with `git fetch origin && git merge origin/dev`: a merge commit, never a rebase, so the
   shas your red-first record cites survive. Then run the gate again.
5. Scrub the pull request's body before you post it, because this repository is public:
   `python3 scripts/public-scrub.py --root . --no-tree --subject <the directory holding the body>`,
   with the body kept outside the repository.
6. Never push to `dev` or `main`: every change reaches them by a pull request. Agents never approve
   a workflow run from a fork, and never merge a pull request whose head repository is not
   `RexRenatus/deck-streak` (ADR-035).

## Writing a DeckStreak skill pack (the pack-wave method)

When your issue creates or changes a pack (an agent duty, an engineering practice or a persona):

- **R0 research.** WebSearch AND Context7 are mandatory. Use primary sources, each with its URL and
  access date, and record every Context7 library id that answered. Note deprecations and their
  dates. Send the orchestrator a checkpoint after R0 with the inventory's size and the planned rows.
  For a persona pack research the subject's pedagogy (for a language: the CEFR companion volume's
  descriptors, comprehensible input and i+1, its script and pronunciation references, learner-error
  corpora; for law: the bar subject's outline, writing and grading IRAC, the Socratic method,
  issue-spotting; for the LSAT: the official sections and question types). For a duty pack
  research the discipline behind the duty: learning science for readings and the leech doctor,
  feedback research for the writing tutor, assessment design for practice questions, notification
  ethics for the digest.
- **R1 coverage matrix, in the pack's SPEC.** Every practice maps to a check (blocking or advisory)
  or to an exclusion with its reason, citing a tracked issue `#N`.
- **R2 portable, runnable checks** that validate the agent's OUTPUT or the repository through
  `--root` or `--subject`, each with an examined count; zero examined is VOID.
- **R3 severity:** blocking for facts, privacy and safety (a cited source for law content, the CEFR
  ratio, no dates or timelines, the scrubber's deny list); advisory for voice and heuristics.
- **R4 to R6:** existing ids keep their meaning; no new dependency without an ADR; tests red first,
  each check proved by a planted-defect fixture.
- **R8 documents:** SKILL.md with the exhaustive catalog and exact counts; `checks.json`; a SPEC with
  the coverage matrix and References; an ADR naming what it was chosen against.
- `python3 .packs/scripts/pack-lint.py` judges the pack's shape.

## Hand back

- The pull request number and its head sha.
- The red-first record, and the acceptance commands' output.
- The tail of `bash scripts/check.sh` (every stage line), and `python3 scripts/pack-rows.py`'s
  summary for the packs you touched.
- The Context7 library ids that answered, and the external crates or packages you added.
- `## SKILL FEEDBACK`: where a pack, the SPEC or this brief was unclear or wrong, with the evidence.

## Private environment notes (filled by the orchestrator, never committed)

`<the tool environment to source, the machine's limits, the paths of the vendored phoenix-v2
checkout, and anything else specific to where you run>`
