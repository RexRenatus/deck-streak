# SPEC-344: the Apple build runs from one job body when an Apple or FFI path changes and on every release tag

- **Issue:** #622 (the app campaign's macOS CI job). **Context(s):** none (the workflows and their
  census, not a bounded context).
- **Campaign row:** SPEC-334 rows 1.2 and 1.5, R20.
- **Decided by:** ADR-355 (D1 to D5), under ADR-335 ("Builds": a macOS CI job builds the Apple
  and FFI paths when they change and on every tag), ADR-340 (the sync server's own unit and its
  budget entry), ADR-345 D4 (the macOS runner admitted to one workflow file by name) and ADR-058
  (`allow-git` names the engine fork and the URL crate's fork, nothing else).
- **Status:** this pull request delivers R1 to R8.

## 1. The problem, measured

Read at DeckStreak `dev` `1eec0870e67e801fc3aa279318219d6986116ec7` (DEV), with #656 at
`b9dbe8caad4ab7fdfc39d4984441406c772149d7` and #657 at `7ab160dc48dfe78a92a9feada1b123185f0fcd01`.
`R=<the DeckStreak checkout>` in every command.

### 1.1 What the Apple build runs on, and who reads it

| # | measured | figure | command |
|---|---|---|---|
| M1 | `xcframework.yml`'s triggers at DEV | a pull request into `dev` (opened, synchronize, reopened) changing `crates/ffi/**`, `Cargo.lock` or the workflow itself, and `workflow_dispatch`; no `push`, so no tag ever starts it (lines 13 to 21) | `git -C $R show 1eec0870:.github/workflows/xcframework.yml \| sed -n '13,21p'` |
| M2 | what it builds at DEV | one job: the two static libraries, the Swift bindings, the modulemap check, the XCFramework, the consumer typecheck, the report and the upload (lines 32 to 148) | `git -C $R show 1eec0870:.github/workflows/xcframework.yml \| sed -n '32,148p'` |
| M3 | its concurrency block | `xcframework-${{ github.event_name == 'pull_request' && github.ref \|\| github.run_id }}`, cancel `${{ github.event_name == 'pull_request' }}` (lines 28 to 30) | same file, `sed -n '28,30p'` |
| M4 | whether `ci` reads it | no: the aggregate `ci` job needs 14 jobs, all in `ci.yml` (line 840) | `git -C $R show 1eec0870:.github/workflows/ci.yml \| sed -n '837,866p'` |
| M5 | whether a ruleset requires it | no: `dev` and `main` require `ci` and `fragment` only (`dev.json` lines 41 to 50, `main.json` lines 31 to 34) | `git -C $R show 1eec0870:.github/rulesets/dev.json` and `main.json` |
| M6 | what #656 changes in it | its own diff (three-dot, since #656's base is `7507d8a2`, not DEV): the `ios/**` path, a fixture step, and two jobs, `harness-wire` and `harness`, the second needing `xcframework`; the file grows to 482 lines. The two-dot diff the brief names also shows `ci.yml` and `mutation-weekly.yml`, which are dev's later changes, not #656's | `git -C $R diff 1eec0870...b9dbe8ca -- .github/workflows/` and `git -C $R diff 1eec0870 b9dbe8ca -- .github/workflows/` |
| M7 | what #657 changes in the workflows | `release.yml` only: an `audit-sync-server` job the release job needs; `xcframework.yml` untouched | `git -C $R diff 1eec0870...7ab160dc --stat` |

### 1.2 The Apple and FFI paths, a measured population

| glob | DEV | #656 | #657 | why it is an input of the Apple build | command |
|---|---|---|---|---|---|
| `crates/ffi/**` | 8 | 13 | 8 | the adapter | `git -C $R ls-tree -r --name-only <sha> \| grep -c '^crates/ffi/'` |
| `ios/**` | 0 | 21 | 0 | the harness tree (#656); every `.swift`, `.xcconfig`, `.plist`, `.xcprivacy` and `project.yml` at #656 is under it (0 outside) | same, `'^ios/'`; and `grep -E '\.swift$\|\.xcconfig$\|\.plist$\|\.xcprivacy$\|project\.yml$' \| grep -v '^ios/'` (0 lines) |
| `Cargo.lock` | 1 | 1 | 1 | the locked graph | `'^Cargo\.lock$'` |
| `Cargo.toml` | 1 | 1 | 1 | the workspace's engine and generator pins, its lints, its profiles and the `[patch]` entry; **not in the filter at DEV** | `'^Cargo\.toml$'`; `git -C $R show 1eec0870:Cargo.toml \| grep -n '^\['` |
| `rust-toolchain.toml` | 1 | 1 | 1 | the compiler the two iOS targets are added to; **not in the filter at DEV** | `'^rust-toolchain'` |
| `.github/workflows/xcframework.yml` | 1 | 1 | 1 | the job body | `'^\.github/workflows/xcframework\.yml$'` |

The adapter depends on the engine and the generator alone (`crates/ffi/Cargo.toml` lines 14 to 19,
ADR-345 D1), so no other workspace crate is an input. Of DEV's history, 28 commits touch
`Cargo.toml` and 84 touch `Cargo.lock`; exactly 1 touches `Cargo.toml` without `Cargo.lock`
(`79e2607c`, a profile change), and 1 commit ever touches `rust-toolchain.toml`. Adding both to the
filter therefore costs about one extra run in DEV's history and closes the two inputs the filter
misses.

```
comm -23 <(git -C $R log --format=%H 1eec0870 -- Cargo.toml | sort) \
         <(git -C $R log --format=%H 1eec0870 -- Cargo.lock | sort)     # 1 line: 79e2607c
git -C $R log --format=%H 1eec0870 -- rust-toolchain.toml | wc -l         # 1
```

### 1.3 The git-sourced crates and the source gate

| # | measured | figure | command |
|---|---|---|---|
| M8 | `source = "git+` lines in `Cargo.lock` | 6 at each of DEV, #656 and #657: 5 from the engine fork, 1 from the URL crate's fork | `git -C $R show <sha>:Cargo.lock \| grep -c 'source = "git+'` |
| M9 | the gate on them | `deny.toml` `[sources]`: `unknown-git = "deny"` (line 61), `allow-git` names exactly the engine fork (line 69) and the URL crate's fork (line 70); `[graph] all-features = true` with no target filter (lines 4 to 5), so the audit judges the Apple targets' graph too. Identical at #656 and #657 | `git -C $R show 1eec0870:deny.toml`; `git -C $R diff 1eec0870 <sha> -- deny.toml` (empty) |
| M10 | who runs it | `cargo deny` in the `audit-rust` stage of `ci.yml`'s `rust` job, on every pull request and every push to `dev` and `main`; `test_engine_pin.py` lines 146 to 149 hold `allow-git` to exactly the two | `test_ci_workflows.py` line 51 (`OWNER_LAYOUT`) |
| M11 | what the Apple build reads | every `cargo` command passes `--locked` (`xcframework.yml` lines 65 and 72; #656's fixture step too) | `grep -n 'cargo ' ` over the file |

Without line 69, `cargo deny` refuses `anki` (`Cargo.lock` lines 60 to 62), `anki_i18n` (148 to 150),
`anki_io` (169 to 171), `anki_proto` (180 to 182) and `anki_proto_gen` (199 to 201); without line
70 it refuses `percent-encoding-iri` (2995 to 2997). At #656 the same six sit at lines 48 to 50,
133 to 135, 154 to 156, 165 to 167, 184 to 186 and 2796 to 2798. **No entry is missing**: the issue's
"allow-git entry the pinned crates need" is already on `dev` (SPEC-055, ADR-058). The FFI adapter
added no git source (its generator comes from the registry at an exact pin). The FSRS-7 crate,
whose own entry ADR-338 decides, is in no lockfile at any of the three refs.

### 1.4 The sync unit and its host-budget entry

| # | measured | figure | command |
|---|---|---|---|
| M12 | the unit and the entry at DEV | `deploy/systemd/deck-streak-sync-server.service`; `deploy/host-budget.json` names it among its units | `git -C $R show 1eec0870:deploy/host-budget.json` |
| M13 | render tests at DEV | `TheTemplatesFitTheHostBudget.test_every_unit_ceiling_matches_the_host_budget_and_high_is_below_max` (`test_deploy_templates.py` lines 974 to 989: every unit renders, the budget names every unit and no other, each ceiling equals its entry); `TheSyncServerRunsAsItsOwnUnit.test_the_sync_server_runs_hardened_within_its_entry_and_the_share_holds_it` (lines 1547 to 1598: the sync unit's own entry, which the unit equals, inside the share); SPEC-337 A3 and A4 | `git -C $R show 1eec0870:scripts/tests/test_deploy_templates.py \| sed -n '974,989p;1547,1598p'` |
| M14 | what #657 adds | two units (the snapshot's archive and its drill) with their entries, and four tests (`test_the_sync_route_alone_is_logged_without_its_key`, `test_the_sync_route_carries_no_web_cookie`, `test_the_sync_family_runs_as_its_own_user`, `test_the_sync_server_reaches_loopback_peers_only`) | `git -C $R diff 1eec0870...7ab160dc -- deploy/host-budget.json scripts/tests/test_deploy_templates.py` |

**No render test is missing**: the issue's third item was delivered by SPEC-337 (#617) and is
extended by SPEC-340 (#628). This delivery adds none.

### 1.5 What the census asks of a new trigger or job

Read at DEV, `scripts/tests/test_ci_workflows.py` (6483 lines) and
`scripts/tests/test_workflow_concurrency.py` (2067 lines).

| rule | where | what a new trigger or job meets |
|---|---|---|
| read-only token | `test_ci_workflows.py` 114 to 121 | every workflow's `permissions` is exactly `{contents: read}` |
| pinned actions | 123 to 128, `entries` 1446 to 1457, `PINNED` line 32 | every `uses` anywhere matches `owner/repo[/path]@<40 hex>`; `entries` collects a call job's `uses` as well, so a local call `./.github/workflows/<file>` is **refused today** |
| the admitted runner | `ADMITTED_RUNNERS` line 39, `admitted_runner` 85 to 90, tests 130 to 150 | the macOS runner is admitted to `xcframework.yml` alone, by file name |
| the aggregate | 159 to 165 | every job of `ci.yml` is in `ci`'s `needs` (reads `ci.yml` only) |
| required names | 639 to 652 | no job reports a required context (`ci`, `fragment`) under a non-pull-request event |
| secrets | `secret_and_checkout_problems` from 2950 | no `secrets.*` read but the token, and no `secrets: inherit` on a call job |
| file reads in test modules | census 3663 to 3895, `NOT_WORKFLOW_READS` 3905, `DYNAMIC_IMPORTS` 5294 | a module that imports the reader reads a workflow only through `workflow_file_text`; any other read there, and any dynamic import anywhere in `scripts/tests`, is listed with its exact count |
| the pull-request rule | `test_workflow_concurrency.py` 70 to 91 | a workflow with a `pull_request` trigger has exactly the group `<name>-${{ github.event_name == 'pull_request' && github.ref \|\| github.run_id }}` and cancel `${{ github.event_name == 'pull_request' }}` |
| the release class | 1140 to 1153, with `release_problems` 241 to 285, `closed_by_construction` 487 to 530, `membership` 997 to 1016 and `release_class_problems` 1035 to 1117 | a workflow a tag push can start, and every workflow it calls, is a release workflow: its group reads `github.ref` alone, cancel is false, `queue: max`, and no other block's literal start is a prefix of its own or the reverse |
| calls, in every workflow | `release_class_problems` 1069 to 1084 | every call job, in any workflow, calls `./.github/workflows/<file>` with the file here and taking `workflow_call` (1074, 1078); every release workflow, a callee included, is read by the parser's schema down to its jobs' keys (1088) |
| a workflow with no block | `release_problems` 254 to 255, applied by 1140 to 1153 to tag-started files only | a block is owed by a workflow a tag starts; a callee that holds none meets no rule, and its runs take the caller's block |

**A tag trigger on `xcframework.yml` is refused by construction.** A `push: tags:` filter puts the
file in the release class, whose group must read `github.ref` alone and queue; its `pull_request`
trigger holds it to a group that must equal the pull-request form exactly, which reads
`github.event_name` and `github.run_id`. One block cannot be both, so one of the two rules would
have to be weakened. The same holds for any workflow a tag workflow calls. Measured by reading the
two rules; the build re-measures it in the red-first record of A3.

#656 adds `TheHarnessLinksItsOwnRunsFramework`, which reads `load("xcframework.yml")["jobs"]`
(the hunk at line 6217 of #656's file); it keeps holding once the jobs stay in that file.

## 2. Requirements

R1. The Apple build's job body stays in `.github/workflows/xcframework.yml`, the one file the
    hardening test admits to the macOS runner; `ADMITTED_RUNNERS` is unchanged. Its `on:` becomes
    `workflow_call` and `workflow_dispatch`, with no `pull_request` and no `push`. It carries no
    concurrency block: each caller's block governs the run. No expression in it reads
    `github.event.pull_request`, `github.head_ref` or `github.base_ref`, so a tag run executes the
    same steps a change run does.
R2. `.github/workflows/apple-on-change.yml`, named `apple-on-change`, runs on a pull request into
    `dev` (opened, synchronize, reopened) whose paths are exactly `crates/ffi/**`, `ios/**`,
    `Cargo.lock`, `Cargo.toml`, `rust-toolchain.toml`, `.github/workflows/xcframework.yml` and
    `.github/workflows/apple-on-change.yml`. Its one job calls `./.github/workflows/xcframework.yml`.
    Its concurrency block is the pull-request rule's, under its own name.
R3. `.github/workflows/apple-on-tag.yml`, named `apple-on-tag`, runs on a push of a tag and on
    nothing else. Its tag filter is exactly `release.yml`'s, with no branch and no path filter (a
    path filter is never evaluated for a tag push). Its one job calls
    `./.github/workflows/xcframework.yml`. Its block is the release class's:
    `apple-on-tag-${{ github.ref }}`, cancel-in-progress `false`, `queue: max`.
R4. Each caller defaults its token to `contents: read` and passes no secret (no `secrets:` key). No
    job of the three is in `ci`'s `needs`, and no ruleset requires one: the Apple build stays an
    advisory check (ADR-355 D2).
R5. The pin census judges the population it judges today: every value the walk `entries` finds
    under a `uses` key, in the same order, under the same examined count, refused with the same
    text (`<file> uses <ref>`). A value is admitted when it is pinned by a full commit SHA, as
    today, or when it is a call job's own `uses` (the value at `jobs.<id>.uses`) and fullmatches
    the concurrency census's `LOCAL_CALL`, which moves into `test_ci_workflows.py` and is imported
    back, so one pattern serves both censuses. That the called file is here and takes
    `workflow_call` stays the release-class rule's (section 1.5, lines 1073 to 1078), which already
    walks every call in every workflow and is not copied. Every reference the census refused
    before stays refused.
R6. Every `cargo` command in `xcframework.yml`'s run steps passes `--locked`, so a change run and a
    tag run each resolve only the graph the dependency audit judged against `deny.toml`.
    `deny.toml`, `Cargo.lock` and `Cargo.toml` are unchanged (section 1.3).
R7. No deploy file and no render test changes: the sync unit and its budget entry are rendered and
    held by SPEC-337's A3 and A4 on `dev` and extended by SPEC-340 (section 1.4).
R8. `release.yml`, `ci.yml` and the rulesets are unchanged. Neither caller builds a release asset,
    uploads, signs, publishes or deploys.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the change caller runs the build on each Apple and FFI path and on no other: its trigger is a pull request into `dev` with R2's types and exactly R2's paths; a planted change to `crates/ffi/src/lib.rs`, `ios/Harness/Info.plist`, `Cargo.lock`, `Cargo.toml`, `rust-toolchain.toml` or `.github/workflows/xcframework.yml` starts it, and one to `crates/api/src/main.rs`, `docs/specs/x.md`, `web/app/package.json`, `Cargo.toml.orig` or `deploy/host-budget.json` does not, each matched as GitHub matches a path filter (`**` crosses a slash, `*` does not); its one job calls the callee; its block is the pull-request rule's | `test_ci_workflows.py` `TheAppleBuildRunsFromOneBody.test_the_change_caller_runs_the_build_on_each_apple_path` |
| A2 | the tag caller runs on every release tag and on nothing else: its `on` is `push` with a `tags` filter alone; over `v1.2.3`, `v0.0.1` and `v10.20.30` and over `v1.2`, `v1.2.3.4`, `latest`, `1.2.3`, `v1.2.3-rc1`, `v1.x.3` and `vfoo` it admits exactly the names `release.yml`'s filter admits; its one job calls the callee | `test_release_workflow.py` `TheAppleBuildRunsOnEveryReleaseTag.test_the_tag_caller_runs_the_build_on_every_release_tag` |
| A3 | the callee takes only `workflow_call` and `workflow_dispatch`, holds no concurrency block, keeps the admitted runner on every job, and no expression in it reads a pull-request-only context; a planted step reading `github.head_ref` is refused by name | `test_ci_workflows.py` `TheAppleBuildRunsFromOneBody.test_one_job_body_serves_the_change_and_the_tag` |
| A4 | the pin census admits a local call only as a call job's own `uses`: the live tree's call jobs are exactly the two callers' calls of `./.github/workflows/xcframework.yml`; the walk that marks a call job's own `uses` yields, marks dropped, exactly what `entries(workflow, "uses")` yields, for every live workflow; and the pin test, run by name over a planted directory as `test_a_yaml_workflow_is_held_to_the_same_hardening_rules` runs it, admits a planted call job `./.github/workflows/xcframework.yml` and refuses, each with a message ending `planted.yml uses <ref>`, a call job `./.github/workflows/x.yml@dev`, a call job naming another repository's workflow by a branch, a step `uses: ./.github/actions/x` and a step `uses: ./.github/workflows/xcframework.yml` | `test_ci_workflows.py` `WorkflowsAreHardened.test_a_call_job_is_a_local_call_or_pinned` |
| A5 | the concurrency census classes the callers by their triggers: `apple-on-tag.yml` and `xcframework.yml` are release workflows, `apple-on-change.yml` is not, and the release-class rule finds nothing over the live tree | `test_workflow_concurrency.py` `TheAppleCallersAreClassedByTheirTriggers.test_the_tag_caller_and_its_callee_are_release_workflows` |
| A6 | every `cargo` command in the callee passes `--locked`; a planted step without it is refused by its command | `test_ci_workflows.py` `TheAppleBuildRunsFromOneBody.test_the_apple_build_resolves_only_the_locked_graph` |

The red each must show first, at the base the build cuts from:

- A1: `AssertionError` naming the missing `apple-on-change.yml` (the presence assertion runs before
  any read).
- A2: `AssertionError` naming the missing `apple-on-tag.yml`.
- A3: `AssertionError`: `xcframework.yml` runs on `['pull_request', 'workflow_dispatch']`, not
  `workflow_call` (the trigger assertion runs first).
- A4: `AssertionError`: the call jobs are `[]`, not the two callers' (the population assertion
  runs before any plant is judged).
- A5: `AssertionError`: `xcframework.yml` is a `reacher`, not a `release` workflow.
- A6: not red: the base already passes `--locked` on both commands (section 1.3, M11); the rows of
  section 8 prove the test fails when a command drops it.

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_change_caller_runs_the_build_on_each_apple_path
A2: python3 -m unittest discover -s scripts/tests -p test_release_workflow.py -k test_the_tag_caller_runs_the_build_on_every_release_tag
A3: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_one_job_body_serves_the_change_and_the_tag
A4: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_a_call_job_is_a_local_call_or_pinned
A5: python3 -m unittest discover -s scripts/tests -p test_workflow_concurrency.py -k test_the_tag_caller_and_its_callee_are_release_workflows
A6: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_apple_build_resolves_only_the_locked_graph
```

## 4. File manifest

| file | context | change |
|---|---|---|
| `.github/workflows/xcframework.yml` | none (CI) | changed: `on:` is `workflow_call` and `workflow_dispatch`; the concurrency block removed; the header comment names its two callers (R1) |
| `.github/workflows/apple-on-change.yml` | none (CI) | added: the change caller (R2, R4) |
| `.github/workflows/apple-on-tag.yml` | none (CI) | added: the tag caller (R3, R4) |
| `scripts/tests/test_ci_workflows.py` | none (tests) | changed: `LOCAL_CALL` defined here; the pin test reads call jobs apart from steps (R5); `test_a_call_job_is_a_local_call_or_pinned` (A4); `path_glob`, a path filter as GitHub matches it (`**` crosses a slash, `*` does not); the class `TheAppleBuildRunsFromOneBody` (A1, A3, A6) |
| `scripts/tests/test_release_workflow.py` | none (tests) | changed: the class `TheAppleBuildRunsOnEveryReleaseTag` (A2), beside `tag_glob`, which it reuses; `test_ci_workflows.py` cannot import it, since this module imports from that one |
| `scripts/tests/test_workflow_concurrency.py` | none (tests) | changed: `LOCAL_CALL` imported from `test_ci_workflows` in place of its own definition (line 968, R5); the class `TheAppleCallersAreClassedByTheirTriggers` (A5) |
| `scripts/mutation-rows.d/S34400-S34499.json` | none (rows) | added: section 8's rows |
| `docs/specs/SPEC-344-the-apple-build-runs-from-one-job-body-on-a-change-and-on-every-release-tag.md` | docs | added: this SPEC |
| `docs/decisions/ADR-355-one-apple-job-body-called-by-a-change-caller-and-a-tag-caller.md` | docs | added |
| `docs/schematics/apple-build-on-change-and-on-tag.md` | docs | added |
| `docs/schematics/ffi-adapter-xcframework-and-swift-package.md` | docs | changed: an appended amendment; its lines 75 to 81 state the pull-request trigger this delivery moves to a caller |
| `docs/red-first/SPEC-344.md` | docs | added |
| `changelog.d/apple-build-344.md` | docs | added |

Unchanged, by R6 to R8: `deny.toml`, `Cargo.lock`, `Cargo.toml`, `deploy/**`,
`.github/workflows/release.yml`, `.github/workflows/ci.yml`, `.github/rulesets/*`.

## 5. What this does NOT do

- It adds no `allow-git` entry: every git source the lockfile holds is admitted (section 1.3). The
  FSRS-7 crate's own entry, and the amendment of `test_engine_pin.py`'s exactly-two rule it needs,
  are that crate's delivery (#641).
- It adds no render test for the sync unit or its budget entry: SPEC-337 delivered both (#617), and
  SPEC-340 extends them with the archive and drill units (#628).
- It signs nothing, holds no team id, app id or upload key, and uploads nothing to TestFlight; the
  internal and release upload lanes are #634.
- It cuts no tag. The first real tag run is read when the milestone build is tagged (#636).
- It does not make the Apple build a required check, and does not hold a release on it (#634
  decides what a release build of the app needs).
- It changes nothing the Apple build does inside its jobs: the XCFramework's steps are #624's and
  the harness jobs #616's.
- It takes no figure on a device; device figures are #629's.
- It does not prove the Swift mutants the harness job sweeps (#650).

## 6. Risks

- **The release-class schema refuses a key of `xcframework.yml`.** As a callee of a tag workflow,
  the file is read by `schema_problems` down to its jobs' keys. Detected by A5 and by
  `test_every_workflow_that_can_hold_two_runs_of_a_release_queues_them`; the build runs both after
  the callers land and before it pushes.
- **A test that walks every workflow reads a job with `uses` and no `steps` badly.** The callers
  are the first call jobs in the tree. Detected by running the whole `scripts/tests` suite at the
  callers' commit, not only the six criteria.
- **`zizmor` flags the callers or the callee.** Detected by `ci.yml`'s `workflow-lint` job on the
  build's own pull request.
- **#656 lands after this delivery** and its hunk adding `ios/**` to `xcframework.yml`'s
  pull-request paths no longer applies. `ios/**` is already in the change caller's paths (R2), so
  #656 drops that hunk; A1 holds the path either way.
- **A tag run fails and nobody reads it.** The run is advisory (R4). Detected on the tag's checks
  page when the milestone build is tagged (#636).
- **The pin census's new rule admits more than a local call.** A4's plants refuse a local call
  carrying a ref, a remote call by a branch, a local action in a step and a local workflow named by
  a step. A call to a missing file or to a file without `workflow_call` is refused by the
  release-class rule, which `test_every_workflow_that_can_hold_two_runs_of_a_release_queues_them`
  runs over every workflow file.
- **Moving `LOCAL_CALL` breaks an anchor.** Measured: no mutation row at DEV names it
  (`git grep -n LOCAL_CALL 1eec0870 -- scripts` finds its definition and three reads, all in
  `test_workflow_concurrency.py`).

## 7. How a tag run is proved without a tag

The callee is one file for both events, so the change caller's run on this pull request runs the
exact jobs a tag run runs; A3 holds that no step branches on the event. A2 and A5 read the tag
caller as GitHub reads it: a push with a tag filter equal to `release.yml`'s, in the release class,
calling a file that takes `workflow_call`. Nothing pushes a tag (ADR-355 D4).

## 8. The mutation rows

Band `S34400` to `S34499`, `scripts/mutation-rows.d/S34400-S34499.json`, table `SCRIPT_MUTATIONS`, each row
`[id, target, find, replace, why, killer]` and each `find` occurring exactly once in its target:

| row | target | mutant | killer |
|---|---|---|---|
| 01 | `apple-on-tag.yml` | the tag filter widened to `'v*'` | A2 |
| 02 | `apple-on-tag.yml` | the push filter gains `branches: [main]` beside `tags` | A2 |
| 03 | `apple-on-tag.yml` | `queue: max` removed | `EveryReleaseWorkflowQueuesEveryRun.test_every_workflow_that_can_hold_two_runs_of_a_release_queues_them` |
| 04 | `apple-on-change.yml` | the `Cargo.toml` path removed | A1 |
| 05 | `apple-on-change.yml` | the `rust-toolchain.toml` path removed | A1 |
| 06 | `apple-on-change.yml` | the call gains a ref (`xcframework.yml@dev`) | A4 |
| 07 | `xcframework.yml` | `workflow_call` replaced by a `pull_request` trigger | A3 |
| 08 | `xcframework.yml` | the static-library command's `--locked` removed | A6 |
| 09 | `apple-on-tag.yml` | the call's target renamed to a file that is not there | `EveryReleaseWorkflowQueuesEveryRun.test_every_workflow_that_can_hold_two_runs_of_a_release_queues_them` |
| 10 | `scripts/tests/test_ci_workflows.py` | the call-job mark set on every `uses` the walk finds, a step's included | A4 |
| 11 | `scripts/tests/test_ci_workflows.py` | the local-call admission loosened from `LOCAL_CALL` to a `./` prefix | A4 |
| 12 | `scripts/tests/test_ci_workflows.py` | `path_glob`'s `**` read as `*`, so it stops at a slash | A1 |

## 9. References

SPEC-334 (R20; rows 1.2 and 1.5), ADR-335, ADR-340, ADR-345 (D4), ADR-058, ADR-338, SPEC-336,
SPEC-337, SPEC-339 (R12), SPEC-340, SPEC-190 (the concurrency census), SPEC-034 (the hardening
census), `RELEASING.md` section 3.

## Amendment

The two callers spell their call in GitHub's dedicated self-repository form,
`$/.github/workflows/xcframework.yml`, and `LOCAL_CALL` admits that form and no longer the
workspace-relative `./` one. The linter's self-repository audit refuses the `./` form because it
depends on the runner's file-system state, and the `$/` form does not and is read as a pin. Every
requirement above that names the call, and every planted control of the pin census, reads the new
spelling; what each asserts is unchanged. Chosen against keeping `./` with a linter ignore, which
is a weakening, and against pinning the linter, which is an environment pin.
