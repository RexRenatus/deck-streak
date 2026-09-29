# Red-first record: SPEC-047

The order of work was: the census that the reading use cases touch no streak (50452a9); the
studied rule's tests, the parity golden and inert readings stubs (063e3b0); the tap and settle
tests beside stubs in coordination (56c49f7); the owner's route test beside a stub route that
answered 501 (831ff6e); the implementation, the migration and the refreshed query cache (c27e169);
the mutation rows (ce9a96d); and the tests that pin the XP constants, the grant sources and the
track (the last commit before the SPEC's promotion).

The parity golden was generated from the predecessor's own function, `preread_tracking.py:is_studied`,
at `27ee2bc`, over synthetic counts. The generator wrote to a scratch registry and output, with
bytecode writing off, and the predecessor's checkout read the same before and after.

Each criterion was run at its red commit with the SPEC's own fenced command, selecting one test,
and the whole of each test file was run there as well: all four tests of `readings_read_tap` and
all three of `readings_settle` failed there, each by assertion. The stubs answered nothing: the
tap answered a reading that was not found, the settle step measured no reading, the studied rule
answered false and the window counted no review. The census (A10) was red until the tap's source
file existed in the examined population; it was green as soon as the stub file joined it, because a
stub touches no streak, and it stays green with the implementation.

The read route's test (A11) was red against a stub that answered 501, and the two assertions the
review asked for sit in the tap and route tests: `a_second_read_tap_changes_nothing` asserts
covered 5, studied 0 and verdict open, and `the_read_route_answers_only_the_owner` asserts covered 5,
studied 4 and verdict studied.

```red-first
A1: red at 56c49f7: the tap answered: Err(NotFound)
A1: green at c27e169
A2: red at 56c49f7: assertion `left == right` failed: one caller of the read tick: []
A2: green at c27e169
A3: red at 56c49f7: the tap answered: Err(NotFound)
A3: green at c27e169
A4: red at 063e3b0: the stub answered false where the golden says a reading is studied
A4: green at c27e169
A5: red at 063e3b0: assertion failed: window.counts(rule, generated)
A5: green at c27e169
A6: red at 56c49f7: assertion `left == right` failed: the open reading is measured
A6: green at c27e169
A7: red at 56c49f7: the reading has a row
A7: green at c27e169
A8: red at 56c49f7: the tap answered: Err(NotFound)
A8: green at c27e169
A9: red at 56c49f7: the reading has a row
A9: green at c27e169
A10: red at 50452a9: read_tap.rs is not in the examined population
A10: green at 56c49f7
A11: red at 831ff6e: assertion `left == right` failed: left: 501, right: 200
A11: green at c27e169
```
