//! The router's ledger (SPEC-041 R6, R7, R8, R11): the decisions, the deliveries that claim a key,
//! the queue of held celebrations, the in-app feed and the owner's settings, the five tables of
//! `migrations/004101_notifications_router.sql`. Every write here runs inside the caller's
//! `BEGIN IMMEDIATE` write, so one decision is one transaction.

use deck_streak_kernel::{Db, KernelError, UtcMillis};
use serde::Serialize;
use sqlx::SqliteConnection;

use crate::occasion::{Surface, Tier};

/// The decision ledger, in the pack's `phx.notifications.decision.v1` shape.
pub const DECISIONS_TABLE: &str = "notification_decisions";
/// The deliveries: one row claims a key in its scope, unique on the kind and the scoped key.
pub const DELIVERIES_TABLE: &str = "notification_deliveries";
/// The queue of held celebrations, and of abandoned ones until a recap names them.
pub const QUEUE_TABLE: &str = "notification_queue";
/// The in-app feed the Mini App pulls.
pub const FEED_TABLE: &str = "in_app_feed";
/// The owner's notification settings, by key.
pub const SETTINGS_TABLE: &str = "notification_settings";
/// The owner's latest message to the bot, which a T1 celebration reacts to (SPEC-084 R13).
pub const OWNER_MESSAGE_TABLE: &str = "owner_last_message";

/// A decision's row.
pub(crate) struct DecisionRow<'a> {
    pub(crate) dedupe_key: &'a str,
    pub(crate) kind: String,
    pub(crate) surface: Surface,
    pub(crate) arm: &'static str,
    pub(crate) reason: Option<&'static str>,
    pub(crate) tier_requested: Tier,
    pub(crate) tier_rendered: Tier,
    pub(crate) study_day: i64,
    pub(crate) created_at: UtcMillis,
}

/// A delivery's claim on its key.
pub(crate) struct ClaimRow<'a> {
    pub(crate) kind: &'a str,
    pub(crate) dedupe_key: &'a str,
    pub(crate) scope: &'a str,
    pub(crate) surface: Surface,
    pub(crate) study_day: i64,
    pub(crate) lapse_id: Option<i64>,
    pub(crate) created_at: UtcMillis,
}

/// A held celebration, as the queue keeps it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct HeldRow {
    pub(crate) id: i64,
    pub(crate) kind: String,
    pub(crate) dedupe_key: String,
    pub(crate) surface: Surface,
    pub(crate) tier_requested: Tier,
    pub(crate) tier_pending: Tier,
    pub(crate) text: String,
    pub(crate) hold: String,
    pub(crate) tries: i64,
    pub(crate) deferred_at: i64,
    pub(crate) study_day: i64,
    /// The token of the flush that claimed the row; `None` for a row no flush has claimed.
    pub(crate) claim: Option<String>,
}

/// One item of the in-app feed, as the feed route serves it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FeedItem {
    /// The kind of the occasion it renders.
    pub kind: String,
    /// Its text.
    pub text: String,
    /// The tier it rendered at, which the Mini App animates (SPEC-084 R10).
    pub tier: String,
    /// When it was appended, in epoch milliseconds.
    pub created_at: i64,
}

/// The owner's value for `key`, when one is set.
pub(crate) async fn setting(
    connection: &mut SqliteConnection,
    key: &str,
) -> Result<Option<String>, KernelError> {
    Ok(
        sqlx::query_scalar!("SELECT value FROM notification_settings WHERE key = ?", key)
            .fetch_optional(connection)
            .await?,
    )
}

/// Claims `claim`'s key in its scope: the new delivery's id, or `None` when a delivery holds it.
pub(crate) async fn claim(
    connection: &mut SqliteConnection,
    claim: &ClaimRow<'_>,
) -> Result<Option<i64>, KernelError> {
    let surface = claim.surface.as_str();
    let created_at = claim.created_at.epoch_millis();
    Ok(sqlx::query_scalar!(
        r#"INSERT INTO notification_deliveries
               (kind, dedupe_key, scope, surface, study_day, lapse_id, created_at)
           VALUES (?, ?, ?, ?, ?, ?, ?)
           ON CONFLICT DO NOTHING
           RETURNING id AS "id!""#,
        claim.kind,
        claim.dedupe_key,
        claim.scope,
        surface,
        claim.study_day,
        claim.lapse_id,
        created_at,
    )
    .fetch_optional(connection)
    .await?)
}

/// Releases the claim `id`: its occasion was not delivered.
pub(crate) async fn release(connection: &mut SqliteConnection, id: i64) -> Result<(), KernelError> {
    sqlx::query!("DELETE FROM notification_deliveries WHERE id = ?", id)
        .execute(connection)
        .await?;
    Ok(())
}

/// The deliveries of `kind` in the lapse `lapse_id`, besides the claim `besides`: how many, and the
/// latest one's study day.
pub(crate) async fn lapse_sends(
    connection: &mut SqliteConnection,
    kind: &str,
    lapse_id: i64,
    besides: i64,
) -> Result<(i64, Option<i64>), KernelError> {
    let row = sqlx::query!(
        r#"SELECT COUNT(*) AS "sends!: i64", MAX(study_day) AS "last: i64"
           FROM notification_deliveries WHERE kind = ? AND lapse_id = ? AND id != ?"#,
        kind,
        lapse_id,
        besides,
    )
    .fetch_one(connection)
    .await?;
    Ok((row.sends, row.last))
}

/// Records one decision.
pub(crate) async fn record(
    connection: &mut SqliteConnection,
    row: &DecisionRow<'_>,
) -> Result<(), KernelError> {
    let surface = row.surface.as_str();
    let (requested, rendered) = (row.tier_requested.as_str(), row.tier_rendered.as_str());
    let created_at = row.created_at.epoch_millis();
    sqlx::query!(
        "INSERT INTO notification_decisions (dedupe_key, kind, surface, arm, reason, \
         tier_requested, tier_rendered, study_day, created_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        row.dedupe_key,
        row.kind,
        surface,
        row.arm,
        row.reason,
        requested,
        rendered,
        row.study_day,
        created_at,
    )
    .execute(connection)
    .await?;
    Ok(())
}

/// Holds a celebration on the queue, with `tries` failed sends behind it.
pub(crate) async fn hold(
    connection: &mut SqliteConnection,
    row: &HeldRow,
    created_at: UtcMillis,
) -> Result<(), KernelError> {
    let surface = row.surface.as_str();
    let (requested, pending) = (row.tier_requested.as_str(), row.tier_pending.as_str());
    let created_at = created_at.epoch_millis();
    sqlx::query!(
        "INSERT INTO notification_queue (kind, dedupe_key, surface, tier_requested, tier_pending, \
         text, hold, tries, state, deferred_at, study_day, created_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, 'held', ?, ?, ?)",
        row.kind,
        row.dedupe_key,
        surface,
        requested,
        pending,
        row.text,
        row.hold,
        row.tries,
        row.deferred_at,
        row.study_day,
        created_at,
    )
    .execute(connection)
    .await?;
    Ok(())
}

/// The held celebrations, in the order they were held; the router ranks them.
pub(crate) async fn held(connection: &mut SqliteConnection) -> Result<Vec<HeldRow>, KernelError> {
    let rows = sqlx::query!(
        r#"SELECT id AS "id!", kind, dedupe_key, surface, tier_requested, tier_pending, text, hold,
                  tries, deferred_at, study_day
           FROM notification_queue WHERE state = 'held' ORDER BY id"#
    )
    .fetch_all(connection)
    .await?;
    rows.into_iter()
        .map(|row| {
            Ok(HeldRow {
                id: row.id,
                kind: row.kind,
                dedupe_key: row.dedupe_key,
                surface: decoded(Surface::parse(&row.surface))?,
                tier_requested: decoded(Tier::parse(&row.tier_requested))?,
                tier_pending: decoded(Tier::parse(&row.tier_pending))?,
                text: row.text,
                hold: row.hold,
                tries: row.tries,
                deferred_at: row.deferred_at,
                study_day: row.study_day,
                claim: None,
            })
        })
        .collect()
}

/// One abandoned celebration a recap has yet to name: its row id, its key, what held it, and the
/// claim it was abandoned under, which is `Some` only when its fate is unknown ("may have been
/// sent").
pub(crate) type Abandoned = (i64, String, String, Option<String>);

/// The abandoned celebrations no recap has named yet, oldest first.
pub(crate) async fn abandoned(
    connection: &mut SqliteConnection,
) -> Result<Vec<Abandoned>, KernelError> {
    let rows = sqlx::query!(
        r#"SELECT id AS "id!", dedupe_key, hold, claim FROM notification_queue
           WHERE state = 'abandoned' ORDER BY id"#
    )
    .fetch_all(connection)
    .await?;
    Ok(rows
        .into_iter()
        .map(|row| (row.id, row.dedupe_key, row.hold, row.claim))
        .collect())
}

/// Claims every held celebration for the flush whose token is `claim`: each moves `held ->
/// sending` in one conditional update that matches only `held` rows, so a row another flush
/// claimed is never read for sending. The rows it claimed, in the order they were held.
pub(crate) async fn claim_held(
    connection: &mut SqliteConnection,
    claim: &str,
) -> Result<Vec<HeldRow>, KernelError> {
    let rows = sqlx::query!(
        r#"UPDATE notification_queue SET state = 'sending', claim = ?
           WHERE state = 'held'
           RETURNING id AS "id!", kind, dedupe_key, surface, tier_requested, tier_pending, text,
                     hold, tries, deferred_at, study_day"#,
        claim
    )
    .fetch_all(connection)
    .await?;
    let mut held = rows
        .into_iter()
        .map(|row| {
            Ok(HeldRow {
                id: row.id,
                kind: row.kind,
                dedupe_key: row.dedupe_key,
                surface: decoded(Surface::parse(&row.surface))?,
                tier_requested: decoded(Tier::parse(&row.tier_requested))?,
                tier_pending: decoded(Tier::parse(&row.tier_pending))?,
                text: row.text,
                hold: row.hold,
                tries: row.tries,
                deferred_at: row.deferred_at,
                study_day: row.study_day,
                claim: Some(claim.to_owned()),
            })
        })
        .collect::<Result<Vec<_>, KernelError>>()?;
    held.sort_by_key(|row| row.id);
    Ok(held)
}

/// The `sending` rows whose claim lapsed at or before `now`: their id, key and claim. A claim is
/// the instant its flush's lease lapses, so a claimant that is still sending is past its lease
/// and a claimant that died is gone; either way the push may have reached the owner.
pub(crate) async fn lapsed_claims(
    connection: &mut SqliteConnection,
    now: i64,
) -> Result<Vec<(i64, String, String)>, KernelError> {
    let rows = sqlx::query!(
        r#"SELECT id AS "id!", dedupe_key, claim AS "claim!" FROM notification_queue
           WHERE state = 'sending' AND CAST(claim AS INTEGER) <= ? ORDER BY id"#,
        now
    )
    .fetch_all(connection)
    .await?;
    Ok(rows
        .into_iter()
        .map(|row| (row.id, row.dedupe_key, row.claim))
        .collect())
}

/// Marks the held celebration `id` abandoned: it waits for a recap to name it.
pub(crate) async fn abandon(
    connection: &mut SqliteConnection,
    id: i64,
    tries: i64,
) -> Result<(), KernelError> {
    sqlx::query!(
        "UPDATE notification_queue SET state = 'abandoned', tries = ? \
         WHERE id = ? AND state = 'held'",
        tries,
        id
    )
    .execute(connection)
    .await?;
    Ok(())
}

/// Marks the celebration `id`, claimed under `claim`, abandoned, its claim cleared: its fate is
/// known (expired, over the bound, or its send failed). `false` when the row is no longer this
/// claim's.
pub(crate) async fn abandon_claimed(
    connection: &mut SqliteConnection,
    id: i64,
    claim: &str,
    tries: i64,
) -> Result<bool, KernelError> {
    let done = sqlx::query!(
        "UPDATE notification_queue SET state = 'abandoned', claim = NULL, tries = ? \
         WHERE id = ? AND state = 'sending' AND claim = ?",
        tries,
        id,
        claim
    )
    .execute(connection)
    .await?;
    Ok(done.rows_affected() == 1)
}

/// Marks the `sending` celebration `id` abandoned with its claim kept, which a recap reads as
/// "may have been sent": its claimant is gone or past its lease, and the push may have reached.
pub(crate) async fn abandon_lapsed(
    connection: &mut SqliteConnection,
    id: i64,
) -> Result<bool, KernelError> {
    let done = sqlx::query!(
        "UPDATE notification_queue SET state = 'abandoned' WHERE id = ? AND state = 'sending'",
        id
    )
    .execute(connection)
    .await?;
    Ok(done.rows_affected() == 1)
}

/// Holds the celebration `id`, claimed under `claim`, again after a failed send, with `tries`
/// failed sends behind it and held for `hold`; its first deferral time is kept. `false` when the
/// row is no longer this claim's.
pub(crate) async fn relatch(
    connection: &mut SqliteConnection,
    id: i64,
    claim: &str,
    tries: i64,
    hold: &str,
) -> Result<bool, KernelError> {
    let done = sqlx::query!(
        "UPDATE notification_queue SET state = 'held', claim = NULL, tries = ?, hold = ? \
         WHERE id = ? AND state = 'sending' AND claim = ?",
        tries,
        hold,
        id,
        claim
    )
    .execute(connection)
    .await?;
    Ok(done.rows_affected() == 1)
}

/// Gives every celebration still claimed under `claim` back to the queue, untouched: a flush that
/// ended before it reached them did not send them.
pub(crate) async fn release_claims(
    connection: &mut SqliteConnection,
    claim: &str,
) -> Result<(), KernelError> {
    sqlx::query!(
        "UPDATE notification_queue SET state = 'held', claim = NULL \
         WHERE state = 'sending' AND claim = ?",
        claim
    )
    .execute(connection)
    .await?;
    Ok(())
}

/// Removes the celebration `id` from the queue once delivered, when it is still claimed under
/// `claim`: `false` when another flush abandoned it by name meanwhile.
pub(crate) async fn settle_claimed(
    connection: &mut SqliteConnection,
    id: i64,
    claim: &str,
) -> Result<bool, KernelError> {
    let done = sqlx::query!(
        "DELETE FROM notification_queue WHERE id = ? AND state = 'sending' AND claim = ?",
        id,
        claim
    )
    .execute(connection)
    .await?;
    Ok(done.rows_affected() == 1)
}

/// Removes the celebration `id` from the queue: named by a recap.
pub(crate) async fn settle(connection: &mut SqliteConnection, id: i64) -> Result<(), KernelError> {
    sqlx::query!("DELETE FROM notification_queue WHERE id = ?", id)
        .execute(connection)
        .await?;
    Ok(())
}

/// The celebrations of `kind` delivered on study day `since` or later at `min` or above (SPEC-084
/// R4).
pub(crate) async fn delivered_at_or_above(
    connection: &mut SqliteConnection,
    kind: &str,
    since: i64,
    min: Tier,
) -> Result<i64, KernelError> {
    let min = min.as_str();
    Ok(sqlx::query_scalar!(
        r#"SELECT COUNT(*) AS "count!: i64" FROM notification_decisions
           WHERE arm = 'send' AND kind = ? AND study_day >= ? AND tier_rendered >= ?"#,
        kind,
        since,
        min,
    )
    .fetch_one(connection)
    .await?)
}

/// The celebrations of `kind` of study day `since` or later still held at `min` or above: an
/// abandoned one no longer holds its slot (SPEC-084 R4).
pub(crate) async fn held_at_or_above(
    connection: &mut SqliteConnection,
    kind: &str,
    since: i64,
    min: Tier,
) -> Result<i64, KernelError> {
    let min = min.as_str();
    Ok(sqlx::query_scalar!(
        r#"SELECT COUNT(*) AS "count!: i64" FROM notification_queue
           WHERE state IN ('held', 'sending') AND kind = ? AND study_day >= ? AND tier_pending >= ?"#,
        kind,
        since,
        min,
    )
    .fetch_one(connection)
    .await?)
}

/// Appends one item to the in-app feed, unseen.
pub(crate) async fn append_feed(
    connection: &mut SqliteConnection,
    dedupe_key: &str,
    kind: &str,
    tier: Tier,
    text: &str,
    created_at: UtcMillis,
) -> Result<(), KernelError> {
    let (tier, created_at) = (tier.as_str(), created_at.epoch_millis());
    sqlx::query!(
        "INSERT INTO in_app_feed (dedupe_key, kind, tier, text, seen_at, created_at) \
         VALUES (?, ?, ?, ?, NULL, ?)",
        dedupe_key,
        kind,
        tier,
        text,
        created_at,
    )
    .execute(connection)
    .await?;
    Ok(())
}

/// Every unseen item of the in-app feed, oldest first, marked seen at `now` in the same write, so
/// each is served once (R12).
///
/// # Errors
///
/// [`KernelError::Database`] when the read or the write fails.
pub async fn take_unseen_feed(db: &Db, now: UtcMillis) -> Result<Vec<FeedItem>, KernelError> {
    let mut write = db.write().await?;
    let rows = sqlx::query!(
        r#"SELECT id AS "id!", kind, text, tier, created_at FROM in_app_feed
           WHERE seen_at IS NULL ORDER BY id"#
    )
    .fetch_all(&mut *write)
    .await?;
    let seen_at = now.epoch_millis();
    sqlx::query!(
        "UPDATE in_app_feed SET seen_at = ? WHERE seen_at IS NULL",
        seen_at
    )
    .execute(&mut *write)
    .await?;
    write.commit().await?;
    Ok(rows
        .into_iter()
        .map(|row| FeedItem {
            kind: row.kind,
            text: row.text,
            tier: row.tier,
            created_at: row.created_at,
        })
        .collect())
}

/// A value the table's check admits, or the decode error of one it would refuse.
fn decoded<T>(value: Option<T>) -> Result<T, KernelError> {
    value.ok_or_else(|| {
        KernelError::Database(sqlx::Error::Decode(
            "a notifications row holds a value its column's check refuses".into(),
        ))
    })
}
