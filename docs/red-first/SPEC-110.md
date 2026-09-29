# Red-first record: SPEC-110

The order of work: the SPEC promoted, ADR-110 accepted and SPEC-042 R12 amended (4468501); the
one-router census naming each drill reply and its caller (ae11464); the goldens (1910847); the
vault's inert shapes and the drill tables (99ae1ca); the vault's tests (d1d3acf) and their
implementation in four commits (81e05e0, c493f6e, 20bfa78, c479aac); the coordination inert shapes
(5eeebf4), tests (a2e0e1a) and implementation (91115c9, 61fe55a); the daemon's test (1b8c78e) and
implementation (3f0b83a, 25eb6f9, 2edd603); the bot's inert shapes (edfe7f2), tests (d876a07) and
implementation (5449642); the api's inert routes (45e4c60), test (4739275) and implementation
(df5ceb0); the mutation rows (bb90d4d); and the pins of the stem
refusal and the token prefixes (ef4ec29).

The goldens were generated from the predecessor's own functions at `27ee2bc`, with a scratch
`--registry` and a scratch `--out`, under `PYTHONDONTWRITEBYTECODE=1`. The predecessor's checkout
was left as it was: no change and no bytecode file added.

Each criterion was run at its red commit with the SPEC's own fenced command, selecting one test, and
failed by assertion for its own criterion, not by a compile error, a missing fixture or an empty
selection. The stubs compiled and refused or returned nothing: every drill `not_active`, no list,
no queue, empty keys, zero constants, no grade parse, a job table without the post-back, an empty
memory port, a token that encoded to nothing, commands that sent no reply and routes that answered
501.

```red-first
A1: red at d1d3acf: assertion `left == right` failed: active of {"notes":{"a-note":""},"today":200}; left: Array []
A1: green at c493f6e
A2: red at d1d3acf: assertion `left == right` failed: the reason "   "; left: "   ", right: ""
A2: green at c493f6e
A3: red at d1d3acf: assertion `left == right` failed: queue of the case-brief notes; left: Array []
A3: green at c493f6e
A4: red at d1d3acf: assertion `left == right` failed; left: NotActive, right: Appended { title: "Duty of care" }
A4: green at c479aac
A5: red at d1d3acf: assertion `left == right` failed; left: NotActive, right: EmptyAnswer
A5: green at c479aac
A6: red at a2e0e1a: the first answer is appended: NotActive
A6: green at 91115c9
A7: red at d1d3acf: the note write fails: Ok(NotActive)
A7: green at c479aac
A8: red at d1d3acf: assertion `left == right` failed: the graded parse of "bare-rule-later"; left: Null
A8: green at c493f6e
A9: red at a2e0e1a: assertion `left == right` failed: the clamped amount, on the study day, track law, scope once; left: []
A9: green at 91115c9
A10: red at a2e0e1a: assertion `left == right` failed: one grant; left: 0
A10: green at 91115c9
A11: red at d1d3acf: assertion `left == right` failed: the key of "law-1"; left: "law-1", right: "drill:law-1"
A11: green at c493f6e
A12: red at a2e0e1a: a missing root is the job's error: the job ran with outcome Ok
A12: green at 91115c9
A13: red at a2e0e1a: assertion `left == right` failed: the next poll completes the grade row; left: 0
A13: green at 91115c9
A14: red at a2e0e1a: the table lists the drill post-back
A14: green at 91115c9
A15: red at 1b8c78e: assertion `left == right` failed: at most ten entries: []; left: 0
A15: green at 3f0b83a
A16: red at d876a07: assertion `left == right` failed: the token of irac-one; left: String("")
A16: green at 5449642
A17: red at d876a07: the command sent no reply
A17: green at 5449642
A18: red at d876a07: the command sent no reply
A18: green at 5449642
A19: red at 4739275: assertion `left == right` failed; left: 501
A19: green at df5ceb0
A20: red at d1d3acf: drill_answers is exported and erased
A20: green at 20bfa78
A21: red at d1d3acf: assertion `left == right` failed: the constant vault_bridge.DRILL_POSTBACK_XP; left: Number(0), right: Number(15)
A21: green at c493f6e
A22: red at d1d3acf: assertion failed: has_answer(&mut tx, "irac-1").await.expect("a read")
A22: green at c479aac
A23: red at d1d3acf: xp 9 is refused by the table
A23: green at 20bfa78
A24: red at d1d3acf: assertion `left == right` failed; left: [], right: ["Free Recall", "Rule"]
A24: green at c493f6e
```

## 2026-09-29: post-green killer tests

These tests were added after the criteria were green, to kill the mutants the land bar's
mutation-verdict named missed (274 examined, 30 missed). They are disclosed here and are not
red-first criteria. Each was shown to pass with the mutant applied to the tests then in the tree,
and to fail with it once the new test was added.

- cdf44965: `drill_kills` in the vault, ten tests: the frontmatter strip and the comment cut, the
  answer headings, the deferral reason, the rollup tie, the ready label and self-check heading, the
  open refusal, the missing-folder list, the graded and active folders, and the recent grades.
- e278dc0c: `drill_paid_count::a_poll_reports_how_many_it_graded_and_how_many_it_paid` and
  `drill_answer::the_answer_heading_names_the_local_clock_of_the_rule` in coordination.
- cdd877b6: `drill_replies` in the bot, four tests: the button label, the list tail, the prompt cut
  and the view's deferral and answer button.
- c1edcd25: `drill_routes::the_list_counts_what_awaits_grading_and_what_is_deferred` in the api.
- 8a79433b: `drill_vault::a_configured_vault_opens_over_its_active_drills` in the daemon.
- 0fe0f0d1: two equivalence records for `type_of` (a date at the stem's start gives the empty prefix
  the fallback arm also gives).
- bf12e2d8 and 80e94331: mutation rows S11028 to S11040 for the pay count, the recent grade's xp, the
  graded folder, the open refusal, the daemon open, the constants and the post-back minute.
