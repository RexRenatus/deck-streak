-- SPEC-047 R1, R4, R5, R8, R9: what the owner's tap and the settle step record on a stored
-- reading, owned by readings (docs/CONTEXT-MAP.md). No new table: the columns join the reading
-- they describe, so the export and the erase that already cover `readings` cover them too.
--
-- `read_at` is when the owner first tapped the reading, unset until then, and it never moves
-- (R1). `studied_count` is how many of its covered cards the last settle found studied, and
-- `studied_verdict` where the measure stands: `open` while its window runs, `studied` once the
-- count crossed the majority, `retired` when the window closed below it, and a retired reading
-- can still turn `studied` on late reviews made inside its window (R5). `studied_at` is the
-- instant it turned studied. `vault_tick` records whether the tap's `I read it` line was written
-- in the vault: `none` before a tap, `written`, or `pending` when the tick failed and the next
-- tap retries it (R8). Instants are epoch milliseconds.
ALTER TABLE readings ADD COLUMN read_at INTEGER;
ALTER TABLE readings ADD COLUMN studied_count INTEGER NOT NULL DEFAULT 0
    CHECK (studied_count >= 0);
ALTER TABLE readings ADD COLUMN studied_verdict TEXT NOT NULL DEFAULT 'open'
    CHECK (studied_verdict IN ('open', 'studied', 'retired'));
ALTER TABLE readings ADD COLUMN studied_at INTEGER;
ALTER TABLE readings ADD COLUMN vault_tick TEXT NOT NULL DEFAULT 'none'
    CHECK (vault_tick IN ('none', 'written', 'pending'));
