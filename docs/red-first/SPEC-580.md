# SPEC-580 red-first record

A1, A3, A6, A8, A10 and A11 are red at the red commit, which adds the tests alone, and green at the commit that
delivers the text or the still each one reads. A2, A4, A5, A7, A12 and A13 plant their refusal over a fixture the test
spells, beside a control that reads green; their helpers are stubbed to return no finding at the red commit, so each
planted refusal is red there, and green at the commit that writes the helpers. A9 is the control itself and is
`not red`. Every line below was measured locally by running the test module by its path
(`python3 -B scripts/tests/<module>.py -v`) on a tree byte-identical to the commit it names.

```text
A1: python3 -m unittest discover -s scripts/tests -p test_identity_docs.py -k test_the_readme_opening_is_the_identity
A2: python3 -m unittest discover -s scripts/tests -p test_identity_docs.py -k test_an_opening_naming_a_barred_word_is_refused
A3: python3 -m unittest discover -s scripts/tests -p test_identity_docs.py -k test_the_readme_built_on_names_each_item
A4: python3 -m unittest discover -s scripts/tests -p test_identity_docs.py -k test_a_built_on_section_missing_an_item_is_refused
A5: python3 -m unittest discover -s scripts/tests -p test_identity_docs.py -k test_a_trademark_line_naming_a_holder_is_refused
A6: python3 -m unittest discover -s scripts/tests -p test_identity_docs.py -k test_notice_names_each_item
A7: python3 -m unittest discover -s scripts/tests -p test_identity_docs.py -k test_a_missing_or_incomplete_notice_is_refused
A8: python3 -m unittest discover -s scripts/tests -p test_identity_docs.py -k test_agents_line_8_is_the_identity
A9: python3 -m unittest discover -s scripts/tests -p test_identity_docs.py -k test_a_complete_planted_tree_reads_green
A10: python3 -m unittest discover -s scripts/tests -p test_brand_files.py -k test_the_readme_opens_with_the_title_then_the_hero
A11: python3 -m unittest discover -s scripts/tests -p test_brand_files.py -k test_the_still_is_listed_by_its_pinned_sha256
A12: python3 -m unittest discover -s scripts/tests -p test_brand_files.py -k test_a_hero_with_no_reduced_motion_source_is_refused
A13: python3 -m unittest discover -s scripts/tests -p test_brand_files.py -k test_a_missing_or_altered_still_is_refused
```

```red-first
A1: red at 98b67d059fdc363dd9dfc387d87c9f4a6628462d: AssertionError: "A spaced-repetition ... before 1.0." not found in "# DeckStreak ![DeckStreak](docs/assets/deckstreak-hero-tide-light.gif) DeckStreak turns your ..."
A1: green at d4139479097fa12458bef0bdf7e85fec2061de9c
A2: red at 98b67d059fdc363dd9dfc387d87c9f4a6628462d: AssertionError: Lists differ: [] != [('barred-word', 'README.md')]
A2: green at d8f619fec7c6839d2faeefedbb16319ed5dde97f
A3: red at 98b67d059fdc363dd9dfc387d87c9f4a6628462d: AssertionError: examined 0 Built on items: the population is empty, so nothing was judged
A3: green at d4139479097fa12458bef0bdf7e85fec2061de9c
A4: red at 98b67d059fdc363dd9dfc387d87c9f4a6628462d: AssertionError: Lists differ: [] != [('built-on-item-missing', 'README.md')]
A4: green at d8f619fec7c6839d2faeefedbb16319ed5dde97f
A5: red at 98b67d059fdc363dd9dfc387d87c9f4a6628462d: AssertionError: Lists differ: [] != [('trademark-line', 'README.md')]
A5: green at d8f619fec7c6839d2faeefedbb16319ed5dde97f
A6: red at 98b67d059fdc363dd9dfc387d87c9f4a6628462d: AssertionError: examined 0 NOTICE items: the population is empty, so nothing was judged
A6: green at d4139479097fa12458bef0bdf7e85fec2061de9c
A7: red at 98b67d059fdc363dd9dfc387d87c9f4a6628462d: AssertionError: Lists differ: [] != [('notice-missing', 'NOTICE')]
A7: green at d8f619fec7c6839d2faeefedbb16319ed5dde97f
A8: red at 98b67d059fdc363dd9dfc387d87c9f4a6628462d: AssertionError: 'DeckStreak is a ...[33 chars]tudy' != 'DeckStreak is a spaced-repetition study game for the web,[34 chars]tudy'
A8: green at d4139479097fa12458bef0bdf7e85fec2061de9c
A9: not red: the planted complete tree, the control; it reads green at every commit, so each planted refusal's red is its own
A10: red at 98b67d059fdc363dd9dfc387d87c9f4a6628462d: AssertionError: Lists differ: ["hero-not-below-title: README.md: lines 2[131 chars]f)'"] != []
A10: green at d4139479097fa12458bef0bdf7e85fec2061de9c
A11: red at 98b67d059fdc363dd9dfc387d87c9f4a6628462d: AssertionError: examined 0 still entries: the population is empty, so nothing was judged
A11: green at d4139479097fa12458bef0bdf7e85fec2061de9c
A12: red at 98b67d059fdc363dd9dfc387d87c9f4a6628462d: AssertionError: Lists differ: [] != [('hero-no-still', 'README.md')]
A12: green at d8f619fec7c6839d2faeefedbb16319ed5dde97f
A13: red at 98b67d059fdc363dd9dfc387d87c9f4a6628462d: AssertionError: Lists differ: [] != [('still-missing', 'docs/assets/still.png')]
A13: green at d8f619fec7c6839d2faeefedbb16319ed5dde97f
```

## What each red shows

- A1 fails because the base README's opening is another description, not the repository's.
- A3 and A6 fail because the base README has no Built on section and the tree has no NOTICE, so the census examines
  an empty population and says so.
- A8 fails because the base overview line frames another product.
- A10 fails because the base README holds the hero as a bare image line, where the amended header expects the picture.
- A11 fails because the base list holds no entry for the still, so the census examines an empty population.
- A2, A4, A5, A7, A12 and A13 each plant their refusal beside a control that reads green; with the helper stubbed, the
  planted refusal finds nothing, which is exactly what each assertion reads.

A10's test is dev's own test of the README header, kept and amended by the picture's order (SPEC-580 R3). Two commits
edit a test module between the red and the green commits, and both are disclosed here. Commit d8f619f changes the
helpers `opening_findings`, `built_on_findings`, `notice_findings`, `agents_findings` and `still_findings` in
`scripts/tests/test_identity_docs.py` and `scripts/tests/test_brand_files.py` from a stub to their bodies; no test
function or assertion changes. Commit d413947 adds one path, the still's, to `BRAND_PATHS` in
`scripts/tests/test_brand_files.py` beside the texts and the still it lists; no assertion changes.

## The rows, proved by hand

Each row was proved on its own: the target's sha256 recorded, the replacement installed once by script, the killer run
by its explicit id with its own `PYTHONPYCACHEPREFIX` (one test), its first failing assertion read, and the saved bytes
written back with the sha256 compared equal.

- S58001, the still at another path: its killer,
  `test_brand_files.TheStillIsTheBrandSourcesPoster.test_the_still_is_listed_by_its_pinned_sha256`, failed with
  `AssertionError: Lists differ: ["still-not-listed: README.md: the source [43 chars]ng'"] != []`. Restore sha256
  `ec34904c47ff9bea874ed775238e838f103c1d8d7964c94df75b0ff6391e0d0e`.
- S58002, the still under another query: the same killer failed with
  `AssertionError: Lists differ: ['hero-no-still: README.md: line 4 is \'  [57 chars]/\''] != []`. Restore sha256
  `ec34904c47ff9bea874ed775238e838f103c1d8d7964c94df75b0ff6391e0d0e`.
- S58003, the list's pin changed by one digit: the same killer failed with
  `AssertionError: Lists differ: [('92[29 chars]c4b81f6de7850558224482acd69334585', 8377, 'png', [[960, 360]])] != [...]`. Restore sha256
  `2f9a70ae4868d11a886f0b890f55e433818a47b68c715b4daf9ef740b655c37c`.
- S58004, the opening's description changed: its killer,
  `test_identity_docs.TheReadmeOpening.test_the_readme_opening_is_the_identity`, failed with an `AssertionError` that
  the description is not found in the README. Restore sha256
  `ec34904c47ff9bea874ed775238e838f103c1d8d7964c94df75b0ff6391e0d0e`.
- S58005, the sync server's item renamed: its killer,
  `test_identity_docs.TheBuiltOnSection.test_the_readme_built_on_names_each_item`, failed with
  `AssertionError: Lists differ:` the five leads, the third missing. Restore sha256
  `ec34904c47ff9bea874ed775238e838f103c1d8d7964c94df75b0ff6391e0d0e`.
- S58006, the trademark line reworded: the same killer failed with
  `AssertionError: Lists differ: ['trademark-line: README.md: the Built on section lacks the trademark line'] != []`.
  Restore sha256 `ec34904c47ff9bea874ed775238e838f103c1d8d7964c94df75b0ff6391e0d0e`.
- S58007, NOTICE's engine licence reworded: its killer,
  `test_identity_docs.TheNotice.test_notice_names_each_item`, failed with `AssertionError: Lists differ:` the six
  names, `engine` missing. Restore sha256 `ba5cf0d40d10019db37a7dbd24a666fa69c681da665dce99462ba4aac0f334df`.
- S58008, the overview line reverted to the old framing: its killer,
  `test_identity_docs.TheAgentsOverview.test_agents_line_8_is_the_identity`, failed with an `AssertionError` that
  line 8 is not the identity line. Restore sha256
  `89b9e18647e90231ba0d5ed22d18ef67359fe343d9dd70d9b3ae24dfac1b8518`.
- S57904, re-anchored to the picture's image line: its killer,
  `test_brand_files.TheReadmeHeaderShowsTheHero.test_the_readme_opens_with_the_title_then_the_hero`, failed with
  `AssertionError: Lists differ: ['hero-not-listed: README.md: docs/assets/[48 chars]ree'] != []`. Restore sha256
  `ec34904c47ff9bea874ed775238e838f103c1d8d7964c94df75b0ff6391e0d0e`.
