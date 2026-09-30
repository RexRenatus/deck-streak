# Red-first record: SPEC-043

A1 to A3 were run red against an inert runner stub and replayed from a checkout of that commit.
A4 and A5 were committed with their subjects (the manifest, prompts and red-team cases) in one
commit, so they are disclosed as not red. A6 to A14 were run red against inert stubs of the new
modules (an always-passing gate, a runner that always fails, an empty composer, a repository that
records nothing) and green against the real ones.

```red-first
A1: red at c91df66aec45bc4713c237e526e773a4045b6c93: AssertionError: 'CLAUDE_CODE_OAUTH_TOKEN=synthetic-device-key-0f3a9c' not found in '' : the key reaches claude in its environment
A1: green at f0996b603e927dffe20fb71ef7b8e60312e156b5
A2: red at c91df66aec45bc4713c237e526e773a4045b6c93: AssertionError: 0 != 2
A2: green at f0996b603e927dffe20fb71ef7b8e60312e156b5
A3: red at c91df66aec45bc4713c237e526e773a4045b6c93: AssertionError: 0 != 4
A3: green at f0996b603e927dffe20fb71ef7b8e60312e156b5
A4: not red: the settings template and the scan's test were committed together with the manifest, and the scan finds no helper in either
A5: not red: the manifest, prompts and red-team cases were committed with the structure test, and the real probes run only on the box (SPEC-043 section 3a)
A6: red at fad6ac02873d1fb82f43af0becea0fc39c361fd4: cards-exfil-link.md: AiRouteAbsent
A6: green at 18250b29c8c075e29e025e3845c8cc4d9314edb4
A7: red at fad6ac02873d1fb82f43af0becea0fc39c361fd4: assertion `left == right` failed: left: Err(RunFailed)
A7: green at 18250b29c8c075e29e025e3845c8cc4d9314edb4
A8: red at fad6ac02873d1fb82f43af0becea0fc39c361fd4: assertion `left == right` failed: left: Err(RunFailed)
A8: green at 18250b29c8c075e29e025e3845c8cc4d9314edb4
A9: red at fad6ac02873d1fb82f43af0becea0fc39c361fd4: expected withheld, got AiRouteAbsent
A9: green at 18250b29c8c075e29e025e3845c8cc4d9314edb4
A10: red at fad6ac02873d1fb82f43af0becea0fc39c361fd4: assertion `left == right` failed: only the template's two fences open
A10: green at 18250b29c8c075e29e025e3845c8cc4d9314edb4
A11: red at fad6ac02873d1fb82f43af0becea0fc39c361fd4: the_prompt_is_composed_in_the_persona_order panicked at crates/agent/tests/compose.rs:24:46
A11: green at 18250b29c8c075e29e025e3845c8cc4d9314edb4
A12: red at fad6ac02873d1fb82f43af0becea0fc39c361fd4: assertion `left == right` failed: left: Err(RunFailed)
A12: green at 18250b29c8c075e29e025e3845c8cc4d9314edb4
A13: red at fad6ac02873d1fb82f43af0becea0fc39c361fd4: assertion `left == right` failed: left: 0
A13: green at 18250b29c8c075e29e025e3845c8cc4d9314edb4
A14: red at fad6ac02873d1fb82f43af0becea0fc39c361fd4: assertion `left == right` failed: left: []
A14: green at 18250b29c8c075e29e025e3845c8cc4d9314edb4
A15: not red: the runner already read only the credentials directory, so the tests were written against a green runner; each is disclosed with its plant, rows S04318 (an environment fallback), S04319 (another credential path) and S04320 (a token path in a committed file), each proved killed
```

## Round 5 addendum

Round 5 added `test_the_runner_accepts_only_an_exact_loopback_url_and_an_absolute_path` (a sibling of the A2 test) and the runner's
exact-URL and absolute-path checks. It was committed alone and run red against the previous
`agent/run-headless.sh`: five of its six refusal cases failed by assertion (the sixth, a remote
`http://` URL, was already refused), each with `AssertionError: 0 != 2`, and the four loopback forms
launched. The A1 to A3 lines above stand, and each id is recorded once, so the round-5 pair is
quoted here and not repeated in the block above.

```text
red at a6f20dc911efbcf5860541860c1cbaede5657c2e: AssertionError: 0 != 2 : (five refusal subtests)
green at d78d60cd8bd7076f09a493128349e3d7f96485ae: the agent tests pass, with the four loopback forms launching
```

## Amendment addendum, 2026-09-29 (issues #362 and #363)

A16, A17, A19 and A20 were committed alone, against inert stubs where a symbol was missing:
`ProbeGate::new` took the new arguments and returned a `Result` but refused nothing, and migration
004302 was absent. A18 pins an attribute the base already carried, so it could not be red first
and is recorded not red: the red commit a8bdd84 also removed `#[must_use]` from `Verdict` as a
plant, restored in the green commit, and that plant turns A18 red by its assertion (verdict.rs:34),
as row S04325 does. The lines above stand.

A20's body changed after its green commit: its source check now counts the prune as one whole
quoted literal in `crates/agent/src/runs.rs`, so a statement with an added predicate no longer
matches. Replayed at a8bdd84, the new body fails first by the assertion the fence line quotes from
the earlier body, verbatim: `the prune does not use the index: ["SCAN agent_runs"]` (runs.rs:51).

A18's body changed after a8bdd84 too: its scanner now reads only whole attribute lines, which moved
its assertion from verdict.rs:32 to verdict.rs:34. Replayed at a8bdd84, the new body fails by
`` `pub enum Verdict` lost its #[must_use]: ["#[derive(Clone, Debug, PartialEq, Eq)]"] `` (verdict.rs:34).

```red-first
A16: red at a8bdd84ed73f95269f30e14306a595c8d1ca1596: assertion `left == right` failed: left: None right: Some(NoOutputClass) (gate.rs:157)
A16: green at 745d5fded1d72932e6b5065756bc7d887f099a0d
A17: red at a8bdd84ed73f95269f30e14306a595c8d1ca1596: assertion `left == right` failed: left: None right: Some(NoInputClass) (gate.rs:181)
A17: green at 745d5fded1d72932e6b5065756bc7d887f099a0d
A18: not red: the base already carried #[must_use] on Verdict, so the pin was green at the base; removing the attribute (the plant of a8bdd84, and row S04325) turns it red by assertion (verdict.rs:34)
A19: red at a8bdd84ed73f95269f30e14306a595c8d1ca1596: assertion `left == right` failed: left: [] (runs.rs:30)
A19: green at 745d5fded1d72932e6b5065756bc7d887f099a0d
A20: red at a8bdd84ed73f95269f30e14306a595c8d1ca1596: the prune does not use the index (runs.rs:51)
A20: green at 745d5fded1d72932e6b5065756bc7d887f099a0d
```

## Second amendment addendum, 2026-09-29 (issue #404)

A21, A22 and A23 are new criteria, and each has a planted decoy in a test's own string fixture,
never in production source. The red commit 84146ad refactored the two pin tests around a helper
that reads a source text (the base's own check, unchanged) and added the decoys, so each decoy
failed by assertion while the base's real pins stayed green. The green commit ff7f6cc
edits two test files: it changes the bodies of those helpers (`prune_pin_problems`,
`own_statement_problems` and `attributes_of` gain the new rules, with `delete_statements_in` and
`is_one_whole_attribute` added) and edits no assertion of any test. The A18 and A20 lines above
stand: their words are unchanged and the new criteria carry the strengthening.

A21 has two decoys and this fence line quotes the first; the second, a statement with another
spelling, failed at the red commit as `the second statement is not refused: []` (runs.rs:134), the
line of the body as committed at 84146ad, which is also the body the A22 fence line's runs.rs:151
belongs to. The commit 2bce259c later changed that test's body to assert the good source as well.
A second measurement, with that later body put onto the red commit, moves the same failures to
runs.rs:138 and runs.rs:155; the lines above quote the body as committed.

```red-first
A21: red at 84146ad3fb5aa57a995f4cf7de9e517ec67e508c: the decoy is not refused: [] (runs.rs:121)
A21: green at ff7f6cce9ddf398dcb29d3ab4cf7e2881e661937
A22: red at 84146ad3fb5aa57a995f4cf7de9e517ec67e508c: the changed copy is not refused: [] (runs.rs:151)
A22: green at ff7f6cce9ddf398dcb29d3ab4cf7e2881e661937
A23: red at 84146ad3fb5aa57a995f4cf7de9e517ec67e508c: an item line ending in a comment was read as an attribute: ["#[derive(Clone)]", "#[rustfmt::skip] pub fn decoy() {} // ]", "#[must_use]"] (verdict.rs:56)
A23: green at ff7f6cce9ddf398dcb29d3ab4cf7e2881e661937
```

### Fix round 1

The scans still accepted spellings their criteria promised to refuse. Each new test was committed
alone, beside the helpers as they stood, and failed by assertion before the helpers changed. The
A21 to A23 fence lines above stand: a criterion has one red and one green line, so the new tests'
lines are quoted here.

- A23, `a_bracket_inside_a_string_or_a_comment_never_closes_an_attribute` and
  `a_commented_copy_of_the_enum_above_it_is_refused`: red at the commit that adds only the two tests
  and a `declarations_of` that asks only whether the enum is declared; green at the commit that
  changes the helpers (`is_one_whole_attribute` fails closed, `is_a_plain_doc_line` and the real
  `declarations_of` are added) and no assertion.
- A21, `a_prune_spelled_around_the_keyword_scan_beside_a_quoted_copy_is_refused`: red at the commit
  that adds only the test; green at the commit that adds the word count to `prune_pin_problems`.

Disclosure of the edits between red and green: the scans are helper functions inside the two test
files, so the green commits edit those files. Commit b2b13fc changes `is_one_whole_attribute` to fail
closed, adds `is_a_plain_doc_line`, makes `declarations_of` count, and adds one assertion to
`the_verdict_type_is_must_use` that the enum is declared once; it changes no assertion of the two new
tests. Commit 0ec2d29 adds `delete_keywords_in` and its use in `prune_pin_problems`, and changes no
assertion of the new test.

```text
A23: red at 25b1a12108b61d743630a57be0304db9e96938e2: a must_use outside the enum's attributes was read as one: ["#[derive(Clone)]", "#[doc = \"[\"] pub fn decoy() {} // ]", "#[must_use]"] (verdict.rs:110)
A23: red at 25b1a12108b61d743630a57be0304db9e96938e2: assertion `left == right` failed, left: 1, right: 2 (verdict.rs:125)
A23: green at b2b13fcce4d70a3e8fd98c46313f4b057d037c9a
A21: red at 4d9ddc5fbc7dcfd0d246923ac0b1456b5ec23101: the decoy is not refused: DELETE FROM main.agent_runs WHERE created_at + 0 < ?1: [] (runs.rs:215)
A21: green at 0ec2d29665d407e779bcf94df465cd10b57065b9
```

### Fix round 2

Two more classes were open. The word count read prose as a keyword, and the verdict pin read
neither a raw-identifier declaration nor a conditional attribute in every spelling. Each rule is one
sentence and each killer is generated from the rule's population. Each new test was committed alone
before the helpers changed, and failed by assertion. The A21 and A23 fence lines above stand.

- Class rule, prune pin: the word count reads the source with every prose comment removed, where
  prose is a comment, in any form, that holds no double quote, and the statement count still reads
  comments. `a_delete_word_in_any_comment_form_is_not_counted_but_the_statement_in_one_is` was
  generated from the eight forms of a Rust comment (`//`, `///`, `//!`, `/* */`, `/** */`, `/*! */`,
  a nested block, a block over several lines): a benign delete word in each is not refused, and the
  whole statement in each is refused as a second statement.
- Class rule, verdict pin: identifiers and attribute paths compare after the `r#` prefix is
  removed, and an attribute is conditional when the last segment of its path is `cfg` or
  `cfg_attr`. `every_spelling_of_the_declaration_and_of_a_conditional_attribute_is_read` was
  generated from two declaration spellings (`Verdict`, `r#Verdict`) and four attribute spellings
  (`cfg`, `r#cfg`, `cfg_attr`, `r#cfg_attr`), each with and without spaces inside the brackets: 16
  members. Each member was compiled once in a scratch crate and none was rejected.
- The three named tests, `prose_that_names_the_delete_beside_the_prune_is_not_counted`,
  `a_raw_identifier_declaration_is_the_verdict_enum` and
  `a_conditional_attribute_on_the_enum_is_refused`, were committed alone first as well. The
  conditional test's commit carries a stub `is_conditional` that answers false, so the file
  compiles and the test fails by assertion; the last verdict helper commit replaces the stub.

Disclosure of the edits between red and green: the scans are helper functions inside the two test
files, so the green commits edit those files. Commit 1fdacad0 adds `comment_spans`,
`raw_string_len`, `char_literal_len` and `code_of`, and makes `delete_keywords_in` read `code_of`.
Commit cb94c178 adds `without_raw_prefixes` and `declares_the_verdict` and uses it in
`declarations_of` and `attributes_of`. Commit 9fdf9391 replaces the stub `is_conditional`. Commits b943730, 89d84ed and 4d11e2e are the verdict tests; they were
committed between the prune red and green and edit verdict.rs, not runs.rs. No commit
changes an assertion of the tests above.

```text
A21: red at e34a4e49a57786221353ba20bd5baef6997499fa: prose was counted: /// The delete reads `created_at` through its index.: ["the source writes the word delete 2 times, not once"] (runs.rs:253)
A21: red at f2d220350de15bb5c41baa56bf999fd54a595506: a benign delete word in a comment was counted: // The delete reads created_at through its index.: ["the source writes the word delete 2 times, not once"] (runs.rs:298)
A21: green at 1fdacad0bdbae66c1e6af24e5dfb5ca502f179d8
A23: red at b943730aea15ad5588f469eda7ffee1289cb371b: assertion `left == right` failed, left: 0, right: 1 (verdict.rs:148)
A23: red at 89d84eddc0f7386339680fce6f1400d07d4c49f3: a conditional attribute was not seen: ["#[derive(Clone)]", "#[cfg(any())]", "#[must_use]"] (verdict.rs:182)
A23: red at 4d11e2e6c80c343534643f7b85407009b3c23ce1: a conditional attribute was not refused: #[must_use] #[cfg(any())] (verdict.rs:220)
A23: green at 9fdf9391f05e91e1ae1163ba1194c36b79dcf8fa
```

## Round 3

Two more classes were open. The prune pin's exception for quoted comments let a moved prune escape
beside a quoted copy, and an escaped character literal desynced its scan. The verdict pin let a
copy of the enum in a comment, or compiled out, lend its attributes beside a respelled live enum,
and refused a raw `#[r#must_use]`. Each rule is one sentence, and each killer is generated from
its rule's population.

- Class rule, prune pin: every count reads the code with every comment removed, whatever the
  comment holds; the run count takes only the literal handed to `sqlx::query!`; a character literal
  is measured to its closing quote, escaped ones included. Populations, each printed and asserted
  by its test: 21 copies of the statement that does not run beside 3 prunes that run elsewhere (63
  members), 13 character literals, and the 16 comment forms.
- Class rule, verdict pin: a declaration is the word `enum` followed, across whitespace and
  comments, by `Verdict` with the raw prefix removed, on every line, and `must_use` is compared
  after the raw prefix is removed. Population: 12 copies of the enum times 6 spellings of the live
  one, 72 members.
- The tests were committed alone first, against the head's helper bodies (the round's red tree is
  the tests with `char_literal_len` and `code_of` as they stood at 229cf0c1, and with
  `statements_run_in`, `declarations_in` and `is_must_use` extracted in 616bd997 from 229cf0c1's
  inline expressions with the same behaviour), and the helpers followed in the green commit. The
  green commit also added two population-count assertions (`assert_eq!(members, 63, ..)` and
  `assert_eq!(literals.len(), 13, ..)`, each true at both commits) and five `eprintln!` lines; no
  other assertion changed between the two.

```text
A21: red at 616bd997928bfc9cd990eb172feb788cf78f523e: a copy that does not run was read as the prune: // "DELETE FROM agent_runs WHERE created_at < ?1": [] (runs.rs:495)
A21: red at 616bd997928bfc9cd990eb172feb788cf78f523e: a character literal hid a second statement: '\'': left [] right ["the source writes the word delete 2 times, not once"] (runs.rs:534)
A21: red at 616bd997928bfc9cd990eb172feb788cf78f523e: prose was counted: // The "prune" is one delete, through the index on created_at.: left ["the source writes the word delete 2 times, not once"] right [] (runs.rs:377)
A21: red at 616bd997928bfc9cd990eb172feb788cf78f523e: a benign delete word in a comment was counted: // The "prune" is one delete, through the index on created_at.: left ["the source writes the word delete 2 times, not once"] right [] (runs.rs:430)
A21: green at 2ebc705c3b2679f421b300c9f5910668b2f604f9
A23: red at 616bd997928bfc9cd990eb172feb788cf78f523e: #[r#must_use] #[derive(Clone)] pub enum Verdict {: ["#[derive(Clone)]", "#[r#must_use]"] (verdict.rs:357)
A23: green at 2ebc705c3b2679f421b300c9f5910668b2f604f9
```

## Round 5

PR #414's fifth round (issue #404); the `Round 5 addendum` above belongs to SPEC-043's first
delivery. The round-4 review reopened the lexical class a third time: a raw literal holding the call
was read as the call, a string or block comment closing on the declaration's line lent the enum its
`#[must_use]`, and U+200E or U+200F between `enum` and `Verdict` hid the declaration. The class is
closed by one rule, not by a fourth rule in the hand lexer (ADR-293).

- Class rule, both pins: each source is read as the tokens `proc-macro2`'s lexer produces, and each
  string, byte-string or C-string literal as the value `syn` cooks. A comment is no token, a doc
  comment is a `doc` attribute whose text is prose, whitespace is whatever separates two tokens, a
  call or an attribute is read only where the token tree puts it, and a source that does not lex is
  refused.
- Populations, each generated from axis tables and asserted by count in its test. Prune pin: 84
  literal holders of the call, 168 members
  (`a_literal_of_any_kind_holding_the_call_is_not_the_call`); 32 comment holders of the statement,
  64 members (`a_comment_of_any_kind_holding_the_statement_is_not_read`); 130 spellings of the call
  (`every_spelling_of_the_call_rustc_reads_as_the_call_is_the_call`); five prunes spelled by an
  escape, a continuation, a raw string or `stringify!`, each beside four copies of the call that
  never run, 20 members
  (`a_prune_spelled_by_an_escape_a_continuation_or_tokens_beside_a_dead_copy_is_refused`). Verdict
  pin: 72 copies by spellings
  (`a_copy_of_the_enum_in_code_is_a_declaration_and_in_a_comment_or_a_literal_is_none`); 60 holders
  of `#[must_use]`, 120 members (`a_must_use_held_by_a_literal_or_a_comment_is_not_the_enums`); 60
  holders of a copy of the enum, 120 members
  (`a_copy_of_the_enum_held_by_a_literal_or_a_comment_is_no_declaration`); 113 gaps and spellings
  (`every_gap_rustc_reads_between_the_enums_tokens_is_whitespace`).
- The hand lexer is deleted. From `runs.rs`: `comment_spans`, `raw_string_len`, `char_literal_len`,
  `code_of`, `delete_statements_in` and `delete_keywords_in`. From `verdict.rs`:
  `is_one_whole_attribute`, `is_a_plain_doc_line`, `declarations_in`, `name_after`,
  `block_comment_len` and `attributes_of_the_verdict`. `statements_run_in`, `prune_pin_problems`,
  `own_statement_problems`, `declarations_of`, `attributes_of` and `is_must_use` are rewritten over
  the tokens.
- The tests were committed alone first, against the head's helpers, and every red is an assertion.
  The red commit changes assertions it inherits, each because rustc reads a comment as no code: a
  commented copy of the statement, asserted refused as a second statement in
  `prose_that_names_the_delete_beside_the_prune_is_not_counted` (1 member) and in the comment-forms
  test (16 members), is now asserted not refused, and that test is renamed
  `a_delete_word_or_the_statement_in_any_comment_form_is_not_counted`;
  `every_copy_of_the_enum_beside_every_spelling_of_it_is_a_second_declaration` is renamed
  `a_copy_of_the_enum_in_code_is_a_declaration_and_in_a_comment_or_a_literal_is_none`, its nine
  copies in code stay a second declaration and its three held copies (a block comment, line comments
  and a string, 18 of its 72 members) are now none;
  `a_commented_copy_of_the_enum_above_it_is_refused` asserted the decoy is two declarations and now
  asserts it is refused. The verdict fixtures gain a body (`{ A }`) so each lexes, and the four
  assertions of `the_verdict_type_is_must_use` are gathered, unchanged, into `verdict_pin_problems`.
- The green commit changes one assertion: `the_prune_reads_agent_runs_through_the_created_at_index`
  asserted the source writes the quoted statement once, and the run count now requires the cooked
  text of the literal handed to `sqlx::query!` to equal the statement, so the separate assertion is
  removed. No other assertion changed between the two.

```text
A21: red at 35fa6ddc1f050da28da8e0dcc2f3fc7893289e9e: a literal holding the call was not counted, or was read as the call: let _ = "sqlx::query!(\"DELETE FROM agent_runs WHERE created_at < ?1\", cutoff)";: left ["the source holds 2 delete statements, not one", "the source writes the word delete 2 times, not once"] right ["the source writes the word delete 2 times, not once"] (runs.rs:654)
A21: red at 35fa6ddc1f050da28da8e0dcc2f3fc7893289e9e: a commented copy of the statement was read as code: // DELETE FROM agent_runs WHERE created_at < ?1: left ["the source holds 2 delete statements, not one"] right [] (runs.rs:443)
A21: red at 35fa6ddc1f050da28da8e0dcc2f3fc7893289e9e: a comment was read as code: // sqlx::query!("DELETE FROM agent_runs WHERE created_at < ?1", cutoff): left ["the source holds 2 delete statements, not one"] right [] (runs.rs:699)
A21: red at 35fa6ddc1f050da28da8e0dcc2f3fc7893289e9e: a prune spelled around the word count was not refused: fn _unused(cutoff: i64) { let _ = sqlx::query!("DELETE FROM agent_runs WHERE created_at < ?1", cutoff); }: left [] right ["the source writes the word delete 2 times, not once"] (runs.rs:813)
A21: red at 35fa6ddc1f050da28da8e0dcc2f3fc7893289e9e: a spelling rustc reads as the call was refused: "let done = sqlx\t::query!(\"DELETE FROM agent_runs WHERE created_at < ?1\", cutoff);": left ["the code hands the statement to sqlx::query! 0 times, not once"] right [] (runs.rs:780)
A21: red at 35fa6ddc1f050da28da8e0dcc2f3fc7893289e9e: a source that does not lex was read: let done = sqlx::query!("DELETE FROM agent_runs WHERE created_at < ?1", cutoff;: [] (runs.rs:830)
A21: red at 35fa6ddc1f050da28da8e0dcc2f3fc7893289e9e: a commented copy of the statement was read as code: left ["the source holds 2 delete statements, not one"] right [] (runs.rs:388)
A21: green at 834dd9a52979e868b99eb0c9cd15d519f8e7591a
A23: red at 35fa6ddc1f050da28da8e0dcc2f3fc7893289e9e: a held #[must_use] was read as the enum's: const _HELD: &str = "\n#[must_use]\n#[derive(Clone)]\n"; #[derive(Clone, Debug)] pub enum Verdict { (verdict.rs:508)
A23: red at 35fa6ddc1f050da28da8e0dcc2f3fc7893289e9e: a copy in a comment or a literal was read as a declaration: /*: left 2 right 1 (verdict.rs:389)
A23: red at 35fa6ddc1f050da28da8e0dcc2f3fc7893289e9e: a held copy was read as a declaration: const _HELD: &str = "#[must_use] (verdict.rs:535)
A23: red at 35fa6ddc1f050da28da8e0dcc2f3fc7893289e9e: whitespace rustc skips was not skipped: "#\t[must_use]\n#[derive(Clone)]\npub enum Verdict {\n    A,\n}\n": left ["the enum lost its #[must_use]: [\"#[derive(Clone)]\"]"] right [] (verdict.rs:575)
A23: green at 834dd9a52979e868b99eb0c9cd15d519f8e7591a
```
