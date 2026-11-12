# Schematic: the --rows-from plan's read, from its text to each verb's refusal (SPEC-645, ADR-668)

Kinds: a data flow (section 1) and a table (section 2).

Every `path:line` below was read at `bd953ba7`, the base, in `scripts/mutation_rows.py` unless it
names another file; the cure's names are SPEC-645's.

## 1. The plan's read, each shape's path

```mermaid
flowchart TD
    A["prove, legs or merge with --rows-from PLAN"] --> B["chosen (:1194): the plan read and parsed (:1207, unchanged)"]
    B -- unreadable or not JSON --> V["the verb's own refusal on stdout, exit 2 (:1224, :1305, :1336, unchanged)"]
    B -- parsed --> C{"an object? [CURE R1, A1 to A3]"}
    C -- no --> X1["PlanRefused: PLAN must hold a top-level object"]
    C -- yes --> D{"rows absent, or an array? [CURE R2, A4]"}
    D -- no --> X2["PlanRefused: PLAN: rows is not an array of row ids"]
    D -- yes --> E{"each entry a string? [CURE R3, A5]"}
    E -- no --> X3["PlanRefused: PLAN: rows[i] is not a string"]
    E -- yes --> F["the selection: each id looked up, an unknown one refused (:1209-1213, unchanged)"]
    F --> G["the rows proved, dealt into legs or merged"]
    X1 --> Y["dispatch: mutation_rows: REFUSED: sentence on stderr, exit 2"]
    X2 --> Y
    X3 --> Y
```

At the base, `C` to `E` do not exist: `chosen` adds `plan.get("rows", [])` (`:1208`) to the ids it
gathers. A plan that is not an object raises `AttributeError` there; a `rows` value that is a
number or null raises `TypeError` there; a `rows` string reaches `F` as its characters and an
object as its keys; an entry that is a number raises `TypeError` at the join of `:1212`, and one
that is an array or an object at the lookup of `:1210`. Each crash is a traceback and exit 1, since
neither the verbs' `except` tuples nor `dispatch` catch it. The cure's boxes are one reader,
`plan_rows`, and `dispatch`'s three arms print its refusal; A6 pins the `F` path, which is
unchanged.

## 2. Where each step lives

| step | at the base | after |
|---|---|---|
| the plan's read | `chosen`, `:1206-1208` | `plan_rows`, called from `chosen` |
| the refusal's print | none | `dispatch`'s three `PlanRefused` arms, for `prove`, `legs` and `merge` |
| the verbs' own refusals | `prove`, `legs`, `merge` at `:1224`, `:1305`, `:1336` | unchanged |
| the tests | none | `TheRowsPlanRefusesAWrongShape` in `scripts/tests/test_mutation_rows.py` |
