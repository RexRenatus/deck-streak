# SPEC-692: the signing step names the placed certificate's shape and openssl's refusals by fixed classes, and re-wraps the key under PBES2

- **Issue:** none. **Context(s):** `scripts` (`scripts/ios_lane.py`).
- **Decided by:** ADR-711.
- **Status:** built when its pull request lands. Which of the two forms the keychain
  imports is not measured until the first lane run after the land reads the step's
  import line.

## 1. The problem, measured

Measured at this SPEC's base, in `scripts/ios_lane.py` (543 lines) and
`scripts/tests/test_ios_lane.py` (1435 lines):

- `sign()` (lines 301 to 412) decodes the placed certificate with the lenient base64 decode
  at line 326, after the version call at line 323, and prints nothing of what the decode
  gave. A placed text the decode refuses, as a raw PEM text pasted in place of its base64
  most often is, raises there as an uncaught exception after the version line.
- A failed read is named from openssl's standard error by `READ_FAILURES` (lines 92 to 96):
  `wrong-password`, `legacy-algorithm` and `option-refused`, else `unrecognised`. A read
  refused because openssl cannot open or load its input, or because its ASN.1 decoding
  refuses the bytes, falls to `unrecognised`, so a run whose two reads both fail that way
  names neither how the certificate was placed nor what openssl refused.
- The re-wrap (lines 330 to 347) protects the key and the certificate with
  `PBE-SHA1-3DES` and seals the bundle with a `sha1` MAC. The import (lines 352 to 365)
  runs once through `tool()`, whose refusal is the whole-tool line
  `security import failed with exit 1`, and nothing retries it.
- The test module runs 29 tests, all passing on Linux. Its default certificate fixture is
  512 random bytes, whose first octet is random.

## 2. Requirements

R1. Before the version call and the certificate read, the signing step decodes the placed
    certificate once and prints one line, `certificate shape: class <class>`, naming its shape by
    one class of a closed set tested in this order: `der-sequence`, `pem-armour`, `base64-text`,
    `other`. No byte of the certificate is printed.
R2. A placed certificate that does not decode from base64 prints the class `other`, then refuses
    with `the certificate does not decode from base64` before any tool runs.
R3. The read classes of SPEC-352 section 8 gain `could-not-read` and `asn1-decode`, tested after
    `option-refused` and before `unrecognised`; openssl's standard error is still never printed,
    logged, written to a file or put in a refusal.
R4. The re-wrap protects the key and the certificate with AES-256-CBC under PBES2 and seals the
    bundle with a SHA-256 MAC, naming no SHA-1 based algorithm. When the keychain refuses that
    import, the step names the refused form, makes the job keychain anew, re-wraps once under the
    SHA-1 based compatibility form and imports once more; a second refusal refuses naming both
    forms, and no third import and no build tool runs.

## 3. Acceptance criteria of SPEC-692

| id | criterion | decided by |
|---|---|---|
| A1 | with recording shims on `PATH`, each planted certificate prints exactly one shape line naming its class (one case per class, the DER case in both length forms), written out before the version call and the certificate read, and a marker the certificate holds reaches neither the step's output nor its errors | `test_ios_lane.py`, `test_the_certificate_shape_is_printed_as_one_fixed_class_before_the_first_read` |
| A2 | a placed certificate that does not decode prints the class `other`, then the one fixed refusal, and the shims record no tool | `test_ios_lane.py`, `test_a_certificate_that_does_not_decode_is_refused_after_its_class_before_any_tool` |
| A3 | each failed read prints exactly one line naming one class of the grown closed set, chosen in order (one case per class, each case's text holding the words of every class after its own), and a marker planted in openssl's standard error reaches neither the step's output nor its refusal | `test_ios_lane.py`, `test_each_failed_certificate_read_prints_one_class_and_nothing_of_openssl` |
| A4 | the exact call list's re-wrap passes the PBES2 form, no argument names a SHA-1 based algorithm, and the job keychain is offered that bundle once | `test_ios_lane.py`, `test_each_signing_tool_is_given_exactly_its_arguments` |
| A5 | one refused import is followed by one line naming it, the job keychain made anew, one re-wrap under the compatibility form and one more import; two refused imports refuse naming both forms, with no third import, no build tool and no include file | `test_ios_lane.py`, `test_a_refused_keychain_import_is_imported_once_more_under_the_compatibility_form` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_ios_lane.py -k test_the_certificate_shape_is_printed_as_one_fixed_class_before_the_first_read
A2: python3 -m unittest discover -s scripts/tests -p test_ios_lane.py -k test_a_certificate_that_does_not_decode_is_refused_after_its_class_before_any_tool
A3: python3 -m unittest discover -s scripts/tests -p test_ios_lane.py -k test_each_failed_certificate_read_prints_one_class_and_nothing_of_openssl
A4: python3 -m unittest discover -s scripts/tests -p test_ios_lane.py -k test_each_signing_tool_is_given_exactly_its_arguments
A5: python3 -m unittest discover -s scripts/tests -p test_ios_lane.py -k test_a_refused_keychain_import_is_imported_once_more_under_the_compatibility_form
```

Each criterion's test was committed red before the code that makes it pass, and
`docs/red-first/SPEC-692.md` records each red and green by commit. The default
certificate fixture becomes a synthetic DER sequence, a header over a random body, so
every signing test's shape line is `der-sequence`. SPEC-352's exact output lists for a
failed read gain the shape line first, and no assertion is removed.

## 4. File manifest

| path | context | change |
|---|---|---|
| `scripts/ios_lane.py` | scripts | changed: the shape class and the decode refusal (R1, R2), two refusal classes (R3), `FORMS` and the one compatibility import (R4) |
| `scripts/tests/test_ios_lane.py` | tests | changed: three tests added, the call-list and refusal-class tests amended, the shim's refused imports, and the build-tool line test sets the step's own fixed lines aside |
| `docs/specs/SPEC-692-the-signing-step-names-the-placed-certificates-shape-and-openssls-refusals-by-fixed-classes-and-re-wraps-the-key-under-pbes2.md` | this SPEC | added |
| `docs/decisions/ADR-711-the-signing-step-names-the-placed-certificates-shape-and-openssls-refusals-by-fixed-classes-and-re-wraps-the-key-under-pbes2.md` | its decision record | added |
| `docs/red-first/SPEC-692.md` | the red-first record | added |
| `docs/specs/SPEC-352-testflight-lane.md` | docs | changed: insert-only amendment of R9 and R13 |
| `docs/decisions/ADR-363-testflight-lane.md` | docs | changed: insert-only amendment bullet |
| `docs/schematics/testflight-lane.md` | docs | changed: insert-only class rows and flow lines |
| `scripts/mutation-rows.d/S69200-S69299.json` | mutation | added: one row per new arm |
| `changelog.d/testflight-sign-shape.md` | docs | added: the changelog fragment |

## 5. What this does NOT cover

- The profile's parse and its refusals: unchanged here.
- Any workflow, trigger, runner, secret, environment or repository variable, and any path
  under `ios/`: none changes.
- Removing the compatibility form once a lane run reads `pbes2` accepted: a later change,
  read at the lane run after it.
- No SHA-1 is computed by the lane or its test: the compatibility form names a SHA-1 based
  algorithm as an openssl argument only, and the digest census stands.

## 6. Risks

- **The keychain refuses both forms.** The step then refuses naming both and runs no build
  tool; the bundle or the runner image is what changes (ADR-711).
- **The vocabulary misses.** openssl's text for a refused read may hold none of the new
  phrases. The read then still prints `unrecognised`, and that line is the measurement that
  says so.
- **A DER object that is not a bundle.** `der-sequence` names the encoding, not the content:
  a certificate exported without its key reads `der-sequence` and then fails its read as
  `asn1-decode`, and the two lines together name it.

## 7. What only CI proves

After the land, the first lane run's signing step is read by name: its shape line, any read
class, and its import line. A `sha1-compat` import there is recorded in ADR-711 with that run's id.

CI's hygiene job runs `test_ios_lane.py` whole, and its mutation jobs prove the rows of
`scripts/mutation-rows.d/S69200-S69299.json`.

## 8. Formal model

Not applicable by surface: a closed classification and one bounded retry (two forms, never
a third) in one Python step, with no shared protocol, no concurrency and no state machine
across processes. The bound is held by the exact call-list tests and the rows, and
`config/formal.json` names no lane path.

## 9. References

- SPEC-352 R9, R13 and section 8; SPEC-566; ADR-363 D3 and D6; ADR-711.
- openssl `pkcs12`: `-export` with `-certpbe`, `-keypbe` and `-macalg`, and `-legacy`.
- Python's `base64.b64decode`, whose lenient decode refuses a text whose padding does not
  close.
