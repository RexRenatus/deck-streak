-- SPEC-071 R4 (ADR-087): the digest of the owner's courses file, recorded with the settings
-- generation, owned by the kernel (docs/CONTEXT-MAP.md). At start, a digest that differs from the
-- recorded one bumps the generation in the same write, so the next cycle's change gate recomputes
-- (SPEC-023). NULL while no courses file was ever loaded. An erase clears it with the generation.
-- Additive: no row of the table changes.
ALTER TABLE settings_generation ADD COLUMN courses_digest TEXT;
