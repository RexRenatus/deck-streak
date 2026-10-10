# Red-first record: SPEC-381

The SPEC, ADR-392 and the schematic's new last section were committed first (fe2cf104). The census
of every AI call site (A9) was then committed alone (5a0590a7), before any stub. The stubs and the
tests of the other fourteen criteria followed in two commits, the server half (003d8659) and the
web half (b8b6c3ab): each stub keeps every input and lacks only its behaviour, so each red below is
an assertion on the missing behaviour, never a build failure. Each criterion's code was written
after its red was read, and each red is quoted from the run at the commit it names.

## The fence, line by line

Each of the 15 lines of SPEC-381 section 3's fence resolves to a test this delivery adds.

| # | criterion | test | added or named |
|---|---|---|---|
| 1 | A1 | `crates/api/tests/sensitive_decks.rs` `no_deck_is_kept_away_until_the_learner_marks_one` | added (step 3) |
| 2 | A2 | `crates/ingest/tests/sensitive.rs` `a_marked_deck_refuses_its_own_cards_its_childrens_and_the_cards_a_filtered_deck_borrowed` | added (step 3) |
| 3 | A3 | `crates/ingest/tests/sensitive.rs` `an_unresolved_deck_or_an_unreadable_set_reads_as_kept_away` | added (step 3) |
| 4 | A4 | `crates/agent/tests/deck_gate.rs` `a_kept_away_deck_is_withheld_before_the_runner_starts` | added (step 3) |
| 5 | A5 | `crates/agent/tests/deck_gate.rs` `an_unreadable_gate_is_withheld_before_the_runner_starts` | added (step 3) |
| 6 | A6 | `crates/agent/tests/deck_gate.rs` `cards_with_no_deck_scope_are_withheld_before_the_runner_starts` | added (step 3) |
| 7 | A7 | `crates/readings/tests/sensitive_day_set.rs` `a_kept_away_decks_cards_are_never_selected_into_a_day_set` | added (step 3) |
| 8 | A8 | `crates/coordination/tests/readings_resolve.rs` `an_unreadable_set_resolves_no_day_and_records_nothing` | added (step 3) |
| 9 | A9 | `crates/agent/tests/deck_gate_census.rs` `every_ai_call_site_passes_the_deck_gate_or_carries_no_deck_content` | added (step 2, alone) |
| 10 | A10 | `crates/api/tests/sensitive_decks.rs` `a_cross_site_or_signed_out_change_marks_nothing` | added (step 3) |
| 11 | A11 | `crates/ingest/tests/sensitive.rs` `the_marks_are_exported_and_an_erase_leaves_every_deck_readable` | added (step 3) |
| 12 | A12 | `web/app/src/lib/study/ai-decks.test.ts` `a deck's switch is off until it is turned on, and turning it on saves the mark` | added (step 3) |
| 13 | A13 | `web/app/src/lib/study/ai-decks.test.ts` `a deck under a kept-away deck shows on and cannot be changed` | added (step 3) |
| 14 | A14 | `web/app/src/lib/study/ai-decks.test.ts` `a failed save turns the switch back and says so` | added (step 3) |
| 15 | A15 | `web/app/src/lib/study/ai-decks-locales.test.ts` `every locale holds every AI-and-your-decks key` | added (step 3) |

The web lines run from `web/app`, where the file argument is the same path without its `web/app/`
prefix.

## Two disclosures of the stubs

- A9 is green from the server half's commit on: the stub `decide` asks the gate before `compose(`
  and the runner and drops its verdict, which is all the census pins. Its green line is therefore
  at 003d8659, the commit after its red.
- The migration's table had to be declared where the shipped schema and data-rights tests look for
  every table (the context map's row, `privacy.json`, `PRIVACY.md`, the data-rights declaration,
  the symmetry test's seed and the kernel's schema register), so those landed with the server
  half's stubs. Two shipped symmetry tests,
  `the_exported_tables_equal_the_erased_tables_over_every_port` and
  `erase_leaves_the_cron_fire_ledger_and_the_schema_table_untouched`, were red at 003d8659 with
  `ExportMismatch { context: "ingest", table: "sensitive_decks", problem: Omitted }`, because the
  declared table's export was the stub's, and turn green with A11.

## The reds and greens

Each line's command is the criterion's line in SPEC-381 section 3's fence, run at the commit named.

```red-first
A1: red at 003d865917abf764aef500aca8aacb261a578af9: crates/api/tests/sensitive_decks.rs:162:5: assertion `left == right` failed: a fresh server marks no deck; left: (501, Null); right: (200, Object {"decks": Array []})
A2: red at 003d865917abf764aef500aca8aacb261a578af9: crates/ingest/tests/sensitive.rs:85:5: assertion `left == right` failed; left: every one of the 7 labelled cards `Admitted`; right: `own card`, `child's card`, `borrowed from a child`, `sits in a marked filtered deck` `KeptAway`, the other three `Admitted`
A3: red at 003d865917abf764aef500aca8aacb261a578af9: crates/ingest/tests/sensitive.rs:128:5: assertion `left == right` failed; left: all five `Admitted`; right: `home deck unknown` and `current deck unknown` `Unresolved`, `marks unread` and `marks' read failed` `Unreadable`, `control` `Admitted`
A4: red at 003d865917abf764aef500aca8aacb261a578af9: crates/agent/tests/deck_gate.rs:111:5: assertion `left == right` failed; left: Delivered(Delivered { output: "a reading", .. }); right: Withheld(Withheld { class: "deck-sensitive", findings: ["1 card(s) kept away"] })
A5: red at 003d865917abf764aef500aca8aacb261a578af9: crates/agent/tests/deck_gate.rs:111:5: same left; right: Withheld(Withheld { class: "deck-unreadable", findings: ["2 card(s) not judged"] })
A6: red at 003d865917abf764aef500aca8aacb261a578af9: crates/agent/tests/deck_gate.rs:111:5: same left; right: Withheld(Withheld { class: "deck-unreadable", findings: ["cards carry no deck scope"] })
A7: red at 003d865917abf764aef500aca8aacb261a578af9: crates/readings/tests/sensitive_day_set.rs:80:5: assertion `left == right` failed: every card of the kept-away root, of its child and borrowed from it is held back; left: [("Alpha", [101, 102, 103]), ("Beta", [301, 302])]; right: [("Alpha", []), ("Beta", [301, 302])]
A8: red at 003d865917abf764aef500aca8aacb261a578af9: crates/coordination/tests/readings_resolve.rs:287:5: a failed read of the marks ends the resolution with its named error: Ok(Resolved { run: RunId(1), .. })
A9: red at 5a0590a7c3233102f9ff7ba87b3e1cff1dfd190a: crates/agent/tests/deck_gate_census.rs:718:5: assertion `left == right` failed: the census refused; left: ["crates/agent/src/duty.rs: decide calls no deck gate (`self.deck_gate.judge(`)"]; right: []
A9: green at 003d865917abf764aef500aca8aacb261a578af9
A10: red at 003d865917abf764aef500aca8aacb261a578af9: crates/api/tests/sensitive_decks.rs:211:5: assertion `left == right` failed: neither change marked the deck; left: (501, Null); right: (200, Object {"decks": Array []})
A11: red at 003d865917abf764aef500aca8aacb261a578af9: crates/ingest/tests/sensitive.rs:165:5: assertion `left == right` failed: the export carries every mark, every column; left: None; right: Some([{"created_at": 1700000000000, "deck_id": 1}, {"created_at": 1700000000500, "deck_id": 4}])
A12: red at b8b6c3abf41855cf1d662c17a9a9205b30d0a09c: web/app/src/lib/study/ai-decks.test.ts:139:68: AssertionError: expected [ [], [ …(4) ], '', [] ] to deeply equal [ [ [ '2', true ] ], [ …(4) ], …(2) ]: turning Spanish's switch on asked the server for nothing and the server holds no mark
A13: red at b8b6c3abf41855cf1d662c17a9a9205b30d0a09c: web/app/src/lib/study/ai-decks.test.ts:174:21: AssertionError: expected [ [ …(4) ], …(3) ] to deeply equal [ [ …(4) ], …(3) ]: every switch shows off; Default is marked and on, and Kana and Hiragana under it are on, locked and read "Kept away because Default is."
A14: red at b8b6c3abf41855cf1d662c17a9a9205b30d0a09c: web/app/src/lib/study/ai-decks.test.ts:221:46: AssertionError: expected [ [], [ [ …(4) ], [ …(4) ] ], '' ] to deeply equal [ [ [ '2', true ] ], …(2) ]: Spanish's switch is left on with no message; it is turned back off and the status reads "That change was not saved. Try again."
A15: red at b8b6c3abf41855cf1d662c17a9a9205b30d0a09c: web/app/src/lib/study/ai-decks-locales.test.ts:77:19: AssertionError: expected { en: [ …(8) ], …(6) } to deeply equal { en: [], 'zh-Hans': [], …(5) }: each of the seven locales lacks all eight keys ("en lacks ai_decks_title" through "zh-Hant lacks ai_decks_back")
```
