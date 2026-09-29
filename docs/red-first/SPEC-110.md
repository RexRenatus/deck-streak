# Red-first record: SPEC-110

The order of work: the SPEC promoted, ADR-110 accepted and SPEC-042 R12 amended (4468501); the
one-router census naming each drill reply and its caller (ae11464); the goldens (1910847); the
vault's inert shapes and the drill tables (99ae1ca); the vault's tests (d1d3acf) and their
implementation in four commits (81e05e0, c493f6e, 20bfa78, c479aac); the coordination inert shapes
(5eeebf4), tests (a2e0e1a) and implementation (91115c9, 61fe55a); the daemon's test (1b8c78e) and
implementation (3f0b83a, 25eb6f9, 2edd603); the bot's inert shapes (edfe7f2), tests (d876a07) and
implementation (5449642); the api's inert routes (45e4c60), test (4739275) and implementation
(df5ceb0); the economy declaration (058671d); the mutation rows (bb90d4d); and the pins of the stem
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
