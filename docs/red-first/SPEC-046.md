# Red-first record: SPEC-046

The order of work was: the SPEC promoted and ADR-046 accepted (b2dc138); inert stubs for the
symbols the tests name, and the workspace's three new dependencies (2419e13); the two new tables
seeded in the data-rights census (bb63fd5); the anchor parity goldens and their registration
(91eb075); the readings' acceptance tests (3a7ddeb) and their implementation (f32737f); a default
new-words method on the notes port (0bbabda); the compose amendment's tests (239d8d7) and its
implementation (bee65b7); the generation's acceptance tests (2744b9e) and its implementation
(9720d0f); the second law golden and the packs' reading rows (f957be0); and the hand-proved rows
with the tests that pin their constants (93035e4).

The parity goldens were generated from the predecessor's own functions, `preread.py:anchor_for_note`
and `preread.py:is_anchor_usable`, at `27ee2bc`, over synthetic note texts with markup, entities,
full-width characters and short fields: 112 cases for the anchor and 32 for its usability. The
generator wrote to a scratch registry and output, with bytecode writing off, and the predecessor's
checkout read the same before and after, with no bytecode in it.

The readings tests were committed (3a7ddeb) beside stubs whose behaviour was empty: a word target
of zero, no sections, no anchor, no gate finding, an id and a count of nothing, and a data-rights
port that declared two tables. Each criterion was run there with the SPEC's own fenced command,
selecting one test, and the whole of each test file was run there as well: every test in
`form`, `coverage`, `minutes` and `rights` failed, each by assertion. The generation tests were
committed (2744b9e) beside a use case that resolved no topic, and all eleven of `readings_generate`
failed there. Seven of them fail at the test's own `expect("the topic ended the day")`, which is
the assertion that a topic reached an end; the remaining four fail on a count that is zero where a
topic, an attempt or a resolution was expected.

```red-first
A1: red at 3a7ddeb: assertion `left == right` failed: the band's floor; left: 0, right: 800
A1: green at f32737f
A2: not red: the rows read the golden readings and the existing law golden was already inside the band; the second law golden and this test arrived together, so no earlier commit had a test to fail
A3: red at 3a7ddeb: the finding names the note whose anchor is absent: []
A3: green at f32737f
A4: red at 3a7ddeb: a seed note left uncited is named: []
A4: green at f32737f
A5: red at 3a7ddeb: "- item" is a list marker
A5: green at f32737f
A6: red at 3a7ddeb: assertion `left == right` failed: the anchor of "   \t\n  "; left: Some("   \t\n  "), right: Some("")
A6: green at f32737f
A7: red at 2744b9e: the topic ended the day
A7: green at 9720d0f
A8: red at 2744b9e: the topic ended the day
A8: green at 9720d0f
A9: not red: the blocking rows read the golden readings and every one already passed on them; this test arrived with the second law golden, so no earlier commit had a test to fail
A10: red at 2744b9e: assertion `left == right` failed; left: 0
A10: green at 9720d0f
A11: red at 2744b9e: assertion `left == right` failed; left: 0
A11: green at 9720d0f
A12: red at 2744b9e: the topic ended the day
A12: green at 9720d0f
A13: red at 2744b9e: the topic ended the day
A13: green at 9720d0f
A14: red at 2744b9e: assertion `left == right` failed; left: 0
A14: green at 9720d0f
A15: red at 2744b9e: the topic ended the day
A15: green at 9720d0f
A16: red at 3a7ddeb: assertion `left == right` failed: two hundred words is one minute; left: 0, right: 1
A16: green at f32737f
A17: red at 3a7ddeb: assertion `left == right` failed: all four tables are exported, in the declaration's order; left: ["reading_topic_days", "reading_runs"], right: ["reading_topic_days", "reading_runs", "readings", "reading_attempts"]
A17: green at f32737f
A18: red at 2744b9e: the topic ended the day
A18: green at 9720d0f
A19: red at 2744b9e: assertion `left == right` failed: no day set is resolved; left: 0
A19: green at 9720d0f
```

DISCLOSURE, the compose amendment (not a criterion): the SPEC's manifest did not list the agent's
`compose.rs`, and the prompt's three new slots, `{{form}}`, `{{word_target}}` and `{{repair}}`,
are refused by it as unknown slots. On the orchestrator's ruling they are trusted engine text:
fence-checked like the rules and the persona, never wrapped as untrusted, and the repair value is
built only from the engine's own text and is empty on the first attempt. The amendment is a manifest
row and this sentence. Its two tests were committed red first (239d8d7): `a_fence_in_the_form_the_target_or_the_repair_is_refused`
failed with `left: Err(UnknownSlot("form")), right: Err(FenceInTrusted)` and
`the_form_target_and_repair_slots_are_trusted_engine_text` failed with `a prompt:
UnknownSlot("form")`; both pass at bee65b7, and the agent's tests pass whole.

DISCLOSURE, A8 and the generation file (`readings_generate.rs`): its body changed at 9720d0f, the
generation's green commit, in two ways. The file's lint allowance grew from `expect_used`,
`unwrap_used` and `panic` to also allow `format_push_string`, and the fixture of the
unusable-seed test gained `rig.notes.0.remove(&9_999)`, because its notes fixture wrongly held the
note the test means to be missing. No assertion changed.

DISCLOSURE, the readings tests: `coverage.rs`'s lint allowance grew by `format_push_string` at
f32737f; no assertion changed.

DISCLOSURE, A2 and A9 are pack-dependent: they run the packs' own probes from a checkout of the
packs and skip where that checkout is absent, and both are marked not red above because they
arrived together with the golden they read. A9 runs the nine blocking rows that read a reading;
`ask-before-tell` and `every-choice-explained` examine no reading and are not asserted.

DISCLOSURE, tests added after green (not criteria): `repair.rs` and `attempts.rs` under
`crates/readings/tests` pin the repair's cap, its quote floor and the retention's whole value for
the hand-proved rows (93035e4). They pass on the code they were written against, which is why they
are killers for rows and not acceptance tests. The persona test's list of golden readings names the
second law golden (f957be0).
