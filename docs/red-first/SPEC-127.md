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
```

## Fix round 1 (PR #357)

The verifier measured two defects the first round's tests did not pin: the restore ran in the wrong
order (a step left the live Caddyfile importing a missing block), and A1 and A5 asserted the word
"reload", which the fake's own "reload refused" satisfies. A6, A7 and A8 are new; A1 and A5 assert
the script's own words. The original lines above stand. A7 and A8 were committed first (2e42a60)
against the head's `deploy.sh` (4b2037d) and were red by assertion; the order fix followed
(9594d22), then the message assertions and A6 (5bf7f4a). A1, A5 and A6 are also replayed on dev's
`deploy.sh` (f10a483), where the messages do not exist.

```red-first
A1: red at 5bf7f4a: on dev's deploy.sh, AssertionError: 'the Caddy reload failed' not found in 'reload refused\n' : the message names the failed reload
A5: red at 5bf7f4a: on dev's deploy.sh, AssertionError: 'the Caddy reload failed' not found in 'reload refused\n'
A6: red at 5bf7f4a: on dev's deploy.sh, AssertionError: 'restoring reload' not found in 'reload refused\n' : a second failure is named distinctly
A7: red at 2e42a60: on 4b2037d's deploy.sh, AssertionError: None != '# DeckStreak\'s site block (SPEC-032 R6;[1829 chars]n}\n'
A8: red at 2e42a60: on 4b2037d's deploy.sh, AssertionError: True is not false : the live Caddyfile imports a block
A1: green at cd9e1b1aeff832698e98f39c2a32e6bf2a9b521a
A5: green at cd9e1b1aeff832698e98f39c2a32e6bf2a9b521a
A6: green at cd9e1b1aeff832698e98f39c2a32e6bf2a9b521a
A7: green at cd9e1b1aeff832698e98f39c2a32e6bf2a9b521a
A8: green at cd9e1b1aeff832698e98f39c2a32e6bf2a9b521a
```

Replays on the other script, by the same tests: A1, A5 and A6 are not red on 4b2037d's `deploy.sh`
(the round-one script already prints the message, reloads again after restoring and names the
second failure; the assertions are what changed, and rows S12706 and S12707 kill their removal).
A7 is not red on dev's `deploy.sh` (dev moves no block aside before the swap, so the block stays in
place). A8 is red on dev's `deploy.sh` for another reason: dev has no restore rename, so
'refused-rename' is not in the move log.
