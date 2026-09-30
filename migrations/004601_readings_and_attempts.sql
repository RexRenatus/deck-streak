-- SPEC-046 R8, R9, R10, R12, R15: the readings the generation stores, and every attempt it made,
-- owned by readings (docs/CONTEXT-MAP.md). The predecessor's `preread_readings` maps onto
-- `readings`, and its attempt telemetry onto `reading_attempts`.
--
-- `readings` holds one row per stored reading: its id (the first 32 hex digits of the SHA-256 of
-- the topic, the first study day it was generated and the day set's digest, R9), the persona that
-- wrote it, its text, its word count and minutes, the new-card and note counts, the covered card
-- ids as a JSON array, when it was generated and the record's version. The vault copy's path is
-- kept when the write succeeded; a failed write keeps the reading and records `failed` (R9). A
-- night the day set did not change carries the reading and raises `carried_nights` (R10).
--
-- `reading_attempts` holds one row per model attempt (R12): the run, the topic and study day, the
-- attempt's number (the first, or the one repair), the gate a repair named, the verdict, the cause
-- of an unavailable attempt or the class a gate failure carried, and what the run measured. It is
-- kept ninety days, exported and erased. Instants are epoch milliseconds.
CREATE TABLE readings (
    id TEXT PRIMARY KEY CHECK (length(id) = 32 AND id NOT GLOB '*[^0-9a-f]*'),
    topic TEXT NOT NULL CHECK (topic GLOB 'law/?*' OR topic GLOB 'language/?*'),
    study_day INTEGER NOT NULL,
    digest TEXT NOT NULL CHECK (length(digest) = 64 AND digest NOT GLOB '*[^0-9a-f]*'),
    persona TEXT NOT NULL CHECK (length(persona) > 0),
    text TEXT NOT NULL CHECK (length(text) > 0),
    word_count INTEGER NOT NULL CHECK (word_count > 0),
    reading_minutes INTEGER NOT NULL CHECK (reading_minutes > 0),
    new_cards INTEGER NOT NULL CHECK (new_cards > 0),
    note_count INTEGER NOT NULL CHECK (note_count > 0),
    card_ids TEXT NOT NULL CHECK (
        json_valid(card_ids) AND json_type(card_ids) = 'array'
        AND json_array_length(card_ids) = new_cards
    ),
    generated_at INTEGER NOT NULL,
    version INTEGER NOT NULL CHECK (version = 1),
    vault_status TEXT NOT NULL CHECK (vault_status IN ('written', 'vault_write_failed')),
    vault_path TEXT CHECK (vault_path IS NULL OR length(vault_path) > 0),
    carried_nights INTEGER NOT NULL DEFAULT 0 CHECK (carried_nights >= 0),
    created_at INTEGER NOT NULL,
    CHECK ((vault_status = 'written') = (vault_path IS NOT NULL))
) STRICT;

CREATE INDEX readings_by_topic ON readings (topic, generated_at);

CREATE TABLE reading_attempts (
    id INTEGER PRIMARY KEY,
    run_id INTEGER NOT NULL REFERENCES reading_runs (id),
    topic TEXT NOT NULL CHECK (topic GLOB 'law/?*' OR topic GLOB 'language/?*'),
    study_day INTEGER NOT NULL,
    attempt INTEGER NOT NULL CHECK (attempt IN (1, 2)),
    repair_gate TEXT CHECK (
        repair_gate IN ('complete', 'roster', 'anchors', 'band', 'no_list_markers', 'contract')
    ),
    verdict TEXT NOT NULL CHECK (verdict IN ('passed', 'gate_failed', 'unavailable')),
    cause TEXT CHECK (
        cause IN (
            'key_missing',
            'refused_shape',
            'key_rejected',
            'capacity_exhausted',
            'proxy_unreachable',
            'run_failed',
            'turn_cap',
            'time_cap',
            'budget_cap'
        )
    ),
    class TEXT CHECK (class IS NULL OR length(class) > 0),
    gate TEXT CHECK (
        gate IN ('complete', 'roster', 'anchors', 'band', 'no_list_markers', 'contract')
    ),
    turns INTEGER NOT NULL CHECK (turns >= 0),
    input_tokens INTEGER NOT NULL CHECK (input_tokens >= 0),
    output_tokens INTEGER NOT NULL CHECK (output_tokens >= 0),
    cost_micro_usd INTEGER NOT NULL CHECK (cost_micro_usd >= 0),
    duration_ms INTEGER NOT NULL CHECK (duration_ms >= 0),
    created_at INTEGER NOT NULL,
    CHECK ((attempt = 2) = (repair_gate IS NOT NULL)),
    CHECK ((verdict = 'unavailable') = (cause IS NOT NULL)),
    CHECK ((verdict = 'gate_failed') = (gate IS NOT NULL))
) STRICT;

CREATE INDEX reading_attempts_by_run ON reading_attempts (run_id);
CREATE INDEX reading_attempts_by_created ON reading_attempts (created_at);
