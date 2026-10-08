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
the moved rows S12725 and S12730 need.
A38 measures the places instead of listing them. It runs each Caddy step through its eight exits (a
first install, a re-install, a removal, an install refused at validation, a reload that fails, a rename
that fails in each script and an install with no configuration), in both layouts, the Caddyfile beside
the Caddy directory and set apart, with each listed setting the layout does not give pointed at a
directory of its own, and diffs the whole tree around each step. Each directory a step changes is a
place. Bash's execution trace and python's audit hook name every path each step touches, and each such
path must lie in a place. A second pass makes every path but the places read-only and must change
exactly what the first pass changed. Each place, in each state (writable, read-only, read-only with a
stale writable previous copy), for a first install, a re-install and a removal, with no trigger, a
failed rename and a failed reload, is then a member that either succeeds, or exits 1 with that script's
refusal (or with the reload's own message when a reload fails after a write) and leaves the tree byte
for byte as it was. A removal whose rename fails in a writable directory is not a member: it fails
after its writes and the removal promises no message on every exit; A7 pins that exit in the default
layout, and A38's measured runs take it in both layouts. The test fails closed: a step that changes no
block, a trace that names no block, and a second pass that makes nothing read-only each fail it. A
listed setting that chooses where a step writes adds a place to the diff, and that place's read-only
members fail A38 until the step checks the place before its first write.

Each Caddy step also refuses, before it reads or writes anything else, every entry of the environment
it received whose name starts with `DECKSTREAK_DEPLOY_` and is not one of the settings `deploy.sh`
lists in `SETTINGS`, whatever bytes follow the prefix, and names the one it refused. It reads the
entries as the kernel keeps them (`/proc/self/environ`, through bash's `mapfile` builtin, so the
script runs no command of its own first), because bash makes a variable only of a name it can spell
and hands every other entry to each program it runs. It also refuses a listed setting it received twice or without a value, a deploy
variable of its own shell that `SETTINGS` does not list (`${!DECKSTREAK_DEPLOY_@}`), and a run in which
it can read no environment at all, so the refusal is default-deny over the whole prefix: no unlisted
deploy setting reaches any function, trap, sourced file, child shell, host body or the renderer,
whatever form reads it and whether or not a test runs the branch that reads it, unless a name outside
the prefix has the shell run code before the refusal (the first limit below). A Caddy step therefore
needs a readable `/proc/self/environ`. `rollback.sh caddy-remove` runs only the lines that find
`deploy.sh` and exec it. A39 proves the refusal for the bare prefix, for each listed setting with a
suffix and for the prefix with each byte a name may hold (any but NUL and `=`) first, in the middle and
last, on both steps, with the tree unchanged; proves it for a listed setting given twice or without a
value, a prefixed entry without `=`, a run that received no environment and a deploy variable made
before the step starts by the file `BASH_ENV` names (a name outside the prefix, so it belongs to the first
limit below; the measured result for that member is a refusal, which does not show that the refusal covers
start-up code); and compares every command bash's DEBUG trap records before the refusal
(installed through `BASH_ENV` with `set -T`, so functions, subshells and the exec'd `deploy.sh` report
too) with the declared opening, exactly, so no command added before the refusal can read a setting
first. The Caddy functions consume `DECKSTREAK_DEPLOY_HOST`, `DECKSTREAK_DEPLOY_ELEVATE` and
`DECKSTREAK_DEPLOY_CHECKOUT` through their helpers, and each is listed. A new setting is refused until
it is listed.

Names outside that prefix are not settings: `PATH`, `HOME`, `TMPDIR`, the locale and the host
command's own names belong to the tools a step runs, and the refusal leaves them alone (ADR-198).
A name outside the prefix that the shell reads as code when it starts can run before the refusal
and stop it, and it is part of this limit. The reason it is left open: an entry that makes the shell
run start-up code can already run any code in the step, which is strictly more than an unlisted setting
can do, and the refusal guards against a misconfigured setting, not against code already placed in the
step's environment.
Neither test reads a script's text for the names it uses: a scan of read forms recognises only the forms
it lists, so it cannot close the names outside the prefix, and it is not a check here. A write on a
branch that none of the eight exits reaches in either layout is not measured by A38. Both limits are
left open by design (ADR-198).

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
| A38 | each Caddy step, run through its eight exits (a first install, a re-install, a removal, an install refused at validation, a reload that fails, a rename that fails in each script, an install with no configuration) in both layouts, changes and names paths only in the places its whole-tree diff measures, and changes the same paths when every other path is read-only; for each place in each state (writable, read-only, read-only with a stale writable previous copy), for a first install, a re-install and a removal, with no trigger, a failed rename and a failed reload, every member succeeds, or exits 1 with that script's refusal (or the reload's own message when a reload fails after a write) and leaves the tree byte for byte unchanged; a removal whose rename fails in a writable directory fails after its writes and is not a member; a step that changes or names no block, or a pass that makes nothing read-only, fails it (#423, #424) | `test_deploy_scripts.py` `every_directory_a_caddy_script_writes_or_undoes_is_checked_before_the_first_write` |
| A39 | each Caddy step, before it reads or writes anything else, refuses every entry of the environment it received whose name starts with `DECKSTREAK_DEPLOY_` and that `deploy.sh`'s `SETTINGS` does not list, whatever bytes follow the prefix, exits 1 naming it and leaves the tree unchanged (the bare prefix, each listed setting with a suffix, and the prefix with each byte but NUL and `=` first, in the middle and last, on both steps); refuses a listed setting given twice or without a value, a run that received no environment, and a deploy variable made before the step starts by the file `BASH_ENV` names (a name outside the prefix: the first limit, with a refusal as its measured result); and before the refusal each step's own commands are exactly its declared opening, as bash's DEBUG trap records them (#423) | `test_deploy_scripts.py` `a_caddy_step_reads_only_the_settings_it_names_and_refuses_any_other` |

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
A39: python3 -m unittest discover -s scripts/tests -p test_deploy_scripts.py -k a_caddy_step_reads_only_the_settings_it_names_and_refuses_any_other
```

The test file's `World.run` now starts the script in its own session and kills the whole group on a
timeout, so a stuck stub cannot orphan the host script. Rows S12718 to S12746 in
`scripts/mutation-rows.d/S12700-S12799.json` pin the absent-candidate guard of the undo (killer A16),
the undo after the block write (A18), after the candidate copy (A31) and after the import append
(A19), the removal's link guard (A20), the install's path guard (A23, A24), the absent-block undo
(A37), the undo of the kept copy and of the rename (A29, A30), the removal's guard for a hard link or
a pipe (A25), the refusal of a block copy that fails (A36), the Caddyfile checks of both scripts (A34),
the removal's four-name guard (A33), the directory checks and the directory line of each script (A38),
and the refusal of a setting the script does not list: its exit, the two steps it covers, the whole
prefix it reads, the environment it reads it from, the refusal of an environment it cannot read, and
its refusal of an entry it received unlisted, without a value or twice (A39). Files changed:
`deploy/deploy.sh`, `deploy/README.md`, `scripts/tests/test_deploy_scripts.py`,
`scripts/mutation-rows.d/S12700-S12799.json`,
`docs/red-first/SPEC-127.md`, `docs/decisions/ADR-198-the-install-undoes-every-write-and-a-linked-candidate-is-refused.md`
and `changelog.d/fix-caddy-undo-423.md`.

### What this amendment does NOT do

- It does not change what a successful install or removal does when every deploy setting in the
  environment is listed (#423, #424).
- It does not change the removal's refusal of a link at its candidate path, which stays A20 and A21 (#424).
- It does not refuse or classify an environment name outside the `DECKSTREAK_DEPLOY_` prefix, nor
  keep one that the shell reads as code when it starts from running before the refusal (an entry that
  makes the shell run start-up code can already run any code in the step, more than an unlisted setting
  can do), and A38
  does not measure a write on a branch that none of its eight exits reaches (ADR-198; #423).
- It does not name a failed temporary directory of the operator's own (#451), nor check the site
  import when the Caddyfile is set apart (#452).

## Amendment (SPEC-353): both layouts serve the site

This amendment lifts the second half of this file's last exclusion, which left the site import
unchecked when the Caddyfile is set apart (#452). SPEC-353 decides it, under ADR-364.

Both layouts serve the site: the Caddyfile in the Caddy directory, which is the default, and a
Caddyfile that `DECKSTREAK_DEPLOY_CADDYFILE` names elsewhere. In either, the import line the install
adds names the site block by its absolute path in the Caddy directory, and the candidate each step
validates and adapts is written in the live Caddyfile's own directory, the directory the reload
reads it from (SPEC-353 R1, R2). That directory therefore holds the candidate as well as the
rename target and the Caddyfile's previous copy. The Caddy directory is an absolute path of plain
characters (SPEC-353 R3).

Two of this file's tests change with it, and no criterion changes. The helper of A7 to A9 resolves
each `import` line of the live Caddyfile as Caddy resolves it, instead of matching the line's text.
A38's stale copies are the names each step writes in each directory: the block's previous copy in
the Caddy directory, and the candidate and the Caddyfile's previous copy in the Caddyfile's
directory.

The other half of that exclusion, a failed temporary directory of the operator's own, stays open
(#451).

The insertion this amendment makes is this section, appended after the file's last line, and
nothing above it is edited (SPEC-038 section 8, ruling (i)).

## Amendment: the reload names its adapter

This amendment closes #674. Every `caddy reload` in `caddy_install` and `caddy_remove` read the live
file by its name: Caddy picks an adapter from the file name when none is given, so a Caddyfile that
`DECKSTREAK_DEPLOY_CADDYFILE` names under a name that does not start with `Caddyfile` and does not
end in `.caddyfile` was read as JSON, while the validate and adapt calls beside it already named
`--adapter caddyfile`. Each reload now names the adapter too, whatever the file is called: the
reload and the restoring reload of the install, and the reload and the restoring reload of the
removal (ADR-127, the amendment of the same title).

## Acceptance criteria of the reload-adapter amendment

| id | criterion | test |
|---|---|---|
| A40 | with `DECKSTREAK_DEPLOY_CADDYFILE` naming a file whose name does not start with `Caddyfile`, an install and a removal each reload it, and a failing first reload of each is followed by the restoring reload; every one of those reloads carries `--adapter caddyfile` and reads the live file (#674) | `test_deploy_scripts.py` `a_reload_reads_a_caddyfile_under_another_name_with_the_caddyfile_adapter` |

The fence command for A40 is the one test by name, which runs in CI only because
`test_deploy_scripts.py` executes the deploy scripts:

```acceptance
A40: python3 -m unittest discover -s scripts/tests -p test_deploy_scripts.py -k a_reload_reads_a_caddyfile_under_another_name_with_the_caddyfile_adapter
```

Four rows in `scripts/mutation-rows.d/S12700-S12799.json` pin it, one per changed reload line (the
install's reload, the install's restoring reload, the removal's reload and the removal's restoring
reload), each killed by the test of A40. Rows S12703 and S12707 contain a restoring reload line, so
their find text moves to the new line with the same meaning.

The insertion this amendment makes is this section, appended after the file's last line, and
nothing above it is edited (SPEC-038 section 8, ruling (i)).
