---
status: accepted
decision-makers: "the DeckStreak architect"
---

# The profile's shape is named from its bytes by fixed classes, never by a tool

## Context and Problem Statement

The lane's signing step decodes the placed profile from base64 and searches the decoded bytes for
an XML property list. When the search found none, the step failed with an AttributeError on the
missing match and named nothing of what it had read, so a wrong stored value showed no sign of
what it was (SPEC-704, section 1). The step must never print any part of the value or of its
decoded bytes (SPEC-352 R13), so the profile can only describe itself by what carries none of it.

## Decision Drivers

- A wrong profile names its shape on the first run that reads it, before any tool runs.
- Nothing of the stored value or of its decoded bytes is printed: only a fixed class and two
  counts.
- A profile that holds an XML property list is read exactly as before.
- The tests decide every class on Linux, with no signing material and no macOS tool.

## Decision Outcome, and what each decision was chosen against

### D1. Six fixed classes, named from the decoded bytes alone and tested in a fixed order

`profile_shape` names the decoded bytes by the first of `der-xml-plist`, `der-no-plist`,
`base64-text`, `binary-plist`, `xml-plist` and `other` whose test they meet, and `sign` prints the
class in one line before its parse, with the decoded byte count. Chosen against `security cms -D`,
which reads a signed profile only on macOS and would move the success path onto a tool the tests
cannot run; against printing a prefix or a digest of the bytes, which names part of the value;
and against an open description, which no test can hold to a closed set.

### D2. A DER sequence is read by its tag and its length's form, never by what the length covers

The SEQUENCE tag 0x30 and a first length octet in one of X.690's three forms (short, indefinite,
or long with its length octets present) name a DER sequence; the length's value is never checked
against the bytes. Chosen against a full DER parse, which the lane would have to carry for a class
it does not need, and against reading the length's coverage, which a signed profile's indefinite
form (0x30 0x80) does not state.

### D3. A profile with no XML property list is refused by its class

Where the search finds no XML property list, `sign` refuses with `the profile holds no XML
property list: class <class>` before any tool runs, in place of the AttributeError. Chosen against
keeping the AttributeError after the new line, which leaves a traceback as the step's error, and
against refusing every class but the two property-list classes, which would refuse a profile the
search can still read.

### D4. The line counts what the decode discards

The line also counts the stored text's characters outside the base64 alphabet and whitespace,
which the non-strict decode drops without a word. Chosen against a strict decode, which would
change how the existing decode reads a wrapped value, and against counting whitespace, which would
make every wrapped value look damaged.

## Consequences

- Good: a wrong profile names its class and its counts on its first run, and nothing of the value
  reaches the log.
- Good: the change is one tuple and one function beside the lane's other module-level helpers, and
  a few lines of `sign` between its decode and its parse.
- Bad: a class names a shape, not a cause: a `der-no-plist` profile may be truncated or of the
  wrong kind, and the counts are the only further sign.

## Confirmation

SPEC-704's nine tests read red at the commit that adds them and green at the commit that changes
the signing step. After the land, the lane's first push run that holds the change prints the line
in its step `the archive, signed`, read there by name; no run has read it yet.

## What would make this wrong

- A signed profile that holds its property list in a form the search cannot find, which the step
  would then refuse by its class where it should read it.
- A stored value whose decoded bytes meet an earlier class's test by chance, such as base64 text
  that opens with the digit 0, so its class reads `der-no-plist`.

## More Information

SPEC-704; SPEC-352 R8 and R13; ADR-363.
