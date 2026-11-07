# Red-first record: SPEC-581

SPEC-581 (R1 to R4, A1 to A4). SPEC-581, ADR-600 and the schematic `docs/schematics/tagline.md`
were committed first (2569b2bc). The test edits of A1 and A2 were committed next and alone
(a730b5ef): `web/app/src/lib/smoke.test.ts` lines 19 and 21 and `web/app/src/routes/today.test.ts`
line 82, over the seven catalogs as dev held them, whose line 3 still named Anki. Each red below is
therefore its criterion's assertion on the tagline itself, never a missing name or a fixture. Line 3
of the seven catalogs came next and alone (5579bca1), and that commit changes no test file. The
changelog fragment and this record came last.

Each line below was measured on the builder's own tree, byte-identical to the commit it names, by
running its fence line from the repository root through the workspace's pinned pnpm: twice at the
red commit and once at the green one. Each run's test count is quoted, and a run that selected no
test would be refused.

```text
A1: pnpm exec vitest run web/app/src/lib/smoke.test.ts -t "has a tagline in every locale that names no Anki and no Telegram"
A2: pnpm exec vitest run web/app/src/routes/today.test.ts -t "Today names the app and the screen"
A3: pnpm exec vitest run web/app/src/lib/study/late-line.test.ts -t "every locale holds the late line"
A4: pnpm exec vitest run web/app/src/lib/study/update-now.test.ts -t "every locale holds the offer words"
```

```red-first
A1: red at a730b5ef95fe0e8944f695991899c3fb17b16ac2: AssertionError: expected 'Turn your daily Anki reviews into a s…' to match /^(?![\s\S]*(?:Anki|Telegram))[\s\S]+$/
A1: green at 5579bca1786c274d3829418dfcf8d71c89e57731
A2: red at a730b5ef95fe0e8944f695991899c3fb17b16ac2: AssertionError: expected 'DeckStreak Turn your daily Anki revie…' to contain 'Turn your daily reviews into a streak…'
A2: green at 5579bca1786c274d3829418dfcf8d71c89e57731
A3: not red: the streak-word guard of the late line, green before and after; the control that every translation kept its streak word
A4: not red: the streak-word guard of the update offer, green before and after; the control that every translation kept its streak word
```

At a730b5ef, A1 reads `Tests 1 failed | 2 skipped (3)`: its first failing locale is English, whose
tagline was received as "Turn your daily Anki reviews into a streak worth keeping.". A2 reads
`Tests 1 failed | 3 skipped (4)`: the rendered Today screen still holds the old English line. Both
read the same at the second run. A3 reads `Tests 1 passed | 2 skipped (3)` and A4 reads
`Tests 1 passed | 13 skipped (14)` at both runs. At 5579bca1, A1 and A3 each read
`Tests 1 passed | 2 skipped (3)`, A2 reads `Tests 1 passed | 3 skipped (4)` and A4 reads
`Tests 1 passed | 13 skipped (14)`, and the whole web suite, `pnpm exec vitest run web/app`, reads
`Test Files 127 passed (127)` and `Tests 845 passed (845)`.

A3 and A4 are the control. They read each locale's tagline for its own streak word (streak, racha,
série, 連続記録, 연속 기록, 连续记录, 連續紀錄), and neither test changed at the red commit or at the
green one, so every translation kept its streak word when Anki left it.
