# Red-first record: SPEC-072

The order of work: the SPEC promoted and ADR-072 accepted (6b6b216); the goldens (40b6981); the
census of `xp_settlement` writers and `settle` callers (3672453); ingest's tier tests beside an
inert `parse_tier` (805ac4d) and their implementation (2965fe5); progression's tests beside inert
stubs (a25cc88) and their implementation (de00943); the fold's tests beside inert steps (1703ca1)
and the steps (4b63c0c); the level routes' test beside routes that answered 404 (6d36fc5) and the
routes (3413ecc); the bot's `/level` test beside a menu that answered with the unknown-command help
(9703b06) and the command (8a70fb2); the level screen's tests beside a stub screen (b324a6f) and the
screen (48df2cb); the census fix that reads the settled rows through the crate root (22f2390); the
whole-value pins of the derived registry, the table names and the step names (23d6f7e); and the
SPEC's mutation rows, each proved KILLED by its full id (dfb73e0, then 5ea3f7b).

The goldens were generated from the predecessor's own functions at `27ee2bc`, with a scratch
`--registry` and a scratch `--out`, under `PYTHONDONTWRITEBYTECODE=1`; the predecessor's checkout
was left as it was: no change and no bytecode file added.

Each criterion was run at its red commit, selecting its own test, and failed by assertion, not by a
compile error, a missing fixture or an empty selection. The stubs compiled and returned nothing:
zero XP, no rows, an empty title, a parse that found no tier, steps that settled no day, and a
screen that rendered no list and no region. The web red was run in the worktree; every other red
was replayed from a scratch `git worktree add` tree.

A12's census failed at a25cc88 with the fold's step not yet calling `settle`; its green is 22f2390,
after the crate-root re-export left the census needle to the recompute steps alone.

A12 gained an arm in fix round 1: the census now also reads `SettleRequest`, so a grouped import
of the operation and its request outside the recompute steps is found. Its red is the changed test
beside a planted grouped import in `level_up.rs` (5dfd74a); its green removes the plant (168c835).
The record carries one pair for the criterion, this one; the base census's own red (a25cc88) and
green (22f2390) are the ones named in the paragraph above.

A30 (R14) and A31 (R24) were added in fix round 1. A30's test covers code already in the head, so
its red is the test beside a planted swap of the level before and after in `sync_cycle.rs`
(1f4b006) and its green removes the plant (578a351). A31's red is the composed router answering 503
`law_tiers_unavailable` (8467b95) and its green wires the law tiers' source (6d82561).

The pins added after the implementation (`the_derived_registry_and_the_tables_are_pinned_whole`,
`the_step_names_and_the_level_up_kind_are_pinned_whole`, `a_day_settles_the_bonus_sources`) pin
values the implementation already held, so they are not red: their evidence is the mutation row
each one kills (S07211 to S07214, S07218 to S07221).

```red-first
A1: red at a25cc88: assertion `left == right` failed: the XP of {"ease":1,"ivl":0,"rtype":0,"tier":null}; left: 0, right: 4
A1: green at de00943
A2: red at a25cc88: the constant constants.XP_BASE: 0.0 against 10
A2: green at de00943
A3: red at 805ac4d: assertion `left == right` failed: the tier of " T1 Subject::Topic "; left: None, right: Some("T1")
A3: green at 2965fe5
A4: red at 805ac4d: assertion `left == right` failed; left: [(1, None), (2, None), (3, None), (4, None)], right: [(1, Some("T3")), (2, None), (3, None), (4, Some("T4"))]
A4: green at 2965fe5
A5: red at 1703ca1: assertion `left == right` failed: the language source holds the language card's reviews and no law review; left: None, right: Some(34)
A5: green at 4b63c0c
A6: red at 1703ca1: assertion `left == right` failed; left: None, right: Some(24)
A6: green at 4b63c0c
A7: red at a25cc88: assertion `left == right` failed: a recompute over less leaves the closed day's amount; left: 60, right: 100
A7: green at de00943
A8: red at a25cc88: assertion `left == right` failed; left: [], right: [("reviews", 40)]
A8: green at de00943
A9: red at a25cc88: assertion `left == right` failed; left: [], right: [("reviews", 90)]
A9: green at de00943
A10: red at a25cc88: assertion `left == right` failed: one row per day, source and track, however often it settles; left: 0, right: 3
A10: green at de00943
A11: red at a25cc88: a grant's source is not a derived one: 10
A11: green at de00943
A12: red at 5dfd74a: assertion `left == right` failed; left: ["crates/coordination/src/level_up.rs calls settle outside the recompute steps, and only the owner's correction may"], right: []
A12: green at 168c835
A13: red at a25cc88: assertion `left == right` failed: both tables; left: 60, right: 100
A13: green at de00943
A14: red at a25cc88: assertion `left == right` failed: the bonuses of {"backlog_zero":true,"graduations":3,"score_total":95,"streak_days":7,"studied":true}; left: []
A14: green at de00943
A15: red at 1703ca1: assertion `left == right` failed: the backfilled day has no snapshot, the closing day and the current day have; left: [(20000, None), (20001, None), (20002, None)]
A15: green at 4b63c0c
A16: red at a25cc88: assertion `left == right` failed; left: "", right: "Sprout"
A16: green at de00943
A17: red at 1703ca1: the recompute's XP crosses a level
A17: green at 4b63c0c
A18: red at 1703ca1: the days earned XP
A18: green at 4b63c0c
A19: red at a25cc88: assertion `left == right` failed: the run of {"day_results":[[10,false],[10,false],[70,false]]}; left: 0, right: 1
A19: green at de00943
A20: red at a25cc88: assertion `left == right` failed: the base of the two-table day; left: 0, right: 149
A20: green at de00943
A21: red at a25cc88: assertion `left == right` failed; left: 0, right: 550
A21: green at de00943
A22: red at a25cc88: assertion `left == right` failed: a reading's grant is not review XP; left: 0, right: 300
A22: green at de00943
A23: red at a25cc88: assertion `left == right` failed: the consistency bonus of {"base":1000,"buff":false,"reviews":800,"reviews_law":0,"run":1}; left: 0, right: 149
A23: green at de00943
A24: red at a25cc88: assertion `left == right` failed: the arming of {"backlog_zero":100,"buff":false,"rollup_reviews":40,"skip":false}; left: false, right: true
A24: green at de00943
A25: red at a25cc88: the same day, not a skip, is armed
A25: green at de00943
A26: red at 6d36fc5: assertion `left == right` failed: /api/level None; left: 404, right: 401
A26: green at 3413ecc
A27: red at 9703b06: Level 1: Sprout in These are the commands I answer (the unknown-command help)
A27: green at 8a70fb2
A28: red at b324a6f: TestingLibraryElementError: Unable to find an accessible element with the role "list" and name "Today's XP by source"
A28: green at 48df2cb
A29: red at b324a6f: TestingLibraryElementError: Unable to find an accessible element with the role "region" and name "Consistency run"
A29: green at 48df2cb
A30: red at 1f4b006: assertion `left == right` failed: one line, for the level reached; left: [], right: ["🐣 Level 3: Sprout"]
A30: green at 578a351
A31: red at 8467b95: {"reason":"law_tiers_unavailable"}; left: 503; right: 200
A31: green at 6d82561
A32: red at eb6119fa: assertion `left == right` failed; left: [], right: ["crates/coordination/src/shortcut.rs calls settle outside the recompute steps, and only the owner's correction may", "crates/quests/src/chained_user.rs calls settle through tally_again, progression's alias of settle, and only coordination's code may", ...]
A32: green at d2697435
A33: red at 99fbf46d: assertion `left == right` failed; left: [], right: ["crates/markets/src/via_prog.rs calls settle through tally, progression's alias of settle, and only coordination's code may"]
A33: green at 7a6a659d
A34: red at 05cfedf2: assertion `left == right` failed: trees the census judges wrongly: 829 (members escaping: 815; controls judged wrongly: 14)
A34: green at 078fc173
```

Addendum (2026-09-29, issue 397): A32 was added by the census amendment (ADR-197). Its test was
committed alone beside the unchanged census (eb6119fa), where the planted renamed re-exports and
their callers produced no refusal, and it failed by assertion, over the whole test file with only
its own test failing. The green commit (d2697435) edits a test file, `xp_census.rs`, because the
census is that file's own code: it reads progression's re-exports there. A12 stays green through
both commits, with the same examined counts on the real tree (157 crate source files, 15
migrations, 8 planted crate source files).

Addendum (2026-09-29, round 1 of the review of issue 397). Two fixes, each with its tests committed
alone first, at a head where each failed by assertion, then its code.

The grouped module renaming and the chain read before its link (A32's second test). Red at
3742e717: `panicked at crates/progression/tests/xp_census.rs:576:5:` with
"assertion `left == right` failed", the left list holding the `crate_link_user.rs` and
`early_user.rs` refusals and lacking `crates/quests/src/ledger_user.rs calls settle through
ledger, progression's alias of settle, and only coordination's code may`; 2 passed, 1 failed.
Green at 311dddede: 3 passed. Two mutants of the census survived the census before this test and
are killed by it: the alias loop run as a single pass (`M1-fixpoint-single-pass: SURVIVED (2
passed)`) and a `)` that never counts as public (`M14-restricted-pub-not-public: SURVIVED (2
passed)`); after it, each is KILLED by
`the_census_follows_a_grouped_module_renaming_and_a_chain_read_before_its_link`.

The crate alias, the type alias, the `pub(` close and the word match (A33). Red at 99fbf46d: 3
passed, 4 failed, each by assertion, and each with its own line:

- `the_census_follows_a_crate_alias`, at `xp_census.rs:624:5`: `left: []`, `right:
  ["crates/markets/src/via_prog.rs calls settle through tally, progression's alias of settle, and
  only coordination's code may"]`.
- `the_census_follows_a_type_alias`, at `xp_census.rs:650:5`: `left: []`, `right:
  ["crates/quests/src/typed.rs calls settle through Wrapped, progression's alias of
  SettleRequest, and only coordination's code may"]`.
- `a_private_alias_behind_an_attribute_is_not_a_reexport`, at `xp_census.rs:686:5`: left holds
  `crates/quests/src/homonym.rs calls settle through gated` and the `link.rs` refusal; right holds
  the `link.rs` refusal alone.
- `the_operation_is_matched_as_a_word_not_a_prefix`, at `xp_census.rs:717:5`: left holds
  `crates/quests/src/day.rs calls settle, and only coordination's code may` besides the two owed
  refusals; right holds the two.

Green at 7a6a659d: 7 passed. Both green commits, 311dddede and 7a6a659d, edit a test file,
`xp_census.rs`, because the census is that file's own code; no assertion changes between either
red and its green. A12 keeps its examined counts on the real tree through every commit
(159 crate source files, 15 migrations, 8 planted crate source files), the same as at the base of
the round. The three rows S07230 to S07232 were proved KILLED by their full ids on a committed
tree: `rows: examined 3: killed 3, survived 0, void 0`.

Addendum (2026-09-29, round 2 of the review of issue 397). The class of names bound to
progression's crate was reopened by six spellings, so the fix is one rule and one generated test.

The population test, `the_census_refuses_every_member_of_the_binding_population`, was committed
alone (a7049a43) beside the unchanged census. Red: `panicked at
crates/progression/tests/xp_census.rs:953:5:` by assertion, `class members: examined 30`, and all 30
members escaped the census. The fix (f94ee7eb) makes it green: 11 passed, the same 30 members
refused and the 30 controls (the 10 binding forms naming a crate that is not progression, in the
same spelling, each with the three caller shapes) accepted.
Its green commit edits a test file, `xp_census.rs`, because the census is that file's own code. Its
two helper bindings were renamed (`fill` to `expand`, `file` to `caller`) for clippy's
`similar_names`; no assertion changed.

The three tests of the class rule, applied alone to the census of 724b8d74 (the round-2 head, the
census a7049a43 leaves unchanged), are red: 7 passed, 3 failed, each by assertion (at
`xp_census.rs:811`, `:893` and `:963` in the order named),
`the_census_reads_a_raw_identifier_as_its_plain_name`,
`the_census_follows_a_crate_alias_however_it_is_written` and
`the_census_follows_a_crate_renamed_by_a_manifest`; green at f94ee7eb.

Rows S07233-CENSUS-MANIFEST-RENAME, S07234-CENSUS-GLOB-OPENS-MEMBER and
S07235-CENSUS-RAW-IDENTIFIER, each `KILLED: its killer passed without the mutant and failed with it`
on a committed tree. A12 on the real tree reads 160 crate source files, 15 migrations, 8 planted
crate source files; the file count is one higher than round 1 because the merged base added a
source file.

Addendum (2026-09-30, round 3 of the review of issue 397). The class of callers was reopened across
member crates and past comments, so the population test is regenerated from tables and the census
takes one rule.

The generated population test, and the manifest test's unreadable manifest, were committed alone
(dddd87e2) beside the unchanged census. Red, each by assertion:
`panicked at crates/progression/tests/xp_census.rs:1697:5:`, "members that escape the census: 864
of 12307", the first escaping member `comment in a manifest: F07 manifest table /
crates/@M/Cargo.toml / after a header, line 0`, the test printing `class members: examined 12307`
and `class members escaping: 864; class controls refused: 9153`; and
`panicked at crates/progression/tests/xp_census.rs:1906:5:` in
`the_census_follows_a_crate_renamed_by_a_manifest`, whose refused list lacked the unreadable
manifest; 9 passed, 2 failed. The rule (24a77705) makes both green: 11 passed,
`class members escaping: 0; class controls refused: 0`, the 12307 members refused, the 12338
controls accepted and every manifest read to its end. Its green commit edits a test file,
`xp_census.rs`, because the census is that file's own code; no assertion changed between the red
and the green.

A stratified sample of 2571 cases of the population, every axis value with members and controls,
was compiled under the pinned toolchain (cargo 1.97.0 `metadata`, rustc 1.97.0 `--emit=metadata`,
edition 2024): rc 0 for each. Each member was compiled beside progression as the one crate that
defines `settle`, and each control beside another crate in its place, but for a private glob's
control, which is compiled beside progression and calls its holder's own function.

A12 on the real tree reads 160 crate source files, 15 migrations, 8 planted crate source files, and
refuses none, as before the round.

Addendum (2026-09-30, round 6 of the review of issue 397). Round 5's review generated a population
from Cargo's documentation, TOML 1.0 and the Rust Reference and found the textual census open again,
so the compiler becomes the census (ADR-197, its decision of round 6; section 12).

The killer, `the_census_refuses_every_caller_the_compiler_finds`, was committed alone (05cfedf2)
beside the unchanged census of round 3. Red, by assertion: `panicked at
crates/progression/tests/xp_census.rs:4211:5:` with "assertion `left == right` failed: trees the
census judges wrongly: 829", the left side naming the first tree judged wrongly that a worker
reported (an escaping member of the reading-scope axis in each run measured), the test printing
`killer examined 2218 tree(s)` with `members escaping: 815; controls judged wrongly: 14`; 11 passed,
1 failed. The census of round 6 (078fc173) makes it green: `killer examined 2218 tree(s)` with
`members escaping: 0; controls judged wrongly: 0`, each of its 17 axes printing its examined members
and controls. The green commit edits a test file, `xp_census.rs`, because the census is that file's
own code, and it adds the probe to progression's production code: the build script
`crates/progression/build.rs` and the `cfg_attr` on `settle`, neither of which changes a build
without the census's variable. No assertion of the killer changed between the red and the green.

The population's validity was confirmed under the pinned toolchain (cargo and rustc 1.97.0, edition
2024) apart from the census, in four builds of each case (debug assertions on and off, each with the
unwind and the abort panic strategy): round 5's cases were labelled by its own compile of every
case, and a stratified sample of 142 cases, one member and one control of each of round 5's axes and
every case of the axes this round adds, gave rc 0 in all four builds for 141. The one other,
`concat_idents!`, gave rc 101 in each, since the macro is unstable on a stable toolchain, and it is
not in the population: no caller can write it on this toolchain.

Round 4's population (12307 members and 12338 controls, round 3's 144 and 144 among them) was judged
by this census once, outside CI: every member refused by its own caller's file and every control
accepted.

A12 on the real tree compiles every target of every workspace package in the four passes, and its
libraries and binaries alone in four more, since members have dev-dependencies, and refuses none.

Addendum (2026-09-30, round 7 of the review of issue 397). Round 6's review found that a build
script's cfg, read by code in a package that cannot name `settle`, can hide a call the census's
four passes never compile. ADR-197's decision of round 7 refuses the shape by the resolve graph:
every package with a build script that is, or depends on, progression is refused, and progression's
own build script is admitted at one pinned digest.

The tests were committed alone (8c883728) beside the unchanged census of round 6, whose file is
`crates/progression/tests/xp_census.rs`. Six tests are new; on the unchanged census four are red by
assertion and two are green, as they are meant to be:

- `a_build_script_in_a_package_that_depends_on_settle_is_refused_by_name` is red: the package with
  the build script is accepted (`refused by name: []`).
- `a_one_byte_edit_of_progressions_build_script_is_refused_on_the_pin` is red (`refused on the pin:
  []`).
- `a_corrupt_lock_file_is_refused_by_the_fail_closed_arm` is red: the census refuses, but with
  cargo's own words and not the census's by-name refusal that the test reads.
- `the_census_refuses_every_build_script_that_can_name_settle` is red: "every build-script member is
  refused, naming its package", `left: 0`, `right: 104`, the test printing `examined 0 build-script
  member(s) of 104 generated`.
- `a_build_script_in_a_package_that_cannot_name_settle_is_accepted` is green, as a control: nothing
  refuses a build script that cannot reach `settle`.
- `the_git_dependency_build_scripts_are_measured_and_the_kind_is_disclosed` is green, since it only
  prints: it reports `examined 26 git-dependency build-script member(s), 15 refused`, the same 15
  by their caller's file as the census of round 6 refuses, and it asserts nothing about the rest,
  which are the disclosed kind (SPEC-072 section 12).

The graph refusal (dfd9ca1e) makes the four green: 104 of 104 build-script members are refused,
naming their package, and 104 of 104 controls are accepted. The green commit edits the census, which
is that file's own code, and adds `sha2` to progression's dev-dependencies for the pin. Three more
tests came with the green commit and were never red: the edge kinds, a build script reached through
a build dependency, through a dev dependency, and in a package that depends on progression by name.
Their red is that of the tests above (a build script beside a dependency on progression is accepted
by the earlier census), and each is pinned by a row (S07276 to S07285).

The killer's counts do not change with the speed-ups: `killer examined 2218 tree(s)` with `members
escaping: 0; controls judged wrongly: 0`, each of its 17 axes printing the same examined members and
controls before and after the change that keeps one target for each worker. A12 on the real tree
reads 170 crate source files, 16 migrations and 13 planted crate source files, and refuses none.

A35, round 7: the record in the form the probes read.

```red-first
A35: red at 8c883728: assertion `left == right` failed: every build-script member is refused, naming its package; left: 0, right: 104
A35: green at dfd9ca1e
```

A36, round 8: the census's verdict depends only on the tree it judges, and its owner is found by the
path of its manifest. 09c160bb commits the round-8 tests alone, over the census of the round before;
14 of them are red by assertion there (`test result: FAILED. 21 passed; 14 failed`), and 8632611d
makes them green (`35 passed; 0 failed; 1 ignored`). Each red line, by test:

- the owner's lookup over nine synthetic graphs (no member at the owner's manifest, two, a renamed
  owner, a path, git or registry package carrying its name, no build script, two): every graph is
  accepted, `left: [... "the owner renamed: Ok([])", ...]`, `right: []`;
- the owner without a build script, the owner's manifest under another package name, and each of
  three graphs with a git package carrying the owner's name (beside an edited pin, a member's script,
  a disarmed owner): each `[]` where a refusal by name is asserted;
- a cargo configuration above the tree, and one in cargo's home: `[]` where a refusal is asserted;
- a member's code reading a variable the host sets: `[]` where a refusal is asserted;
- a variable the tree does not set, on a tree that reads it: `left: []`, `right: ["crates/m/src/lib.rs
  calls settle, and only coordination's code may"]`;
- a target copied from another tree's census: the same `left: []` against the same `right`;
- the order pair on one target: `left: [[caller], []]`, `right: [[], [caller]]`, so the verdict of
  the second tree follows the first;
- the ten trees of main's round-7 generation: several are accepted (`[]`) where a refusal by name is asserted;
- the population of 46 cases: `cases judged wrongly: 7; disagreements: 2`, each disagreement a
  verdict on a warmed target that differs from the verdict on a fresh one.

Two tests are green by design at the red commit: the one that names each use in its package and file
(the restore of a test the census's earlier change had weakened, red under the plant of S07284), and
the one that judges a reused git URL as a fresh URL, which the lock file already pins.

```red-first
A36: red at 09c160bb: the_owner_is_the_member_at_its_manifest_and_every_other_lookup_is_unique_or_refused: assertion `left == right` failed; right: []
A36: red at 09c160bb: a_target_copied_from_another_trees_census_does_not_move_the_verdict: assertion `left == right` failed: tree two on a copy of tree one's target, and alone
A36: red at 09c160bb: the_order_pair_p3_is_judged_alike_in_both_orders_on_one_target: assertion `left == right` failed: the second tree of each order on a target the first tree used
A36: red at 09c160bb: verify_round_seven_population_is_judged_as_each_case_expects_on_any_target: cases judged wrongly: 7; disagreements: 2
A36: green at 8632611d: examined 46 case(s) of verify round 7's population, digest b0fbee86a359b5cc5b3a963a38cd1e1afb629c2d7b18ab00c5242d103c4d28f0 / differential: seed 0x4250008, 10 chain(s), 40 warmed verdict(s) compared with the fresh one; disagreements: 0 / killer examined 2218 tree(s); members escaping: 0; controls judged wrongly: 0
```
