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
