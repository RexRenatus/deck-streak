//! Export and erase are symmetric over the real registry of every context's data-rights port: the
//! tables the export carries whole are the tables the erase clears or resets; every table of the
//! migrated schema is declared by exactly one port, the one the ownership register names; the
//! exempt tables survive an erase untouched; and `privacy.json` names every table a port exports
//! or erases in exactly one category (SPEC-021 A1, A2, A4, A8; R1, R4, R5; CHARTER 13).
//!
//! The probe is the predecessor's completeness test's (`test_privacy_completeness.py` at
//! `27ee2bc`): it seeds every table of a fully migrated temporary database with rows no erase
//! leaves, reads every table, exports, erases, and reads every table again. A table is exported
//! when the export carries every one of its rows, and erased when none of its rows survives the
//! erase unchanged. Every row here is synthetic.

// An integration test is test code: its helpers panic on a failed fixture, and it prints the
// examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use deck_streak_coordination::data_rights::CRON_FIRES_TABLE;
use deck_streak_coordination::data_rights_registry::{erase_all, export_all, ports};
use deck_streak_coordination::jobs::FireDate;
use deck_streak_coordination::ledger::{CronLedger, SqliteCronLedger};
use deck_streak_ingest::data_rights::WRITE_CLASS_STOP_TABLE;
use deck_streak_kernel::data_rights::SCHEMA_VERSION_TABLE;
use deck_streak_kernel::{Db, Declaration, Disposition, UtcMillis};
use deck_streak_privacy::{Erasure, SCHEMA_KEY};
use serde_json::{Map, Value};
use sqlx::AssertSqlSafe;
use tempfile::TempDir;

/// Statements that leave every table of the schema holding rows no erase leaves: 101 rows in each
/// table that takes rows, so an export that pages or limits its read comes up short (the
/// predecessor's lesson), and every column a reset writes moved off its reset value.
const SEEDS: [&str; 45] = [
    "UPDATE settings_generation SET generation = 7, courses_digest = '0123456789abcdef' \
     WHERE id = 1",
    "UPDATE ingest_state SET anchor_newest_review_id = 1700000000123, anchor_card_count = 57, \
     anchor_card_fingerprint = 9001, anchor_study_day = 20000, \
     anchor_recomputed_at = 1700000000456, anchor_settings_generation = 7, rescore_pending = 1, \
     refused_at = 1700000000789, refused_reason = 'recompute_failed', \
     window_floor = 1690000000000, window_count = 12 WHERE id = 1",
    "UPDATE owner_last_message SET message_id = 4242, arrived_at = 1700000000789 WHERE id = 1",
    "UPDATE write_class_stop SET stopped = 1, set_by = 'counts', reason = 'review_log_rows', \
     set_at = 1700000000999 WHERE id = 1",
    "WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 101) \
     INSERT INTO sync_runs (trigger, study_day, started_at, finished_at, status, reason, \
     attempts, full_download, created_at) \
     SELECT CASE i % 2 WHEN 0 THEN 'owner' ELSE 'scheduled' END, 20000 + i, 1000 * i, \
     1000 * i + 500, CASE i % 3 WHEN 0 THEN 'error' ELSE 'ok' END, \
     CASE i % 3 WHEN 0 THEN 'server_error' ELSE NULL END, i % 4, i % 2, 1000 * i FROM n",
    "WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 101) \
     INSERT INTO skip_days (study_day, due_count, state, reason, cards_moved, tariff_unfunded, \
     undone, undone_at, created_at) \
     SELECT 20000 + i, i, CASE i % 3 WHEN 0 THEN 'failed' ELSE 'applied' END, \
     CASE i % 3 WHEN 0 THEN 'push_failed' ELSE NULL END, i, i % 2, \
     CASE WHEN i % 3 = 1 THEN 1 ELSE 0 END, CASE WHEN i % 3 = 1 THEN 1000 * i + 7 END, \
     1000 * i FROM n",
    "WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 101) \
     INSERT INTO skip_card_snapshot (skip_id, card_id, prior_due, prior_queue, prior_type, \
     prior_interval, prior_ease_factor, prior_original_deck_id, prior_original_due, left_due, \
     left_queue, left_type, left_interval, left_ease_factor, left_mtime, created_at) \
     SELECT i, 1700000000000 + i, i, 2, 2, i, 2500, 0, 0, i + 1, 2, 2, i, 2500, 1000 * i, \
     1000 * i FROM n",
    "WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 101) \
     INSERT INTO cron_fires (job_id, fire_date, first_seen_at, updated_at, last_fire_at, \
     ok_count, error_count, catchup_count, missed_count, last_outcome, created_at) \
     SELECT 'synthetic_daily', 20000 + i, 1000 * i, 1000 * i + 1, 1000 * i + 1, 1, 0, 1, 0, \
     'ok', 1000 * i FROM n",
    "WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 101) \
     INSERT INTO xp_ledger (study_day, source, track, amount, scope, created_at) \
     SELECT 20000 + i, 'synthetic:grant:' || i, CASE i % 2 WHEN 0 THEN 'language' ELSE 'law' END, \
     i, CASE i % 3 WHEN 0 THEN 'once' ELSE 'per-day' END, 1000 * i FROM n",
    "WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 101) \
     INSERT INTO reading_runs (trigger, study_day, started_at, finished_at, outcome, class, \
     reason, unmapped_decks, created_at) \
     SELECT CASE i % 2 WHEN 0 THEN 'owner' ELSE 'scheduled' END, 20000 + i, 1000 * i, \
     1000 * i + 500, CASE i % 3 WHEN 0 THEN 'could_not_tell' WHEN 1 THEN 'resolved' \
     ELSE 'paused' END, CASE i % 3 WHEN 0 THEN 'rail_broken' ELSE NULL END, \
     CASE i % 3 WHEN 0 THEN 'sync_failed' ELSE NULL END, i % 4, 1000 * i + 500 FROM n",
    "WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 101) \
     INSERT INTO reading_topic_days (run_id, study_day, topic, state, class, reason, digest, \
     card_ids, note_ids, new_cards, created_at) \
     SELECT i, 20000 + i, 'law/synthetic-' || i, CASE i % 2 WHEN 0 THEN 'no_new_cards' \
     ELSE 'could_not_tell' END, CASE i % 2 WHEN 0 THEN NULL ELSE 'config_fault' END, \
     CASE i % 2 WHEN 0 THEN NULL ELSE 'day_set_fetch_saturated' END, NULL, '[]', '[]', 0, \
     1000 * i + 500 FROM n",
    "WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 101) \
     INSERT INTO agent_runs (duty, template, subject, verdict, cause, class, turns, \
     input_tokens, output_tokens, cost_micro_usd, duration_ms, created_at) \
     SELECT 'synthetic-duty', 'synthetic-template', 'law/synthetic-' || i, \
     CASE i % 3 WHEN 0 THEN 'unavailable' WHEN 1 THEN 'delivered' ELSE 'withheld' END, \
     CASE i % 3 WHEN 0 THEN 'turn_cap' ELSE NULL END, \
     CASE i % 3 WHEN 2 THEN 'output-links' ELSE NULL END, \
     i % 30, 10 * i, 20 * i, 1000 * i, 100 * i, 1000 * i FROM n",
    "WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 101) \
     INSERT INTO daily_rollup (study_day, reviews, learn_count, review_count, relearn_count, \
     filtered_count, seconds, answered, passed, true_retention, graduations, decks_studied, \
     avg_answer_seconds, young_answered, young_passed, mature_answered, mature_passed, \
     mature_count, young_count, leech_active, backlog, due_today, card_state_src, score, \
     consistency, retention, workload, volume, mastery, score_at_close, settled_at, fingerprint, \
     created_at, updated_at) \
     SELECT 20000 + i, i, 1, i - 1, 0, 0, 7.5 * i, 1, 1, 100.0, 0, 1, 7.5, 1, 1, 0, 0, \
     CASE i % 2 WHEN 0 THEN i ELSE NULL END, CASE i % 2 WHEN 0 THEN 2 ELSE NULL END, \
     CASE i % 2 WHEN 0 THEN 1 ELSE NULL END, CASE i % 2 WHEN 0 THEN 3 ELSE NULL END, \
     CASE i % 2 WHEN 0 THEN 4 ELSE NULL END, \
     CASE i % 2 WHEN 0 THEN 'live:' || (1000 * i) ELSE NULL END, i % 101, 70.0, 100.0, 0.0, \
     40.0, 0.0, CASE i % 3 WHEN 0 THEN i % 101 ELSE NULL END, \
     CASE i % 3 WHEN 0 THEN 1000 * i ELSE NULL END, 'synthetic-' || i, 1000 * i, 1000 * i FROM n",
    "WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 101) \
     INSERT INTO daily_lang_stats (study_day, course, reviews, seconds, answered, passed, \
     created_at) \
     SELECT 20000 + i, CASE i % 2 WHEN 0 THEN 'qaa' ELSE 'qab' END, i, 7.5 * i, 1, 1, 1000 * i \
     FROM n",
    "WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 101) \
     INSERT INTO notification_settings (key, value, created_at) \
     SELECT 'synthetic_setting_' || i, CASE i % 2 WHEN 0 THEN '0' ELSE '1' END, 1000 * i FROM n",
    "WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 101) \
     INSERT INTO notification_deliveries (kind, dedupe_key, scope, surface, study_day, lapse_id, \
     created_at) \
     SELECT CASE i % 2 WHEN 0 THEN 'celebration' ELSE 'comeback' END, 'synthetic:' || i, \
     CASE i % 2 WHEN 0 THEN '' ELSE 'lapse:19990:day:' || (20000 + i) END, \
     CASE i % 3 WHEN 0 THEN 'mini-app' ELSE 'bot' END, 20000 + i, \
     CASE i % 2 WHEN 0 THEN NULL ELSE 19990 END, 1000 * i FROM n",
    "WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 101) \
     INSERT INTO notification_decisions (dedupe_key, kind, surface, arm, reason, tier_requested, \
     tier_rendered, study_day, created_at) \
     SELECT 'synthetic:' || i, CASE i % 3 WHEN 2 THEN 'habit:withheld' ELSE 'celebration' END, \
     'bot', CASE i % 3 WHEN 0 THEN 'send' WHEN 1 THEN 'defer' ELSE 'withhold' END, \
     CASE i % 3 WHEN 0 THEN NULL WHEN 1 THEN 'quiet' ELSE 'quiet_hours' END, 'T2', \
     CASE i % 3 WHEN 0 THEN 'T2' ELSE 'T0' END, 20000 + i, 1000 * i FROM n",
    "WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 101) \
     INSERT INTO notification_queue (kind, dedupe_key, surface, tier_requested, tier_pending, text, \
     hold, tries, state, deferred_at, study_day, created_at) \
     SELECT 'celebration', 'synthetic:' || i, 'bot', 'T4', 'T2', 'synthetic ' || i, \
     CASE i % 2 WHEN 0 THEN 'quiet' ELSE 'send' END, i % 3, \
     CASE i % 5 WHEN 0 THEN 'abandoned' ELSE 'held' END, 1000 * i, 20000 + i, 1000 * i FROM n",
    "WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 101) \
     INSERT INTO in_app_feed (dedupe_key, kind, tier, text, seen_at, created_at) \
     SELECT 'synthetic:' || i, 'celebration', 'T2', 'synthetic ' || i, \
     CASE i % 2 WHEN 0 THEN NULL ELSE 1000 * i + 1 END, 1000 * i FROM n",
    "WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 101) \
     INSERT INTO instrument_reports (instrument, study_day, schema_version, report_json, \
     created_at) \
     SELECT 'synthetic_' || i, 20000 + i, 1 + i % 3, '{\"report\":null}', 1000 * i FROM n",
    "WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 101) \
     INSERT INTO xp_settlement (study_day, source, track, amount, closed, created_at) \
     SELECT 20000 + i, 'reviews', CASE i % 2 WHEN 0 THEN 'language' ELSE 'law' END, \
     i, i % 2, 1000 * i FROM n",
    "WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 101) \
     INSERT INTO buffs (study_day, kind, created_at) SELECT 20000 + i, 'ascendant', 1000 * i FROM n",
    "WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 101) \
     INSERT INTO badges_earned (badge_key, tier, name, emoji, study_day, celebrated_at, created_at) \
     SELECT 'badge_' || i, 0, 'Badge ' || i, 'x', 20000 + i, NULL, 1000 * i FROM n",
    "INSERT INTO records (kind, value, study_day, previous, celebrated_at, created_at) VALUES \
     ('best_score', 80, 20100, 70, NULL, 1000), ('most_reviews', 300, 20100, 200, 5000, 2000), \
     ('most_minutes', 90, 20100, 60, NULL, 3000)",
    "INSERT INTO streak_state (track, current_days, longest_days, freezes, last_study_day, \
     comeback_armed, created_at) VALUES ('language', 9, 12, 2, 20100, 1, 1000), \
     ('law', 4, 6, 0, 20100, 0, 2000)",
    "WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 101) \
     INSERT INTO freeze_events (study_day, delta, reason, created_at) \
     SELECT 20000 + i, CASE i % 2 WHEN 0 THEN -1 ELSE 1 END, \
     CASE i % 3 WHEN 0 THEN 'consumed' WHEN 1 THEN 'streak_earn' ELSE 'shop' END, 1000 * i FROM n",
    "WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 101) \
     INSERT INTO habit_strength (study_day, strength, created_at) \
     SELECT 20000 + i, i / 200.0, 1000 * i FROM n",
    "WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 101) \
     INSERT INTO relight_due (study_day, created_at) SELECT 20000 + i, 1000 * i FROM n",
    "UPDATE governor_state SET lapse_since = 19990, standby = 1, notified_day = 19995 WHERE id = 1",
    "WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 101) \
     INSERT INTO coin_ledger (study_day, source, reference, delta, created_at) \
     SELECT 20000 + i, CASE i % 3 WHEN 0 THEN 'mint' WHEN 1 THEN 'shop' ELSE 'fine' END, \
     CASE i % 3 WHEN 0 THEN '' ELSE 'synthetic:' || i END, \
     CASE i % 3 WHEN 0 THEN i % 41 ELSE -(i % 7) END, 1000 * i FROM n",
    "WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 101) \
     INSERT INTO chests (study_day, origin, session_start, rarity, payout_xp, state, choice, \
     announced, created_at) \
     SELECT 20000 + i, CASE i % 3 WHEN 0 THEN 'weekly' WHEN 1 THEN 'session' ELSE 'challenge' END, \
     CASE i % 3 WHEN 1 THEN 1000 * i ELSE 0 END, \
     CASE i % 4 WHEN 0 THEN 'legendary' WHEN 1 THEN 'common' WHEN 2 THEN 'rare' ELSE 'epic' END, \
     i, CASE i % 4 WHEN 0 THEN 'resolved' WHEN 1 THEN 'sealed' WHEN 2 THEN 'opened' \
     ELSE 'vaulted' END, CASE i % 3 WHEN 0 THEN 'token' ELSE '' END, i % 2, 1000 * i FROM n",
    "UPDATE pity SET since_epic = 4, since_legendary = 9 WHERE id = 1",
    "WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 101) \
     INSERT INTO xp_tokens (chest_id, granted_at, activated_at, window_ends_at, consumed, \
     created_at) \
     SELECT i, 1000 * i, CASE i % 2 WHEN 0 THEN 1000 * i + 1 ELSE 0 END, \
     CASE i % 2 WHEN 0 THEN 1000 * i + 7200001 ELSE 0 END, i % 2, 1000 * i FROM n",
    "UPDATE chest_settings SET per_day_max = 5, vault_hour = 18 WHERE id = 1",
    "UPDATE economy_state SET pass_ends_at = 1700000001800, surcharge_ends_at = 1700000172800 \
     WHERE id = 1",
    "WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 101) \
     INSERT INTO drill_answers (drill_id, study_day, surface, created_at) \
     SELECT 'synthetic-drill-' || i, 20000 + i, CASE i % 2 WHEN 0 THEN 'bot' ELSE 'mini_app' END, \
     1000 * i FROM n",
    "WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 101) \
     INSERT INTO drill_grades (drill_id, drill_type, subject, xp, study_day, created_at) \
     SELECT 'synthetic-drill-' || i, 'irac', 'synthetic subject ' || i, 10 + i % 16, 20000 + i, \
     1000 * i FROM n",
    "WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 101) \
     INSERT INTO inbox_captures (stem, capture_key, kind, source, attachment, captured_at, state, \
     destination, filed_day, created_at) \
     SELECT '2025-01-01-photo-synthetic' || i, 'synthetic' || i, \
     CASE i % 2 WHEN 0 THEN 'photo' ELSE 'text' END, \
     CASE i % 2 WHEN 0 THEN 'telegram' ELSE 'miniapp' END, \
     CASE i % 2 WHEN 0 THEN '2025-01-01-photo-synthetic' || i || '.jpg' ELSE NULL END, \
     1000 * i, CASE i % 3 WHEN 0 THEN 'filed' ELSE 'captured' END, \
     CASE i % 3 WHEN 0 THEN 'Synthetic folder' ELSE NULL END, \
     CASE i % 3 WHEN 0 THEN 20000 + i ELSE NULL END, 1000 * i FROM n",
    "WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 101) \
     INSERT INTO language_progress (course, name, flag, mastery_pct, current_band, mature_cards, \
     total_cards, current_unit, bands, updated_at, created_at) \
     SELECT 'c' || i, 'Course ' || i, 'f', i / 2.0, \
     CASE i % 6 WHEN 0 THEN 'A1' WHEN 1 THEN 'A2' WHEN 2 THEN 'B1' WHEN 3 THEN 'B2' \
     WHEN 4 THEN 'C1' ELSE 'C2' END, i % 5, i % 5 + 3, \
     CASE i % 4 WHEN 0 THEN NULL ELSE i END, \
     '[{\"band\":\"A1\",\"total\":3,\"mature\":2,\"pct\":70.5,\"achieved\":false}]', \
     2000 * i, 1000 * i FROM n",
    "WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 101) \
     INSERT INTO band_milestones (course, band, study_day, baseline, celebrated_at, created_at) \
     SELECT 'c' || i, CASE i % 6 WHEN 0 THEN 'A1' WHEN 1 THEN 'A2' WHEN 2 THEN 'B1' \
     WHEN 3 THEN 'B2' WHEN 4 THEN 'C1' ELSE 'C2' END, 20000 + i, i % 2, \
     CASE WHEN i % 2 = 1 THEN 1000 * i WHEN i % 3 = 0 THEN NULL ELSE 5000 + i END, \
     1000 * i FROM n",
    "INSERT INTO law_dues (id, study_day, backlog, due_today, updated_at, created_at) \
     VALUES (1, 20100, 4, 6, 5000, 1000)",
    "WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 101) \
     INSERT INTO minutes_log (code, study_day, minutes, note, created_at) \
     SELECT CASE i % 2 WHEN 0 THEN 'qaa' ELSE 'qab' END, 20000 + i, 1 + i % 600, \
     CASE i % 3 WHEN 0 THEN '' ELSE 'synthetic note ' || i END, 1000 * i FROM n",
    "WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 101) \
     INSERT INTO writing_log (code, study_day, created_at) \
     SELECT CASE i % 2 WHEN 0 THEN 'qaa' ELSE 'qab' END, 20000 + i, 1000 * i FROM n",
    "WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 101) \
     INSERT INTO passkeys (telegram_user_id, credential_id, user_handle, credential, counter, \
     backup_state, created_at, last_used_at) \
     SELECT 4242, CAST('synthetic-credential-' || i AS BLOB), zeroblob(16), \
     'synthetic credential ' || i, i, i % 2, 1000 * i, \
     CASE WHEN i % 2 = 0 THEN 1000 * i + 500 END FROM n",
    "WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 101) \
     INSERT INTO sensitive_decks (deck_id, created_at) SELECT 7000 + i, 1000 * i FROM n",
];

/// Prints how many items a check examined and refuses zero (the tdd pack's examined contract).
fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}

/// The repository's root, two levels above this crate.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// An identifier, quoted for `SQLite`.
fn quoted(identifier: &str) -> String {
    format!("\"{}\"", identifier.replace('"', "\"\""))
}

/// Every table of the database's schema but `SQLite`'s own, by name.
async fn schema_tables(db: &Db) -> Vec<String> {
    sqlx::query_scalar(
        "SELECT name FROM sqlite_master WHERE type = 'table' \
         AND name NOT LIKE 'sqlite\\_%' ESCAPE '\\' ORDER BY name",
    )
    .fetch_all(db.reader())
    .await
    .expect("the schema's tables")
}

/// Every row of `table`, as a JSON object of its columns; a blob reads as its hex text.
async fn rows(db: &Db, table: &str) -> Vec<Value> {
    let columns: Vec<String> =
        sqlx::query_scalar("SELECT name FROM pragma_table_info(?1) ORDER BY cid")
            .bind(table)
            .fetch_all(db.reader())
            .await
            .expect("the table's columns");
    assert!(!columns.is_empty(), "{table} has no column");
    let pairs: Vec<String> = columns
        .iter()
        .map(|column| {
            let name = quoted(column);
            format!(
                "'{}', CASE typeof({name}) WHEN 'blob' THEN hex({name}) ELSE {name} END",
                column.replace('\'', "''")
            )
        })
        .collect();
    let sql = format!(
        "SELECT json_object({}) FROM {}",
        pairs.join(", "),
        quoted(table)
    );
    let texts: Vec<String> = sqlx::query_scalar(AssertSqlSafe(sql))
        .fetch_all(db.reader())
        .await
        .expect("the table's rows");
    texts
        .iter()
        .map(|text| serde_json::from_str(text).expect("a row reads as JSON"))
        .collect()
}

/// Rows as a sorted list of their JSON texts: equal lists are equal multisets of rows.
fn multiset(rows: &[Value]) -> Vec<String> {
    let mut texts: Vec<String> = rows.iter().map(Value::to_string).collect();
    texts.sort();
    texts
}

/// Every port's declaration, in the registry's order.
fn declarations() -> Vec<Declaration> {
    ports()
        .iter()
        .map(|port| port.declaration().expect("a registered port's declaration"))
        .collect()
}

/// The ownership register's section for the workspace's own tables: table to owning context.
fn register() -> BTreeMap<String, String> {
    let map = fs::read_to_string(root().join("docs/CONTEXT-MAP.md")).expect("the context map");
    let section = map
        .split("### DeckStreak's own tables\n")
        .nth(1)
        .expect("the register of DeckStreak's own tables");
    section
        .lines()
        .take_while(|line| !line.starts_with('#'))
        .filter(|line| line.starts_with("| `"))
        .filter_map(|line| {
            let mut cells = line
                .split('|')
                .skip(1)
                .map(|cell| cell.trim().trim_matches('`'));
            Some((cells.next()?.to_owned(), cells.next()?.to_owned()))
        })
        .collect()
}

/// A migrated temporary database.
async fn migrated() -> (TempDir, Db) {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let db = Db::open(&directory.path().join("deck_streak.db"))
        .await
        .expect("the database opens");
    (directory, db)
}

/// One run of the probe: every table's rows before the erase, the export, the erase's report,
/// and every table's rows after it.
struct Probe {
    tables: Vec<String>,
    before: BTreeMap<String, Vec<Value>>,
    export: Map<String, Value>,
    erasure: Erasure,
    after: BTreeMap<String, Vec<Value>>,
    db: Db,
    _directory: TempDir,
}

impl Probe {
    async fn run() -> Self {
        let (directory, db) = migrated().await;
        let mut write = db.write().await.expect("a write");
        for seed in SEEDS {
            sqlx::query(seed)
                .execute(&mut *write)
                .await
                .expect("a seed");
        }
        write.commit().await.expect("the seeds commit");
        let tables = examined("table(s) of the seeded schema", schema_tables(&db).await);
        let mut before = BTreeMap::new();
        for table in &tables {
            let rows = rows(&db, table).await;
            assert!(
                !rows.is_empty(),
                "{table} holds no row: give it one in SEEDS, so the probe can judge it"
            );
            before.insert(table.clone(), rows);
        }
        let exported = export_all(&db).await.expect("the export");
        let Value::Object(export) = exported.as_json().clone() else {
            panic!("the export is one JSON object");
        };
        let erasure = erase_all(&db).await.expect("the erase");
        let mut after = BTreeMap::new();
        for table in &tables {
            after.insert(table.clone(), rows(&db, table).await);
        }
        Self {
            tables,
            before,
            export,
            erasure,
            after,
            db,
            _directory: directory,
        }
    }

    /// The tables whose every row the export carries, and no other.
    fn exported(&self) -> BTreeSet<String> {
        self.tables
            .iter()
            .filter(|table| {
                self.export
                    .get(table.as_str())
                    .and_then(Value::as_array)
                    .is_some_and(|rows| multiset(rows) == multiset(&self.before[*table]))
            })
            .cloned()
            .collect()
    }

    /// The tables none of whose rows survives the erase unchanged.
    fn erased(&self) -> BTreeSet<String> {
        self.tables
            .iter()
            .filter(|table| {
                let before: BTreeSet<String> = multiset(&self.before[*table]).into_iter().collect();
                let after: BTreeSet<String> = multiset(&self.after[*table]).into_iter().collect();
                !before.is_empty() && before.is_disjoint(&after)
            })
            .cloned()
            .collect()
    }
}

#[tokio::test]
async fn the_exported_tables_equal_the_erased_tables_over_every_port() {
    let probe = Probe::run().await;
    let declarations = examined("port(s) in the registry", declarations());
    let declared: BTreeSet<String> = declarations
        .iter()
        .flat_map(Declaration::tables)
        .filter(|rights| !matches!(rights.disposition, Disposition::Exempt { .. }))
        .map(|rights| rights.table.to_owned())
        .collect();
    let exported = probe.exported();
    let erased = probe.erased();
    println!(
        "examined {} exported and {} erased table(s)",
        exported.len(),
        erased.len()
    );
    assert_eq!(
        exported, erased,
        "the export carries whole exactly the tables the erase clears or resets"
    );
    assert_eq!(
        exported, declared,
        "the measured tables are the ones the ports declare exported or reset"
    );
    // Every table key the export carries holds that table's rows whole.
    let keys: BTreeSet<String> = probe
        .export
        .keys()
        .filter(|key| *key != SCHEMA_KEY)
        .cloned()
        .collect();
    assert_eq!(
        keys, exported,
        "a key of the export holds a table's rows short"
    );
    probe.db.close().await;
}

#[tokio::test]
async fn every_table_of_the_schema_is_declared_by_exactly_one_port() {
    let (_directory, db) = migrated().await;
    let tables = examined("table(s) of the migrated schema", schema_tables(&db).await);
    let declarations = examined("port(s) in the registry", declarations());
    assert_eq!(census(&tables, &declarations), Vec::<String>::new());
    // Each table's one port is the context the ownership register names as its owner (ADR-008).
    let register = register();
    let owners: BTreeMap<String, String> = declarations
        .iter()
        .flat_map(|declaration| {
            declaration
                .tables()
                .iter()
                .map(|rights| (rights.table.to_owned(), declaration.context().to_owned()))
        })
        .collect();
    let registered: BTreeMap<String, String> = tables
        .iter()
        .map(|table| {
            let owner = register.get(table).cloned();
            (
                table.clone(),
                owner.unwrap_or_else(|| "unregistered".to_owned()),
            )
        })
        .collect();
    assert_eq!(
        owners, registered,
        "a table's port is not its registered owner"
    );

    // A table no port declares is refused by its name, and so is a table two ports declare.
    let mut write = db.write().await.expect("a write");
    sqlx::query(
        "CREATE TABLE planted_future_table (id INTEGER PRIMARY KEY, secret TEXT NOT NULL, \
         created_at INTEGER NOT NULL) STRICT",
    )
    .execute(&mut *write)
    .await
    .expect("the planted table");
    write.commit().await.expect("committed");
    let planted = schema_tables(&db).await;
    assert_eq!(
        census(&planted, &declarations),
        ["planted_future_table is declared by no port"]
    );
    let first = declarations[0].clone();
    let mut twice = declarations.clone();
    twice.push(first.clone());
    let mut doubled: Vec<String> = first
        .tables()
        .iter()
        .map(|rights| {
            format!(
                "{} is declared by 2 ports: {context}, {context}",
                rights.table,
                context = first.context()
            )
        })
        .collect();
    doubled.sort();
    assert_eq!(census(&tables, &twice), doubled);
    db.close().await;
}

/// Every refusal of the census: a table of the schema that no port, or more than one, declares,
/// and a declared table no migration creates.
fn census(tables: &[String], declarations: &[Declaration]) -> Vec<String> {
    let mut owners: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for declaration in declarations {
        for rights in declaration.tables() {
            owners
                .entry(rights.table)
                .or_default()
                .push(declaration.context());
        }
    }
    let mut refused = Vec::new();
    for table in tables {
        match owners.get(table.as_str()).map(Vec::as_slice) {
            Some([_]) => {}
            None => refused.push(format!("{table} is declared by no port")),
            Some(contexts) => refused.push(format!(
                "{table} is declared by {} ports: {}",
                contexts.len(),
                contexts.join(", ")
            )),
        }
    }
    for table in owners.keys() {
        if !tables.iter().any(|name| name == table) {
            refused.push(format!("{table} is declared and no migration creates it"));
        }
    }
    refused
}

#[tokio::test]
async fn erase_leaves_the_cron_fire_ledger_and_the_schema_table_untouched() {
    let probe = Probe::run().await;
    for table in examined(
        "exempt table(s)",
        vec![
            CRON_FIRES_TABLE,
            SCHEMA_VERSION_TABLE,
            WRITE_CLASS_STOP_TABLE,
        ],
    ) {
        let before = &probe.before[table];
        assert!(!before.is_empty(), "{table} held rows before the erase");
        assert_eq!(
            &probe.after[table], before,
            "{table} changed across the erase"
        );
        assert!(
            !probe.export.contains_key(table),
            "{table} is exempt from the export too"
        );
    }
    assert_eq!(probe.before[CRON_FIRES_TABLE].len(), 101);
    assert_eq!(
        probe.erasure.kept,
        [
            SCHEMA_VERSION_TABLE,
            WRITE_CLASS_STOP_TABLE,
            CRON_FIRES_TABLE
        ],
        "the erase reports the exempt tables it kept"
    );
    // The guard still holds: a fire claimed before the erase cannot be claimed again after it.
    let ledger = SqliteCronLedger::new(probe.db.clone());
    let claimed_again = ledger
        .claim(
            "synthetic_daily",
            FireDate::from_epoch_day(20_001),
            UtcMillis::from_epoch_millis(20_001 * 86_400_000),
        )
        .await
        .expect("the claim runs");
    assert!(
        !claimed_again,
        "an erase re-armed the catch-up double-send guard"
    );
    probe.db.close().await;
}

/// The erase mode each table a port exports or erases must carry in `privacy.json`: `delete` for a
/// table the erase empties, `anonymise` for a singleton it resets in place.
fn erase_modes(declarations: &[Declaration]) -> BTreeMap<&'static str, &'static str> {
    declarations
        .iter()
        .flat_map(Declaration::tables)
        .filter_map(|rights| match rights.disposition {
            Disposition::ExportAndErase => Some((rights.table, "delete")),
            Disposition::ResetInPlace { .. } => Some((rights.table, "anonymise")),
            Disposition::Exempt { .. } => None,
        })
        .collect()
}

/// Every table a category of `inventory` names in its stores, with the categories naming it.
fn named_tables(inventory: &Value) -> BTreeMap<String, Vec<&Value>> {
    let mut named: BTreeMap<String, Vec<&Value>> = BTreeMap::new();
    for category in inventory["categories"].as_array().into_iter().flatten() {
        let tables: BTreeSet<&str> = category["stores"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .filter_map(|store| store.split_once('.').map(|(table, _)| table))
            .collect();
        for table in tables {
            named.entry(table.to_owned()).or_default().push(category);
        }
    }
    named
}

/// Every refusal of `privacy.json` against the ports: a table a port exports or erases that no
/// category names, or more than one does, or whose category exports it not or erases it by another
/// mode; and a table a category names that no port exports or erases.
fn judge(inventory: &Value, declarations: &[Declaration]) -> Vec<String> {
    let modes = erase_modes(declarations);
    let named = named_tables(inventory);
    let mut refused = Vec::new();
    for (table, mode) in &modes {
        match named.get(*table).map(Vec::as_slice) {
            None => refused.push(format!("{table} is named by no category")),
            Some([category]) => {
                let id = category["id"].as_str().unwrap_or("?");
                if category["erase"] != *mode {
                    refused.push(format!(
                        "{table}'s category {id} erases it by {}, and its port by {mode}",
                        category["erase"]
                    ));
                }
                if category["export"] != Value::Bool(true) {
                    refused.push(format!("{table}'s category {id} does not export it"));
                }
            }
            Some(categories) => refused.push(format!(
                "{table} is named by {} categories",
                categories.len()
            )),
        }
    }
    for table in named.keys() {
        if !modes.contains_key(table.as_str()) {
            refused.push(format!(
                "{table} is named by a category, and no port exports or erases it"
            ));
        }
    }
    refused
}

#[test]
fn privacy_json_names_every_table_the_ports_export_or_erase() {
    let text = fs::read_to_string(root().join("privacy.json")).expect("privacy.json is readable");
    let inventory: Value = serde_json::from_str(&text).expect("privacy.json is JSON");
    let declarations = declarations();
    let modes = erase_modes(&declarations);
    examined(
        "table(s) the ports export or erase",
        modes.keys().collect::<Vec<_>>(),
    );
    assert_eq!(judge(&inventory, &declarations), Vec::<String>::new());
    // Every one of those tables has its one category, carrying the port's erase mode.
    let named = named_tables(&inventory);
    let carried: BTreeMap<&str, &str> = modes
        .keys()
        .filter_map(|table| {
            let category = named.get(*table)?.first()?;
            Some((*table, category["erase"].as_str()?))
        })
        .collect();
    assert_eq!(carried, modes);

    // An inventory that forgets a table's category, or erases it by another mode, is refused by
    // the table's name.
    let (table, mode) = modes.iter().next().map(|(t, m)| (*t, *m)).expect("a table");
    let id = named[table][0]["id"]
        .as_str()
        .expect("a category id")
        .to_owned();
    let mut forgotten = inventory.clone();
    if let Some(categories) = forgotten["categories"].as_array_mut() {
        categories.retain(|category| category["id"] != id.as_str());
    }
    assert_eq!(
        judge(&forgotten, &declarations),
        [format!("{table} is named by no category")]
    );
    let other = if mode == "delete" {
        "anonymise"
    } else {
        "delete"
    };
    let mut mismatched = inventory.clone();
    if let Some(categories) = mismatched["categories"].as_array_mut() {
        for category in categories.iter_mut() {
            if category["id"] == id.as_str() {
                category["erase"] = Value::from(other);
            }
        }
    }
    assert_eq!(
        judge(&mismatched, &declarations),
        [format!(
            "{table}'s category {id} erases it by \"{other}\", and its port by {mode}"
        )]
    );
}
