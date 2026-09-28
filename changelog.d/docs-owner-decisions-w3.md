### Changed

- The skip day writes its reschedule back to Anki, by the owner's decision (ADR-089). ADR-037's
  no-upload condition is superseded for the skip-day path only, and every other path keeps the
  proof that it records zero uploads. CHARTER constraint 4 stands as written.
  - SPEC-083 specifies the write and its exact undo, with an acceptance criterion for each of the
    owner's guardrails: incremental syncs only, with any full-sync demand aborting; the owner's
    confirm only; the cards previewed before the write; their prior state recorded first; and an
    undo that never overwrites a card changed since. Each is proven against the recording fake sync
    server, and a control first proves that the recorder sees a planted upload.
  - ADR-083 records the option the owner took and how the write is made, on a working copy that is
    discarded after each push. ADR-037, SPEC-001 and SPEC-022 gain insert-only amendments, and the
    skip day's schematics show the write and the undo's compare.
- Four features the predecessor left inert are revived as feature issues: the smoke-bomb spend
  (#279) and the skip-day switch with its monthly bridge cap (#280) in W5, the collection atlas as a
  data series the agent reads (#282) in W6, and the per-source XP re-pricing (#281) in W7.
