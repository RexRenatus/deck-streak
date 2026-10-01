# The streak calendar: markers derived on read

`GET /api/streak` serves `calendar` beside the counts (SPEC-076 section 27, ADR-302).

```mermaid
flowchart LR
  F[recompute fold] -->|settles review XP| X[(xp_settlement)]
  F -->|writes| S[(streak_state)]
  R[GET /api/streak] -->|ONE read transaction| X
  R --> S
  X -->|days with amount above nothing, per track| C[calendar::language / calendar::law]
  C -->|CALENDAR_DAYS days ending at the served day| R
  R -->|calendar.language, calendar.law| W[streak screen]
```

The walk (`replay::walk`, shared with `replay::language`) visits each day from the first study day
to the served day. A transition that spent a freeze puts `freeze` on the one non-skip day between the
last study day and the return day; a transition that broke the run puts `break` on its own day; each
skip day carries `skip`. The window then keeps the last `CALENDAR_DAYS` days.
