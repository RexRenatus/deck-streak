# Red-first record: SPEC-351

SPEC-351, ADR-362 and the schematic were committed first (67aa77b1). A1's and A2's tests,
with SPEC-337 A6's amended list of the route's children, were then committed alone with the guard
absent, and read red by assertion. The guard followed in its own commit.

```red-first
A1: red at 4507a945: AssertionError: 101 != 0 : 101 respelling(s) of the route the edge serves
A1: green at c76f55c1
A2: red at 4507a945: AssertionError: 660 != 0 : 660 served spelling(s) of a refused login not counted
A2: green at c76f55c1
```
