-- SPEC-043 (amendment of 2026-09-29), issue 363: the daily prune of `agent_runs` deletes every run
-- older than its retention with `DELETE FROM agent_runs WHERE created_at < ?1`. Without an index on
-- the column that range reads, each daily prune scans the whole table; with it the delete seeks
-- the old rows alone. The migration adds no table and no column.
CREATE INDEX agent_runs_by_created_at ON agent_runs (created_at);
