# Red-first record: SPEC-075

This record is the red-first record of SPEC-075's one pull request, which closes #79 and #80. It
records the fifteen Rust and formal criteria (A1 to A8 and A11 to A17), delivered by PG1a; the web
criteria A9 and A10 are appended below them by the builder that delivers the Mini App (ruling 101),
and no line above is ever rewritten.

The order of work: the SPEC promoted with ADR-075 and its schematic, the registry and the four
goldens (1cab4b5, 83d73a0); the Rust criteria's tests beside inert drafts that compiled and
answered nothing (ce0c995); the Lean entry `formal/lean/Formal/Exchange.lean`, its vector writer, the
vectors it wrote and A13's test, while `exchange_rates` was still inert (8f97749); the implementation
(7f5e751); the Lean covers stamped and the vectors rewritten (e0406d6, d678e46); the rows of
`scripts/mutation-rows.d/S07500-S07599.json`.

Each criterion was run at its red commit in its own test binary, with `--test-threads=1`, and
failed by assertion, not by a compile error, a missing fixture or an empty selection. The drafts
compiled and returned nothing: no best day and no board rows, every source its own bucket, no rate,
no XP row and no graduation, no window and an empty readout; the two routes were not registered, so
each request was answered 404. A1's and A3's inputs are abbreviated below with `...`.

```red-first
A1: red at ce0c995: assertion `left == right` failed: {"rollups":[[20000,19],[19999,33],...]...}; left: Array [], right: Array [Object {"detail": Number(19636), "label": String("🏅 Best day"), "value": Number(95)}, ...]
A1: green at 7f5e751
A2: red at ce0c995: assertion `left == right` failed: /api/board None; left: 404, right: 401
A2: green at 7f5e751
A3: red at ce0c995: assertion `left == right` failed: {"rollups":[{"day":20000,"graduations":4}],"rows":[{"amount":30,"day":20000,"source":"quest:1"},...]}; left: 0, right: 1
A3: green at 7f5e751
A4: red at ce0c995: assertion `left == right` failed; left: [], right: [SourceRate { source: "focus", total_xp: 110, graduated_cards: 0, rate: None, rate_defined: false }, SourceRate { source: "reviews", total_xp: 7, graduated_cards: 2, rate: Some(3.5), rate_defined: true }]
A4: green at 7f5e751
A5: red at ce0c995: assertion `left == right` failed: "quest:2"; left: "quest:2", right: "quest:"
A5: green at 7f5e751
A6: red at ce0c995: assertion `left == right` failed; left: None, right: Some((StudyDay(19998), StudyDay(20000)))
A6: green at 7f5e751
A7: red at ce0c995: assertion `left == right` failed: {"days":1,"today":20000}; left: Object {"end": Null, "start": Null}, right: Object {"end": Number(20000), "start": Number(20000)}
A7: green at 7f5e751
A8: red at ce0c995: assertion `left == right` failed; left: [], right: [SourceRate { source: "focus", total_xp: 9, graduated_cards: 0, rate: None, rate_defined: false }, SourceRate { source: "quest:", total_xp: 75, graduated_cards: 4, rate: Some(18.75), rate_defined: true }, ...]
A8: green at 7f5e751
A11: red at ce0c995: assertion `left == right` failed; left: None, right: Some(DayScore { day: StudyDay(19998), score: 88 })
A11: green at 7f5e751
A12: red at ce0c995: assertion `left == right` failed; left: [], right: [XpRow { study_day: StudyDay(19999), source: "reviews", amount: 12 }, XpRow { study_day: StudyDay(20000), source: "quest:1", amount: 30 }]
A12: green at 7f5e751
A13: red at 8f97749: assertion `left == right` failed: {"graduations":[],"rates":[{"graduated_cards":0,"rate_defined":false,"source":"quest:","total_xp":5}],"rows":[{"amount":5,"day":1,"source":"quest:1"}]}; left: [], right: [Object {"graduated_cards": Number(0), "rate_defined": Bool(false), "source": String("quest:"), "total_xp": Number(5)}]
A13: green at 7f5e751
A14: red at ce0c995: assertion `left == right` failed: days=x; left: 404, right: 400
A14: green at 7f5e751
A15: red at ce0c995: assertion `left == right` failed; left: 404, right: 200
A15: green at 7f5e751
A16: red at ce0c995: assertion `left == right` failed; left: {}, right: {StudyDay(19998): 0, StudyDay(19999): 7}
A16: green at 7f5e751
A17: red at ce0c995: assertion `left == right` failed; left: 404, right: 200
A17: green at 7f5e751
```

The web criteria A9 and A10 (ruling 101), appended by the builder that delivered the Mini App. Each
was run by its fence command at its red commit (3b80599), against an inert `BoardSection.svelte` and
an inert `ExchangeCard.svelte` that rendered nothing, and failed by assertion: the test's population
guard found no board row and no readout row to judge.

```red-first
A9: red at 3b80599: AssertionError: examined 0 board row(s): the population is empty, so nothing was judged: expected 0 to be greater than 0
A9: green at b81606b
A10: red at 3b80599: AssertionError: examined 0 readout row(s): the population is empty, so nothing was judged: expected 0 to be greater than 0
A10: green at b81606b
```

Disclosure for A9 and A10: the green commit b81606b also adds test material, and only adds it. It
carries the new `exchange.test.ts` and `board.test.ts` for the two parsers, 13 added lines in
`BoardSection.test.ts` and new cases in the two route tests, with no existing assertion edited,
removed or skipped; the red commit 3b80599 held the A9 and A10 tests as they stand.
