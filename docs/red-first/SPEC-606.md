# Red-first record: SPEC-606

SPEC-606 (R1 to R6, A1 to A4). The SPEC, ADR-627 and the schematic were committed first, in the
commit before the tests. The tests of A1 to A4 and the two pins that grow (the two existing score
breakdown tests, whose row literals now end with the pillar's sentence) were then written over a
tree that carries no pillar sentence and no new message key, and each read red for its own
criterion. Every red below is quoted from the run at that commit, locally. A1 to A3 are Vitest
tests, A4 is a cargo test.

```text
A1: pnpm exec vitest run web/app/src/lib/score/ScoreBreakdown.test.ts -t "each pillar says in plain words what it measures beside its figure"
A2: pnpm exec vitest run web/app/src/lib/score/ScoreBreakdown.test.ts -t "the retention gap still says what retention measures"
A3: pnpm exec vitest run web/app/src/lib/score/pillar-about-locales.test.ts -t "every locale carries every text the score breakdown shows with the English figures"
A4: cargo test -p deck-streak-analytics --test pillar_about -- --exact each_pillar_sentence_states_its_pillars_own_figures
```

```red-first
A1: red at b089ee1f918d0fc5d6f3a1daea5f95fa7dad74da: AssertionError: Consistency: the row's last element: expected 'METER' to be 'P' // Object.is equality
A1: green at 966fb4c5a34b643bc132a2fd66bf5024b346f7ba
A2: red at b089ee1f918d0fc5d6f3a1daea5f95fa7dad74da: AssertionError: the retention row's last element: expected 'DIV' to be 'P' // Object.is equality
A2: green at 966fb4c5a34b643bc132a2fd66bf5024b346f7ba
A3: red at b089ee1f918d0fc5d6f3a1daea5f95fa7dad74da: AssertionError: expected { en: [], 'zh-Hans': [ …(8) ], …(5) } to deeply equal { en: [], 'zh-Hans': [], …(5) }
A3: green at 966fb4c5a34b643bc132a2fd66bf5024b346f7ba
A4: red at b089ee1f918d0fc5d6f3a1daea5f95fa7dad74da: assertion `left == right` failed left: [] right: ["70", "30", "25", "50", "0", "95", "100", "5", "70", "1", "5", "40", "80", "100", "3", "21", "100", "3", "30"]
A4: green at 966fb4c5a34b643bc132a2fd66bf5024b346f7ba
```

The two pins that grow read red at the same commit, b089ee1f918d0fc5d6f3a1daea5f95fa7dad74da: `shows five pillars and an absent retention as a gap` (AssertionError: expected 'Retention No review answered yet' to be 'Retention No review answered yet The …' // Object.is equality) and `shows a retention the day has as its value and its bar` (AssertionError: expected 'Retention 85' to be 'Retention 85 The share of review card…' // Object.is equality).

The tests were committed over a tree that carries no pillar sentence (b089ee1f), and read red by
name in the pull request run at its first push. The sentences, the translations and the rows came
in 966fb4c5 with the tests unchanged, and the run at this delivery head reads them green. The
commit after 966fb4c5 changes only this record.
