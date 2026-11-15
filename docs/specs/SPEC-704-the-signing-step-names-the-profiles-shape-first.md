# SPEC-704: the signing step names the profile's shape first

- **Issue:** none. **Context(s):** the lane script (`scripts/ios_lane.py`, its `sign` step).
- **Decided by:** ADR-723. It extends SPEC-352 R8, the signing step's check of the placed profile
  before any tool runs.
- **Status:** built when its pull request lands. The lane's first signed archive after the land
  prints the new line in its step `the archive, signed`.

## 1. The problem, measured

The lane's step `the archive, signed` failed in app job 132546500010 (run 43594481191) at
`scripts/ios_lane.py` line 307 in `sign`, with `AttributeError: 'NoneType' object has no attribute
'group'`. Line 306 decodes the placed profile from base64, and line 307's search for an XML
property list (`<?xml` to `</plist>`) in the decoded bytes found none, so the parse called
`.group()` on no match. The step named nothing of what it had read: not the profile's shape, not
its size, and not whether the stored text held characters the decode had discarded.

- At this SPEC's base, `sign` (line 301) decodes at line 306 with Python's non-strict
  `base64.b64decode`, which discards every character outside the base64 alphabet before its
  padding check, and parses at line 307 with no check that the search matched. Measured by
  `grep -n -E 'b64decode|re.search' scripts/ios_lane.py`.
- A signed profile is a DER SEQUENCE that holds the XML property list, so its decoded bytes open
  0x30. The tests' `profile` fixture (`scripts/tests/test_ios_lane.py`) is built that way: 0x30
  0x82, random bytes, a property list and random bytes.
- No line the step printed before its parse named the profile in any form, and a line that named
  it by a part of its bytes would print part of a credential, which the lane never does
  (SPEC-352 R13).

## 2. Requirements

- **R1.** Before its parse, `sign` prints one line naming the decoded profile's shape by one class
  of a closed set of six, tested in this order: `der-xml-plist` (a DER sequence holding an XML
  property list), `der-no-plist` (a DER sequence holding none), `base64-text` (base64 text),
  `binary-plist` (a binary property list), `xml-plist` (an XML property list with no DER wrapper)
  and `other`. A DER sequence is read by its tag and its length's form only, never by what the
  length covers: byte 0 is 0x30 and byte 1 is a length's first octet in one of X.690's three
  forms, the short (below 0x80), the indefinite (0x80) or the long (0x81 to 0xFE, with that many
  length octets present; 0xFF is reserved). Base64 text is one or more bytes, each a base64
  alphabet character or whitespace; a binary property list opens with `bplist00`; an XML property
  list is what the step's search for `<?xml` to `</plist>` finds. The line gives the decoded byte
  count and the count of the stored text's characters outside both the base64 alphabet (A to Z, a
  to z, 0 to 9, `+`, `/` and the `=` pad) and whitespace, so a wrapped value's line breaks count 0,
  in the form `profile shape: class <class>, <n> decoded bytes, <k> stored characters outside the
  base64 alphabet`, and it is flushed before any tool runs.
- **R2.** Where the search finds no XML property list, `sign` refuses in the form `the profile
  holds no XML property list: class <class>`, in place of the AttributeError and before any tool
  runs: one line on standard error and exit 1.
- **R3.** Where the search finds an XML property list, the parse and every line after it keep their
  behaviour.
- **R4.** No line, refusal, file or log holds a byte, word or prefix of the stored value or of its
  decoded bytes: only the class and the two counts.
- **R5.** No `security cms -D` anywhere: the class is read from the decoded bytes by the step
  itself, never by a macOS tool.

## 3. Acceptance criteria of SPEC-704

| id | criterion | decided by |
|---|---|---|
| A1 | A DER sequence holding an XML property list, the fixture's long form and the indefinite form, is named `der-xml-plist` in the step's first line, and the step goes on past the parse | `test_profile_shape_der_xml_plist_is_named_and_the_step_goes_on` |
| A2 | A DER sequence holding none, in the short form, as two bytes and in the long form with its one length octet, is named `der-no-plist` and refused by it with no tool run; two placed characters outside the alphabet count 2 and line breaks count 0 | `test_profile_shape_der_no_plist_is_named_and_refused` |
| A3 | Base64 text with a line break of its own is named `base64-text` and refused by it with no tool run | `test_profile_shape_base64_text_is_named_and_refused` |
| A4 | A binary property list is named `binary-plist` and refused by it with no tool run | `test_profile_shape_binary_plist_is_named_and_refused` |
| A5 | An XML property list with no DER wrapper is named `xml-plist`, and the step goes on past the parse | `test_profile_shape_xml_plist_is_named_and_the_step_goes_on` |
| A6 | Bytes no earlier class names, among them a 0x30 whose length's first octet is the reserved 0xFF or names a length octet that is absent, are named `other` and refused with no tool run | `test_profile_shape_other_is_named_and_refused` |
| A7 | The signing run prints the profile line once and first; set aside, its other lines are the mask lines and the build tool's two redacted lines, as before | `test_a_build_tool_prints_only_its_error_and_warning_lines_each_value_redacted` |
| A8 | Each failed certificate read prints the profile line and then its one class line, and no other line but the mask lines | `test_each_failed_certificate_read_prints_one_class_and_nothing_of_openssl` |
| A9 | A failed legacy read prints the profile line and then the first read's class line, and no other line but the mask lines | `test_a_failed_legacy_read_refuses_naming_both_classes_and_runs_no_further_tool` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_ios_lane.py -k test_profile_shape_der_xml_plist_is_named_and_the_step_goes_on
A2: python3 -m unittest discover -s scripts/tests -p test_ios_lane.py -k test_profile_shape_der_no_plist_is_named_and_refused
A3: python3 -m unittest discover -s scripts/tests -p test_ios_lane.py -k test_profile_shape_base64_text_is_named_and_refused
A4: python3 -m unittest discover -s scripts/tests -p test_ios_lane.py -k test_profile_shape_binary_plist_is_named_and_refused
A5: python3 -m unittest discover -s scripts/tests -p test_ios_lane.py -k test_profile_shape_xml_plist_is_named_and_the_step_goes_on
A6: python3 -m unittest discover -s scripts/tests -p test_ios_lane.py -k test_profile_shape_other_is_named_and_refused
A7: python3 -m unittest discover -s scripts/tests -p test_ios_lane.py -k test_a_build_tool_prints_only_its_error_and_warning_lines_each_value_redacted
A8: python3 -m unittest discover -s scripts/tests -p test_ios_lane.py -k test_each_failed_certificate_read_prints_one_class_and_nothing_of_openssl
A9: python3 -m unittest discover -s scripts/tests -p test_ios_lane.py -k test_a_failed_legacy_read_refuses_naming_both_classes_and_runs_no_further_tool
```

## 4. File manifest

| path | context | change |
|---|---|---|
| `scripts/ios_lane.py` | the lane script: the class tuple and `profile_shape` between `numbers()` and `app()`, and `sign` after its decode | changed: the six classes and the function that names one; `sign` prints the profile line and refuses a profile with no XML property list by its class |
| `scripts/tests/test_ios_lane.py` | the lane's tests: the class tests after the profile test, and three tests that read the signing step's printed lines | changed: six class tests and their shared line helper added; three assertions admit the profile line |
| `docs/specs/SPEC-704-the-signing-step-names-the-profiles-shape-first.md` | this SPEC | added |
| `docs/decisions/ADR-723-the-profiles-shape-is-named-from-its-bytes-by-fixed-classes-never-by-a-tool.md` | the decision | added |
| `docs/red-first/SPEC-704.md` | the red-first record of the nine criteria | added in the record commit after the change |
| `changelog.d/testflight-profile-shape.md` | the changelog fragment | added |
| `scripts/mutation-rows.d/S70400-S70499.json` | the band's mutation rows on the lane script | added |

## 5. What this does NOT cover

- The re-set of the stored value: this change names a wrong profile, and placing a right one is
  outside the tree (SPEC-352).
- A stored text that fails to decode, by its padding or a character outside ASCII: line 306's
  decode raises as before, ahead of the line (SPEC-352).
- The macOS tool route: no `security cms -D` reads the profile here, so a profile only that tool
  could read is named by its bytes alone (SPEC-352).
- The post-land read: the class the stored value reads is known only from the first lane run that
  holds this change, read after the land (SPEC-352).

## 6. Risks

- **A profile the search cannot read is now refused by name.** A signed profile whose property
  list is not XML reads `der-no-plist` and is refused with its counts, where the step failed with
  an AttributeError before; the refusal is the point, and the two counts tell a truncated value
  from a wrong one.
- **The DER read is shallow.** Any bytes that open 0x30 with a first length octet in one of the
  three forms read as a DER sequence, base64 text that opens with the digit 0 among them. The
  order is fixed and tested, so a misread names the earlier class, never a class outside the six.
- **The line has its own flush.** The mask lines the step prints next flush the same buffer before
  any tool runs, so no test can tell the line's own flush from theirs: it is written explicitly
  rather than as a keyword a mutant could switch off unseen.

## 7. What only CI proves

- The `hygiene` job runs `test_ios_lane.py` by name, the module as a whole.
- The mutation jobs prove the band's rows on `scripts/ios_lane.py` and the generated mutants of
  the changed lines.
- The first lane push run after the land prints the line in its step `the archive, signed`, read
  there by name for the class the stored value reads.

## 8. Formal model

None: `config/formal.json` names no path of the lane script.

## 9. References

- SPEC-352 R8 and R13, and ADR-363: the signing step's checks and its printed lines.
- ADR-723: the decision this SPEC carries.
- X.690: the three forms of a length's first octet.
- Python's `base64.b64decode` (characters outside the alphabet discarded before the padding check)
  and `plistlib` (a binary property list's `bplist00` header).
