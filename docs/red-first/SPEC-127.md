# Red-first record: SPEC-127

The tests of A1 to A5 were committed first (2df59de) against dev's `deploy.sh`, on the existing
fakes: the `caddy` stub gained a reload that fails on demand and a log of the previous copies it can
see. Each was red by assertion. The fix and the band's rows followed (83dfdb4); the row S12702 then
showed the Caddyfile restore was unpinned by A1 (the Caddyfile is unchanged on a second install), so
its killer became A3 (cbf28ee).

```red-first
A1: red at 2df59de: AssertionError: '...new.example.org {...' != '...app.example.org {...' (the new block stays after a failed reload)
A2: red at 2df59de: AssertionError: 'restoring reload' not found in 'reload refused\n' : a second failure is named distinctly
A3: red at 2df59de: AssertionError: True is not false : the new block is removed
A4: red at 2df59de: AssertionError: Lists differ: ['0'] != ['1']
A5: red at 2df59de: AssertionError: None != '# DeckStreak's site block ...' (the removal deleted the block before its failed reload)
A1: green at cbf28ee1338f1d2f643704d185ef7b86ffa5d06f
A2: green at cbf28ee1338f1d2f643704d185ef7b86ffa5d06f
A3: green at cbf28ee1338f1d2f643704d185ef7b86ffa5d06f
A4: green at cbf28ee1338f1d2f643704d185ef7b86ffa5d06f
A5: green at cbf28ee1338f1d2f643704d185ef7b86ffa5d06f
A6: red at 5bf7f4a: on dev's deploy.sh, AssertionError: 'restoring reload' not found in 'reload refused\n' : a second failure is named distinctly
A7: red at 2e42a60: on 4b2037d's deploy.sh, AssertionError: None != '# DeckStreak\'s site block (SPEC-032 R6;[1829 chars]n}\n'
A8: red at 2e42a60: on 4b2037d's deploy.sh, AssertionError: True is not false : the live Caddyfile imports a block
A6: green at cd9e1b1aeff832698e98f39c2a32e6bf2a9b521a
A7: green at cd9e1b1aeff832698e98f39c2a32e6bf2a9b521a
A8: green at cd9e1b1aeff832698e98f39c2a32e6bf2a9b521a
A9: red at a0b0b5b485b0d2981c8a081f0d8ea32cd6843aa0: with the removal's two restore lines swapped, AssertionError: True is not false : the live Caddyfile imports a block
A9: green at b766c2a1308977b850073d346d5e8d96c4968c9c
A10: red at 3b4267ef6c19a8ef15346e7390de15c0334a61bb: AssertionError: 'deploy: refused' not found in '' : the removal says which step stopped
A10: green at a224cc8d0fdf1e81a6f0e9bf45670fbf7d00735f
A11: not red: the removal's first refusal already printed `deploy: refused` at a4036b3; the test pins it (#361)
A12: red at 87822bc23867f75e83f827f203a04d171aeadd78: AssertionError: 'the candidate Caddyfile could not be written' not found in "deck-streak-host: line 8: .../deck-streak.candidate: Is a directory\n/usr/bin/mv: cannot overwrite non-directory ..." : the removal names the failed write
A13: red at 87822bc23867f75e83f827f203a04d171aeadd78: AssertionError: 'deploy: refused' not found in "find: '.../deck-streak.candidate': No such file or directory\n" : the removal says so with no candidate left
A14: red at ce8c72cd3c9090b41513e5ccb35935cfebe51247: AssertionError: 'the candidate Caddyfile could not be written' not found in "grep: .../Caddyfile: Is a directory\ncp: -r not specified; omitting directory '.../Caddyfile'\n" : grep's read failure is a failed write, not a no-match
A15: not red: grep keeping no line already succeeded at 24b880e; the test pins the status that the two-step write must keep (#384)
A12: green at dcfcc9cf4f3173bcdbeecfee48f493a66cd7757a
A13: green at dcfcc9cf4f3173bcdbeecfee48f493a66cd7757a
A14: green at dcfcc9cf4f3173bcdbeecfee48f493a66cd7757a
A16: red at da30fb25261d1fafaa295e59e206010bd1063605: AssertionError: 'the Caddy configuration was refused' not found in "find: '/tmp/tmpons55uop/host/etc/caddy/deck-streak.candidate': No such file or directory\n" : the refusal is printed
A17: red at da30fb25261d1fafaa295e59e206010bd1063605: AssertionError: 'the Caddy configuration was refused' not found in 'grep: /tmp/tmpwuzhmnj1/host/etc/caddy/deck-streak.candidate: Is a directory\ndeck-streak-host: line 11: /tmp/tmpwuzhmnj1/host/etc/caddy/deck-streak.candidate: Is a directory\n' : the refusal is printed
A18: red at da30fb25261d1fafaa295e59e206010bd1063605: AssertionError: 'the Caddy configuration was refused' not found in 'deck-streak-host: line 9: /tmp/tmpwak27vmv/host/etc/caddy/deck-streak.caddy: Permission denied\n' : the refusal is printed
A19: red at da30fb25261d1fafaa295e59e206010bd1063605: AssertionError: 'the Caddy configuration was refused' not found in 'deck-streak-host: line 11: /tmp/tmp1413f6eo/host/etc/caddy/deck-streak.candidate: Permission denied\n' : the refusal is printed
A20: red at da30fb25261d1fafaa295e59e206010bd1063605: AssertionError: b'' != b'example.org {\n\trespond 200\n}\nimport deck-streak.caddy\n' : the live Caddyfile is byte for byte
A21: red at da30fb25261d1fafaa295e59e206010bd1063605: AssertionError: 0 == 0 : a dangling link refuses the removal
A22: not red: the test was added with the fix already in place (5192f49); see the replay below the fence
A23: red at 159b1877c6207ef0596e20ea9793276745b3c911: AssertionError: 0 == 0 : a linked candidate refuses the install
A24: red at 159b1877c6207ef0596e20ea9793276745b3c911: AssertionError: 0 == 0 : a hard-linked block refuses the install
A25: red at 159b1877c6207ef0596e20ea9793276745b3c911: AssertionError: b'' != b'example.org {\n\trespond 200\n}\nimport deck-streak.caddy\n' : the live Caddyfile is byte for byte
A26: red at 159b1877c6207ef0596e20ea9793276745b3c911: AssertionError: the removal waited for a reader of a FIFO at its candidate path
A27: red at 159b1877c6207ef0596e20ea9793276745b3c911: AssertionError: 'the Caddy configuration was refused' not found in "deck-streak-host: line 18: /tmp/tmptfxp_pw2/host/etc/caddy/deck-streak.caddy: Permission denied\nfind: '/tmp/tmptfxp_pw2/host/etc/caddy/deck-streak.caddy': No such file or directory\n" : the refusal is printed
A28: red at 159b1877c6207ef0596e20ea9793276745b3c911: AssertionError: False is not true : the directory and its file are kept
A29: red at 159b1877c6207ef0596e20ea9793276745b3c911: AssertionError: 'the Caddy configuration was refused' not found in "cp: cannot create regular file '/tmp/tmpfevq3l54/host/etc/caddy/Caddyfile.previous': Permission denied\n" : the refusal is printed
A30: red at 159b1877c6207ef0596e20ea9793276745b3c911: AssertionError: 'the Caddy configuration was refused' not found in '' : the refusal is printed
A32: red at 159b1877c6207ef0596e20ea9793276745b3c911: AssertionError: 'the Caddy configuration was refused' not found in "cp: cannot create regular file '/tmp/tmph36m10n5/host/etc/caddy/deck-streak.caddy.previous': Permission denied\n" : the refusal is printed
A31: not red: the test passes at 159b187's and the head's `deploy.sh` (on dev's it fails: the install there has no undo for the copy), so it is the killer that S12720 needs once the install guard refuses a link before the copy (see the addendum below the fence)
A33: red at 9c1ac0df482428d364ced130cbad6447fecc1ffe: AssertionError: 0 == 0 : the removal refuses
A34: red at 9c1ac0df482428d364ced130cbad6447fecc1ffe: AssertionError: deploy.sh waited on a FIFO at the live Caddyfile
A35: red at 9c1ac0df482428d364ced130cbad6447fecc1ffe: AssertionError: 'the Caddy configuration was refused' not found in "deck-streak-host: line 23: /tmp/tmp96dogtc2/host/etc/caddy/deck-streak.caddy: Permission denied\nfind: cannot delete '/tmp/tmp96dogtc2/host/etc/caddy/deck-streak.candidate': Permission denied\n" : the refusal is printed
A36: not red: the test passes at the head's `deploy.sh`, whose install already refuses a block it cannot copy aside, so it is the killer that S12730 needs once the install guard refuses a directory it cannot write (see the addendum below the fence)
A37: not red: the test passes at the head's `deploy.sh`, whose undo already tolerates an absent block, so it is the killer that S12725 needs once the install guard refuses before the undo runs (see the addendum below the fence)
A38: red at ccc7a07165cf9bd5a3ecfa389df1ab1055cde766: AssertionError: False is not true : no refusal line: "cp: cannot create regular file '/tmp/tmpmooww968/host/etc/cfdir/Caddyfile.previous': Permission denied\n"
A39: not red: the test passes at the head that holds A38's red test, because `deploy.sh` already reads only the settings the population varies; it is the guard that keeps the population's axes complete (see the addendum below the fence)
A16: green at 28588e0fbbb556f5c686ac937c2180377c78bfbd
A17: green at 28588e0fbbb556f5c686ac937c2180377c78bfbd
A18: green at 28588e0fbbb556f5c686ac937c2180377c78bfbd
A19: green at 28588e0fbbb556f5c686ac937c2180377c78bfbd
A20: green at 28588e0fbbb556f5c686ac937c2180377c78bfbd
A21: green at 28588e0fbbb556f5c686ac937c2180377c78bfbd
A23: green at 2c25d22b82dd41f8f38a0754039e3549a496f45d
A24: green at 2c25d22b82dd41f8f38a0754039e3549a496f45d
A25: green at 2c25d22b82dd41f8f38a0754039e3549a496f45d
A26: green at 2c25d22b82dd41f8f38a0754039e3549a496f45d
A27: green at 2c25d22b82dd41f8f38a0754039e3549a496f45d
A28: green at 2c25d22b82dd41f8f38a0754039e3549a496f45d
A29: green at 2c25d22b82dd41f8f38a0754039e3549a496f45d
A30: green at 2c25d22b82dd41f8f38a0754039e3549a496f45d
A32: green at 2c25d22b82dd41f8f38a0754039e3549a496f45d
A33: green at dc8e42216b229361a1def85bd1230884c9f11e00
A34: green at dc8e42216b229361a1def85bd1230884c9f11e00
A35: green at dc8e42216b229361a1def85bd1230884c9f11e00
A38: green at b46e2daa3a29f0a4db575f971f107febb989e105
```

## Fix round 1 (PR #357)

The verifier measured two defects the first round's tests did not pin: the restore ran in the wrong
order (a step left the live Caddyfile importing a missing block), and A1 and A5 asserted the word
"reload", which the fake's own "reload refused" satisfies. A6, A7 and A8 are new; A1 and A5 assert
the script's own words. The original lines above stand and A6 to A8 join them in the record's
fence. The fresh red lines for A1 and A5, whose assertions changed, are quoted below the fence,
because the probe records one red per criterion. A7 and A8 were committed first (2e42a60) against
the head's `deploy.sh` (4b2037d) and were red by assertion; the order fix followed (9594d22), then
the message assertions and A6 (5bf7f4a). A1, A5 and A6 are also replayed on dev's `deploy.sh`
(f10a483), where the messages do not exist.

```text
A1: red at 5bf7f4a: on dev's deploy.sh, AssertionError: 'the Caddy reload failed' not found in 'reload refused\n' : the message names the failed reload
A5: red at 5bf7f4a: on dev's deploy.sh, AssertionError: 'the Caddy reload failed' not found in 'reload refused\n'
A1: green at cd9e1b1aeff832698e98f39c2a32e6bf2a9b521a
A5: green at cd9e1b1aeff832698e98f39c2a32e6bf2a9b521a
```

Replays on the other script, by the same tests: A1, A5 and A6 are not red on 4b2037d's `deploy.sh`
(the round-one script already prints the message, reloads again after restoring and names the
second failure; the assertions are what changed, and rows S12706 and S12707 pin the messages).
A7 is not red on dev's `deploy.sh` (dev moves no block aside before the swap, so the block stays in
place). A8 is red on dev's `deploy.sh` for another reason: dev has no restore rename, so
'refused-rename' is not in the move log.

## Fix round 2 (PR #357)

The verifier measured that SPEC-127 R4's restore order (the block before the Caddyfile) was stated
and not pinned: a script with the two restore lines swapped passed every test, and with the block's
restore rename failing it left a live Caddyfile importing a missing block. A9 is new: a `mv` that
refuses the rename back onto the block, and a removal whose reload fails. A9 joins the record's
fence. Its red commit (a0b0b5b) carries the test beside the head's `deploy.sh` with the two restore
lines swapped, red by assertion; the next commit puts the order back and adds the row S12710, and A9
is green there.

Replays on the other scripts, by the same test: A9 is red on dev's `deploy.sh` for another reason,
as A8 is ('refused-rename' is not in the move log, because dev has no restore rename), and it is not
red on 4b2037d's `deploy.sh`, whose removal already restores the block first.

## Amendment 2026-09-29 (issue #361)

A10 covers the removal's second refusal, `caddy adapt --validate`: its stub flag makes `adapt` refuse
while `validate` passes. The test was committed first (3b4267e) against the unchanged `deploy.sh`,
red by assertion because stderr was empty; the one-line fix and the row S12711 followed (a224cc8).

A11 covers the removal's first refusal, `caddy validate`: its stub flag (`caddy-refuses`) makes
`validate` refuse. The refusal already printed its message at `a4036b3`, so the test is green from
the start and is recorded `not red`; it pins the line. Four plants on a copy of the head's
`deploy.sh` each turn A11 red by assertion: the first refusal's echo removed (`'deploy: refused' not
found in ''`), its `exit 1` made `exit 0` (`0 == 0 : a refused validation`), its `find` removed
(`Lists differ: ['deck-streak.candidate'] != []`), and its echo sent to stdout (`'deploy: refused'
not found in ''`). The second refusal's echo removed leaves A11 green and turns A10 red, so the two
tests are independent. Row S12712 pins the first refusal's message (killer A11).

## Amendment 2026-09-29 (issue #384)

A failed write of `caddy-remove`'s candidate file used to be hidden by `|| true`, and each refusal's
cleanup then failed on the absent candidate before it printed `deploy: refused`. A12 and A13 were
committed first (87822bc) against the unchanged `deploy.sh`, red by assertion; A14 and A15 followed
(ce8c72c), A14 red by assertion and A15 recorded `not red` because `grep` keeping no line already
succeeded. The fix and rows S12713 to S12717 came in dcfcc9c. dcfcc9c changes `deploy.sh` and the rows and edits no test file;
the one later commit that edits a test file is f369d2f, which adds a presence assertion to A15 (the
removal's reload ran) so the probe's absence-only rule passes, and A15 stays `not red`. Rows S12711 and
S12712 keep their killers and are re-anchored on the changed lines.

## Amendment 2026-09-29 (issues #423 and #424)

The tests of A16 to A21 were committed first (da30fb2) against dev's `deploy.sh` and run under
`LC_ALL=C`, because `find` and `mv` quote differently by locale: exactly seven tests were red by
assertion (A16 has two, one per refusal) and the thirty-one tests that dev holds stayed green. The one
red line of A16 above is the validation test's; the adapt test's is the same message. The fix
followed (28588e0), which changes `deploy.sh` and edits no test file. After it, thirty-eight tests
pass. The tests' subprocess now runs in its own session and the whole group is killed on a timeout.

A link to `/dev/full` at the candidate path was not made a test: the run ends in under a second with a
non-zero status at dev and at the head, so it pins nothing that a plain unwritable path does not.

A22 covers the install with a link to the live Caddyfile at its candidate path. The row S12720 was
first pinned by A17, whose test still passed with the row's mutant because the later import append
refuses a directory too, so the test of A22 was added (5192f49, the only later commit that edits a test
file) and the row's killer moved to it. On dev's `deploy.sh` the test is red by assertion, replayed under `LC_ALL=C`:

```text
A22: red on dev's deploy.sh: AssertionError: 'the Caddy configuration was refused' not found in "cp: '/tmp/tmp2vnj2dok/host/etc/caddy/Caddyfile' and '/tmp/tmp2vnj2dok/host/etc/caddy/deck-streak.candidate' are the same file\n" : the refusal is printed
```

With it, thirty-nine tests pass at the head.

## Amendment 2026-09-29, second round (issues #423 and #424)

The tests of A23 to A32 were committed first (159b187) against `deploy.sh` as it stood, and run whole
under `LC_ALL=C`: nine tests were red by assertion, and the thirty-nine that stood stayed green. The
test of A26 turns a timeout into a failure, so a hang is red by assertion and not an error. The test
of A31 is green at that commit: it is the killer that the row S12720 needs, because the install guard
now refuses a link at the candidate path before the copy, which turns A22's test into a no-op for that
row. The fix followed (2c25d22), which changes `deploy.sh` and edits no test file. After it, forty-nine
tests pass.

One later commit edits a test file (5cd0e70): the tests of A30 and A31 gain a presence assertion on the
live Caddyfile, beside their absence assertions. It changes no verdict: A30 stays red at 159b187 and green
at 2c25d22, and forty-nine tests still pass.

The rows S12701 and S12704 were re-anchored to the changed lines, the killer of S12720 moved to the
test of A31, and the rows S12723 to S12730 were added; every row of the band is proved killed by its
full id on a clean committed tree.

## Amendment 2026-09-29, third round (issues #423 and #424)

The tests of A33 to A37 were committed first (9c1ac0df) against `deploy.sh` as it stood, and run whole
under `LC_ALL=C`: fifteen subtests of three tests were red by assertion (A33 ten, A34 two, A35 three),
with no error, and the fifty-one other tests stayed green. Every pipe case turns a timeout
into a failure, so a hang is red by assertion and not an error. The tests of A36 and A37 are green at
that commit: they are the killers that the rows S12730 and S12725 need, because the guard now refuses
before the copy and before the undo, which turns the old killers of those two rows into no-ops. The fix
followed (dc8e4221), which changes `deploy.sh` and edits no test file. After it, fifty-four tests
pass.

One later commit (bacacc2b) edits a test file: the test of A14 plants a file with mode 0 at the Caddyfile in place of
a directory, because the removal's new precondition refuses a directory first and turned the old plant
into a no-op for the row S12714. It changes no verdict: fifty-four tests still pass.

The rows S12725 and S12730 moved to the tests of A37 and A36, and the rows S12731 to S12735 were added;
each moved row's mutant survives its old killer and is killed by its new one, and every row of the band
is proved killed by its full id on a clean committed tree.

## Amendment 2026-09-29, fourth round (issues #423 and #424)

The test of A38 was committed first (ccc7a071) against `deploy.sh` as it stood, and run whole under
`LC_ALL=C`: twelve of its seventy-eight members were red by assertion, with no error, and the fifty-five
other tests stayed green. The red line above is the first member to fail. The test of A39 is green at
that commit, because it reads the settings and passes at the head's `deploy.sh`. The fix followed
(b46e2daa), which changes `deploy.sh` and edits no test file. After it, fifty-six tests pass and A38
examines seventy-eight members.

The rows S12736 and S12737 were added in the next commit, and the rows S12731 and S12733 moved to the
test of A38, because the new line turns the old killer of each into a no-op. Each of the four mutants
survives the tests that the previous head held and is killed by the test of A38, and every row of the
band is proved killed by its full id on a clean committed tree.

The fourth round's next change derives the places of A38 from the path settings through a map, and the
test asserts that the map's keys equal the settings list. It is a test-only strengthening that pins
behaviour already so at the head, with no production change. Its red is a further path setting in the
script and in the settings list, and the test of A39 has its own red with the setting in the script alone.
With the setting in both, A38 fails with
the line `a setting is no place`, and A39 passes. With the setting in the
script only, A39 fails with the line `a new setting is an axis`. Both plants
were run in a scratch copy, never in the worktree.
