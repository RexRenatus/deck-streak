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
