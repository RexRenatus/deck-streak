# Schematic: the rows the scripts class leaves to the leg report

Data flow of SPEC-625, decided by ADR-647: how a selected row reaches a class's examined count, and
where a KILLED row that no class credits is named instead. Every name below is a definition of
`scripts/mutation-verdict.py`: `selected_rows`, `judge_rows`, `classify`, `klass_rows` and the new
`leave_rows_to_the_leg_report`.

```mermaid
flowchart TD
    plan["plan: selected_rows writes plan.rows and plan.row_overlaps"]
    leg["mutation-rows leg: one verdict per selected row"]
    judge["judge_rows: every selected row must read KILLED"]
    notkilled["FAIL or VOID by name, as before"]
    credit{"overlap path classifies as the judged class?"}
    counted["carried: the class's rows count, its examined count"]
    gate{"judged class is scripts?"}
    crediting{"overlap absent, or its class outside rust, oracle and scripts?"}
    silent["no new line"]
    count["mutation: scripts: rows left to the leg report: n of s selected"]
    named["mutation: scripts: row left to the leg report: id and its reason"]

    plan --> judge
    leg --> judge
    judge -->|"not KILLED"| notkilled
    judge -->|"KILLED"| credit
    credit -->|"yes"| counted
    judge -->|"after the loop"| gate
    gate -->|"no: rust or oracle"| silent
    gate -->|"yes"| crediting
    crediting -->|"no row"| silent
    crediting -->|"one or more rows"| count
    count --> named
```

The left rows reach no count: the edge into `carried` is the only one that adds to a class's
examined count, and it is the one the base already had. A row's reason is `its changed line is in
<path>` when the plan records an overlap path for it, and `it covers no changed line` when it does
not.
