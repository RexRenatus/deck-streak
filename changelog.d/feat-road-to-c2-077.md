### Added

- The Road to C2, in the curriculum crate (SPEC-077, part one of two): each card's memory state,
  parsed from its data in the ingest crate, and its mastery; each course's progress across its unit
  bands from A1 to C2, with its current band and current unit; the law mastery pillar; and the
  curriculum constants, each equal to the predecessor's.
- The band-up, in the recompute's progress step: it stores each course's progress on the current
  study day, records a course seen for the first time as a silent baseline, and pays a band later
  than the stored one its XP, its badge and its celebration once, through the one router.
- The law block's view: its fields equal to the predecessor's, the law dues pending before the
  first recompute, the leeches and the law mastery pillar pending until their port is wired, and
  the block omitted when there is no law activity.
- The next milestone reads the courses' stored mature cards.
- The `language_progress`, `band_milestones` and `law_dues` tables, exported and erased with the
  learner's data.
- A TLA+ model that a band-up is paid and celebrated once across recomputes that interleave or
  crash, and a Lean proof of the band rules and the law pillar's range, checked against recorded
  vectors.
