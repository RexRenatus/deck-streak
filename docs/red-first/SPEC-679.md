# SPEC-679: the red-first record

```red-first
A1: red at 464ae4a: AssertionError: 0 != 1 : ADR-342 holds 0 heading(s) naming macOS and SPEC-679, not one
A1: green at 63e8fce
A2: red at 464ae4a: AssertionError: 0 != 1 : the macOS amendment of ADR-342 names 0 minimum macOS version(s), not one
A2: green at 63e8fce
A3: red at 464ae4a: AssertionError: 'body text at 13 pt' not found in '' : the macOS amendment of ADR-342 lacks: body text at 13 pt
A3: green at 63e8fce
A4: not red: each literal it asserts holds at the base too; it guards the iOS destination against the Mac target's edits
A5: not red: every SPEC file is named by the convention at the base too; it guards the new SPEC's name
A6: not red: no SPEC number is held twice at the base too; it guards the new SPEC's number
A7: not red: no SPEC in the judged folder reads planned at the base too; it guards the new SPEC's Status line
```

The green commit `63e8fce` carries the amendment, the SPEC and the fragment, and `git diff --quiet 63e8fce HEAD -- docs/decisions/ADR-342-one-universal-iphone-and-ipad-app-driven-by-touch-keys-or-an-8bitdo-remote.md docs/specs/SPEC-679-a-native-macos-destination-for-the-one-app-target.md changelog.d/macos-destination-679.md` reads rc 0.

The first push also read one red that is no criterion's: `test_the_finder_is_defined_once_and_a_planted_copy_is_caught` in `scripts/tests/test_mutation_workflows.py` found the test module's `indent` helper equal in body to the mutants finder's `indent_of`. It was cured in `7b9e3a6`, which counts the same leading spaces by a regular expression match and changes no assertion, and never by a change to that census; the four tests read green at `7b9e3a6`, and `git diff --quiet 7b9e3a6 HEAD -- scripts/tests/test_macos_destination_record.py` reads rc 0.

A8: red at 7b2937d: AssertionError: Lists differ: [] != ['[iOS, macOS]']
A8: green at 186b5d6
A9: red at 7b2937d: AssertionError: '.macOS(.v14)' not found in 'platforms: [.iOS(.v17)],' : the engine package declares macOS at the minimum
A9: green at 186b5d6
A10: red at 7b2937d: AssertionError: Lists differ: ["ios/App/Sources/CardFaceView.swift: UIVi[1494 chars]OS('] != []
A10: green at 186b5d6
A11: red at 7b2937d: AssertionError: False is not true : the factory states the zoom
A11: green at 186b5d6
A12: red at 7b2937d: test_a_mac_card_view_empties_its_context_menu: XCTAssertEqual failed
A12: green at 186b5d6

The record test `test_adr_342_carries_one_macos_amendment` now also reads the iPad app run unchanged on a Mac among the options ADR-342 rejects. On a planted tree, this base's ADR-342 without that option's line beside the edited test module, it read red, `AssertionError: 'The iPad app run unchanged on a Mac: rejected because' not found in` ending `: the macOS amendment of ADR-342 lacks: The iPad app run unchanged on a Mac: rejected because`; on the real tree it reads green at `7b2937d`.

The first push also read one red that is no criterion's: `test_the_finder_is_defined_once_and_a_planted_copy_is_caught` found the destination test module's `depth` helper equal in body to the mutants finder's `indent_of`. The green commit `186b5d6` cures it, and that module is the one test file the commit changes: it imports `indent_of` from the finder in place of its own copy, and no assertion changes. The cured module reads red over the red commit's forms on a planted tree, by the same `Lists differ` assertion, and green at `186b5d6`.

The green commit `186b5d6` carries the macOS forms, and `git diff --quiet 186b5d6 HEAD` over the criteria tests and the files they read reads rc 0.
