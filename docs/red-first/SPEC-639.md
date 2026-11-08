# Red-first record: SPEC-639

SPEC-639 (A1 to A14). The tree check came first, `scripts/tests/test_web_verify.py` alone at
ec249674, over the base tree, which held no phone project, no state table, no theme audit, no
exceptions table, no baseline and no screenshot expectation. Its seven real cases read red by their
own population assertions, each `examined 0`, in a local run of `python3 -m unittest test_web_verify
-v` in `scripts/tests` (16 tests, 7 failures, the 9 planted cases green). The gates followed at
4453be7a, without baselines, with the planted pair at full contrast, the planted target at full size
and an empty launch theme, so the pull request run's `web` job read the snapshot, planted-page and
theme criteria red by name. 4453be7a holds the tree of 1e1cd20f byte for byte (`git diff` between
them prints nothing), and that run ran at 1e1cd20f.

```text
A1: python3 -m unittest discover -s scripts/tests -p test_web_verify.py -k test_the_config_holds_the_kept_project_and_the_six_phone_projects
A1: python3 -m unittest discover -s scripts/tests -p test_web_verify.py -k test_a_planted_missing_phone_project_is_refused
A2: python3 -m unittest discover -s scripts/tests -p test_web_verify.py -k test_every_route_has_its_learner_states
A2: python3 -m unittest discover -s scripts/tests -p test_web_verify.py -k test_a_planted_route_with_no_state_is_refused
A2: python3 -m unittest discover -s scripts/tests -p test_web_verify.py -k test_every_screen_that_shows_its_clock_holds_it
A2: python3 -m unittest discover -s scripts/tests -p test_web_verify.py -k test_a_planted_clock_the_states_leave_running_is_refused
A2: python3 -m unittest discover -s scripts/tests -p test_web_verify.py -k test_a_planted_clock_each_state_holds_passes
A3: python3 -m unittest discover -s scripts/tests -p test_web_verify.py -k test_a_planted_real_looking_fixture_value_is_refused
A4: pnpm --dir web/app exec playwright test tests/verify/a11y-themes.spec.ts
A5: python3 -m unittest discover -s scripts/tests -p test_web_verify.py -k test_a_planted_audit_without_target_size_is_refused
A5: python3 -m unittest discover -s scripts/tests -p test_web_verify.py -k test_a_planted_audit_missing_a_wcag_tag_is_refused
A6: pnpm --dir web/app exec playwright test tests/verify/planted.spec.ts -g "a planted low-contrast pair is refused by axe at a phone size"
A6: pnpm --dir web/app exec playwright test tests/verify/planted.spec.ts -g "a planted small target is refused by axe at a phone size"
A7: pnpm --dir web/app exec playwright test tests/verify/planted.spec.ts -g "a planted host palette is refused as not the app's own theme"
A8: pnpm --dir web/app exec playwright test tests/verify/snapshots.spec.ts
A9: pnpm --dir web/app exec playwright test tests/verify/planted.spec.ts -g "a planted colour change is refused by the snapshot gate"
A9: pnpm --dir web/app exec playwright test tests/verify/planted.spec.ts -g "the clean planted page passes axe and matches its baseline"
A10: python3 -m unittest discover -s scripts/tests -p test_web_verify.py -k test_a_planted_missing_baseline_is_refused
A11: python3 -m unittest discover -s scripts/tests -p test_web_verify.py -k test_a_planted_bare_disable_is_refused
A12: pnpm --dir web/app exec vitest run src/lib/a11y-coverage.test.ts
A13: python3 -m unittest discover -s scripts/tests -p test_web_verify.py -k test_the_screenshot_expectation_allows_no_margin
A13: python3 -m unittest discover -s scripts/tests -p test_web_verify.py -k test_a_planted_screenshot_margin_is_refused
A14: python3 -m unittest discover -s scripts/tests -p test_public_scrub.py -k test_a_clean_screenshot_baseline_passes_the_tree_and_history_scans
A14: python3 -m unittest discover -s scripts/tests -p test_public_scrub.py -k test_a_baseline_holding_a_text_chunk_is_refused_by_its_type
A14: python3 -m unittest discover -s scripts/tests -p test_public_scrub.py -k test_a_malformed_baseline_is_refused
A14: python3 -m unittest discover -s scripts/tests -p test_public_scrub.py -k test_a_non_png_file_under_the_baselines_is_refused_as_binary
A14: python3 -m unittest discover -s scripts/tests -p test_public_scrub.py -k test_a_png_one_folder_outside_the_baselines_is_refused_as_binary
A14: python3 -m unittest discover -s scripts/tests -p test_public_scrub.py -k test_a_png_named_in_upper_case_is_refused_as_binary
A14: python3 -m unittest discover -s scripts/tests -p test_public_scrub.py -k test_a_private_literal_inside_a_baseline_is_still_found
```

```red-first
A1: red at ec249674: scripts/tests/test_web_verify.py:387: AssertionError: examined 0 phone project calls in the config: the population is empty, so nothing was judged
A1: green at f592e928
A2: red at ec249674: scripts/tests/test_web_verify.py:392: AssertionError: examined 0 routes in the state table: the population is empty, so nothing was judged
A2: green at f592e928
A3: red at ec249674: scripts/tests/test_web_verify.py:397: AssertionError: examined 0 literal values in the state table and fixtures: the population is empty, so nothing was judged
A3: green at f592e928
A4: red at 4453be7a: the red of A6 is this criterion's red: the theme audit's rules did not refuse the planted pair and the planted target (job 128590479548)
A4: green at f592e928
A5: red at ec249674: scripts/tests/test_web_verify.py:402: AssertionError: examined 0 tags named in the audit: the population is empty, so nothing was judged
A5: green at f592e928
A6: red at 4453be7a: web/app/tests/verify/planted.spec.ts:61 and :72: expect(received).toContain(expected), each test in all six phone projects (job 128590479548)
A6: green at f592e928
A7: red at 4453be7a: web/app/tests/verify/planted.spec.ts:79: expect(received).not.toEqual(expected), in all six phone projects (job 128590479548)
A7: green at f592e928
A8: red at 4453be7a: web/app/tests/verify/snapshots.spec.ts:16: A snapshot doesn't exist at ...-linux.png, for each of 71 states in each of the four 390-wide projects (job 128590479548)
A8: green at f592e928
A9: red at 4453be7a: web/app/tests/verify/planted.spec.ts:85: A snapshot doesn't exist at .../planted-linux.png, in phone-light (job 128590479548)
A9: green at f592e928
A10: red at ec249674: scripts/tests/test_web_verify.py:406: AssertionError: examined 0 baseline images: the population is empty, so nothing was judged
A10: green at f592e928
A11: red at ec249674: scripts/tests/test_web_verify.py:410: AssertionError: examined 0 verify modules: the population is empty, so nothing was judged
A11: green at f592e928
A12: not red: the kept coverage check, web/app/src/lib/a11y-coverage.test.ts, is unchanged by this delivery and passes before and after it
A13: red at ec249674: scripts/tests/test_web_verify.py:414: AssertionError: examined 0 screenshot expectations in the config: the population is empty, so nothing was judged
A13: green at f592e928
A14: red at d0155ed0957f3c764c472b84f20f7bbba63425bb: scripts/tests/test_public_scrub.py:938: AssertionError: 1 != 0 : public-scrub: web/app/tests/verify/__screenshots__/phone-light/verify/snapshots.spec.ts/today-first-linux.png: binary
A14: green at 24aaafd37bb17fd728e50a9bcdc93e4030b9afaf
```

The baselines came from one render run. At 1df6f097 the `web` job of the pull request run (job
128617176047) ran in the render mode and wrote every baseline it rendered under its stage logs, by
its committed name, failing each test that wrote one: 285 writes, 71 in each of the four 390-wide
projects for the snapshots and the planted image in phone-light. The clean planted page read that
image back and passed there; that pass is not its green line. b376bf4c commits those 285 images
alone, d3bab86f takes the render line out of the workflow again, and f592e928 changes the four
planted values alone (the pair's ink to `#b0b0b0`, the target to 16 by 16 CSS pixels with a gap of
2, and every host palette entry applied), so each refusal test's final assertion is the one the
first run read red. Each green line names f592e928: the run at this delivery's head ran its web app
and Python tests byte for byte, because the commits after it change only documents, the mutation
rows and the changelog fragment.

A2 changed: a screen that shows its own clock now holds it in each state, and a planted state that
leaves it running is refused. Its new real case, `test_every_screen_that_shows_its_clock_holds_it`,
came alone with `scripts/tests/test_web_verify.py` at 8070077e183d2578afbbed32ca8b5308a3b14e52 and read red there in a
local run of `python3 -m unittest test_web_verify -v` in `scripts/tests` (19 tests, 1 failure), at
`scripts/tests/test_web_verify.py:450`, with exactly three findings, one for each of the `/remote`
states `first`, `empty` and `error`:
`AssertionError: Lists differ: ["clock: /remote first leaves the page's o[133 chars]ing"] != []`.
The two planted clock cases passed in that run. 4f00404e0bafdbc38d9ad72661470f442acb18cc holds the clock in the
state table and the fixture, and the case reads green from it on.

A14 is new. Its four red-first cases came alone with `scripts/tests/test_public_scrub.py` at
d0155ed0957f3c764c472b84f20f7bbba63425bb and read red there in a local run of `python3 -m unittest test_public_scrub -v`
in `scripts/tests` (47 tests, 10 failures, every one in those four cases). Its three cases that
refuse a non-PNG file under the baselines, a PNG one folder outside them and a `.PNG` name as binary
passed there, because the scrub refused every binary before. 7211d7761d166b2df907fb9b9cc4938e0b6ddee4 admits
the clean baselines, and A14's first CI green is the pull request run 42463446614 at
ed16cfdbf192581a2980cc3a6bd5cee6b89463d7, whose hygiene job (job 128677485838) read `test_public_scrub` green.
24aaafd37bb17fd728e50a9bcdc93e4030b9afaf then drops a loop bound that no outcome depended on, so the green line
names it:
`git -C /tmp/ds-worktrees/web-verify-639 diff --quiet 24aaafd37bb17fd728e50a9bcdc93e4030b9afaf HEAD -- scripts/public-scrub.py scripts/tests/test_public_scrub.py`
exits 0, so the run at this delivery's head runs the scrub and its test as they stand there.

894bae5f re-renders eleven `/remote` baselines under `web/app/tests/verify/__screenshots__`. It
holds images only and changes no test code or assertion between A14's red and its green.
