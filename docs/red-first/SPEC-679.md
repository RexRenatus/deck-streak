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

The green commit `63e8fce` carries the amendment, the SPEC and the fragment, and `git diff --quiet 63e8fce HEAD -- scripts/tests/test_macos_destination_record.py docs/decisions/ADR-342-one-universal-iphone-and-ipad-app-driven-by-touch-keys-or-an-8bitdo-remote.md docs/specs/SPEC-679-a-native-macos-destination-for-the-one-app-target.md changelog.d/macos-destination-679.md` reads rc 0.
