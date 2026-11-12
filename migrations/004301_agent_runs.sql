-- SPEC-043 R11, R12, R16: one row per duty run, owned by the agent context (docs/CONTEXT-MAP.md).
--
-- `verdict` is how the run ended. `cause` is set for an unavailable run alone, from the closed set
-- the runner and the caps name; `class` is set for a withheld run alone (never the content). A run
-- with no AI route configured is `ai_route_absent`: nothing ran, so it carries no telemetry.
-- Instants are epoch milliseconds and cost is millionths of a US dollar. The table is exported and
-- erased (R13), and it is retained 90 days (privacy.json).
CREATE TABLE agent_runs (
    id INTEGER PRIMARY KEY,
    duty TEXT NOT NULL CHECK (length(duty) > 0),
    template TEXT NOT NULL,
    subject TEXT NOT NULL,
    verdict TEXT NOT NULL CHECK (
        verdict IN ('delivered', 'withheld', 'unavailable', 'ai_route_absent')
    ),
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
    class TEXT CHECK (length(class) > 0),
    turns INTEGER CHECK (turns >= 0),
    input_tokens INTEGER CHECK (input_tokens >= 0),
    output_tokens INTEGER CHECK (output_tokens >= 0),
    cost_micro_usd INTEGER CHECK (cost_micro_usd >= 0),
    duration_ms INTEGER CHECK (duration_ms >= 0),
    created_at INTEGER NOT NULL,
    CHECK ((verdict = 'unavailable') = (cause IS NOT NULL)),
    CHECK ((verdict = 'withheld') = (class IS NOT NULL)),
    CHECK (
        (verdict = 'ai_route_absent') = (
            turns IS NULL AND input_tokens IS NULL AND output_tokens IS NULL
            AND cost_micro_usd IS NULL AND duration_ms IS NULL
        )
    )
) STRICT;
