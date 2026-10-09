//! A preset's move to the scheduler's defaults, proposed to the owner (SPEC-387, ADR-401).
//!
//! [`PresetRead`] reads every preset of the private copy under the collection's shared lock: its
//! stored parameter vector and the field that vector came from (R2), its desired retention, its
//! decks and its count of non-new cards (R1). The defaults a proposal carries are the ones the
//! engine reports through its deck-options read (R3, R11), so this crate names no scheduler
//! package. The read reaches the engine through the port's own `open`, by type inference only: this
//! module names no path into the engine's crate and none of its write methods (R8, A8).
//!
//! [`PresetStore`] records a proposal with the values it would replace (R4) and settles it once the
//! owner's change has synced back (R7). Nothing here writes to the collection: the owner
//! applies the proposal in their own Anki app, on ADR-301's advisory rung (ADR-401 D2, D12).

use std::path::Path;

use chrono::{DateTime, Local, TimeZone};
use deck_streak_kernel::{Db, KernelError, UtcMillis};
use sqlx::SqliteConnection;

use crate::engine::{EngineError, RslibEngine, bounded, open};
use crate::lock::CollectionLock;
use crate::settings::SyncSettings;

/// How many rows of the copy's config hold the rollover hour: zero when it is unset. The engine's
/// own rollover read answers 4 for an unset hour, so it cannot tell the two apart.
const ROLLOVER_ROWS: &str = "SELECT count() FROM config WHERE key = 'rollover'";

/// The count of non-new cards whose memory state a parameter change recomputes, for the decks the
/// JSON array `?1` names: the engine's own recompute set on save, a card homed in one of those
/// decks or borrowed from one by a filtered deck, that is not new (R1; SPEC-387 section 1c).
const NON_NEW_CARDS: &str = "SELECT count() FROM cards \
    WHERE (did IN (SELECT value FROM json_each(?1)) \
    OR (odid != 0 AND odid IN (SELECT value FROM json_each(?1)))) AND type != 0";

/// The field of a preset a stored vector came from (R2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParameterField {
    /// The current generation's field.
    Fsrs6,
    /// The previous generation's field.
    Fsrs5,
    /// The oldest generation's field.
    Fsrs4,
    /// No field holds values.
    Empty,
}

impl ParameterField {
    /// The field's name as the record stores it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Fsrs6 => "fsrs6",
            Self::Fsrs5 => "fsrs5",
            Self::Fsrs4 => "fsrs4",
            Self::Empty => "empty",
        }
    }

    /// The field the record's name `text` names.
    fn parse(text: &str) -> Option<Self> {
        [Self::Fsrs6, Self::Fsrs5, Self::Fsrs4, Self::Empty]
            .into_iter()
            .find(|field| field.as_str() == text)
    }
}

/// One preset of the copy, as the read returns it (R1).
#[derive(Debug, Clone, PartialEq)]
pub struct Preset {
    /// The preset's id in the collection.
    pub id: i64,
    /// The preset's name.
    pub name: String,
    /// The stored parameter vector, from [`Preset::field`]; empty when no field holds values.
    pub vector: Vec<f32>,
    /// The field the vector came from.
    pub field: ParameterField,
    /// The preset's desired retention.
    pub desired_retention: f32,
    /// The ids of the decks that name the preset, ascending.
    pub deck_ids: Vec<i64>,
    /// The count of non-new cards whose memory state a parameter change recomputes.
    pub non_new_cards: i64,
}

/// Every preset of the copy, and the defaults the engine reports (R1, R3).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PresetSnapshot {
    /// Every preset, ascending by id.
    pub presets: Vec<Preset>,
    /// The released scheduler's default parameters, as the engine's deck-options read reports
    /// them.
    pub defaults: Vec<f32>,
}

impl PresetSnapshot {
    /// The preset with the id `id`.
    #[must_use]
    pub fn preset(&self, id: i64) -> Option<&Preset> {
        self.presets.iter().find(|preset| preset.id == id)
    }
}

/// Why a preset path stopped. No variant carries text from the engine.
#[derive(Debug, thiserror::Error)]
pub enum PresetError {
    /// The engine failed.
    #[error(transparent)]
    Engine(#[from] EngineError),
    /// The copy does not exist yet, so there is nothing to read and nothing is created.
    #[error("the private copy does not exist yet")]
    NoCopy,
    /// The copy's configured UTC offset differs from the process's zone: the engine's day
    /// computation would rewrite it, so the read refuses before it (ADR-401 D13).
    #[error("the copy's configured UTC offset differs from the process's zone")]
    ZoneDiffers,
    /// The copy holds no rollover hour: the engine's day computation would write one, so the read
    /// refuses before it (ADR-401 D13).
    #[error("the copy holds no rollover hour")]
    RolloverUnset,
    /// The copy holds no normal deck, which the engine's deck-options read needs.
    #[error("the copy holds no normal deck")]
    NoNormalDeck,
    /// The engine reported no default parameters.
    #[error("the engine reported no default parameters")]
    NoDefaults,
    /// No preset of the copy has the id asked for.
    #[error("no preset has that id")]
    UnknownPreset,
    /// No proposal has the id asked for.
    #[error("no proposal has that id")]
    UnknownProposal,
    /// A proposal's record does not have the shape this module writes.
    #[error("a proposal's record is malformed")]
    Malformed,
    /// The collection's lock could not be taken or released.
    #[error("the collection's lock could not be taken")]
    Lock,
    /// The record's database failed.
    #[error(transparent)]
    Database(#[from] KernelError),
}

/// The preset read port (R1, ADR-401 D9). Only [`RslibEngine`] implements it.
pub trait PresetRead {
    /// Every preset of the copy at `collection`, and the engine's defaults, read at `now`.
    ///
    /// # Errors
    ///
    /// [`PresetError::NoCopy`] when the copy does not exist, [`PresetError::ZoneDiffers`] or
    /// [`PresetError::RolloverUnset`] before any engine read that would write the copy, and
    /// [`PresetError::Engine`] when the engine fails.
    fn presets(&self, collection: &Path, now: UtcMillis) -> Result<PresetSnapshot, PresetError>;
}

impl PresetRead for RslibEngine {
    fn presets(&self, collection: &Path, now: UtcMillis) -> Result<PresetSnapshot, PresetError> {
        if !collection.is_file() {
            return Err(PresetError::NoCopy);
        }
        let mut col = open(collection)?;
        // ADR-401 D13: the deck-options read computes the engine's day first, which in client mode
        // rewrites a configured offset that differs from the process's zone and writes a rollover
        // hour the copy lacks. The offset is read as the skip's facts check reads it
        // (`CollectionWrite::facts` in engine.rs, compared in skip_write.rs's zone check).
        if col.get_configured_utc_offset() != Some(offset_west(now)) {
            return Err(PresetError::ZoneDiffers);
        }
        let rollover: i64 = col
            .storage
            .db()
            .query_row(ROLLOVER_ROWS, [], |row| row.get(0))
            .map_err(|_| EngineError::EngineFailed)?;
        if rollover == 0 {
            return Err(PresetError::RolloverUnset);
        }
        // Every normal deck, with the preset it names; a filtered deck names none.
        let mut decks = Vec::new();
        for (deck, _) in col.get_all_deck_names(false).map_err(bounded)? {
            let preset = col
                .get_deck(deck)
                .map_err(bounded)?
                .and_then(|found| found.config_id());
            if let Some(preset) = preset {
                decks.push((deck, preset.0));
            }
        }
        let normal = decks
            .first()
            .map(|(deck, _)| *deck)
            .ok_or(PresetError::NoNormalDeck)?;
        let options = col.get_deck_configs_for_update(normal).map_err(bounded)?;
        let defaults = options
            .defaults
            .and_then(|defaults| defaults.config)
            .map(|config| config.fsrs_params_6)
            .unwrap_or_default();
        if defaults.is_empty() {
            return Err(PresetError::NoDefaults);
        }
        // The deck-options read lists every preset, but pre-fills an empty FSRS-6 field from the
        // older ones, so each preset's own fields are read from its stored config.
        let ids: Vec<i64> = options
            .all_config
            .into_iter()
            .filter_map(|listed| listed.config)
            .map(|config| config.id)
            .collect();
        let mut presets = Vec::with_capacity(ids.len());
        for id in ids {
            let Some(stored) = col.get_deck_config(id.into(), false).map_err(bounded)? else {
                continue;
            };
            let (vector, field) = stored_vector(
                &stored.inner.fsrs_params_6,
                &stored.inner.fsrs_params_5,
                &stored.inner.fsrs_params_4,
            );
            let mut deck_ids: Vec<i64> = decks
                .iter()
                .filter(|(_, preset)| *preset == id)
                .map(|(deck, _)| deck.0)
                .collect();
            deck_ids.sort_unstable();
            let listed = format!(
                "[{}]",
                deck_ids
                    .iter()
                    .map(i64::to_string)
                    .collect::<Vec<_>>()
                    .join(",")
            );
            let non_new_cards: i64 = col
                .storage
                .db()
                .query_row(NON_NEW_CARDS, [listed], |row| row.get(0))
                .map_err(|_| EngineError::EngineFailed)?;
            presets.push(Preset {
                id,
                name: stored.name,
                vector,
                field,
                desired_retention: stored.inner.desired_retention,
                deck_ids,
                non_new_cards,
            });
        }
        presets.sort_by_key(|preset| preset.id);
        col.close(None).map_err(bounded)?;
        Ok(PresetSnapshot { presets, defaults })
    }
}

/// The stored vector and the field it came from: the first field that holds values, the FSRS-6
/// one, then the FSRS-5 one, then the FSRS-4 one, else empty, the precedence the engine schedules
/// with (R2).
fn stored_vector(fsrs6: &[f32], fsrs5: &[f32], fsrs4: &[f32]) -> (Vec<f32>, ParameterField) {
    [
        (fsrs6, ParameterField::Fsrs6),
        (fsrs5, ParameterField::Fsrs5),
        (fsrs4, ParameterField::Fsrs4),
    ]
    .into_iter()
    .find(|(values, _)| !values.is_empty())
    .map_or((Vec::new(), ParameterField::Empty), |(values, field)| {
        (values.to_vec(), field)
    })
}

/// Whether `vector` holds exactly the values of `other`, compared bit for bit: a value that
/// parses back to another number is another vector (R4, R7).
fn same(vector: &[f32], other: &[f32]) -> bool {
    vector.len() == other.len()
        && vector
            .iter()
            .zip(other)
            .all(|(value, other)| value.to_bits() == other.to_bits())
}

/// Reads every preset of the copy `settings` names under the collection's shared lock, on the
/// blocking pool (R1).
///
/// # Errors
///
/// [`PresetError::Lock`] when the lock cannot be taken or released, and every refusal of
/// [`PresetRead::presets`].
pub async fn read_presets<R>(
    reader: &R,
    settings: &SyncSettings,
    now: UtcMillis,
) -> Result<PresetSnapshot, PresetError>
where
    R: PresetRead + Clone + Send + 'static,
{
    let held = CollectionLock::new(settings.lock_path())
        .shared()
        .await
        .map_err(|_| PresetError::Lock)?;
    let reader = reader.clone();
    let copy = settings.copy_path();
    let read = match tokio::task::spawn_blocking(move || reader.presets(&copy, now)).await {
        Ok(read) => read,
        Err(error) => std::panic::resume_unwind(
            error
                .try_into_panic()
                .unwrap_or_else(|error| Box::new(error.to_string())),
        ),
    };
    let released = held.release().map_err(|_| PresetError::Lock);
    let snapshot = read?;
    released?;
    Ok(snapshot)
}

/// The process's zone offset at `now`, in minutes WEST of UTC as the engine stores the configured
/// UTC offset, read through chrono's `Local` as the engine and the skip read it.
fn offset_west(now: UtcMillis) -> i32 {
    DateTime::from_timestamp_millis(now.epoch_millis()).map_or(i32::MIN, |at| {
        -(Local
            .offset_from_utc_datetime(&at.naive_utc())
            .local_minus_utc()
            / 60)
    })
}

/// Where a proposal stands (R7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProposalState {
    /// Recorded, and not yet seen applied.
    Open,
    /// The copy holds the proposed vector.
    Moved,
    /// The copy holds neither the prior vector nor the proposed one.
    Diverged,
}

impl ProposalState {
    /// The state's name as the record stores it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Moved => "moved",
            Self::Diverged => "diverged",
        }
    }

    /// The state the record's name `text` names.
    fn parse(text: &str) -> Option<Self> {
        [Self::Open, Self::Moved, Self::Diverged]
            .into_iter()
            .find(|state| state.as_str() == text)
    }
}

/// One proposal, as recorded (R4, R7).
#[derive(Debug, Clone, PartialEq)]
pub struct Proposal {
    /// The record's id.
    pub id: i64,
    /// The preset it names.
    pub preset_id: i64,
    /// The preset's name when it was proposed.
    pub preset_name: String,
    /// The stored vector the proposal would replace: the undo values.
    pub prior_vector: Vec<f32>,
    /// The field the prior vector came from.
    pub prior_field: ParameterField,
    /// The proposed vector: the engine's defaults.
    pub proposed_vector: Vec<f32>,
    /// The preset's desired retention when it was proposed.
    pub desired_retention: f32,
    /// The count of non-new cards whose memory state the change recomputes.
    pub non_new_cards: i64,
    /// Where it stands.
    pub state: ProposalState,
    /// When it was seen settled, in epoch milliseconds: the preset's change point.
    pub settled_at: Option<i64>,
    /// Whether the desired retention was unchanged when it was seen moved.
    pub retention_kept: Option<bool>,
    /// When it was recorded, in epoch milliseconds.
    pub created_at: i64,
}

/// What `propose` did (R4).
#[derive(Debug, Clone, PartialEq)]
pub enum ProposeOutcome {
    /// The preset is already on the defaults; nothing was recorded.
    OnDefaults,
    /// A new proposal was recorded.
    Recorded(Proposal),
    /// The preset already has this open proposal; nothing was recorded.
    AlreadyOpen(Proposal),
}

/// What `verify` did (R7).
#[derive(Debug, Clone, PartialEq)]
pub enum VerifyOutcome {
    /// The proposal was settled now, as it reads after the settle.
    Settled(Proposal),
    /// Nothing was recorded: the proposal is still open, or was settled before.
    Unchanged(Proposal),
}

/// The proposal record, over the service's database (R4, R7, R9).
#[derive(Clone)]
pub struct PresetStore {
    db: Db,
}

impl PresetStore {
    /// A store over `db`.
    #[must_use]
    pub fn new(db: Db) -> Self {
        Self { db }
    }

    /// Proposes the defaults `defaults` for `preset`, at `now` (R4). A preset already on the
    /// defaults records nothing, and neither does one with an open proposal: that proposal is
    /// answered again. The lookup and the insert share one immediate transaction, and the partial
    /// unique index refuses a second open row whatever path reaches it.
    ///
    /// # Errors
    ///
    /// [`PresetError::Database`] when the read or the write fails; nothing is recorded then.
    pub async fn propose(
        &self,
        preset: &Preset,
        defaults: &[f32],
        now: UtcMillis,
    ) -> Result<ProposeOutcome, PresetError> {
        if let Some(outcome) = same(&preset.vector, defaults).then_some(ProposeOutcome::OnDefaults)
        {
            return Ok(outcome);
        }
        let mut write = self.db.write().await?;
        if let Some(open) = open_proposal(&mut write, preset.id).await? {
            return Ok(ProposeOutcome::AlreadyOpen(open));
        }
        let proposed = defaults.to_vec();
        let prior = encode(&preset.vector);
        let field = preset.field.as_str();
        let offered = encode(&proposed);
        let retention = f64::from(preset.desired_retention);
        let created_at = now.epoch_millis();
        let id = sqlx::query_scalar!(
            r#"INSERT INTO preset_proposals (preset_id, preset_name, prior_vector, prior_field,
                   proposed_vector, desired_retention, non_new_cards, state, created_at)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'open', ?8) RETURNING id AS "id!""#,
            preset.id,
            preset.name,
            prior,
            field,
            offered,
            retention,
            preset.non_new_cards,
            created_at
        )
        .fetch_one(&mut *write)
        .await
        .map_err(KernelError::from)?;
        write.commit().await.map_err(KernelError::from)?;
        Ok(ProposeOutcome::Recorded(Proposal {
            id,
            preset_id: preset.id,
            preset_name: preset.name.clone(),
            prior_vector: preset.vector.clone(),
            prior_field: preset.field,
            proposed_vector: proposed,
            desired_retention: preset.desired_retention,
            non_new_cards: preset.non_new_cards,
            state: ProposalState::Open,
            settled_at: None,
            retention_kept: None,
            created_at,
        }))
    }

    /// Settles the proposal `proposal` by what the snapshot `snapshot` holds for its preset, at
    /// `now` (R7): `moved` when the preset holds the proposed vector, with whether its desired
    /// retention was kept; still open, recording nothing, while it holds the prior vector; and
    /// `diverged` when it holds neither, or the copy no longer holds the preset. The settle is
    /// guarded by the row's open state in the same immediate transaction, so a settled proposal
    /// is never settled again and is answered as it stands.
    ///
    /// # Errors
    ///
    /// [`PresetError::UnknownProposal`] when no proposal has the id, and
    /// [`PresetError::Database`] when the read or the write fails.
    pub async fn verify(
        &self,
        snapshot: &PresetSnapshot,
        proposal: i64,
        now: UtcMillis,
    ) -> Result<VerifyOutcome, PresetError> {
        let mut write = self.db.write().await?;
        let found = proposal_on(&mut write, proposal)
            .await?
            .ok_or(PresetError::UnknownProposal)?;
        let (state, retention_kept) = match snapshot.preset(found.preset_id) {
            Some(held) if same(&held.vector, &found.proposed_vector) => (
                ProposalState::Moved,
                Some(i64::from(
                    held.desired_retention.to_bits() == found.desired_retention.to_bits(),
                )),
            ),
            Some(held) if same(&held.vector, &found.prior_vector) => (ProposalState::Open, None),
            _ => (ProposalState::Diverged, None),
        };
        if state == ProposalState::Open {
            write.commit().await.map_err(KernelError::from)?;
            return Ok(VerifyOutcome::Unchanged(found));
        }
        let settled_at = now.epoch_millis();
        let name = state.as_str();
        let settled = sqlx::query!(
            "UPDATE preset_proposals SET state = ?2, settled_at = ?3, retention_kept = ?4 \
             WHERE id = ?1 AND state = 'open'",
            proposal,
            name,
            settled_at,
            retention_kept
        )
        .execute(&mut *write)
        .await
        .map_err(KernelError::from)?
        .rows_affected();
        let read = proposal_on(&mut write, proposal)
            .await?
            .ok_or(PresetError::UnknownProposal)?;
        write.commit().await.map_err(KernelError::from)?;
        Ok(if settled == 1 {
            VerifyOutcome::Settled(read)
        } else {
            VerifyOutcome::Unchanged(read)
        })
    }
}

/// A proposal's row, as the record holds it.
struct ProposalRow {
    id: i64,
    preset_id: i64,
    preset_name: String,
    prior_vector: String,
    prior_field: String,
    proposed_vector: String,
    desired_retention: f64,
    non_new_cards: i64,
    state: String,
    settled_at: Option<i64>,
    retention_kept: Option<i64>,
    created_at: i64,
}

impl ProposalRow {
    /// The proposal the row records.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the column holds an f32 widened exactly, so narrowing it restores the value"
    )]
    fn proposal(self) -> Result<Proposal, PresetError> {
        Ok(Proposal {
            id: self.id,
            preset_id: self.preset_id,
            preset_name: self.preset_name,
            prior_vector: decode(&self.prior_vector).ok_or(PresetError::Malformed)?,
            prior_field: ParameterField::parse(&self.prior_field).ok_or(PresetError::Malformed)?,
            proposed_vector: decode(&self.proposed_vector).ok_or(PresetError::Malformed)?,
            desired_retention: self.desired_retention as f32,
            non_new_cards: self.non_new_cards,
            state: ProposalState::parse(&self.state).ok_or(PresetError::Malformed)?,
            settled_at: self.settled_at,
            retention_kept: self.retention_kept.map(|kept| kept != 0),
            created_at: self.created_at,
        })
    }
}

/// The proposal `id`, when one has it.
async fn proposal_on(
    connection: &mut SqliteConnection,
    id: i64,
) -> Result<Option<Proposal>, PresetError> {
    let row = sqlx::query_as!(
        ProposalRow,
        r#"SELECT id AS "id!", preset_id, preset_name, prior_vector, prior_field, proposed_vector,
                  desired_retention, non_new_cards, state, settled_at, retention_kept, created_at
           FROM preset_proposals WHERE id = ?1"#,
        id
    )
    .fetch_optional(connection)
    .await
    .map_err(KernelError::from)?;
    row.map(ProposalRow::proposal).transpose()
}

/// The open proposal for the preset `preset`, when it has one: the partial unique index holds at
/// most one.
async fn open_proposal(
    connection: &mut SqliteConnection,
    preset: i64,
) -> Result<Option<Proposal>, PresetError> {
    let row = sqlx::query_as!(
        ProposalRow,
        r#"SELECT id AS "id!", preset_id, preset_name, prior_vector, prior_field, proposed_vector,
                  desired_retention, non_new_cards, state, settled_at, retention_kept, created_at
           FROM preset_proposals WHERE preset_id = ?1 AND state = 'open'"#,
        preset
    )
    .fetch_optional(connection)
    .await
    .map_err(KernelError::from)?;
    row.map(ProposalRow::proposal).transpose()
}

/// A vector as the record stores it: a JSON array of numbers, each printed so that it parses back
/// to the same value.
fn encode(vector: &[f32]) -> String {
    let values: Vec<String> = vector.iter().map(f32::to_string).collect();
    format!("[{}]", values.join(","))
}

/// The vector `text` records, as [`encode`] wrote it.
fn decode(text: &str) -> Option<Vec<f32>> {
    let inner = text.strip_prefix('[')?.strip_suffix(']')?;
    if inner.is_empty() {
        return Some(Vec::new());
    }
    inner.split(',').map(|value| value.parse().ok()).collect()
}

/// What the owner asked the preset role for (ADR-401 D10).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresetCommand {
    /// Every preset, the main one first (R5).
    List,
    /// A proposal for the preset with this id (R4).
    Propose(i64),
    /// A settle of the proposal with this id (R7).
    Verify(i64),
}

/// Answers `command` over the copy `settings` names, read by `reader`, and the record in `db`, at
/// `now`: the text the role prints. The copy is read once, under the collection's shared lock.
///
/// # Errors
///
/// Every refusal of [`read_presets`], [`PresetError::UnknownPreset`] when a proposal names a
/// preset the copy does not hold, and every refusal of the store's `propose` and `verify`.
pub async fn answer<R>(
    reader: &R,
    db: &Db,
    settings: &SyncSettings,
    command: PresetCommand,
    now: UtcMillis,
) -> Result<String, PresetError>
where
    R: PresetRead + Clone + Send + 'static,
{
    let snapshot = read_presets(reader, settings, now).await?;
    let store = PresetStore::new(db.clone());
    Ok(match command {
        PresetCommand::List => listing(&snapshot).join("\n"),
        PresetCommand::Propose(id) => {
            let preset = snapshot.preset(id).ok_or(PresetError::UnknownPreset)?;
            let outcome = store.propose(preset, &snapshot.defaults, now).await?;
            propose_text(preset, &outcome)
        }
        PresetCommand::Verify(id) => verify_text(&store.verify(&snapshot, id, now).await?),
    })
}

/// The values of `vector` on one line, comma-separated, each printed as the shortest text that
/// parses back to the same value (R6).
fn values_line(vector: &[f32]) -> String {
    vector
        .iter()
        .map(|value| format!("{value}"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The proposal's text for the owner (R6): the preset, the values, the steps in their own Anki
/// app, what the change recomputes and what it leaves, a replaced fit, and the undo.
#[must_use]
pub fn proposal_text(proposal: &Proposal) -> String {
    let mut lines = vec![
        format!(
            "Preset {} {}: move its FSRS parameters to the scheduler's defaults.",
            proposal.preset_id, proposal.preset_name
        ),
        format!("Values: {}", values_line(&proposal.proposed_vector)),
        "In your Anki app, open this preset's options and paste the values above into its FSRS \
         parameters."
            .to_owned(),
        "Leave \"Reschedule cards on change\" off.".to_owned(),
        "Keep the preset's other options, and leave desired retention as it is.".to_owned(),
        "Save, then sync.".to_owned(),
        format!(
            "What changes: the memory state of {} non-new card(s) is recomputed from their own \
             reviews.",
            proposal.non_new_cards
        ),
        "No review, due date or interval changes.".to_owned(),
    ];
    if proposal.prior_field == ParameterField::Fsrs6 && !proposal.prior_vector.is_empty() {
        lines.push("These defaults replace a fit of the current scheduler generation.".to_owned());
    }
    lines.push(if proposal.prior_vector.is_empty() {
        "Undo: clear the parameters box.".to_owned()
    } else {
        format!(
            "Undo: paste these values back: {}",
            values_line(&proposal.prior_vector)
        )
    });
    lines.join("\n")
}

/// What `propose` answered `outcome` for `preset`, for the owner (R4).
#[must_use]
pub fn propose_text(preset: &Preset, outcome: &ProposeOutcome) -> String {
    match outcome {
        ProposeOutcome::OnDefaults => format!(
            "{} is already on the scheduler's defaults; nothing is recorded.",
            preset.name
        ),
        ProposeOutcome::Recorded(proposal) => format!(
            "Proposal {} recorded.\n{}",
            proposal.id,
            proposal_text(proposal)
        ),
        ProposeOutcome::AlreadyOpen(proposal) => format!(
            "Proposal {} is already open for this preset; nothing is recorded.\n{}",
            proposal.id,
            proposal_text(proposal)
        ),
    }
}

/// What `verify` answered, for the owner (R7).
#[must_use]
pub fn verify_text(outcome: &VerifyOutcome) -> String {
    let (proposal, settled_now) = match outcome {
        VerifyOutcome::Settled(proposal) => (proposal, true),
        VerifyOutcome::Unchanged(proposal) => (proposal, false),
    };
    let at = proposal.settled_at.unwrap_or_default();
    match (proposal.state, settled_now) {
        (ProposalState::Open, _) => format!(
            "Proposal {} is still open: as of the last sync the preset still holds the values \
             it would replace. Nothing is recorded.",
            proposal.id
        ),
        (ProposalState::Moved, true) => format!(
            "Proposal {} moved at {at}: the preset holds the proposed values, desired retention \
             {}.",
            proposal.id,
            if proposal.retention_kept == Some(true) {
                "unchanged"
            } else {
                "changed"
            }
        ),
        (ProposalState::Diverged, true) => format!(
            "Proposal {} diverged at {at}: the preset holds neither the values it replaced nor \
             the proposed ones.",
            proposal.id
        ),
        (state, false) => format!(
            "Proposal {} is already settled {} at {at}; nothing is recorded.",
            proposal.id,
            state.as_str()
        ),
    }
}

/// One line per preset, the one with the most non-new cards first, then by id (R5).
#[must_use]
pub fn listing(snapshot: &PresetSnapshot) -> Vec<String> {
    let mut presets: Vec<&Preset> = snapshot.presets.iter().collect();
    presets.sort_by(|a, b| {
        b.non_new_cards
            .cmp(&a.non_new_cards)
            .then_with(|| a.id.cmp(&b.id))
    });
    presets
        .into_iter()
        .map(|preset| {
            format!(
                "{} {}: field {}, {} the defaults, desired retention {}, {} deck(s), {} non-new \
                 card(s)",
                preset.id,
                preset.name,
                preset.field.as_str(),
                if same(&preset.vector, &snapshot.defaults) {
                    "on"
                } else {
                    "not on"
                },
                preset.desired_retention,
                preset.deck_ids.len(),
                preset.non_new_cards
            )
        })
        .collect()
}
