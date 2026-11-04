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
```
