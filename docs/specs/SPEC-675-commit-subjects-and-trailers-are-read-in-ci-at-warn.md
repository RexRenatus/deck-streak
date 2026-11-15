# SPEC-675: commit subjects and trailers are read in CI at warn

Path: `docs/specs/SPEC-675-commit-subjects-and-trailers-are-read-in-ci-at-warn.md`.

- **Issue:** #1392, which this SPEC closes.
- **Context(s):** the repository's Python tests, their planted fixtures, the rows band and the
  documents. No workflow, `scripts/check.sh`, script, production source, existing test or commit
  changes.
- **Decided by:** ADR-698 (commit subjects and trailers are a census of each run's range that
  starts at warn). It cites ADR-450 D6 (the promotion rule and the kill criteria) and SPEC-436 R21
  (the `WARN` and `STALE` lines), and amends neither.
- **Schematic:** none added: one test module reads the commit text of its run's range.
- **Status:** delivered by the pull request that adds this file, with its tests and
  `docs/red-first/SPEC-675.md`. Read at dev `b9a08e60`.

## 1. The problem, measured

The pull request template asks for a Conventional Commit title (`.github/pull_request_template.md`
line 11), the contributor guides ask for `type(scope): description` and no attribution trailers
(`CONTRIBUTING.md`, `AGENTS.md`, `docs/CODE_STYLE.md`), and the builder brief bounds a
subject at 72 characters (`docs/BUILDER-BRIEF.md`). No check reads a commit message: no workflow
step reads a commit range's messages, and no test module does.

Read at dev `b9a08e60`, over dev's last 300 first-parent commits and 327 commits of the open pull
requests, the arms of section 2 find no site. Each of dev's 300 is a merge GitHub wrote, 90 of them
by a bot account. The open pull requests' 327 hold 308 subjects of the house's types (docs 131,
test 110, ci 23, fix 18, feat 14, chore 9, refactor 2, perf 1; 307 of them scoped, none with `!`)
and 19 merges git or GitHub wrote; 0 carry a `Co-authored-by` or `Signed-off-by` trailer and 0 a
`BREAKING CHANGE` footer. Two arms were measured and not taken: a refusal of every trailer key
git's rule reads finds 5 hits, all false (4 a pull request title with no scope alone in the last
paragraph of a merge's body, 1 a line of prose); a 72-character bound on every subject finds 32,
all merge subjects GitHub wrote from a branch name.

That is a lower bound. Dev's first-parent history holds only its merges, so the commits each merge
brought in are read only in CI; two messages of the open pull requests were not read by this
measurement and stay in the population, unmeasured. No exact count over the whole population is
claimed: each run's census prints its own.

## 2. Requirements

R1. The census prints `examined N commit(s)` over its population; an empty population fails
    naming `examined 0 commit(s)`, which is VOID, never a pass.
R2. The population is every commit `git log HEAD^1..HEAD` lists in the checkout the `hygiene` job
    holds, each read as its id, its parent count, whether its author's name ends `[bot]`, and its
    raw message. On a pull request the checkout is the merge ref, so the range is the merge ref's
    own commit and every commit of the pull request; on a push it is the pushed tip and every
    commit its first parent lacks. No commit is left out by its author or its parents. One `read`
    line prints the population by kind: judged, merges and reverts a tool wrote, and by a bot.
R3. A shallow checkout, and a head with no first parent, fail as VOID by name.
R4. A judged subject is a Conventional Commit: a type, an optional scope in parentheses, an
    optional `!`, a colon, a space, and a description that opens with a character other than a
    space. The type is one of `build`, `chore`, `ci`, `docs`, `feat`, `fix`, `perf`, `refactor`,
    `release`, `revert`, `style` and `test`, read without regard to case. A subject not of that
    shape is the site `no-type -`.
R5. A subject of that shape with another type is the site `other-type <type>`.
R6. A judged subject longer than 72 characters is the site `long-subject <length>`.
R7. A commit of two parents or more whose subject is wholly one of the merge forms GitHub or git
    writes is a merge a tool wrote: `Merge pull request #<n> from <branch>`, the merge ref's
    `Merge <id> into <id>`, `Merge branch '<branch>'` with an optional ` of <url>` and
    ` into <branch>`, and `Merge remote-tracking branch '<branch>'` with an optional
    ` into <branch>`. Its subject is counted and not judged. A merge commit with any other subject,
    and a commit of one parent with a merge subject, are judged.
R8. A subject that is wholly `Revert "<subject>"` or `Reapply "<subject>"`, in a message that holds
    the line `This reverts commit <id>.`, is a revert git wrote, counted and not judged.
R9. The trailer block is read by git's rule. The message ends before a line that is `---` or opens
    `--- `. Its last paragraph, when a blank line comes before it, is the block when each of its
    lines is a trailer, or when a line opens with a prefix git writes by itself (`Signed-off-by:`
    or `(cherry picked from commit `) and at least a quarter of its lines are trailers. A trailer
    line opens with a key of ASCII letters, digits and hyphens, then optional spaces or tabs and a
    colon; a line that opens with a space or a tab, after a trailer, continues that trailer.
R10. A trailer whose key is `Co-authored-by` or `Signed-off-by`, in any case, is the site
    `attribution-trailer <key>` on every commit, a merge included. Every other key is counted on a
    line `INFO commit-message <id> trailer <key>` and never refused. A `BREAKING CHANGE:` footer
    holds a space in its token, so it is no trailer.
R11. Each site of the population that is not listed prints `WARN commit-message <id> <form> <key>`,
    `<id>` the first twelve characters of the commit's id; it passes at `warn` and fails at
    `fail`.
R12. Each listed entry's commit is read by its id as well. An entry that no site of the population
    or of those commits matches prints `STALE commit-message <id> <form> <key>` and fails at both
    levels; an id the checkout does not hold reads no commit, so its entry is stale.
R13. `LISTED` holds the sites of commits reachable from dev's tip, each with its reason. It is
    empty at the build's cut (section 1); each site the `hygiene` job of the second push prints is
    named in the red-first record with that job's id, and a site of a commit reachable from dev's
    tip is listed there with its reason.
R14. `LEVEL` is `warn`. Moving it to `fail` is its own delivery, under ADR-450 D6's promotion rule.
R15. The planted populations are text files under `scripts/tests/fixtures/commit-messages/`, each
    named `*.fixture` and read as text: each commit opens with a line
    `== <id> <parents> <author>`, and its message is every line up to the next such line.
R16. The census changes no commit, workflow, script, production source or existing test, and
    reads commit text only.

## 3. Acceptance criteria of SPEC-675

| id | criterion | decided by |
|---|---|---|
| A1 | R4, R5, R6: of twelve planted subjects, the six of a house type (scoped, bare, with `!`, in capitals, `release`, exactly 72 characters) and a squash title are clean; a subject with no type, with no description and with an empty scope are `no-type -`; `wip` is `other-type wip`; a subject of 73 characters is `long-subject 73` | `test_commit_messages.PlantedSubjects.test_a_house_type_subject_is_clean_and_every_other_subject_is_a_site` |
| A2 | R9, R10: `Co-authored-by` and `Signed-off-by` in the block, a key in capitals, a space before the colon, a folded value and a sign-off under three lines of prose are sites; `Refs` and `BREAKING-CHANGE` are counted; a `BREAKING CHANGE` footer, a co-author under prose, one in a middle paragraph and one after the `---` line are neither | `test_commit_messages.PlantedTrailers.test_an_attribution_key_in_gits_block_is_a_site_and_every_other_key_is_counted` |
| A3 | R7, R8, R10: GitHub's merge subject, a long one, the merge ref's, git's two merge forms and git's revert and reapply subjects are counted and not judged; a co-author trailer on a merge is a site; a merge subject on one parent, another subject on a merge and a revert subject with no body line are judged | `test_commit_messages.PlantedToolSubjects.test_merges_and_reverts_a_tool_wrote_are_counted_and_every_other_subject_is_judged` |
| A4 | R2, R10: the `read` line counts eleven planted commits by kind and by a bot, and one counted key prints its own `INFO` line | `test_commit_messages.PlantedToolSubjects.test_the_read_line_counts_each_kind_and_each_counted_key` |
| A5 | R2, R4, R10: a bot's commit is read by the same forms: a bump with no type and a sign-off is two sites, a bot's Conventional subject is clean | `test_commit_messages.PlantedBots.test_a_bots_commit_is_read_by_the_same_forms` |
| A6 | R11 to R14: over a planted population and listing, the exact `WARN` lines pass at `warn` and fail at `fail`; a listed site of the population and one read by its id print nothing; an entry its commit no longer matches and an id no commit holds print their exact `STALE` lines and fail at both levels; `LEVEL` is `warn` and `LISTED` is empty | `test_commit_messages.PlantedVerdict.test_the_level_decides_whether_an_unlisted_site_fails_and_a_stale_entry_always_does` |
| A7 | R1: an empty planted population fails naming `examined 0 commit(s)`, and a level other than `warn` or `fail` is refused | `test_commit_messages.PlantedVerdict.test_an_empty_population_is_void` |
| A8 | R2, R3: a planted log answer splits into each commit's id, parent count, bot flag and message; a shallow checkout and a head with no first parent are VOID by name | `test_commit_messages.PlantedRange.test_the_log_reader_splits_each_record_and_an_unreadable_range_is_void` |
| A9 | R1, R2, R11, R12: in the `hygiene` job, the run's range holds `HEAD`, its count prints above zero, its `read` line and each `WARN` line print, and nothing is refused at `warn` | `test_commit_messages.TheRunsRange.test_every_commit_of_the_runs_range_is_read_at_the_level_against_the_listing` |

Every planted case is text a fixture or the test spells literally, and every expected site, line
and count is a literal the test writes, never a value the census computes. Each test asserts its
positive artifact before any examined count is refused.

## 4. File manifest

| path | part | change |
|---|---|---|
| `scripts/tests/test_commit_messages.py` | the census and its tests | added: R1 to R15, the `Planted*` tests and `TheRunsRange` |
| `scripts/tests/fixtures/commit-messages/subjects.fixture` | planted subjects | added: R4 to R6 |
| `scripts/tests/fixtures/commit-messages/trailers.fixture` | planted trailers and footers | added: R9, R10 |
| `scripts/tests/fixtures/commit-messages/tools.fixture` | planted merges and reverts | added: R7, R8 |
| `scripts/tests/fixtures/commit-messages/bots.fixture` | planted bot commits | added: R2 |
| `scripts/tests/fixtures/commit-messages/empty.fixture` | the empty population | added: R1 |
| `scripts/mutation-rows.d/S67500-S67599.json` | the rows | added: section 9 |
| `docs/specs/SPEC-675-commit-subjects-and-trailers-are-read-in-ci-at-warn.md` | the spec | added |
| `docs/decisions/ADR-698-commit-subjects-and-trailers-are-a-census-of-each-runs-range-that-starts-at-warn.md` | the decision | added |
| `docs/red-first/SPEC-675.md` | the record | added |
| `changelog.d/commit-message-census.md` | the fragment | added |

## 5. What this does NOT cover

- It reads no pull request title; the template's ask for the title stays a checklist line.
- It changes no workflow: the `hygiene` job's checkout already holds the whole history.
- It rewrites, amends or force-pushes no commit; a site is listed or printed, never changed.
- It reads no commit of a range a run does not hold: a push that carries several merges is read
  for its last, and each earlier merge's commits were read by their own pull request's run.
- It judges no body line, no footer form and no description's wording.

## 6. Risks

- **A tree-scanning guard reads the new module.** `scripts/tests/test_ci_workflows.py` places every
  name a test module uses: this one imports only `re`, `subprocess`, `unittest` and `_support`, and
  calls nothing by a name held in data, so it needs no register entry. `scripts/tests/test_temp_hygiene.py`
  reads every Python test, and this one writes no file. The fixtures end `.fixture`, so no runner
  collects them.
- **A sibling pull request lands first.** The census reads only its run's range, so no sibling's
  change to the tree moves its listing; each list is re-derived at the build's base.
- **A form a tool writes is missed.** A merge or revert subject outside R7 and R8 is judged, so it
  prints at warn and is read before any promotion.

## 7. What only CI proves

- `TheRunsRange` reads the run's range in the `hygiene` job, by name in its module output; each `WARN` line a
  pull request's commits draw prints there.

## 8. Formal model

No state, actor or write path is added. The census is a pure reader of commit text; its classes are
closed by the planted populations of the `Planted*` tests and by section 9's rows. No model checker
runs.

## 9. Mutation rows (S67500-S67599)

Rows S67500 to S67527, in `scripts/mutation-rows.d/S67500-S67599.json`, each over
`scripts/tests/test_commit_messages.py`. Each find is taken from the green commit's text and occurs
exactly once in the file.

| rows | what each mutant removes or breaks | killer |
|---|---|---|
| S67500-S67506, S67523 | the other-type arm, the no-type arm, the length arm, the bound, the scope, the `!`, the case rule, the `release` type | `PlantedSubjects.test_a_house_type_subject_is_clean_and_every_other_subject_is_a_site` |
| S67507-S67510 | the merge arm, its parent rule, the revert arm, its body line | `PlantedToolSubjects.test_merges_and_reverts_a_tool_wrote_are_counted_and_every_other_subject_is_judged` |
| S67511-S67515, S67522, S67524 | the attribution arm, the sign-off key, the block rule, the quarter rule, the `---` end, the folded line, the space before the colon | `PlantedTrailers.test_an_attribution_key_in_gits_block_is_a_site_and_every_other_key_is_counted` |
| S67516-S67519, S67521 | the stale refusal, the fail refusal, the listing, the read by id, `LEVEL`'s `warn` | `PlantedVerdict.test_the_level_decides_whether_an_unlisted_site_fails_and_a_stale_entry_always_does` |
| S67520 | the empty population's refusal | `PlantedVerdict.test_an_empty_population_is_void` |
| S67525-S67527 | the shallow rule, the first-parent rule, the parent count | `PlantedRange.test_the_log_reader_splits_each_record_and_an_unreadable_range_is_void` |
