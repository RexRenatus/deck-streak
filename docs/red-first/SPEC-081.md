# Red-first record: SPEC-081

Recorded 2026-10-03, over the five parts of E2 (the criteria section 3 lists: A1 to A12, A15 and A21).
Each criterion's test was committed beside a stub or a missing port that compiles, so it failed by
assertion, and a later commit turned it green. Every red run was the one target alone, exit 101.

```red-first
A1: red at e685d641: chests_sessions.rs:63:9 assertion left == right failed: the sessions of {...} left: Array [] right: Array [Object {distinct_cards 1, ...}]
A1: green at 0b76ff9c
A2: red at e685d641: chests_roll.rs:37:9 assertion left == right failed: the roll of {...} left: "common" right: "legendary"
A2: green at 0b76ff9c
A3: red at e685d641: chests_roll.rs:62:9 the port does not know the rarity common
A3: green at 0b76ff9c
A4: red at 3afc3f98: assertion left == right failed: the grant of {...} (draws_used 8 vs 0, case 0)
A4: green at e18f1133
A5: red at 3afc3f98: assertion left == right failed: a second recompute of the same study day drew 8 time(s)
A5: green at e18f1133
A6: red at 3afc3f98: assertion left == right failed: the recompute after a failed draw at call 1
A6: green at e18f1133
A7: red at 3afc3f98: assertion left == right failed: the challenge chest of {...}
A7: green at e18f1133
A8: red at c30d979b: chests_open.rs:212:9 the first open of a Sealed Common chest pays its stored payout once (left "no such chest")
A8: green at 86dfb741
A9: red at c30d979b: chests_open.rs:277:5 a capped freeze becomes a token (left "not an epic")
A9: green at 86dfb741
A10: red at c30d979b: chests_open.rs:394:9 the sweep of {...} (left {"grants":[],"resolved":[]})
A10: green at 777f0c5d
A11: red at c30d979b: tokens_window.rs:132:9 the activation of {...} (left "no token held", golden activated id 2)
A11: green at 4a4b7840
A12: red at c30d979b: tokens_window.rs:215:9 the token bonus of {...} (left consumed [], golden [1])
A12: green at 4a4b7840
A15: red at e685d641: chests_roll.rs:194:5 assertion failed: same(&token["window_hours"], &json!(tokens::TOKEN_WINDOW_HOURS))
A15: green at 0b76ff9c
A21: red at 86480d3f: assertion left == right failed: the export holds no `chests` table (left None, right Some(101)); the declaration's reset rows read {} against pity and settings values
A21: green at eb631ab2
```

A21's red stub declared the four tables with empty reset rows and an export and erase that did
nothing; the real port (eb631ab2) turned both of its tests green. The test file was reformatted by
`cargo fmt` between the red measurement and its commit, so the line numbers are not quoted.

Not red-first lines, by design:

- the held-key test (`a_held_quest_chest_key_takes_no_draw_and_writes_nothing`) and the draw-fraction
  test are mutation coverage, green at their commits;
- `formal_vectors_chest.rs` and `formal_vectors_token_bonus.rs` cross-check the Lean vectors and are
  mutation coverage;
- the second A21 test, `the_quests_port_declares_its_tables_and_their_reset_rows`, is mutation
  coverage for the declaration's rows.

A6's red comes from its positive control (chests [] against 3): its absence assertions pass on a
no-write stub, and are not counted as red.
