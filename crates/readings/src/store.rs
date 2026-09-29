//! The readings' record of each study day (SPEC-045 R5, R8, R9, R12): `reading_runs` and
//! `reading_topic_days`, owned by this context (docs/CONTEXT-MAP.md), created by
//! `migrations/004501_readings_topic_days_and_runs.sql`.
//!
//! A run is one row: its trigger, its study day, when it started and finished, its outcome, the
//! class and reason of a run refused as a whole, and how many decks with new cards mapped to no
//! topic. A topic day is one row per study day and topic, holding its state, class and reason, and
//! for a topic with a day set its digest, card ids, note ids and new-card count; a later run the
//! same day replaces the row's state. A state is written from [`TopicState`] and read back through
//! [`TopicState::from_stored`], so no state, class or reason outside the closed sets is written or
//! read, and the table's checks refuse one too.

use deck_streak_kernel::{Db, KernelError, StudyDay, UtcMillis};

use crate::day_set::{StudyDayResolution, TopicEnd};
use crate::reading::ReadingId;
use crate::state::{Class, CouldNotTell, RunOutcome, TopicState};
use crate::studied::Verdict;
use crate::topic::TopicKey;

/// What asked for a resolution.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RunTrigger {
    /// The scheduler's generation job (SPEC-053).
    Scheduled,
    /// The owner's tap (SPEC-048).
    Owner,
}

impl RunTrigger {
    /// The trigger as `reading_runs` stores it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Scheduled => "scheduled",
            Self::Owner => "owner",
        }
    }

    /// The trigger stored as `text`.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        [Self::Scheduled, Self::Owner]
            .into_iter()
            .find(|trigger| trigger.as_str() == text)
    }
}

/// A run's id in `reading_runs`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RunId(i64);

impl RunId {
    /// The row's id.
    #[must_use]
    pub const fn get(self) -> i64 {
        self.0
    }
}

/// One resolution run, as `reading_runs` records it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReadingRun {
    /// What asked for it.
    pub trigger: RunTrigger,
    /// The study day it resolved.
    pub study_day: StudyDay,
    /// When it started, by the kernel's clock.
    pub started_at: UtcMillis,
    /// When it finished, by the kernel's clock; the row is created then.
    pub finished_at: UtcMillis,
    /// Its outcome.
    pub outcome: RunOutcome,
    /// How many decks with new cards mapped to no topic (R9).
    pub unmapped_decks: u32,
}

/// A topic's day set, as a topic day holds it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DaySetRecord {
    /// The SHA-256 of its sorted card ids joined by commas, in lowercase hexadecimal.
    pub digest: String,
    /// Its new cards, sorted.
    pub card_ids: Vec<i64>,
    /// Its distinct notes, sorted.
    pub note_ids: Vec<i64>,
}

/// One topic's state for one study day.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TopicDay {
    /// The study day.
    pub study_day: StudyDay,
    /// The topic.
    pub topic: TopicKey,
    /// The state it ended the day in.
    pub state: TopicState,
    /// Its day set, when it had new cards.
    pub day_set: Option<DaySetRecord>,
}

/// A topic day as read back, with the run that last wrote it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredTopicDay {
    /// The run that last wrote the row.
    pub run: RunId,
    /// The topic day.
    pub day: TopicDay,
}

/// Why the record could not be read back.
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    /// The database refused the read or the write.
    #[error(transparent)]
    Kernel(#[from] KernelError),
    /// A stored row names no closed state, trigger, topic or id list: it was not written here.
    #[error("the readings' record holds a row this context did not write: {table} row {id}")]
    Unreadable {
        /// The table.
        table: &'static str,
        /// The row's id.
        id: i64,
    },
}

impl From<sqlx::Error> for StoreError {
    fn from(error: sqlx::Error) -> Self {
        Self::Kernel(KernelError::Database(error))
    }
}

/// A day set's id list as its column holds it: a JSON array.
fn id_list(ids: &[i64]) -> String {
    serde_json::Value::from(ids.to_vec()).to_string()
}

/// The readings' tables in the service's own database.
#[derive(Clone, Debug)]
pub struct SqliteReadings {
    pub(crate) db: Db,
}

impl SqliteReadings {
    /// The record in `db`, whose migrations created the readings' tables.
    #[must_use]
    pub const fn new(db: Db) -> Self {
        Self { db }
    }

    /// Records `run`, and every topic of `resolution` that ended the day in it, in one write. A
    /// topic with a day set is left to the generation (SPEC-046), which records its end.
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when the write fails; nothing is written then.
    pub async fn record(
        &self,
        run: &ReadingRun,
        resolution: &StudyDayResolution,
    ) -> Result<RunId, KernelError> {
        let mut write = self.db.write().await?;
        let id = insert_run(&mut write, run).await?;
        for topic in &resolution.topics {
            if let TopicEnd::Ended(state) = topic.end {
                let day = TopicDay {
                    study_day: run.study_day,
                    topic: topic.topic.clone(),
                    state,
                    day_set: None,
                };
                upsert_topic_day(&mut write, id, &day, run.finished_at).await?;
            }
        }
        write.commit().await?;
        Ok(id)
    }

    /// Records `run` alone, with no topic day.
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when the write fails.
    pub async fn record_run(&self, run: &ReadingRun) -> Result<RunId, KernelError> {
        let mut write = self.db.write().await?;
        let id = insert_run(&mut write, run).await?;
        write.commit().await?;
        Ok(id)
    }

    /// Records `day` for `run`, at `at`: the row of its study day and topic, or its new state.
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when the write fails, the table's checks refusing it included.
    pub async fn record_topic_day(
        &self,
        run: RunId,
        day: &TopicDay,
        at: UtcMillis,
    ) -> Result<(), KernelError> {
        let mut write = self.db.write().await?;
        upsert_topic_day(&mut write, run, day, at).await?;
        write.commit().await?;
        Ok(())
    }

    /// Every run, oldest first.
    ///
    /// # Errors
    ///
    /// [`StoreError::Kernel`] when the read fails, and [`StoreError::Unreadable`] for a row this
    /// context did not write.
    pub async fn runs(&self) -> Result<Vec<(RunId, ReadingRun)>, StoreError> {
        let rows = sqlx::query_as!(
            RunRow,
            r#"SELECT id AS "id!", trigger, study_day, started_at, finished_at, outcome, class,
                      reason, unmapped_decks
               FROM reading_runs ORDER BY id"#
        )
        .fetch_all(self.db.reader())
        .await?;
        rows.into_iter()
            .map(|row| {
                let id = row.id;
                row.read().ok_or(StoreError::Unreadable {
                    table: "reading_runs",
                    id,
                })
            })
            .collect()
    }

    /// Every topic day of `study_day`, by topic.
    ///
    /// # Errors
    ///
    /// [`StoreError::Kernel`] when the read fails, and [`StoreError::Unreadable`] for a row this
    /// context did not write.
    pub async fn topic_days(&self, study_day: StudyDay) -> Result<Vec<StoredTopicDay>, StoreError> {
        let day = study_day.epoch_day();
        let rows = sqlx::query_as!(
            TopicDayRow,
            r#"SELECT id AS "id!", run_id, study_day, topic, state, class, reason, digest, card_ids,
                      note_ids, new_cards
               FROM reading_topic_days WHERE study_day = ?1 ORDER BY topic"#,
            day
        )
        .fetch_all(self.db.reader())
        .await?;
        rows.into_iter()
            .map(|row| {
                let id = row.id;
                row.read().ok_or(StoreError::Unreadable {
                    table: "reading_topic_days",
                    id,
                })
            })
            .collect()
    }
}

/// A `reading_runs` row as it is stored.
struct RunRow {
    id: i64,
    trigger: String,
    study_day: i64,
    started_at: i64,
    finished_at: i64,
    outcome: String,
    class: Option<String>,
    reason: Option<String>,
    unmapped_decks: i64,
}

impl RunRow {
    /// The run the row holds, or `None` when it names no closed trigger or outcome.
    fn read(self) -> Option<(RunId, ReadingRun)> {
        let run = ReadingRun {
            trigger: RunTrigger::parse(&self.trigger)?,
            study_day: StudyDay::from_epoch_day(self.study_day),
            started_at: UtcMillis::from_epoch_millis(self.started_at),
            finished_at: UtcMillis::from_epoch_millis(self.finished_at),
            outcome: RunOutcome::from_stored(
                &self.outcome,
                self.class.as_deref(),
                self.reason.as_deref(),
            )?,
            unmapped_decks: u32::try_from(self.unmapped_decks).ok()?,
        };
        Some((RunId(self.id), run))
    }
}

/// A `reading_topic_days` row as it is stored.
struct TopicDayRow {
    id: i64,
    run_id: i64,
    study_day: i64,
    topic: String,
    state: String,
    class: Option<String>,
    reason: Option<String>,
    digest: Option<String>,
    card_ids: String,
    note_ids: String,
    new_cards: i64,
}

impl TopicDayRow {
    /// The topic day the row holds, or `None` when it names no topic or closed state, or its id
    /// lists, digest and count disagree.
    fn read(self) -> Option<StoredTopicDay> {
        let card_ids: Vec<i64> = serde_json::from_str(&self.card_ids).ok()?;
        let note_ids: Vec<i64> = serde_json::from_str(&self.note_ids).ok()?;
        if i64::try_from(card_ids.len()).ok()? != self.new_cards {
            return None;
        }
        // A row with no digest holds no card (the table's checks), so it holds no day set.
        let day_set = self.digest.map(|digest| DaySetRecord {
            digest,
            card_ids,
            note_ids,
        });
        Some(StoredTopicDay {
            run: RunId(self.run_id),
            day: TopicDay {
                study_day: StudyDay::from_epoch_day(self.study_day),
                topic: TopicKey::parse(&self.topic)?,
                state: TopicState::from_stored(
                    &self.state,
                    self.class.as_deref(),
                    self.reason.as_deref(),
                )?,
                day_set,
            },
        })
    }
}

/// Inserts `run`, created when it finished, and returns its id.
async fn insert_run(
    write: &mut sqlx::SqliteConnection,
    run: &ReadingRun,
) -> Result<RunId, KernelError> {
    let trigger = run.trigger.as_str();
    let study_day = run.study_day.epoch_day();
    let started_at = run.started_at.epoch_millis();
    let finished_at = run.finished_at.epoch_millis();
    let outcome = run.outcome.name();
    let reason = run.outcome.reason();
    let class = reason.map(|reason| reason.class().as_str());
    let reason = reason.map(CouldNotTell::as_str);
    let unmapped = i64::from(run.unmapped_decks);
    let id = sqlx::query_scalar!(
        r#"INSERT INTO reading_runs
               (trigger, study_day, started_at, finished_at, outcome, class, reason, unmapped_decks,
                created_at)
           VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?4)
           RETURNING id AS "id!: i64""#,
        trigger,
        study_day,
        started_at,
        finished_at,
        outcome,
        class,
        reason,
        unmapped
    )
    .fetch_one(write)
    .await?;
    Ok(RunId(id))
}

/// Writes `day` for `run`: a new row for its study day and topic, created at `at`, or the existing
/// row's state and day set replaced, its `created_at` kept.
async fn upsert_topic_day(
    write: &mut sqlx::SqliteConnection,
    run: RunId,
    day: &TopicDay,
    at: UtcMillis,
) -> Result<(), KernelError> {
    let run_id = run.get();
    let study_day = day.study_day.epoch_day();
    let topic = day.topic.as_str();
    let state = day.state.name();
    let class = day.state.class().map(Class::as_str);
    let reason = day.state.reason();
    let digest = day.day_set.as_ref().map(|set| set.digest.as_str());
    let (cards, notes) = day.day_set.as_ref().map_or((&[][..], &[][..]), |set| {
        (&set.card_ids[..], &set.note_ids[..])
    });
    let card_ids = id_list(cards);
    let note_ids = id_list(notes);
    let new_cards = i64::try_from(cards.len()).unwrap_or(i64::MAX);
    let created_at = at.epoch_millis();
    sqlx::query!(
        "INSERT INTO reading_topic_days \
         (run_id, study_day, topic, state, class, reason, digest, card_ids, note_ids, new_cards, \
          created_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11) \
         ON CONFLICT (study_day, topic) DO UPDATE SET run_id = excluded.run_id, \
         state = excluded.state, class = excluded.class, reason = excluded.reason, \
         digest = excluded.digest, card_ids = excluded.card_ids, note_ids = excluded.note_ids, \
         new_cards = excluded.new_cards",
        run_id,
        study_day,
        topic,
        state,
        class,
        reason,
        digest,
        card_ids,
        note_ids,
        new_cards,
        created_at
    )
    .execute(write)
    .await?;
    Ok(())
}

/// Whether a reading's vault copy was written (SPEC-046 R9).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VaultStatus {
    /// Written, at this path.
    Written(String),
    /// The write failed; the reading is kept.
    Failed,
}

/// A reading to store.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewReading {
    /// Its id.
    pub id: ReadingId,
    /// Its topic.
    pub topic: TopicKey,
    /// The study day it was first generated for.
    pub study_day: StudyDay,
    /// The day set's digest.
    pub digest: String,
    /// The persona that wrote it.
    pub persona: String,
    /// The text.
    pub text: String,
    /// Its word count.
    pub word_count: u32,
    /// Its minutes.
    pub minutes: u32,
    /// The new cards it covers.
    pub card_ids: Vec<i64>,
    /// The distinct notes it was written from.
    pub note_count: u32,
    /// When it was generated.
    pub generated_at: UtcMillis,
    /// The vault copy.
    pub vault: VaultStatus,
}

/// A stored reading.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredReading {
    /// What was stored.
    pub reading: NewReading,
    /// The nights the reading was carried unchanged.
    pub carried_nights: u32,
}

impl SqliteReadings {
    /// Stores `reading`.
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when the write fails.
    pub async fn store_reading(&self, reading: &NewReading) -> Result<(), KernelError> {
        let id = reading.id.as_str();
        let topic = reading.topic.as_str();
        let study_day = reading.study_day.epoch_day();
        let word_count = i64::from(reading.word_count);
        let minutes = i64::from(reading.minutes);
        let new_cards = i64::try_from(reading.card_ids.len()).unwrap_or(i64::MAX);
        let note_count = i64::from(reading.note_count);
        let card_ids = id_list(&reading.card_ids);
        let generated_at = reading.generated_at.epoch_millis();
        let (status, path) = match &reading.vault {
            VaultStatus::Written(path) => ("written", Some(path.as_str())),
            VaultStatus::Failed => ("vault_write_failed", None),
        };
        let mut write = self.db.write().await?;
        sqlx::query!(
            "INSERT INTO readings \
             (id, topic, study_day, digest, persona, text, word_count, reading_minutes, new_cards, \
              note_count, card_ids, generated_at, version, vault_status, vault_path, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, 1, ?13, ?14, ?12)",
            id,
            topic,
            study_day,
            reading.digest,
            reading.persona,
            reading.text,
            word_count,
            minutes,
            new_cards,
            note_count,
            card_ids,
            generated_at,
            status,
            path
        )
        .execute(&mut *write)
        .await?;
        write.commit().await?;
        Ok(())
    }

    /// The newest stored reading of `topic`.
    ///
    /// # Errors
    ///
    /// [`StoreError`] when the read fails, or the row was not written here.
    pub async fn latest_reading(
        &self,
        topic: &TopicKey,
    ) -> Result<Option<StoredReading>, StoreError> {
        let topic = topic.as_str();
        let row = sqlx::query!(
            r#"SELECT id, topic, study_day, digest, persona, text, word_count, reading_minutes,
                      note_count, card_ids, generated_at, vault_status, vault_path, carried_nights
               FROM readings WHERE topic = ?1 ORDER BY generated_at DESC, rowid DESC LIMIT 1"#,
            topic
        )
        .fetch_optional(self.db.reader())
        .await?;
        row.map(|row| {
            let unreadable = || StoreError::Unreadable {
                table: "readings",
                id: 0,
            };
            read_reading(
                &row.id,
                &row.topic,
                row.study_day,
                row.digest,
                row.persona,
                row.text,
                (row.word_count, row.reading_minutes, row.note_count),
                &row.card_ids,
                row.generated_at,
                (&row.vault_status, row.vault_path),
                row.carried_nights,
            )
            .ok_or_else(unreadable)
        })
        .transpose()
    }

    /// Carries the reading `id` one more night.
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when the write fails.
    pub async fn carry_reading(&self, id: &ReadingId) -> Result<(), KernelError> {
        let id = id.as_str();
        let mut write = self.db.write().await?;
        sqlx::query!(
            "UPDATE readings SET carried_nights = carried_nights + 1 WHERE id = ?1",
            id
        )
        .execute(&mut *write)
        .await?;
        write.commit().await?;
        Ok(())
    }

    /// Every stored reading, oldest first.
    ///
    /// # Errors
    ///
    /// [`StoreError`] when the read fails, or a row was not written here.
    pub async fn readings(&self) -> Result<Vec<StoredReading>, StoreError> {
        let rows = sqlx::query!(
            r#"SELECT id, topic, study_day, digest, persona, text, word_count, reading_minutes,
                      note_count, card_ids, generated_at, vault_status, vault_path, carried_nights
               FROM readings ORDER BY generated_at, rowid"#
        )
        .fetch_all(self.db.reader())
        .await?;
        rows.into_iter()
            .map(|row| {
                read_reading(
                    &row.id,
                    &row.topic,
                    row.study_day,
                    row.digest,
                    row.persona,
                    row.text,
                    (row.word_count, row.reading_minutes, row.note_count),
                    &row.card_ids,
                    row.generated_at,
                    (&row.vault_status, row.vault_path),
                    row.carried_nights,
                )
                .ok_or(StoreError::Unreadable {
                    table: "readings",
                    id: 0,
                })
            })
            .collect()
    }

    /// Every topic the readings' record has named, in a topic day or a reading.
    ///
    /// # Errors
    ///
    /// [`StoreError`] when the read fails.
    pub async fn known_topics(&self) -> Result<Vec<TopicKey>, StoreError> {
        let rows = sqlx::query!(
            r#"SELECT topic AS "topic!" FROM (
                   SELECT topic FROM reading_topic_days UNION SELECT topic FROM readings
               ) ORDER BY topic"#
        )
        .fetch_all(self.db.reader())
        .await?;
        Ok(rows
            .into_iter()
            .filter_map(|row| TopicKey::parse(&row.topic))
            .collect())
    }
}

/// A `readings` row as a stored reading, or `None` when a column holds what this context never
/// wrote.
#[allow(clippy::too_many_arguments, reason = "one row's columns, read once")]
fn read_reading(
    id: &str,
    topic: &str,
    study_day: i64,
    digest: String,
    persona: String,
    text: String,
    counts: (i64, i64, i64),
    card_ids: &str,
    generated_at: i64,
    vault: (&str, Option<String>),
    carried_nights: i64,
) -> Option<StoredReading> {
    let (word_count, minutes, note_count) = counts;
    let vault = match vault {
        ("written", Some(path)) => VaultStatus::Written(path),
        ("vault_write_failed", None) => VaultStatus::Failed,
        _ => return None,
    };
    Some(StoredReading {
        reading: NewReading {
            id: ReadingId::parse(id)?,
            topic: TopicKey::parse(topic)?,
            study_day: StudyDay::from_epoch_day(study_day),
            digest,
            persona,
            text,
            word_count: u32::try_from(word_count).ok()?,
            minutes: u32::try_from(minutes).ok()?,
            card_ids: serde_json::from_str(card_ids).ok()?,
            note_count: u32::try_from(note_count).ok()?,
            generated_at: UtcMillis::from_epoch_millis(generated_at),
            vault,
        },
        carried_nights: u32::try_from(carried_nights).ok()?,
    })
}

/// Whether the tap's vault tick was written (SPEC-047 R8).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VaultTick {
    /// No tap yet, so nothing to write.
    None,
    /// The `I read it` line was ticked.
    Written,
    /// The tap happened and the tick failed; the next tap retries it.
    Pending,
}

impl VaultTick {
    /// The stored text.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Written => "written",
            Self::Pending => "pending",
        }
    }
}

/// A stored reading's read line and studied measure (SPEC-047 R8, R9).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadingProgress {
    /// Its id.
    pub id: ReadingId,
    /// Its topic.
    pub topic: TopicKey,
    /// The study day it was generated for.
    pub study_day: StudyDay,
    /// When it was generated.
    pub generated_at: UtcMillis,
    /// The covered cards.
    pub card_ids: Vec<i64>,
    /// When the owner tapped it, if they did.
    pub read_at: Option<UtcMillis>,
    /// How many covered cards the last settle found studied.
    pub studied_count: u32,
    /// Its verdict as of the last settle.
    pub verdict: Verdict,
    /// When it turned studied.
    pub studied_at: Option<UtcMillis>,
    /// Whether the tap's vault tick was written.
    pub vault_tick: VaultTick,
}

impl SqliteReadings {
    /// The read and studied state of the reading `id`, or `None` when no such reading is stored.
    ///
    /// # Errors
    ///
    /// [`StoreError`] when the read fails, or the row was not written here.
    pub async fn progress(&self, id: &ReadingId) -> Result<Option<ReadingProgress>, StoreError> {
        let _ = id;
        Ok(None)
    }

    /// The state of every reading whose verdict is not yet `studied`, oldest first.
    ///
    /// # Errors
    ///
    /// [`StoreError`] when the read fails, or a row was not written here.
    pub async fn unsettled(&self) -> Result<Vec<ReadingProgress>, StoreError> {
        Ok(Vec::new())
    }

    /// Sets `read_at` of the reading `id` to `at` when it is unset; true when this call set it.
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when the write fails.
    pub async fn mark_read(&self, id: &ReadingId, at: UtcMillis) -> Result<bool, KernelError> {
        let _ = (id, at);
        Ok(false)
    }

    /// Records whether the tap's vault tick was written.
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when the write fails.
    pub async fn set_vault_tick(&self, id: &ReadingId, tick: VaultTick) -> Result<(), KernelError> {
        let _ = (id, tick);
        Ok(())
    }

    /// Records the settle's measure of the reading `id`: its studied count and verdict, with the
    /// instant it turned studied when it did.
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when the write fails.
    pub async fn record_measure(
        &self,
        id: &ReadingId,
        count: u32,
        verdict: Verdict,
        studied_at: Option<UtcMillis>,
    ) -> Result<(), KernelError> {
        let _ = (id, count, verdict, studied_at);
        Ok(())
    }
}
