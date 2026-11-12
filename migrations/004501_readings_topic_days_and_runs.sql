-- SPEC-045 R5, R6, R8, R9, R12: the readings' record of each study day, owned by readings
-- (docs/CONTEXT-MAP.md). The predecessor's `preread_runs` maps onto these two tables.
--
-- `reading_runs` holds one row per resolution: what triggered it, the study day it resolved (the
-- kernel's rule, as an epoch day), when it started and finished, its outcome, and for a run refused
-- as a whole its class and its reason. `unmapped_decks` counts the decks with new cards that map to
-- no topic (R9); their names are never stored.
--
-- `reading_topic_days` holds one row per study day and topic: the state from R5's closed set, the
-- class and reason a could-not-tell state carries (a failed state carries its reason alone), and
-- for a topic with a day set its digest, its card ids and note ids as JSON arrays, and its count of
-- new cards. A later run the same day replaces the row's state and keeps its `created_at`.
--
-- R6's closed map of reasons to classes is held here too, so no row pairs a reason with the other
-- class. Instants are epoch milliseconds. Both tables are exported and erased (R8).
CREATE TABLE reading_runs (
    id INTEGER PRIMARY KEY,
    trigger TEXT NOT NULL CHECK (trigger IN ('scheduled', 'owner')),
    study_day INTEGER NOT NULL,
    started_at INTEGER NOT NULL,
    finished_at INTEGER NOT NULL CHECK (finished_at >= started_at),
    outcome TEXT NOT NULL CHECK (
        outcome IN ('resolved', 'paused', 'could_not_tell', 'ai_route_absent')
    ),
    class TEXT CHECK (class IN ('rail_broken', 'config_fault')),
    reason TEXT CHECK (
        reason IN (
            'sync_failed',
            'collection_locked',
            'collection_open_failed',
            'day_set_resolve_timeout',
            'taxonomy_missing',
            'day_set_fetch_saturated'
        )
    ),
    unmapped_decks INTEGER NOT NULL CHECK (unmapped_decks >= 0),
    created_at INTEGER NOT NULL,
    CHECK ((outcome = 'could_not_tell') = (reason IS NOT NULL)),
    CHECK ((class IS NULL) = (reason IS NULL)),
    CHECK (
        class IS NULL
        OR (class = 'config_fault') = (reason IN ('taxonomy_missing', 'day_set_fetch_saturated'))
    )
) STRICT;

CREATE TABLE reading_topic_days (
    id INTEGER PRIMARY KEY,
    run_id INTEGER NOT NULL REFERENCES reading_runs (id),
    study_day INTEGER NOT NULL,
    topic TEXT NOT NULL CHECK (topic GLOB 'law/?*' OR topic GLOB 'language/?*'),
    state TEXT NOT NULL CHECK (
        state IN ('ready', 'no_new_cards', 'could_not_tell', 'paused', 'failed', 'ai_route_absent')
    ),
    class TEXT CHECK (class IN ('rail_broken', 'config_fault')),
    reason TEXT CHECK (
        reason IN (
            'sync_failed',
            'collection_locked',
            'collection_open_failed',
            'day_set_resolve_timeout',
            'taxonomy_missing',
            'day_set_fetch_saturated',
            'form_unregistered',
            'seed_empty',
            'anchor_unusable_all',
            'gate_failed:complete',
            'gate_failed:roster',
            'gate_failed:anchors',
            'gate_failed:band',
            'gate_failed:no_list_markers',
            'gate_failed:contract',
            'agent_unavailable:key_missing',
            'agent_unavailable:refused_shape',
            'agent_unavailable:key_rejected',
            'agent_unavailable:capacity_exhausted',
            'agent_unavailable:proxy_unreachable',
            'agent_unavailable:run_failed',
            'agent_unavailable:turn_cap',
            'agent_unavailable:time_cap',
            'agent_unavailable:budget_cap'
        )
    ),
    digest TEXT CHECK (digest IS NULL OR (length(digest) = 64 AND digest NOT GLOB '*[^0-9a-f]*')),
    card_ids TEXT NOT NULL CHECK (json_valid(card_ids) AND json_type(card_ids) = 'array'),
    note_ids TEXT NOT NULL CHECK (json_valid(note_ids) AND json_type(note_ids) = 'array'),
    new_cards INTEGER NOT NULL CHECK (new_cards >= 0),
    created_at INTEGER NOT NULL,
    UNIQUE (study_day, topic),
    CHECK ((state = 'could_not_tell') = (class IS NOT NULL)),
    CHECK ((state IN ('could_not_tell', 'failed')) = (reason IS NOT NULL)),
    CHECK (
        (state = 'could_not_tell') = (
            reason IN (
                'sync_failed',
                'collection_locked',
                'collection_open_failed',
                'day_set_resolve_timeout',
                'taxonomy_missing',
                'day_set_fetch_saturated'
            )
        )
    ),
    CHECK (
        class IS NULL
        OR (class = 'config_fault') = (reason IN ('taxonomy_missing', 'day_set_fetch_saturated'))
    ),
    CHECK (json_array_length(card_ids) = new_cards),
    CHECK ((digest IS NULL) = (new_cards = 0))
) STRICT;

CREATE INDEX reading_topic_days_by_run ON reading_topic_days (run_id);
