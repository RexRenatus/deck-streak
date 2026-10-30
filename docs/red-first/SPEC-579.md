# SPEC-579 red-first record

A1, A7, A10, A13, A15 and A19 are red at the red commit, which adds the tests alone, and green at the commit that
delivers each one. A2 to A6, A8, A9, A11, A12 and A14 plant their refusal over a fixture the test spells, beside a
control that reads green, so each is `not red`. A16 to A18 pin refusals the scrub already made, so each is `not red`
and is held by its mutation row. Every line below was measured locally by running the test module by its path
(`python3 -B scripts/tests/<module>.py -v`) or the web suite (`vitest run` over `src/lib/manifest.test.ts`,
`src/lib/csp.test.ts` and `src/hooks.test.ts`) on a tree byte-identical to the commit it names.

```text
A1: python3 -m unittest discover -s scripts/tests -p test_brand_files.py -k test_every_listed_file_matches_its_entry
A7: python3 -m unittest discover -s scripts/tests -p test_brand_files.py -k test_the_readme_opens_with_the_title_then_the_hero
A10: python3 -m unittest discover -s scripts/tests -p test_brand_files.py -k test_the_app_target_names_the_catalog_and_its_icon_set
A13: pnpm exec vitest run web/app/src/lib/manifest.test.ts -t "the manifest meets the install criteria"
A15: python3 -m unittest discover -s scripts/tests -p test_public_scrub.py -k test_a_listed_binary_passes_the_tree_and_history_scans
A19: python3 -m unittest discover -s scripts/tests -p test_public_scrub.py -k test_a_malformed_list_stops_the_scrub_by_name
```

```red-first
A1: red at 592b067c5926a55205a52af12fcbe4d7eb0f6e59: AssertionError: Lists differ: ['missing: scripts/brand-files.json: no list at the root'] != []
A1: green at 39d6b0a34666969b8624bd0d1a28bf2512a3a8be
A2: not red: a planted refusal over a fixture inside the test (or the control); refused on every run
A3: not red: a planted refusal over a fixture inside the test (or the control); refused on every run
A4: not red: a planted refusal over a fixture inside the test (or the control); refused on every run
A5: not red: a planted refusal over a fixture inside the test (or the control); refused on every run
A6: not red: a planted refusal over a fixture inside the test (or the control); refused on every run
A7: red at 592b067c5926a55205a52af12fcbe4d7eb0f6e59: AssertionError: Lists differ: ["hero-not-below-title: README.md: lines 2[124 chars]s,'"] != []
A7: green at 1e44f9d0f0e553ec16bb0591db0b1d0978c34d8d
A8: not red: a planted refusal over a fixture inside the test (or the control); refused on every run
A9: not red: a planted refusal over a fixture inside the test (or the control); refused on every run
A10: red at 592b067c5926a55205a52af12fcbe4d7eb0f6e59: AssertionError: Lists differ: ['no-catalog-source: ios/app.yml: the targ[300 chars]ons'] != []
A10: green at f466f6f22520937abde2a621d805bf4c1bb886a4
A11: not red: a planted refusal over a fixture inside the test (or the control); refused on every run
A12: not red: a planted refusal over a fixture inside the test (or the control); refused on every run
A13: red at 592b067c5926a55205a52af12fcbe4d7eb0f6e59: AssertionError: expected { name: 'DeckStreak', …(4) } to deeply equal { name: 'DeckStreak', …(4) }
A13: green at 83d9d23bed04b17427f784725508c56d909682dd
A14: not red: a planted refusal over a fixture inside the test (or the control); refused on every run
A15: red at 592b067c5926a55205a52af12fcbe4d7eb0f6e59: AssertionError: 1 != 0 : public-scrub: icons/logo.png: binary
A15: green at 779373967caca41905434377694bf2b6586b6386
A16: not red: a kept refusal of the scrub, pinned; its red is row S57901
A17: not red: a kept refusal of the scrub, pinned; its red is row S57902
A18: not red: a kept refusal of the scrub, pinned; its red is row S57903
A19: red at 592b067c5926a55205a52af12fcbe4d7eb0f6e59: AssertionError: 0 != 2 : examined 2 file(s) against public shapes only; 0 finding(s)
A19: green at 779373967caca41905434377694bf2b6586b6386
```

## What each red shows

- A1 fails because the base holds no list and none of SPEC-579 R1's eleven files; the failure message names all eleven
  as absent.
- A7 fails because the README's third line is the description paragraph, not the hero.
- A10 fails with four findings: the app target lists no `App/Assets.xcassets`, sets no
  `ASSETCATALOG_COMPILER_APPICON_NAME`, and the catalog and its icon set are absent.
- A13 fails because the manifest holds two icon entries, the plain squares the endpoints built, not the brand source's
  three; at the red commit `web/app/static/` holds no image.
- A15 fails because the base refuses the listed binary by the rule `binary` in the tree scan.
- A19 fails on each of its three planted lists (not JSON, a 63-digit sha256, a path named twice): the base ignores the
  list and exits 0, never 2.

A18's test also fails at the red commit, by its last assertion: the base still reports the listed binary as `binary`,
which is A15's missing admission. Its two literal assertions hold there, because the base already searches every binary
for the private literals, so its own criterion cannot be red first; row S57903 is its red.

Each green commit and the head are byte-identical over the test module and every file its criterion reads
(`git diff --quiet <green> <head> -- <paths>` exits 0 for each), so a run at the head reads the green commit's tests and
code.

## The rows, proved by hand

Each row was proved on its own: the target's sha256 recorded, its killer run once on the unmutated tree (one test,
passed), the replacement installed once by script, the killer run by its explicit id with its own
`PYTHONPYCACHEPREFIX`, its failing assertion read, and the saved bytes written back with the sha256 compared equal.

- S57901, admission by path alone: its killer,
  `test_public_scrub.AListedBrandBinaryIsAdmitted.test_a_listed_path_with_other_bytes_is_refused_as_binary`, failed
  with `AssertionError: 0 != 1`: the other bytes at the listed path passed the scrub.
- S57902, admission by content alone: its killer,
  `test_public_scrub.AListedBrandBinaryIsAdmitted.test_listed_bytes_at_an_unlisted_path_are_refused_as_binary`, failed
  with `AssertionError: 0 != 1`: the listed bytes passed at an unlisted path.
- S57903, the admitted branch skipping the search: its killer,
  `test_public_scrub.AListedBrandBinaryIsAdmitted.test_a_private_literal_inside_a_listed_binary_is_still_found`, failed
  with `AssertionError: 0 != 1`: the private literal inside the listed binary was not found.
- S57904, the hero at a path the tree does not hold: its killer,
  `test_brand_files.TheReadmeHeaderShowsTheHero.test_the_readme_opens_with_the_title_then_the_hero`, failed with
  `AssertionError: Lists differ: ['hero-not-listed: README.md: docs/assets/[48 chars]ree'] != []`.
- S57905, another icon set name: its killer,
  `test_brand_files.TheAppCarriesItsIconSet.test_the_app_target_names_the_catalog_and_its_icon_set`, failed with
  `AssertionError: Lists differ: ["no-icon-setting: ios/app.yml: ASSETCATAL[37 chars]et'"] != []`.
