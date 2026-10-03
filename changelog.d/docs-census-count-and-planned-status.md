### Fixed

- SPEC-024's A18 row no longer states a routed-capture count that goes stale with every delivery
  that routes a capture: the census test's own assertion holds it, a docs test refuses a count in
  the row, and an insert-only amendment (section 13) records why.
- SPEC-076 and SPEC-094 no longer read "planned" in their Status: each names the delivery that moved
  it out of `docs/specs/planned/` (#446, #403), and SPEC-002 gains A11, which refuses a judged SPEC
  that reads planned.
