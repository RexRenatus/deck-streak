# SPEC-145: v1.0.0 is released from main after the day alone, and a read-only check proves the tag, CI and the security settings

- **Wave:** W8. **Issue:** #64 (epic #9), both criteria: v1.0.0 is tagged on main and deployed by
  `deploy/deploy.sh`, and CI is green on dev and main with secret scanning, push protection and
  Dependabot on. **Context(s):** the repository (`scripts/release-check.py`, `RELEASING.md`) and
  `deploy` (the deploy from the tag and the restore drill after it, both landed).
- **Decided by:** ADR-010 and ADR-017 (a release is a tag on main; nothing deploys from a branch
  or from CI), ADR-034 (main is never merged back, and a hotfix goes through dev), ADR-035 (required checks come
  from Actions, and forks never reach main) and ADR-145 (v1.0.0 follows the day alone, and the
  repository's settings are read by a check that cannot write).
- **Prerequisites:** SPEC-033, SPEC-034, SPEC-062 and SPEC-064 (landed: the rulesets, the release
  workflow, `deploy/deploy.sh`, and the backups with their restore drill); SPEC-144 (this wave,
  unlanded: the day alone that v1.0.0 follows). **Mutation band:** `S14500-S14599`.
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-145.md` (ADR-016).

## 1. The problem, measured

- **The release path exists, and has shipped once.** At `dev` 703097b `RELEASING.md` gives the
  preparation on dev, the release pull request from dev into main with a merge commit, the
  annotated tag on main, `bash deploy/deploy.sh vX.Y.Z` from the maintainer's machine, no merge
  back into dev, and the rollback (SPEC-034; #360 declared the release model). `v0.1.0` was tagged
  by it. `deploy/deploy.sh` refuses a tag that is not annotated SemVer on `origin/main`, and ends
  `deploy: <tag> is current`.
- **Nothing reads the second criterion.** "CI is green on dev and main" and "secret scanning, push
  protection and Dependabot are on" are facts of GitHub, not of the tree. The rulesets
  (`.github/rulesets/dev.json`, `main.json`) require the contexts `ci` and `fragment`;
  `.github/dependabot.yml` exists. No script reads a branch head's check runs or the repository's
  `security_and_analysis`, and the settings are the owner's to change, never a script's.
- **The runbook does not say where v1.0.0 falls, or that the backups are proved after it.** The
  cutover ends with the day alone (SPEC-144); `RELEASING.md` names neither it nor the restore drill
  (SPEC-064 R5) after a deploy.

## 2. Requirements

R1. `scripts/release-check.py TAG --repo OWNER/REPO` (standard library and the `gh` CLI only)
    runs these checks, each printing `check=<name> verdict=<ok|refused|unreadable> reason=<r>`:
    - `tag`: `TAG` matches `^v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$`, its ref is an
      annotated tag (`git/ref/tags/<tag>` names an object of type `tag`), and its commit is on
      `main` (`compare/<tag commit>...main` answers `identical` or `ahead`);
    - `release`: the release for `TAG` exists, is not a draft, and carries `deck-streak-<tag>.tar.gz`
      and `SHA256SUMS`;
    - `ci-dev` and `ci-main`: every check run on the branch's head commit is `completed` with a
      conclusion of `success`, `skipped` or `neutral`, and each context the branch's ruleset
      requires (`ci`, `fragment`, read from `.github/rulesets/<branch>.json`) is among them with
      `success`;
    - `secret-scanning` and `push-protection`: `security_and_analysis.secret_scanning.status` and
      `security_and_analysis.secret_scanning_push_protection.status` of `repos/<repo>` are
      `enabled`;
    - `dependabot-alerts`: `repos/<repo>/vulnerability-alerts` answers 204;
    - `dependabot-updates`: `repos/<repo>/automated-security-fixes` answers `enabled` true and
      `paused` false, and `.github/dependabot.yml` exists on `main`.
    It ends `RELEASE CHECK OK` (exit 0) or `RELEASE CHECK REFUSED: <n>` (exit 1); usage is 2.
R2. **It only reads.** Every call is `gh api` with the GET method: no `-X`/`--method` other than
    GET, and no `-f`, `-F`, `--field`, `--raw-field` or `--input`. It changes no setting and no ref.
R3. **An answer it cannot read refuses.** A `gh` call that exits non-zero, outlasts
    `RELEASE_CHECK_TIMEOUT` seconds (default 30; a test sets 1), or answers a shape the check does not know makes that check `unreadable`, which
    counts as refused: an admin-read permission the caller lacks never passes as a setting that is
    on.
R4. **The output carries no value of the account.** Each line names the check, its verdict and a
    reason from a closed set (`not_semver`, `lightweight`, `not_on_main`, `no_release`,
    `draft`, `asset_missing`, `run_failed`, `run_pending`, `context_missing`, `disabled`,
    `paused`, `file_missing`, `unreadable`); never a sha, a run's name, a token or a URL.
R5. `RELEASING.md` gains `## 8. v1.0.0, after the cutover`, in this order: the day alone is
    recorded (SPEC-144, `deploy/cutover.md`); the preparation pull request sets `1.0.0` in the
    workspace `Cargo.toml` and every `package.json` that declares a version, and compiles
    `CHANGELOG.md`'s `## [1.0.0]`; the release pull request from `dev` into `main`, merged with a
    merge commit and never merged back (ADR-034); the annotated tag `v1.0.0` on `main`;
    `python3 scripts/release-check.py v1.0.0 --repo <owner>/<repo>` reads `RELEASE CHECK OK`;
    `bash deploy/deploy.sh v1.0.0` ends `deploy: v1.0.0 is current`; and the restore drill (SPEC-064
    R5) runs once after the deploy and passes, so the backups are proved to cover the database the
    release serves. The drill's output and the deploy's lines stay in the maintainer's private
    record.
R6. `RELEASING.md` §4 gains one sentence: after every deploy, the restore drill's next run is the
    proof that the backups cover the database; v1.0.0 runs it at once (§8).
R7. The delivery is done when the release check reads `RELEASE CHECK OK` for `v1.0.0` and the deploy
    reads `deploy: v1.0.0 is current`. Both are quoted in the release pull request as those two
    lines only, with no timing and no host detail (ADR-059).

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | a tag whose commit is not on main is refused with `not_on_main` | `test_a_tag_not_on_main_is_refused` |
| A2 | a lightweight tag and a tag that is not SemVer are refused, each with its reason | `test_a_lightweight_or_non_semver_tag_is_refused` |
| A3 | a release that is a draft, or lacks its tarball or `SHA256SUMS`, is refused | `test_a_release_without_its_assets_is_refused` |
| A4 | one failed and one pending check run on dev's or main's head are each refused | `test_a_failed_or_pending_run_on_dev_or_main_is_refused` |
| A5 | a head whose runs lack a context its ruleset requires is refused with `context_missing` | `test_a_required_context_missing_is_refused` |
| A6 | each of the four settings off, Dependabot paused, and `dependabot.yml` absent are each refused by name | `test_each_security_setting_off_is_refused_by_name` |
| A7 | every `gh` call the check makes is a GET with no field and no input | `test_the_check_only_reads` |
| A8 | a `gh` that answers 403 or times out makes its check `unreadable` and the run refused | `test_an_unreadable_answer_refuses` |
| A9 | every input green reads `RELEASE CHECK OK` and exit 0, and no line carries a sha, a URL or a run's name | `test_every_green_input_reads_release_check_ok` |
| A10 | `RELEASING.md` orders the day alone, the preparation, the release pull request, the tag, the check, the deploy and the restore drill | `test_the_runbook_orders_v1_after_the_day_alone` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_release_check.py -k test_a_tag_not_on_main_is_refused
A2: python3 -m unittest discover -s scripts/tests -p test_release_check.py -k test_a_lightweight_or_non_semver_tag_is_refused
A3: python3 -m unittest discover -s scripts/tests -p test_release_check.py -k test_a_release_without_its_assets_is_refused
A4: python3 -m unittest discover -s scripts/tests -p test_release_check.py -k test_a_failed_or_pending_run_on_dev_or_main_is_refused
A5: python3 -m unittest discover -s scripts/tests -p test_release_check.py -k test_a_required_context_missing_is_refused
A6: python3 -m unittest discover -s scripts/tests -p test_release_check.py -k test_each_security_setting_off_is_refused_by_name
A7: python3 -m unittest discover -s scripts/tests -p test_release_check.py -k test_the_check_only_reads
A8: python3 -m unittest discover -s scripts/tests -p test_release_check.py -k test_an_unreadable_answer_refuses
A9: python3 -m unittest discover -s scripts/tests -p test_release_check.py -k test_every_green_input_reads_release_check_ok
A10: python3 -m unittest discover -s scripts/tests -p test_release_runbook.py -k test_the_runbook_orders_v1_after_the_day_alone
```

## 3a. What the box run judges

The box run (`scripts/box-packs.sh`, ADR-069) judges this over the committed tree and posts its
verdict on the pull request as the `box/packs` status. It has no line in the acceptance fence. The
wiring change handed back with this delivery lifts the release-ops pack's last deferral, whose
issue (#360) is closed.

| id | criterion | decided by |
|---|---|---|
| B1 | every check of the release-ops pack passes over the repository at the release head, examining the three rulesets under `.github/rulesets/`, the release workflow, the workspace `Cargo.toml` and every `package.json` at `1.0.0`, `CHANGELOG.md`'s `## [1.0.0]`, the fragments and `RELEASING.md` | the release-ops pack |

## 4. File manifest

| file | context | change |
|---|---|---|
| `scripts/release-check.py` | repo | added: the read-only check (R1-R4) |
| `scripts/tests/test_release_check.py` | repo | added: A1-A9, with a stub `gh` on `PATH` that records its arguments |
| `scripts/tests/test_release_runbook.py` | repo | added: A10 |
| `RELEASING.md` | docs | changed: §4's sentence and §8 (R5, R6) |
| `scripts/mutation-rows.d/S14500-S14599.json` | repo | added: §9's rows |
| `docs/red-first/SPEC-145.md` | docs | added: the red-first record |
| `docs/specs/SPEC-145-v1-0-0-is-released-from-main-after-the-day-alone-and-a-read-only-check-proves-the-tag-ci-and-the-security-settings.md` | docs | moved from `docs/specs/planned/` |
| `changelog.d/feat-release-check.md` | repo | added: the fragment |

The version bump and `CHANGELOG.md`'s `## [1.0.0]` are the release preparation's own pull request
(`RELEASING.md` §1), not this delivery's.

## 5. What this does NOT do

- It changes no repository setting: secret scanning, push protection and Dependabot are the
  owner's, and the check only reads them (#64).
- It merges nothing back into dev and adds no path to main but the release pull request (#360).
- It adds no deploy workflow: the deploy runs from the maintainer's machine (#42).
- It takes no cutover step; the release follows the day alone (#63).

## 6. Risks

- **A setting read as on when it could not be read.** Prevented by R3, and detected by A8 and row
  S14505.
- **A check that writes.** Prevented by R2, and detected by A7 and row S14504.
- **A pending run read as green.** Prevented by R1's `completed` rule, and detected by A4 and row
  S14502.
- **A tag cut from a branch other than main.** Prevented by R1's `tag` check and `deploy/deploy.sh`'s
  own refusal, and detected by A1 and row S14501.
- **A value of the account in a public pull request.** Prevented by R4's closed reasons, and
  detected by A9 and row S14506.
- **A release that ships before the cutover is proved.** Prevented by R5's order; the check cannot
  read the host's ledger, so the order is the runbook's and ADR-145's.

## 7. Parity goldens

None: the release is DeckStreak's own.

## 8. Tables and the v9 import

No table.

## 9. Mutation rows

| row | target | what it guards | killer |
|---|---|---|---|
| `S14501-ON-MAIN` | `scripts/release-check.py` | the tag's commit is on main | `test_release_check::test_a_tag_not_on_main_is_refused` |
| `S14502-COMPLETED` | `scripts/release-check.py` | a pending run is not green | `test_release_check::test_a_failed_or_pending_run_on_dev_or_main_is_refused` |
| `S14503-CONTEXT` | `scripts/release-check.py` | each required context is present and green | `test_release_check::test_a_required_context_missing_is_refused` |
| `S14504-GET-ONLY` | `scripts/release-check.py` | every call is a read | `test_release_check::test_the_check_only_reads` |
| `S14505-UNREADABLE` | `scripts/release-check.py` | an unreadable answer refuses | `test_release_check::test_an_unreadable_answer_refuses` |
| `S14506-PUSH-PROTECTION` | `scripts/release-check.py` | push protection off is refused | `test_release_check::test_each_security_setting_off_is_refused_by_name` |

No killer calls a network: `gh` is a stub on `PATH` that answers from fixtures, and a stub that
sleeps past `RELEASE_CHECK_TIMEOUT` (1 second in the test) is killed by the check's own
timeout, which A8 bounds.
