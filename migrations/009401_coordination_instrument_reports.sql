-- SPEC-094 R9 and ADR-094: the latest report of each instrument, owned by coordination
-- (docs/CONTEXT-MAP.md). One row per instrument: the instrument's id is the primary key, so a
-- second report replaces the first in one write and the table never holds two of an instrument.
--
-- `study_day` is the study day the run belongs to (an epoch day number) and `schema_version` the
-- shape of the report's JSON; `report_json` is the run's envelope: the report, or null when the run
-- itself failed, and the name of each read that failed. Instants are epoch milliseconds. The table
-- is exported and erased (R9).
CREATE TABLE instrument_reports (
    instrument TEXT PRIMARY KEY,
    study_day INTEGER NOT NULL,
    schema_version INTEGER NOT NULL CHECK (schema_version >= 1),
    report_json TEXT NOT NULL,
    created_at INTEGER NOT NULL
) STRICT;
