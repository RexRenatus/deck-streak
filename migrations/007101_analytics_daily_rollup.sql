-- SPEC-071 R5, R8 to R11, R14, R16 and R18: each study day's rollup and its per-course statistics,
-- owned by analytics (docs/CONTEXT-MAP.md). The predecessor's `daily_rollup` and
-- `daily_lang_stats` map onto these two tables, less the day's XP and its track, which progression
-- and the courses now hold.
--
-- `daily_rollup` holds one row per study day, keyed by the kernel's study day (an epoch day
-- number): the day's metrics (R6), its card state with the provenance of that state (R8, R9),
-- the score and its five pillars (R12, R14), the score the day closed with (R14), the instant the
-- day was settled (R16) and the day's review fingerprint (R18). A card state never recorded is
-- NULL, never 0, and its provenance with it; a recorded one reads `live:<epoch milliseconds>`.
-- `updated_at` moves only when the day is rolled up again, so a late review shows in it.
--
-- `daily_lang_stats` holds one row per study day and course code: the reviews, seconds, answered
-- and passed of that course's cards that day (R11). Instants are epoch milliseconds. Both tables
-- are exported and erased (R23).
CREATE TABLE daily_rollup (
    study_day INTEGER PRIMARY KEY,
    reviews INTEGER NOT NULL CHECK (reviews >= 0),
    learn_count INTEGER NOT NULL CHECK (learn_count >= 0),
    review_count INTEGER NOT NULL CHECK (review_count >= 0),
    relearn_count INTEGER NOT NULL CHECK (relearn_count >= 0),
    filtered_count INTEGER NOT NULL CHECK (filtered_count >= 0),
    seconds REAL NOT NULL CHECK (seconds >= 0),
    answered INTEGER NOT NULL CHECK (answered >= 0),
    passed INTEGER NOT NULL CHECK (passed >= 0 AND passed <= answered),
    true_retention REAL NOT NULL CHECK (true_retention >= 0 AND true_retention <= 100),
    graduations INTEGER NOT NULL CHECK (graduations >= 0),
    decks_studied INTEGER NOT NULL CHECK (decks_studied >= 0),
    avg_answer_seconds REAL NOT NULL CHECK (avg_answer_seconds >= 0),
    young_answered INTEGER NOT NULL CHECK (young_answered >= 0),
    young_passed INTEGER NOT NULL CHECK (young_passed >= 0),
    mature_answered INTEGER NOT NULL CHECK (mature_answered >= 0),
    mature_passed INTEGER NOT NULL CHECK (mature_passed >= 0),
    mature_count INTEGER CHECK (mature_count >= 0),
    young_count INTEGER CHECK (young_count >= 0),
    leech_active INTEGER CHECK (leech_active >= 0),
    backlog INTEGER CHECK (backlog >= 0),
    due_today INTEGER CHECK (due_today >= 0),
    card_state_src TEXT CHECK (card_state_src GLOB 'live:[0-9]*'),
    score INTEGER NOT NULL CHECK (score BETWEEN 0 AND 100),
    consistency REAL NOT NULL,
    retention REAL NOT NULL,
    workload REAL NOT NULL,
    volume REAL NOT NULL,
    mastery REAL NOT NULL,
    score_at_close INTEGER CHECK (score_at_close BETWEEN 0 AND 100),
    settled_at INTEGER,
    fingerprint TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    CHECK (
        (card_state_src IS NULL) = (mature_count IS NULL)
        AND (card_state_src IS NULL) = (young_count IS NULL)
        AND (card_state_src IS NULL) = (leech_active IS NULL)
        AND (card_state_src IS NULL) = (backlog IS NULL)
        AND (card_state_src IS NULL) = (due_today IS NULL)
    ),
    CHECK (score_at_close IS NULL OR settled_at IS NOT NULL)
) STRICT;

CREATE TABLE daily_lang_stats (
    study_day INTEGER NOT NULL,
    course TEXT NOT NULL,
    reviews INTEGER NOT NULL CHECK (reviews > 0),
    seconds REAL NOT NULL CHECK (seconds >= 0),
    answered INTEGER NOT NULL CHECK (answered >= 0),
    passed INTEGER NOT NULL CHECK (passed >= 0 AND passed <= answered),
    created_at INTEGER NOT NULL,
    PRIMARY KEY (study_day, course)
) STRICT;
