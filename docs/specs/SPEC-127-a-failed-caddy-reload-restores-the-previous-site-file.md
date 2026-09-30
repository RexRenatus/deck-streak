# SPEC-127: a failed Caddy reload restores the previous site file

- **Wave:** W2. **Issue:** #321. **Context(s):** `repo` (`deploy/deploy.sh`).
- **Decided by:** ADR-127, which keeps the previous copies until the reload succeeds and restores them
  when it does not.
- **Status:** judged: written at delivery, because it had no planned copy, and delivered with its
  tests and `docs/red-first/SPEC-127.md` (ADR-016).

## 1. The problem, measured

In `deploy.sh`'s `caddy_install` host script, once `caddy validate` and `caddy adapt --validate`
pass on the candidate copy, the candidate is moved over the Caddyfile and the previous block
(`deck-streak.caddy.previous`) is deleted, and only then does `caddy reload` run. Read at `dev`
dd478d7. A reload that fails leaves the new block and the new Caddyfile on disk, Caddy running the
old configuration, and no previous copy to restore: the files and the running configuration differ
until some later reload succeeds. `caddy_remove` has the same order and the same gap: it moves the
candidate over the Caddyfile and deletes the block before its reload. The judgment: the gap is the
same, the issue's intent (files that match what Caddy runs) covers both, and so R4 includes it.
Caddy's own documentation says "If there are any errors loading the new config, Caddy rolls back to
the last working config." (Caddy, Getting started), which is why the running side is safe and only
the disk side is wrong.

## 2. Requirements

R1. `caddy-install` keeps the previous block and the previous Caddyfile until the reload succeeds.
R2. A failed reload restores both, in SPEC-062 R7's order (the Caddyfile first, so that no state
    has it importing a block that is absent, then the block), reloads the restored configuration,
    and exits non-zero with a message that names the failed reload. If the restoring reload also
    fails, a second message says so distinctly, and the exit is still non-zero.
R3. A first install (no previous block) whose reload fails removes the new block and restores the
    previous Caddyfile.
R4. `caddy-remove` follows the same rule: the block and the Caddyfile are kept until its reload
    succeeds, and a failed reload restores both, as R2 says. It moves the block aside only after
    the Caddyfile without the import is in place, in SPEC-062 R7's order, so no step leaves the
    live Caddyfile importing a missing block; a restore returns the block before the Caddyfile.
R5. The previous copies are removed only after a successful reload, and none is left behind after a
    success or a restored failure.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | a failed reload restores the previous block and Caddyfile, reloads them, refuses and names the reload | `test_deploy_scripts.py` `a_failed_reload_restores_the_previous_block_and_caddyfile` |
| A2 | a failed restoring reload is named distinctly and the install still refuses | `test_deploy_scripts.py` `a_failed_restoring_reload_is_named_apart_and_still_refuses` |
| A3 | a first install whose reload fails removes the new block and restores the Caddyfile | `test_deploy_scripts.py` `a_first_install_whose_reload_fails_removes_the_new_block` |
| A4 | the previous copies exist at the reload and are gone after a good one | `test_deploy_scripts.py` `the_previous_copies_outlive_the_reload_and_go_after_a_good_one` |
| A5 | a failed reload of `caddy-remove` restores the block and the Caddyfile, names the failed reload, and a good one removes them | `test_deploy_scripts.py` `a_failed_reload_of_the_removal_restores_the_block_and_caddyfile` |
| A6 | a failed restoring reload of `caddy-remove` is named apart, the removal refuses and both files are restored | `test_deploy_scripts.py` `a_failed_restoring_reload_of_the_removal_is_named_apart` |
| A7 | a removal whose rename onto the Caddyfile fails leaves the block in place | `test_deploy_scripts.py` `a_removal_whose_caddyfile_rename_fails_leaves_the_block_in_place` |
| A8 | a first install whose restore rename fails never leaves a Caddyfile importing a missing block | `test_deploy_scripts.py` `a_first_install_whose_restore_rename_fails_never_imports_a_missing_block` |
| A9 | a removal whose block restore fails never leaves a Caddyfile importing a missing block | `test_deploy_scripts.py` `a_removal_whose_block_restore_fails_never_imports_a_missing_block` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_deploy_scripts.py -k a_failed_reload_restores_the_previous_block_and_caddyfile
A2: python3 -m unittest discover -s scripts/tests -p test_deploy_scripts.py -k a_failed_restoring_reload_is_named_apart_and_still_refuses
A3: python3 -m unittest discover -s scripts/tests -p test_deploy_scripts.py -k a_first_install_whose_reload_fails_removes_the_new_block
A4: python3 -m unittest discover -s scripts/tests -p test_deploy_scripts.py -k the_previous_copies_outlive_the_reload_and_go_after_a_good_one
A5: python3 -m unittest discover -s scripts/tests -p test_deploy_scripts.py -k a_failed_reload_of_the_removal_restores_the_block_and_caddyfile
A6: python3 -m unittest discover -s scripts/tests -p test_deploy_scripts.py -k a_failed_restoring_reload_of_the_removal_is_named_apart
A7: python3 -m unittest discover -s scripts/tests -p test_deploy_scripts.py -k a_removal_whose_caddyfile_rename_fails_leaves_the_block_in_place
A8: python3 -m unittest discover -s scripts/tests -p test_deploy_scripts.py -k a_first_install_whose_restore_rename_fails_never_imports_a_missing_block
A9: python3 -m unittest discover -s scripts/tests -p test_deploy_scripts.py -k a_removal_whose_block_restore_fails_never_imports_a_missing_block
```

The tests use the existing fakes: a `caddy` stub whose `reload` fails on demand (the count of
failures is written to a file, so one file fails the reload once and another twice) and records how
many previous copies sit beside the configuration at each reload; A7 and A8 swap in a `mv` that
refuses one rename onto the Caddyfile, and A9 one that refuses the rename back onto the block. No test reaches a host.

## 4. File manifest

| file | context | change |
|---|---|---|
| `deploy/deploy.sh` | repo | changed: R1 to R5 (`caddy_install`, `caddy_remove`) |
| `scripts/tests/test_deploy_scripts.py` | repo | changed: the `caddy` stub, the failing `mv`s and A1 to A9 |
| `scripts/mutation-rows.d/S12700-S12799.json` | repo | added: the restore clauses as script rows (section 7) |
| `deploy/README.md` | repo | changed: one sentence in the Caddy section |
| `docs/specs/SPEC-127-a-failed-caddy-reload-restores-the-previous-site-file.md` | repo | added |
| `docs/decisions/ADR-127-a-caddy-reload-that-fails-restores-the-copies-kept-until-it-succeeds.md` | repo | added |
| `docs/decisions/ADR-062-a-deploy-installs-only-a-release-whose-provenance-and-digests-verify.md` | repo | changed: amended (appended): one dated line at its end, the unit-guards bullet unchanged |
| `docs/specs/SPEC-062-first-deploy-units-caddy-block-and-https.md` | repo | changed: one dated amendment line at its end (insert-only) |
| `docs/red-first/SPEC-127.md` | repo | added |
| `changelog.d/fix-caddy-restore-127.md` | repo | added |

## 5. What this does NOT do

- It does not run `caddy reload` on a candidate path before the swap. The block must sit in place
  for the import to read it, so the candidate order saves no restore (#321).
- It does not read the health-check list after a restore; the deploy's readiness probes stay as
  SPEC-062 gives them (#321).
- It does not restore anything when the host is unreachable or the script is killed between the
  swap and the reload; the next `caddy-install` starts from the files on disk (#321).
- It does not change the validation of the candidate copy, the rendering of the block or the
  release install path (#321).
- It adds no message of its own to the removal's second refusal (#361).
- It performs no step of the cutover (#164).

## 6. Risks

- **The restoring reload fails too.** The files are already back and the exit is non-zero with a
  message that says so, so the operator reads the state from the message (R2, A2).
- **The Caddyfile's previous copy is left after a crash.** It is named beside the Caddyfile, so the
  next run overwrites it before use and it is never read as current (R1).

## 7. Mutation rows

`scripts/mutation-rows.d/S12700-S12799.json` holds script rows on `deploy/deploy.sh`, each proved
killed by its full id and each naming one test as its killer:

| row | the invariant it pins | killer |
|---|---|---|
| S12701 | the previous block is put back when the reload fails | A1 |
| S12702 | the previous Caddyfile is put back when the reload fails | A3 |
| S12703 | a failed reload exits non-zero | A2 |
| S12704 | the previous copies are not deleted before the reload | A4 |
| S12705 | the removal restores the Caddyfile when its reload fails | A5 |
| S12706 | the install names a failed reload in words of its own | A1 |
| S12707 | the removal reloads again after restoring and names a second failure | A6 |
| S12708 | the removal swaps the Caddyfile before it moves the block aside | A7 |
| S12709 | the install restores the Caddyfile before it takes the new block away | A8 |
| S12710 | the removal restores the block before the Caddyfile | A9 |

## References

SPEC-062 (R7, whose exclusion this closes), ADR-062, ADR-127; #321.

## Amendment, 2026-09-29: both refusals of a removal say so

Issue #361. `deploy.sh caddy-remove` checks the Caddyfile it will leave behind twice, with
`caddy validate` and then `caddy adapt --validate`. Only the first refusal printed
`deploy: refused`; the second removed the candidate and exited non-zero without a message of its own,
so the operator could not tell which deploy step had stopped. Both refusals now print the removal's
own message, remove the candidate and exit non-zero. This lifts the fifth exclusion of section 5, which left
the second refusal's message to #361.

## Acceptance criteria of the 2026-09-29 amendment

| id | criterion | test |
|---|---|---|
| A10 | a removal whose adapted configuration is refused exits non-zero, prints `deploy: refused`, leaves the live Caddyfile and the site block unchanged and leaves no candidate file (#361) | `test_deploy_scripts.py` `a_removal_whose_adapted_configuration_is_refused_says_so` |
| A11 | a removal whose validation is refused exits non-zero, prints `deploy: refused`, leaves the live Caddyfile and the site block unchanged and leaves no candidate file (#361) | `test_deploy_scripts.py` `a_removal_whose_validation_is_refused_says_so` |

```acceptance
A10: python3 -m unittest discover -s scripts/tests -p test_deploy_scripts.py -k a_removal_whose_adapted_configuration_is_refused_says_so
A11: python3 -m unittest discover -s scripts/tests -p test_deploy_scripts.py -k a_removal_whose_validation_is_refused_says_so
```

The `caddy` stub gains a flag file that makes `adapt` refuse while `validate` passes. Row S12711 in
`scripts/mutation-rows.d/S12700-S12799.json` pins the message (killer A10), and row S12712 pins the
first refusal's message (killer A11). Files changed: `deploy/deploy.sh`,
`scripts/tests/test_deploy_scripts.py`, `scripts/mutation-rows.d/S12700-S12799.json`,
`docs/red-first/SPEC-127.md` and `changelog.d/fix-caddy-remove-message-361.md`.

## Amendment, 2026-09-29: a failed candidate write is its own refusal

Issue #384, found in the review of #382. `deploy.sh caddy-remove` wrote its candidate Caddyfile with
`grep ... >"$copy" || true`, and the `|| true` also swallowed a failed write. With no candidate on
disk, each refusal's `find "$copy" -delete` failed under `set -e` inside the refusal group, so the
script exited before it printed `deploy: refused`, and both refusals (the validation and the adapt
check) shared the shape. The strengthened rule: a removal that cannot write its candidate refuses in
words of its own (`deploy: the candidate Caddyfile could not be written`) and exits non-zero, and each
refusal prints `deploy: refused` and exits non-zero whether or not the candidate exists.

The write is now two steps, so that a failed redirection is told apart from `grep`'s exit status: an
empty file is created first (a failure there is the refusal), then `grep` fills it, where status 1
(no line kept) is not a failure and status 2 (the Caddyfile cannot be read) is the same refusal. The
refusals' cleanup deletes the candidate only when it is a file. Nothing else in the removal changes.

The criteria that back the rule are A12 to A15, defined in the section below. The insertions this
amendment makes are these two sections, appended after the file's last line, and nothing above them
is edited (SPEC-038 section 8, ruling (i)). Row S12713 pins the write's refusal (killer A12), rows
S12714 and S12715 pin the two `grep` statuses (killers A14 and A15), and rows S12716 and S12717 pin
the absent-candidate guard of each refusal (killer A13, one test per branch).

## Acceptance criteria of the 2026-09-29 candidate-write amendment

| id | criterion | test |
|---|---|---|
| A12 | a removal whose candidate cannot be written exits non-zero, prints `the candidate Caddyfile could not be written` and leaves the live Caddyfile and the site block unchanged (#384) | `test_deploy_scripts.py` `a_removal_whose_candidate_cannot_be_written_says_so` |
| A13 | a removal refused at validation, and one refused at the adapt check, each exit non-zero and print `deploy: refused` when the candidate is already absent (#384) | `test_deploy_scripts.py` `a_removal_refused_at_validation_with_no_candidate_still_says_so` and `a_removal_refused_at_the_adapt_check_with_no_candidate_still_says_so` |
| A14 | a removal whose Caddyfile cannot be read is refused with the write's message and leaves no candidate file (#384) | `test_deploy_scripts.py` `a_removal_whose_caddyfile_cannot_be_read_says_so` |
| A15 | a removal from a Caddyfile holding only the import line succeeds, because `grep` keeping no line is not a failure (#384) | `test_deploy_scripts.py` `a_removal_from_a_caddyfile_holding_only_the_import_line_is_not_a_failure` |

```acceptance
A12: python3 -m unittest discover -s scripts/tests -p test_deploy_scripts.py -k a_removal_whose_candidate_cannot_be_written_says_so
A13: python3 -m unittest discover -s scripts/tests -p test_deploy_scripts.py -k with_no_candidate_still_says_so
A14: python3 -m unittest discover -s scripts/tests -p test_deploy_scripts.py -k a_removal_whose_caddyfile_cannot_be_read_says_so
A15: python3 -m unittest discover -s scripts/tests -p test_deploy_scripts.py -k a_removal_from_a_caddyfile_holding_only_the_import_line_is_not_a_failure
```

The unwritable candidate is a directory planted at the candidate's path inside the test's own tree;
the `caddy` stub gains a flag file that deletes the candidate before it refuses. Files changed:
`deploy/deploy.sh`, `scripts/tests/test_deploy_scripts.py`, `scripts/mutation-rows.d/S12700-S12799.json`
(rows S12711 and S12712 re-anchored on the changed lines, S12713 to S12717 added),
`docs/red-first/SPEC-127.md` and `changelog.d/fix-candidate-write-384.md`.

## Amendment, 2026-09-29: the install undoes every write it made, and a linked candidate is refused

Issues #423 and #424, both found in the review of #415. The install's `undo` began with an unguarded
`find "$copy" -delete`; with the candidate absent that `find` failed under `set -e`, and the script
exited before it printed `deploy: the Caddy configuration was refused`, the same shape #384 removed
from the removal. Three writes of the install also had no undo at all: the block's `cat >"$block"`,
the candidate's `cp -p "$file" "$copy"`, the import line's append, the kept copy of the Caddyfile and
the rename of the candidate onto it. A failure of any of them left the new block, or a half-written
candidate, in place with a silent exit. The strengthened rule: every write the install makes is
followed by the undo, the undo tolerates an absent candidate and an absent block, and every path that
undoes prints the refusal and exits non-zero. The copy that sets the previous block aside is followed
by the refusal alone, because nothing has been written yet. The helpers the undo uses are defined
before the first write.

The removal's `: >"$copy"` follows a symbolic link. A link at the candidate path that names the live
Caddyfile made the write empty the live Caddyfile, and a link to nowhere made the write create its
target, so the later rename put the link itself in the live Caddyfile's place. The removal now
refuses a candidate path that is a link, with the write's own message, before any write.

The class the guards refuse: a candidate, block or previous-copy path that exists and is not a plain
file with one link. The install checks the block, the block's previous copy, the candidate and the
Caddyfile's previous copy before it writes anything, and refuses with its refusal when one of them is a
link of either kind, a directory, a pipe, a socket, a device or a file with more than one link. The
removal refuses a candidate that is a link, a pipe, a socket, a device or a file with more than one
link, with the write's own message: it deletes nothing for a pipe, a socket, a device or a file with
more than one link, and its cleanup deletes a link to a regular file, which is the run's own candidate
name. A candidate that is a directory is refused by the write itself (A12). A12 pins a candidate path that cannot be written and plants a
directory there; the class is every candidate path that is not a plain file with one link, of which A20
and A21 pin the two links that changed the live state and A22 to A26 pin the rest. `cp -p` alone is
not a guard: it writes through a link to any other existing file.

Both scripts also check, before their first write and with nothing written or deleted when a check
fails, that every directory their writes and their undo touch is writable: the Caddy directory and
the live Caddyfile's own directory, which holds the rename target and the Caddyfile's previous copy.
The two are one directory by default and two when `DECKSTREAK_DEPLOY_CADDYFILE` names a Caddyfile
elsewhere, so both are derived from the path settings and neither from the test layout. The
Caddyfile must be a regular file (a pipe there would hang the run). When the live Caddyfile is a
link to a regular file, a step that replaces or restores it leaves a regular file with the same bytes
and writes nothing through the link. The removal checks the same four names as the install, the block, the block's
previous copy, the candidate and the Caddyfile's previous copy, each absent or a plain file with one
link, and refuses with the write's own message. Only the install promises the refusal on every early
exit; the removal promises no message on every exit. A33 to A37 pin these guards and the two tests that
the moved rows S12725 and S12730 need. A38 generates the class over the path settings: each directory
(the Caddy directory, and the Caddyfile's directory beside it and set apart) in each state (writable,
read-only, read-only holding a stale writable previous copy), for each step (a first install, a
re-install, a removal) and each trigger (none, a failed rename, a failed reload). Every member either
succeeds, or exits 1 with that script's refusal (or with the reload's own message when a reload fails
after a write) and leaves both directories and the live Caddyfile byte for byte as they were. A
removal whose rename fails in a writable directory is not a member: it fails after its writes, the
removal promises no message on every exit, and A7 pins it. A39 reads the settings the Caddy functions consume from `deploy.sh` and
requires that they be exactly the settings A38 varies, so a new path setting fails until it is an axis.

The insertions this amendment makes are these two sections, appended after the file's last line,
and nothing above them is edited (SPEC-038 section 8, ruling (i)).

## Acceptance criteria of the 2026-09-29 install-undo amendment

| id | criterion | test |
|---|---|---|
| A16 | an install refused at validation, and one refused at the adapt check, each exit non-zero, print `the Caddy configuration was refused`, put the previous block back and leave no previous copy when the candidate is already absent (#423) | `test_deploy_scripts.py` `an_install_refused_at_validation_with_no_candidate_still_undoes` and `an_install_refused_at_the_adapt_check_with_no_candidate_still_undoes` |
| A17 | an install whose candidate path cannot be written exits non-zero, prints the refusal, puts the previous block back and leaves the live Caddyfile unchanged (#423) | `test_deploy_scripts.py` `an_install_whose_candidate_path_cannot_be_written_undoes_and_says_so` |
| A18 | an install whose block cannot be written exits non-zero, prints the refusal and leaves the previous block's text and the live Caddyfile unchanged (#423) | `test_deploy_scripts.py` `an_install_whose_block_cannot_be_written_undoes_and_says_so` |
| A19 | an install whose import line cannot be added exits non-zero, prints the refusal, removes the new block and leaves the live Caddyfile unchanged (#423) | `test_deploy_scripts.py` `an_install_whose_import_line_cannot_be_added_undoes_and_says_so` |
| A20 | a removal whose candidate path is a link to the live Caddyfile exits non-zero, prints the write's message and leaves the live Caddyfile byte for byte unchanged (#424) | `test_deploy_scripts.py` `a_removal_whose_candidate_is_a_link_to_the_caddyfile_refuses_before_writing` |
| A21 | a removal whose candidate path is a link to a missing file exits non-zero, prints the write's message, leaves the live Caddyfile unchanged and creates nothing at the link's target (#424) | `test_deploy_scripts.py` `a_removal_whose_candidate_is_a_dangling_link_refuses_before_writing` |
| A22 | an install whose candidate path is a link to the live Caddyfile exits non-zero, prints the refusal, puts the previous block back and leaves the live Caddyfile a file with its bytes unchanged (#423) | `test_deploy_scripts.py` `an_install_whose_candidate_is_a_link_to_the_caddyfile_refuses_and_undoes` |
| A23 | an install whose candidate path is a link to another existing file exits non-zero, prints the refusal, writes nothing through the link and leaves the live Caddyfile a file with its bytes unchanged (#423) | `test_deploy_scripts.py` `an_install_whose_candidate_links_to_another_file_refuses_before_writing` |
| A24 | an install whose block path is a hard link to the live Caddyfile exits non-zero, prints the refusal, leaves the live Caddyfile byte for byte unchanged and leaves no previous copy (#423) | `test_deploy_scripts.py` `an_install_whose_block_is_a_hard_link_to_the_caddyfile_refuses_before_writing` |
| A25 | a removal whose candidate path is a hard link to the live Caddyfile exits non-zero, prints the write's message and leaves the live Caddyfile byte for byte unchanged (#424) | `test_deploy_scripts.py` `a_removal_whose_candidate_is_a_hard_link_to_the_caddyfile_refuses_before_writing` |
| A26 | a removal whose candidate path is a pipe exits non-zero without waiting, prints the write's message and leaves the live Caddyfile byte for byte unchanged (#424) | `test_deploy_scripts.py` `a_removal_whose_candidate_is_a_fifo_refuses_before_writing` |
| A27 | a first install into a Caddy directory that cannot be written exits non-zero, prints the refusal and leaves the Caddyfile and the directory as they were (#423) | `test_deploy_scripts.py` `a_first_install_into_a_read_only_caddy_directory_says_so` |
| A28 | a first install whose block path is a directory exits non-zero, prints the refusal and keeps the directory and the file inside it (#423) | `test_deploy_scripts.py` `a_first_install_never_deletes_a_directory_at_the_block_path` |
| A29 | an install whose previous Caddyfile copy cannot be written exits non-zero, prints the refusal, puts the previous block back, leaves the live Caddyfile unchanged and leaves no candidate (#423) | `test_deploy_scripts.py` `an_install_whose_previous_caddyfile_copy_cannot_be_written_undoes_and_says_so` |
| A30 | an install whose candidate cannot be renamed onto the Caddyfile exits non-zero, prints the refusal, puts the previous block back and leaves the live Caddyfile unchanged and no candidate or previous copy (#423) | `test_deploy_scripts.py` `an_install_whose_candidate_rename_fails_undoes_and_says_so` |
| A31 | an install whose live Caddyfile cannot be read exits non-zero, prints the refusal, puts the previous block back and leaves no candidate or previous copy (#423) | `test_deploy_scripts.py` `an_install_whose_caddyfile_cannot_be_read_undoes_and_says_so` |
| A32 | an install that cannot copy the previous block aside exits non-zero, prints the refusal, leaves the block and the live Caddyfile unchanged and leaves no previous copy (#423) | `test_deploy_scripts.py` `an_install_that_cannot_copy_the_block_in_a_read_only_directory_says_so` |
| A33 | a removal whose block copy, or whose Caddyfile copy, is a link of either kind, a directory or a pipe, or a file with more than one link, exits non-zero, prints the write's own message and leaves the live Caddyfile and the block unchanged (#423, #424) | `test_deploy_scripts.py` `a_removal_refuses_every_previous_copy_that_is_not_a_plain_file` |
| A34 | neither script waits on a pipe at the live Caddyfile: each exits non-zero with its own refusal and writes nothing (#423, #424) | `test_deploy_scripts.py` `neither_script_waits_on_a_fifo_at_the_live_caddyfile` |
| A35 | a first install, a re-install and a removal in a Caddy directory that cannot be written, with a stale file in it, exit non-zero with their own refusal before any write and leave the stale file and the live Caddyfile as they were (#423, #424) | `test_deploy_scripts.py` `a_read_only_caddy_directory_is_refused_before_any_write` |
| A36 | an install whose block cannot be read exits non-zero, prints the refusal and leaves the block, the live Caddyfile and every other file as they were (#423) | `test_deploy_scripts.py` `an_install_whose_block_cannot_be_read_refuses_before_writing` |
| A37 | a first install refused at validation with its block already gone still exits non-zero and prints the refusal (#423) | `test_deploy_scripts.py` `a_first_install_refused_with_its_block_already_gone_still_says_so` |
| A38 | for each directory the two scripts write or undo in (the Caddy directory, and the live Caddyfile's own directory beside it and set apart), in each state (writable, read-only, read-only with a stale writable previous copy), for a first install, a re-install and a removal, with no trigger, a failed rename and a failed reload, every member succeeds, or exits 1 with that script's refusal (or the reload's own message when a reload fails after a write) and leaves both directories and the live Caddyfile byte for byte unchanged; a removal whose rename fails in a writable directory fails after its writes and is not a member (#423, #424) | `test_deploy_scripts.py` `every_directory_a_caddy_script_writes_or_undoes_is_checked_before_the_first_write` |
| A39 | the settings the Caddy functions of `deploy.sh` consume are exactly the path settings A38 varies plus the one input file, so a new setting fails until it is classified (#423) | `test_deploy_scripts.py` `the_caddy_functions_read_only_the_settings_the_directory_population_varies` |

```acceptance
A16: python3 -m unittest discover -s scripts/tests -p test_deploy_scripts.py -k with_no_candidate_still_undoes
A17: python3 -m unittest discover -s scripts/tests -p test_deploy_scripts.py -k an_install_whose_candidate_path_cannot_be_written_undoes_and_says_so
A18: python3 -m unittest discover -s scripts/tests -p test_deploy_scripts.py -k an_install_whose_block_cannot_be_written_undoes_and_says_so
A19: python3 -m unittest discover -s scripts/tests -p test_deploy_scripts.py -k an_install_whose_import_line_cannot_be_added_undoes_and_says_so
A20: python3 -m unittest discover -s scripts/tests -p test_deploy_scripts.py -k a_removal_whose_candidate_is_a_link_to_the_caddyfile_refuses_before_writing
A21: python3 -m unittest discover -s scripts/tests -p test_deploy_scripts.py -k a_removal_whose_candidate_is_a_dangling_link_refuses_before_writing
A22: python3 -m unittest discover -s scripts/tests -p test_deploy_scripts.py -k an_install_whose_candidate_is_a_link_to_the_caddyfile_refuses_and_undoes
A23: python3 -m unittest discover -s scripts/tests -p test_deploy_scripts.py -k an_install_whose_candidate_links_to_another_file_refuses_before_writing
A24: python3 -m unittest discover -s scripts/tests -p test_deploy_scripts.py -k an_install_whose_block_is_a_hard_link_to_the_caddyfile_refuses_before_writing
A25: python3 -m unittest discover -s scripts/tests -p test_deploy_scripts.py -k a_removal_whose_candidate_is_a_hard_link_to_the_caddyfile_refuses_before_writing
A26: python3 -m unittest discover -s scripts/tests -p test_deploy_scripts.py -k a_removal_whose_candidate_is_a_fifo_refuses_before_writing
A27: python3 -m unittest discover -s scripts/tests -p test_deploy_scripts.py -k a_first_install_into_a_read_only_caddy_directory_says_so
A28: python3 -m unittest discover -s scripts/tests -p test_deploy_scripts.py -k a_first_install_never_deletes_a_directory_at_the_block_path
A29: python3 -m unittest discover -s scripts/tests -p test_deploy_scripts.py -k an_install_whose_previous_caddyfile_copy_cannot_be_written_undoes_and_says_so
A30: python3 -m unittest discover -s scripts/tests -p test_deploy_scripts.py -k an_install_whose_candidate_rename_fails_undoes_and_says_so
A31: python3 -m unittest discover -s scripts/tests -p test_deploy_scripts.py -k an_install_whose_caddyfile_cannot_be_read_undoes_and_says_so
A32: python3 -m unittest discover -s scripts/tests -p test_deploy_scripts.py -k an_install_that_cannot_copy_the_block_in_a_read_only_directory_says_so
A33: python3 -m unittest discover -s scripts/tests -p test_deploy_scripts.py -k a_removal_refuses_every_previous_copy_that_is_not_a_plain_file
A34: python3 -m unittest discover -s scripts/tests -p test_deploy_scripts.py -k neither_script_waits_on_a_fifo_at_the_live_caddyfile
A35: python3 -m unittest discover -s scripts/tests -p test_deploy_scripts.py -k a_read_only_caddy_directory_is_refused_before_any_write
A36: python3 -m unittest discover -s scripts/tests -p test_deploy_scripts.py -k an_install_whose_block_cannot_be_read_refuses_before_writing
A37: python3 -m unittest discover -s scripts/tests -p test_deploy_scripts.py -k a_first_install_refused_with_its_block_already_gone_still_says_so
A38: python3 -m unittest discover -s scripts/tests -p test_deploy_scripts.py -k every_directory_a_caddy_script_writes_or_undoes_is_checked_before_the_first_write
A39: python3 -m unittest discover -s scripts/tests -p test_deploy_scripts.py -k the_caddy_functions_read_only_the_settings_the_directory_population_varies
```

The test file's `World.run` now starts the script in its own session and kills the whole group on a
timeout, so a stuck stub cannot orphan the host script. Rows S12718 to S12737 in
`scripts/mutation-rows.d/S12700-S12799.json` pin the absent-candidate guard of the undo (killer A16),
the undo after the block write (A18), after the candidate copy (A31) and after the import append
(A19), the removal's link guard (A20), the install's path guard (A23, A24), the absent-block undo
(A37), the undo of the kept copy and of the rename (A29, A30), the removal's guard for a hard link or
a pipe (A25), the refusal of a block copy that fails (A36), the Caddyfile checks of both scripts (A34),
the removal's four-name guard (A33), and the directory checks and the directory line of each script (A38). Files changed: `deploy/deploy.sh`,
`scripts/tests/test_deploy_scripts.py`, `scripts/mutation-rows.d/S12700-S12799.json`,
`docs/red-first/SPEC-127.md`, `docs/decisions/ADR-198-the-install-undoes-every-write-and-a-linked-candidate-is-refused.md`
and `changelog.d/fix-caddy-undo-423.md`.

### What this amendment does NOT do

- It does not change what a successful install or removal does (#423, #424).
- It does not change the removal's refusal of a link at its candidate path, which stays A20 and A21 (#424).
