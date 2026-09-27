# SPEC-033: the public repository is re-seeded clean, its scrub reads every blob, and its committed rulesets are the enforced ones

- **Wave:** W0. **Issue:** #23 (epic #1). **Context(s):** `repo` (`scripts/`, `.github/`, `docs/`).
- **Decided by:** ADR-033 (this SPEC's own), ADR-004 (the vendored packs), ADR-017 (the branch
  model and hosted CI).
- **Status:** judged: delivered with its tests (the gate-1 remediation, before the repository is
  made public).

## 1. The problem, measured

The first private repository failed the owner's public-readiness gate (#160). The phase-1 scrub had
passed it, and a blob-level review of a mirror clone found two classes it missed:

- **Compiled Python caches.** Twelve `*.pyc` blobs, ten of them still tracked at the tip, embedded
  the maintainer's host path. `scripts/public-scrub.py` reads each file with `read_text`, and a
  file that is not UTF-8 raised `UnicodeDecodeError` and was skipped. The history check read the
  `+` lines of `git show`, which prints `Binary files differ` for a binary.
- **Files that only history held.** The subscription-proxy reference client and its scanner, which
  name a private secret, were in nine commits and absent from the tree. The scrub read only the
  tree.

A measurement on the maintainer's box reproduced this. Every blob of every commit was read (`git
ls-tree -r` of each commit, then `git cat-file blob`), with the public shapes and the private list:
403 blobs examined, 12 binary, 28 findings, all in those 14 blobs. The commit messages and
identities were clean. After the rewrite (ADR-033): 382 blobs, 0 binary, 0 findings.

Two further facts shaped the remedy:

- **A force-push cannot remediate.** GitHub keeps `refs/pull/<n>/head` for every pull request, and
  three Dependabot pull requests pinned the old commits. No user can delete those refs.
- **Repository hygiene.**
  - The committed `.github/rulesets/release-tags.json` carries a `tag_name_pattern` rule that
    GitHub refuses on this plan (HTTP 422). The committed ruleset is therefore not the enforced one.
  - Dependabot opened major-version bumps (TypeScript 7, Vitest 5, @types/node 26) that the house
    radar holds; `stack.json` pins the majors.

## 2. Requirements

R1. `scripts/public-scrub.py --history` reads every blob reachable from `--rev` (default `HEAD`)
    exactly once, with `git rev-list --objects` and `git cat-file --batch`. It applies the tree
    scan's public shapes and private literals to each blob. A finding names the rule, the path, the
    blob's short id and the line, never the value.
R2. A binary file is refused by the rule `binary`, whatever its path, whether it is in the tree or
    anywhere in the scanned history. A file is binary when its first 8000 bytes hold a NUL byte, or
    when it is not UTF-8. The private literals are still searched in its bytes.
R3. A file or blob over the scrub's size limit (2,000,000 bytes) is refused by the rule `oversize`
    instead of being skipped.
R4. `--history` on a shallow repository is VOID (exit 3), because the history it would read is
    incomplete.
R5. The gate's scrub stage (`scripts/check.sh`) runs the history scan. CI checks out with
    `fetch-depth: 0`, so it reads every blob of the pull request's merge commit. The maintainer's box
    adds the private list through `$PERSONA_CORE_DENY_LIST`.
R6. The committed rulesets are the enforced dev-to-main release workflow:
    - `main` accepts only pull requests merged by a merge commit;
    - `dev` accepts only pull requests;
    - both require the `ci` and `fragment` checks, and both block deletion and non-fast-forward;
    - the release-tag ruleset blocks deletion, update and non-fast-forward on `refs/tags/v*`;
    - no ruleset has a bypass actor, and every rule is one the plan enforces.
R7. Dependabot ignores semver-major updates for cargo and npm, so a major moves only with the house
    radar (`stack.json`); its updates still target `dev`.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | a committed binary is refused by rule name | `test_public_scrub.py` |
| A2 | a binary blob that only history holds is refused | `test_public_scrub.py` |
| A3 | a private literal in a deleted file is found in history, by index and never by value | `test_public_scrub.py` |
| A4 | an address shape in a deleted file is found in history | `test_public_scrub.py` |
| A5 | a clean history is examined blob by blob and passes | `test_public_scrub.py` |
| A6 | a shallow history is VOID, not green | `test_public_scrub.py` |
| A7 | an oversize file is refused rather than skipped | `test_public_scrub.py` |
| A8 | the gate's scrub stage reads every blob of history | `test_public_scrub.py` |
| A9 | main merges only by merge commit after `ci` and `fragment` | `test_rulesets.py` |
| A10 | dev takes pull requests only, after `ci` and `fragment` | `test_rulesets.py` |
| A11 | release tags block deletion, update and force, with rules the plan enforces | `test_rulesets.py` |
| A12 | no ruleset has a bypass actor or a rule the plan refuses | `test_rulesets.py` |
| A13 | a major version moves only with the house radar | `test_dependabot.py` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_public_scrub.py -k a_committed_binary_is_refused_by_rule_name
A2: python3 -m unittest discover -s scripts/tests -p test_public_scrub.py -k a_binary_blob_only_history_holds_is_refused
A3: python3 -m unittest discover -s scripts/tests -p test_public_scrub.py -k a_private_literal_in_a_deleted_file_is_found_in_history
A4: python3 -m unittest discover -s scripts/tests -p test_public_scrub.py -k an_address_in_a_deleted_file_is_found_in_history
A5: python3 -m unittest discover -s scripts/tests -p test_public_scrub.py -k a_clean_history_is_examined_blob_by_blob_and_passes
A6: python3 -m unittest discover -s scripts/tests -p test_public_scrub.py -k a_shallow_history_is_void_not_green
A7: python3 -m unittest discover -s scripts/tests -p test_public_scrub.py -k an_oversize_file_is_refused_rather_than_skipped
A8: python3 -m unittest discover -s scripts/tests -p test_public_scrub.py -k the_gate_scrub_stage_reads_every_blob_of_history
A9: python3 -m unittest discover -s scripts/tests -p test_rulesets.py -k main_merges_only_by_merge_commit_after_ci_and_fragment
A10: python3 -m unittest discover -s scripts/tests -p test_rulesets.py -k dev_takes_pull_requests_only_after_ci_and_fragment
A11: python3 -m unittest discover -s scripts/tests -p test_rulesets.py -k release_tags_block_deletion_update_and_force
A12: python3 -m unittest discover -s scripts/tests -p test_rulesets.py -k no_ruleset_has_a_bypass_actor_or_a_refused_rule
A13: python3 -m unittest discover -s scripts/tests -p test_dependabot.py -k a_major_version_moves_only_with_the_house_radar
```

The scrub's tests build their fixtures at run time, in temporary git repositories inside
`tempfile.TemporaryDirectory()`. No planted value is a literal in this repository, and no test
leaves a file behind.

## 4. File manifest

| file | context | change |
|---|---|---|
| `scripts/public-scrub.py` | `repo` | changed: `--history`, `binary`, `oversize`, VOID on a shallow history |
| `scripts/check.sh` | `repo` | changed: the scrub stage reads the history |
| `scripts/tests/test_public_scrub.py` | `repo` | changed: A1 to A8 |
| `scripts/tests/test_rulesets.py` | `repo` | added: A9 to A12 |
| `scripts/tests/test_dependabot.py` | `repo` | added: A13 |
| `.github/rulesets/release-tags.json` | `repo` | changed: the refused `tag_name_pattern` rule removed |
| `.github/dependabot.yml` | `repo` | changed: semver-major updates ignored for cargo and npm |
| `docs/issues-manifest.json` | `repo` | changed: the re-seeded repository's issue ids (numbers unchanged) |
| `docs/decisions/ADR-033-a-leak-is-remediated-by-a-fresh-repository.md` | `repo` | added |
| `docs/specs/SPEC-033-the-public-repository-is-re-seeded-clean.md` | `repo` | added |
| `docs/red-first/SPEC-033.md` | `repo` | added |
| `changelog.d/fix-blob-level-scrub.md` | `repo` | added |

## 5. What this does NOT do

- It deletes nothing. The first private repository is renamed to an archive name and stays private
  and recoverable. Deleting it is a decision for the owner's gate (#160).
- It admits no binary. The icons and social cards that the landing page and the Mini App will need
  are a later decision: an allow-list with a metadata check, made by the delivery that needs them
  (#59).
- It puts no private literal in CI. CI judges by the public shapes and the binary rule, and the
  maintainer's box adds the private list on every verification. That stays so after the pack runner
  exists (#60).
- It does not scan issue and pull-request text in the gate. The issue tooling scrubs every body
  before it posts, and the public-readiness report scans the posted text (#160).
- It asks GitHub for no purge. The archive holds the old commits privately; the owner decides
  whether to delete it (#160).

## 6. Risks

- **A binary the project needs is refused.** Detected by the gate. The delivery that needs one amends
  the rule with an allow-list (#59).
- **The history scan slows as the history grows.** Measured on every run by the stage's seconds in
  `scripts/check.sh`'s summary line.
- **A shallow checkout scans less than the history.** R4 makes it VOID, never green.
- **A ruleset changes on GitHub without the committed file changing.** Detected by the
  public-readiness report, which compares `gh api` output with these files, and by `test_rulesets.py`
  for the committed side.
