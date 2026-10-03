//! The skip day's write to the collection (SPEC-083 R20 to R27, R36; ADR-321 D14 to D20): the only
//! module beside the port that names the write port, [`CollectionWrite`] (A24).
//!
//! In this part it holds the preview (R20): the class's stop read first (R36, A54), then, under the
//! SHARED collection lock on the private copy, the cards the wrapped search (R3) selects, each with
//! its top-level deck and its current due, and the list's digest (D20) that a confirm carries and
//! the take compares. The preview writes nothing.

use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::{DateTime, Datelike, Local, NaiveDate, TimeZone};

use sha2::{Digest, Sha256};

use deck_streak_kernel::{CredentialLoader, Db, ForeignDb, StudyDay, StudyDayRule, UtcMillis};

use crate::engine::{CollectionWrite, DueCard, EngineDay, EngineError, SyncLogin, WriteSync};
use crate::lock::CollectionLock;
use crate::reader::is_study_event;
use crate::settings::{SYNC_PASSWORD, SYNC_USERNAME, SkipSearch, SyncSettings};
use crate::skip::{
    CardState, FailReason, SKIP_MAX_CARDS, SKIP_SPREAD_MAX_DAYS, SKIP_SPREAD_MIN_DAYS,
    SearchRefusal, SkipId, SkipStore, skip_search, skip_spec,
};
use crate::write_class_stop::{ClassStop, WriteClassStop};

/// One card the preview lists (R20): its id, its top-level deck's id and its current due.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PreviewCard {
    /// The card's id.
    pub id: i64,
    /// The id of the top-level deck the card's home deck sits under.
    pub top_level_deck: i64,
    /// The card's current due, as the engine stores it.
    pub due: i64,
}

/// What the preview shows the owner.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Preview {
    /// The class's stop is set, or could not be read: nothing is listed, and the stop says who set
    /// it, why and since when (A54).
    Stopped(ClassStop),
    /// A check of R3 refused: the zone is not pinned, observes daylight saving or differs from the
    /// collection's, or the engine's day is not the study day. Nothing was listed.
    Refused(FailReason),
    /// The cards the wrapped search selects, ascending by id, and their digest (D20).
    Listed {
        /// The cards, ascending by id.
        cards: Vec<PreviewCard>,
        /// The digest of the listed ids ([`list_digest`]).
        digest: String,
    },
}

/// Why the preview listed nothing.
#[derive(Debug, thiserror::Error)]
pub enum PreviewError {
    /// The configured search is not one expression (R3).
    #[error("the skip search is refused")]
    Search(#[from] SearchRefusal),
    /// The collection lock could not be taken or released.
    #[error("the collection lock failed")]
    Lock(#[source] io::Error),
    /// The engine could not read the private copy.
    #[error("the engine could not read the private copy")]
    Engine(#[from] EngineError),
}

/// The lowercase hexadecimal digits, by value.
const HEX_DIGITS: &[u8; 16] = b"0123456789abcdef";

/// The digest of a list of card ids (D20): the first 128 bits of SHA-256 over the ids in ascending
/// order, each written in decimal and ended by a line feed, as 32 lowercase hexadecimal digits.
/// The order the ids arrive in does not change it.
#[must_use]
pub fn list_digest(ids: &[i64]) -> String {
    let mut ascending = ids.to_vec();
    ascending.sort_unstable();
    let mut hasher = Sha256::new();
    for id in ascending {
        hasher.update(format!("{id}\n").as_bytes());
    }
    let mut hex = String::with_capacity(32);
    for byte in &hasher.finalize()[..16] {
        hex.push(char::from(HEX_DIGITS[usize::from(byte >> 4)]));
        hex.push(char::from(HEX_DIGITS[usize::from(byte & 0x0f)]));
    }
    hex
}

/// The preview (R20): the class's stop first, and while it is set nothing else is read; otherwise,
/// under the shared collection lock on the private copy, the cards `search`'s wrap selects and
/// their digest. It writes nothing.
///
/// # Errors
///
/// [`PreviewError::Search`] when the search is not one expression; [`PreviewError::Lock`] when the
/// lock cannot be taken or released; [`PreviewError::Engine`] when the engine cannot read the copy.
pub async fn preview<W: CollectionWrite + Clone + Send + Sync + 'static>(
    writer: &W,
    settings: &SyncSettings,
    search: &SkipSearch,
    stop: &WriteClassStop,
    day: StudyDay,
    rule: StudyDayRule,
) -> Result<Preview, PreviewError> {
    let read = stop.read_stop().await;
    if read.is_stopped() {
        return Ok(Preview::Stopped(read));
    }
    let now = UtcMillis::from_system_time(std::time::SystemTime::now());
    if let Err(reason) = zone_checks(writer, &settings.copy_path(), day, rule, now).await {
        return Ok(Preview::Refused(reason));
    }
    let wrapped = skip_search(search.as_str())?;
    let held = CollectionLock::new(settings.lock_path())
        .shared()
        .await
        .map_err(PreviewError::Lock)?;
    let due = on_pool(writer, &settings.copy_path(), move |writer, path| {
        writer.due_cards(path, &wrapped)
    })
    .await;
    held.release().map_err(PreviewError::Lock)?;
    let cards: Vec<PreviewCard> = due?
        .into_iter()
        .map(|card| PreviewCard {
            id: card.id,
            top_level_deck: card.top_level_deck,
            due: card.due,
        })
        .collect();
    let ids: Vec<i64> = cards.iter().map(|card| card.id).collect();
    Ok(Preview::Listed {
        digest: list_digest(&ids),
        cards,
    })
}

/// The four points between the take's steps where a test acts (D19), each given the path the
/// step left: the working copy after the converge, the partial backup after its write, the working
/// copy after the prior state's commit and before the push. Production passes [`NoHooks`].
pub trait TakeHooks: Send + Sync {
    /// After the converge, before the working copy is checked again.
    fn after_converge(&self, _working: &Path) {}
    /// After the backup's partial file is written, before its restore check.
    fn after_backup(&self, _partial: &Path) {}
    /// After the prior state is committed, before the reschedule.
    fn after_snapshot(&self, _working: &Path) {}
    /// Before the class's stop is read again and the push begins.
    fn before_push(&self, _working: &Path) {}
}

/// The hooks production passes: none acts.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoHooks;

impl TakeHooks for NoHooks {}

/// What a take reads and writes beside the collection.
pub struct TakePorts<'a, W> {
    /// The engine's write port.
    pub writer: &'a W,
    /// The deployment's settings: the private copy, its lock and the endpoint.
    pub settings: &'a SyncSettings,
    /// The configured skip search (R3).
    pub search: &'a SkipSearch,
    /// The class's stop (R36).
    pub stop: &'a WriteClassStop,
    /// The record of skips: the begun row, the prior state and the left state.
    pub store: &'a SkipStore,
    /// The loader of the sync login's two credentials.
    pub credentials: &'a CredentialLoader,
}

/// One take, on the row its caller began (`SkipStore::begin`).
#[derive(Clone, Debug)]
pub struct TakeRequest {
    /// The begun row.
    pub skip: SkipId,
    /// The study day the skip covers.
    pub day: StudyDay,
    /// The study-day rule the day was read under.
    pub rule: StudyDayRule,
    /// The digest the owner's confirm carries, if any (R20).
    pub digest: Option<String>,
    /// The take's instant: the stop's time when a count sets it.
    pub now: UtcMillis,
}

/// What a take answers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TakeAnswer {
    /// The server accepted the push. `moved` are the cards rescheduled, `left_alone` the cards
    /// the preview and the converge did not agree on, `read_back` the moved cards whose state the
    /// push did not keep or that were studied after the converge (R27).
    Accepted {
        /// The cards moved, ascending by id.
        moved: Vec<i64>,
        /// The cards left alone, ascending by id.
        left_alone: Vec<i64>,
        /// The moved cards the read-back lists, ascending by id.
        read_back: Vec<i64>,
    },
    /// The take failed with `reason`, which its row records; a changed list carries the new
    /// preview (R20).
    Failed {
        /// Why.
        reason: FailReason,
        /// The new preview, when the list changed.
        preview: Option<Preview>,
    },
    /// The push began and its outcome is not known yet: the row stays `pending` (R25, R26).
    NotKnownYet,
}

/// The take (R21 to R27, R34 to R36; ADR-321 D14 to D20), on the row its caller began, under the
/// exclusive collection lock. Every refusal and failure settles the row `failed` with its one code
/// (P6), except a push whose outcome is not known, which leaves it `pending` (R25). Every exit
/// discards the working copy, and a backup is kept only after its restore check passed. The take
/// never settles `applied`: coordination does (ADR-321 D10).
pub async fn take<W>(
    ports: &TakePorts<'_, W>,
    request: &TakeRequest,
    hooks: Arc<dyn TakeHooks>,
) -> TakeAnswer
where
    W: CollectionWrite + Clone + Send + Sync + 'static,
{
    let held = match ports.stop.read_stop().await {
        stop if stop.is_stopped() => Err((FailReason::WritesStopped, None)),
        _ => CollectionLock::new(ports.settings.lock_path())
            .exclusive()
            .await
            .map_err(|_| (FailReason::EngineFailed, None)),
    };
    let answer = match held {
        Ok(held) => {
            let answer = steps(ports, request, hooks).await;
            let released = held.release();
            match (answer, released) {
                (answer, Ok(())) => answer,
                (Ok(TakeAnswer::Failed { reason, preview }), Err(_)) => Err((reason, preview)),
                (Ok(answer), Err(_)) => Ok(answer),
                (Err(failed), Err(_)) => Err(failed),
            }
        }
        Err(failed) => Err(failed),
    };
    match answer {
        Ok(answer) => answer,
        Err((reason, preview)) => {
            if ports
                .store
                .settle_failed(request.skip, reason)
                .await
                .is_err()
            {
                tracing::warn!(
                    reason = reason.as_str(),
                    "the take's failure was not recorded"
                );
            }
            TakeAnswer::Failed { reason, preview }
        }
    }
}

/// A take's failure: its code and, when the list changed, the new preview.
type Failure = (FailReason, Option<Preview>);

/// The take's steps 2 to 17, under the lock its caller holds.
async fn steps<W>(
    ports: &TakePorts<'_, W>,
    request: &TakeRequest,
    hooks: Arc<dyn TakeHooks>,
) -> Result<TakeAnswer, Failure>
where
    W: CollectionWrite + Clone + Send + Sync + 'static,
{
    let failed = |reason: FailReason| (reason, None);
    let private = ports.settings.copy_path();
    day_and_zone(ports.writer, &private, request)
        .await
        .map_err(failed)?;
    let wrapped =
        skip_search(ports.search.as_str()).map_err(|_| failed(FailReason::EngineFailed))?;

    let previewed_ids =
        bound_to_preview(ports.writer, &private, &wrapped, request.digest.as_deref()).await?;

    // The working copy and the converge (R21).
    let directory = private
        .parent()
        .map_or_else(PathBuf::new, Path::to_path_buf);
    let working = directory.join(format!("{WORKING_PREFIX}{}.anki2", request.skip.get()));
    let mut discard = Discard(vec![working.clone()]);
    std::fs::copy(&private, &working).map_err(|_| failed(FailReason::EngineFailed))?;
    let login = login(ports).map_err(failed)?;
    converge_outcome(ports.writer.write_sync(&working, &login).await).map_err(failed)?;
    let converged = now_millis();
    call_hook(&hooks, &working, |hooks, path| hooks.after_converge(path)).await;
    day_and_zone(ports.writer, &working, request)
        .await
        .map_err(failed)?;

    // What moves: the previewed cards the search still selects (R21); the rest is left alone.
    let (moving, left_alone) = select(ports.writer, &working, &wrapped, &previewed_ids)
        .await
        .map_err(failed)?;
    let moved: Vec<i64> = moving.iter().map(|card| card.card_id).collect();
    if moved.is_empty() {
        return Ok(TakeAnswer::Accepted {
            moved,
            left_alone,
            read_back: Vec::new(),
        });
    }

    // The counts before, the backup and its restore check (R34, R35).
    let before = counts(ports.writer, &working, &wrapped)
        .await
        .map_err(failed)?;
    backup(
        ports.writer,
        request.skip,
        &working,
        &wrapped,
        &hooks,
        &mut discard,
    )
    .await
    .map_err(failed)?;

    reschedule(ports, request, &hooks, &working, &moving)
        .await
        .map_err(failed)?;

    // The counts after (R35): any other count that moved stops the class.
    let after = counts(ports.writer, &working, &wrapped)
        .await
        .map_err(failed)?;
    let moved_count = i64::try_from(moved.len()).unwrap_or(i64::MAX);
    if let Some(count) = moved_counts(&before, &after, moved_count) {
        if ports.stop.set_by_counts(count, request.now).await.is_err() {
            tracing::warn!(count, "a moved count could not set the class's stop");
        }
        return Err(failed(FailReason::CountsMoved));
    }
    let left = cards_by_id(ports.writer, &working, &moved)
        .await
        .map_err(failed)?;
    ports
        .store
        .record_left(request.skip, &left)
        .await
        .map_err(|_| failed(FailReason::EngineFailed))?;

    // The stop again, then the push (R24 to R26, R36).
    call_hook(&hooks, &working, |hooks, path| hooks.before_push(path)).await;
    if ports.stop.read_stop().await.is_stopped() {
        return Err(failed(FailReason::WritesStopped));
    }
    match ports.writer.write_sync(&working, &login).await {
        WriteSync::Accepted => {}
        WriteSync::FullSyncRequired { .. } => return Err(failed(FailReason::FullSyncRequired)),
        WriteSync::NotStarted(_) => return Err(failed(FailReason::PushFailed)),
        WriteSync::Unknown(_) => return Ok(TakeAnswer::NotKnownYet),
    }

    // The read-back (R27).
    let read_back = read_back(ports.writer, &working, &left, converged).await;
    Ok(TakeAnswer::Accepted {
        moved,
        left_alone,
        read_back,
    })
}

/// The converge's answer (R21): only an accepted exchange lets the take go on. A full-sync demand
/// aborts before the snapshot, the backup or a due change, because a full upload would replace the
/// server's collection with a copy the owner never saw.
fn converge_outcome(outcome: WriteSync) -> Result<(), FailReason> {
    match outcome {
        WriteSync::Accepted => Ok(()),
        WriteSync::FullSyncRequired { .. } => Err(FailReason::FullSyncRequired),
        WriteSync::NotStarted(_) | WriteSync::Unknown(_) => Err(FailReason::ConvergeFailed),
    }
}

/// The preview binds the take (R20): the list read again from the private copy at `private`, its
/// digest compared with the one the confirm carries, and a list over the guard refused (R33).
/// Answers the listed ids.
async fn bound_to_preview<W>(
    writer: &W,
    private: &Path,
    wrapped: &str,
    confirmed: Option<&str>,
) -> Result<Vec<i64>, Failure>
where
    W: CollectionWrite + Clone + Send + Sync + 'static,
{
    let search = wrapped.to_owned();
    let previewed = on_pool(writer, private, move |writer, path| {
        writer.due_cards(path, &search)
    })
    .await
    .map_err(|_| (FailReason::EngineFailed, None))?;
    let ids: Vec<i64> = previewed.iter().map(|card| card.id).collect();
    let digest = list_digest(&ids);
    if confirmed != Some(digest.as_str()) {
        let cards = previewed.iter().map(preview_card).collect();
        return Err((
            FailReason::PreviewChanged,
            Some(Preview::Listed { cards, digest }),
        ));
    }
    if ids.len() >= SKIP_MAX_CARDS {
        return Err((FailReason::TooManyCards, None));
    }
    Ok(ids)
}

/// The prior state of `moving` committed first (R22), then the engine's own reschedule of those
/// cards on the working copy at `working` (R23).
async fn reschedule<W>(
    ports: &TakePorts<'_, W>,
    request: &TakeRequest,
    hooks: &Arc<dyn TakeHooks>,
    working: &Path,
    moving: &[CardState],
) -> Result<(), FailReason>
where
    W: CollectionWrite + Clone + Send + Sync + 'static,
{
    ports
        .store
        .record_prior(request.skip, moving, request.now)
        .await
        .map_err(|_| FailReason::EngineFailed)?;
    call_hook(hooks, working, |hooks, path| hooks.after_snapshot(path)).await;
    let moved: Vec<i64> = moving.iter().map(|card| card.card_id).collect();
    let spec = skip_spec(SKIP_SPREAD_MIN_DAYS, SKIP_SPREAD_MAX_DAYS);
    on_pool(ports.writer, working, move |writer, path| {
        writer.set_due_date(path, &moved, &spec)
    })
    .await
    .map_err(|_| FailReason::WriteFailed)
}

/// The take's selection (R21) on the converged working copy at `working`: the cards the wrapped
/// search still selects that the preview listed, which move, and every other card either names,
/// left alone, ascending by id.
async fn select<W>(
    writer: &W,
    working: &Path,
    wrapped: &str,
    previewed: &[i64],
) -> Result<(Vec<CardState>, Vec<i64>), FailReason>
where
    W: CollectionWrite + Clone + Send + Sync + 'static,
{
    let search = wrapped.to_owned();
    let selected = on_pool(writer, working, move |writer, path| {
        writer.due_cards(path, &search)
    })
    .await
    .map_err(|_| FailReason::EngineFailed)?;
    let moving: Vec<CardState> = selected
        .iter()
        .filter(|card| previewed.contains(&card.id))
        .map(card_state)
        .collect();
    let mut left_alone: Vec<i64> = previewed
        .iter()
        .copied()
        .chain(selected.iter().map(|card| card.id))
        .filter(|id| !moving.iter().any(|card| card.card_id == *id))
        .collect();
    left_alone.sort_unstable();
    left_alone.dedup();
    Ok((moving, left_alone))
}

/// The take's backup (R34, D17): the working copy at `working` written beside it as a partial
/// file, owner-only, checked, and only then renamed into place, after which the older backups go.
/// A partial file that fails is left to `discard`.
async fn backup<W>(
    writer: &W,
    skip: SkipId,
    working: &Path,
    wrapped: &str,
    hooks: &Arc<dyn TakeHooks>,
    discard: &mut Discard,
) -> Result<(), FailReason>
where
    W: CollectionWrite + Clone + Send + Sync + 'static,
{
    let directory = working
        .parent()
        .map_or_else(PathBuf::new, Path::to_path_buf);
    let partial = directory.join(format!(
        "{BACKUP_PREFIX}{}{BACKUP_SUFFIX}{PARTIAL_SUFFIX}",
        skip.get()
    ));
    discard.0.push(partial.clone());
    std::fs::copy(working, &partial).map_err(|_| FailReason::BackupFailed)?;
    std::fs::set_permissions(&partial, std::fs::Permissions::from_mode(BACKUP_MODE))
        .map_err(|_| FailReason::BackupFailed)?;
    call_hook(hooks, &partial, |hooks, path| hooks.after_backup(path)).await;
    if !restore_check(writer, &partial, working, wrapped).await {
        return Err(FailReason::BackupCheckFailed);
    }
    let kept = directory.join(format!("{BACKUP_PREFIX}{}{BACKUP_SUFFIX}", skip.get()));
    std::fs::rename(&partial, &kept).map_err(|_| FailReason::BackupFailed)?;
    discard.0.retain(|path| *path != partial);
    remove_older_backups(&directory, &kept);
    Ok(())
}

/// The name of a take's working copy beside the private copy, before its skip's id.
const WORKING_PREFIX: &str = "skip-working-";
/// The name of a skip's backup beside the private copy, before its skip's id (R34).
const BACKUP_PREFIX: &str = "skip-backup-";
/// The name of a skip's backup after its skip's id.
const BACKUP_SUFFIX: &str = ".anki2";
/// The suffix of a backup written and not yet checked.
const PARTIAL_SUFFIX: &str = ".partial";
/// The suffix of the throwaway copy the restore check opens.
const CHECK_SUFFIX: &str = ".check";
/// The backup's mode: the owner's only, whatever the umask (R34, A53).
const BACKUP_MODE: u32 = 0o600;

/// Removes the files it holds when the take ends, however it ends: the working copy, and a backup
/// not yet checked.
struct Discard(Vec<PathBuf>);

impl Drop for Discard {
    fn drop(&mut self) {
        for path in &self.0 {
            remove_with_journal(path);
        }
    }
}

/// Removes the file at `path` and any journal the engine left beside it.
fn remove_with_journal(path: &Path) {
    for suffix in ["", "-wal", "-shm", "-journal"] {
        let mut name = path.as_os_str().to_owned();
        name.push(suffix);
        let _ = std::fs::remove_file(PathBuf::from(name));
    }
}

/// Whether `name` is a skip's backup or a partial one: `skip-backup-<id>.anki2`, then optionally
/// `.partial`, the id one or more decimal digits.
fn is_backup_name(name: &str) -> bool {
    let whole = name.strip_suffix(PARTIAL_SUFFIX).unwrap_or(name);
    whole
        .strip_prefix(BACKUP_PREFIX)
        .and_then(|rest| rest.strip_suffix(BACKUP_SUFFIX))
        .is_some_and(|id| !id.is_empty() && id.bytes().all(|byte| byte.is_ascii_digit()))
}

/// Removes every checked backup in `directory` but `kept` (R34: one at a time).
fn remove_older_backups(directory: &Path, kept: &Path) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let older = name
            .to_str()
            .is_some_and(|name| is_backup_name(name) && !name.ends_with(PARTIAL_SUFFIX));
        if older && path != kept && entry.file_type().is_ok_and(|kind| kind.is_file()) {
            let _ = std::fs::remove_file(path);
        }
    }
}

/// The sync login, from the two credentials the kernel's loader reads, as the syncer builds it.
fn login<W>(ports: &TakePorts<'_, W>) -> Result<SyncLogin, FailReason> {
    let username = ports
        .credentials
        .load(SYNC_USERNAME)
        .map_err(|_| FailReason::ConvergeFailed)?;
    let password = ports
        .credentials
        .load(SYNC_PASSWORD)
        .map_err(|_| FailReason::ConvergeFailed)?;
    Ok(SyncLogin::new(
        ports.settings.endpoint().as_str(),
        username.expose(),
        password.expose(),
    ))
}

/// The instant now, in milliseconds.
fn now_millis() -> i64 {
    UtcMillis::from_system_time(std::time::SystemTime::now()).epoch_millis()
}

/// Runs `work` on the blocking pool with a clone of `writer` and the path `path`: the engine's
/// calls block, so none runs on the runtime's own threads.
async fn on_pool<W, T>(
    writer: &W,
    path: &Path,
    work: impl FnOnce(&W, &Path) -> T + Send + 'static,
) -> T
where
    W: Clone + Send + 'static,
    T: Send + 'static,
{
    let writer = writer.clone();
    let path = path.to_path_buf();
    match tokio::task::spawn_blocking(move || work(&writer, &path)).await {
        Ok(value) => value,
        Err(error) => std::panic::resume_unwind(
            error
                .try_into_panic()
                .unwrap_or_else(|error| Box::new(error.to_string())),
        ),
    }
}

/// Calls one hook on the blocking pool, so a hook may block as a test's other client does.
async fn call_hook(
    hooks: &Arc<dyn TakeHooks>,
    path: &Path,
    point: impl FnOnce(&dyn TakeHooks, &Path) + Send + 'static,
) {
    let hooks = Arc::clone(hooks);
    let path = path.to_path_buf();
    if let Err(error) = tokio::task::spawn_blocking(move || point(hooks.as_ref(), &path)).await {
        std::panic::resume_unwind(
            error
                .try_into_panic()
                .unwrap_or_else(|error| Box::new(error.to_string())),
        );
    }
}

/// One listed card as the preview shows it.
const fn preview_card(card: &DueCard) -> PreviewCard {
    PreviewCard {
        id: card.id,
        top_level_deck: card.top_level_deck,
        due: card.due,
    }
}

/// One card's scheduling state as the record keeps it.
const fn card_state(card: &DueCard) -> CardState {
    CardState {
        card_id: card.id,
        due: card.due,
        queue: card.queue,
        kind: card.kind,
        interval: card.interval,
        ease_factor: card.factor,
        original_deck: card.original_deck,
        original_due: card.original_due,
        mtime: card.mtime,
    }
}

/// The cards `ids` of the collection at `path`, by id, read without a day computation.
async fn cards_by_id<W>(writer: &W, path: &Path, ids: &[i64]) -> Result<Vec<CardState>, FailReason>
where
    W: CollectionWrite + Clone + Send + Sync + 'static,
{
    let search = format!(
        "cid:{}",
        ids.iter().map(i64::to_string).collect::<Vec<_>>().join(",")
    );
    let cards = on_pool(writer, path, move |writer, path| {
        writer.due_cards(path, &search)
    })
    .await
    .map_err(|_| FailReason::EngineFailed)?;
    Ok(cards.iter().map(card_state).collect())
}

/// The read-back (R27): each moved card whose state after the push is not the state `left` the
/// skip wrote, or whose review log holds a study event after the converge at `converged`.
async fn read_back<W>(writer: &W, path: &Path, left: &[CardState], converged: i64) -> Vec<i64>
where
    W: CollectionWrite + Clone + Send + Sync + 'static,
{
    let ids: Vec<i64> = left.iter().map(|card| card.card_id).collect();
    let Ok(now) = cards_by_id(writer, path, &ids).await else {
        return ids;
    };
    let studied = studied_after(path, converged).await;
    let mut listed: Vec<i64> = left
        .iter()
        .filter(|wrote| {
            !now.contains(wrote)
                || studied
                    .as_ref()
                    .is_none_or(|studied| studied.contains(&wrote.card_id))
        })
        .map(|card| card.card_id)
        .collect();
    listed.sort_unstable();
    listed
}

/// The cards of the collection at `path` with a study event after `after`, or `None` when the
/// review log cannot be read.
async fn studied_after(path: &Path, after: i64) -> Option<Vec<i64>> {
    let db = Db::open_foreign_read_only(path).await.ok()?;
    let rows =
        sqlx::query_as::<_, (i64, i64, i64)>("SELECT cid, type, ease FROM revlog WHERE id > ?1")
            .bind(after)
            .fetch_all(db.reader())
            .await;
    db.close().await;
    Some(
        rows.ok()?
            .into_iter()
            .filter(|(_, kind, ease)| is_study_event(*kind, *ease))
            .map(|(card, _, _)| card)
            .collect(),
    )
}

/// R35's counts of the closed collection at `path`, the due count through the wrapped search.
async fn counts<W>(writer: &W, path: &Path, wrapped: &str) -> Result<Counts, FailReason>
where
    W: CollectionWrite + Clone + Send + Sync + 'static,
{
    let search = wrapped.to_owned();
    let due = on_pool(writer, path, move |writer, path| {
        writer.due_cards(path, &search)
    })
    .await
    .map_err(|_| FailReason::EngineFailed)?
    .len();
    let db = Db::open_foreign_read_only(path)
        .await
        .map_err(|_| FailReason::EngineFailed)?;
    let read = read_counts(&db, i64::try_from(due).unwrap_or(i64::MAX)).await;
    db.close().await;
    read.map_err(|_| FailReason::EngineFailed)
}

/// The counts' five reads, on one read-only connection, beside the due count `due` the engine
/// read.
async fn read_counts(db: &ForeignDb, due: i64) -> Result<Counts, sqlx::Error> {
    let reader = db.reader();
    let cards = sqlx::query_scalar::<_, i64>("SELECT count(*) FROM cards")
        .fetch_one(reader)
        .await?;
    let notes = sqlx::query_scalar::<_, i64>("SELECT count(*) FROM notes")
        .fetch_one(reader)
        .await?;
    let rows = sqlx::query_scalar::<_, i64>("SELECT count(*) FROM revlog")
        .fetch_one(reader)
        .await?;
    let reschedules =
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM revlog WHERE type = 4 AND ease = 0")
            .fetch_one(reader)
            .await?;
    let by_kind = sqlx::query_as::<_, (i64, i64, i64)>(
        "SELECT queue, type, count(*) FROM cards GROUP BY queue, type ORDER BY queue, type",
    )
    .fetch_all(reader)
    .await?
    .into_iter()
    .map(|(queue, kind, count)| ((queue, kind), count))
    .collect();
    Ok(Counts {
        cards,
        notes,
        review_log_rows: rows,
        reschedule_rows: reschedules,
        cards_by_queue_and_type: by_kind,
        due,
    })
}

/// The backup's restore check (D17): the partial backup at `partial` holds the working copy at
/// `working` byte for byte, and a throwaway copy of it, opened by the engine, counts as the working
/// copy does. The backup itself is never opened, because the engine's open may write.
pub async fn restore_check<W>(writer: &W, partial: &Path, working: &Path, wrapped: &str) -> bool
where
    W: CollectionWrite + Clone + Send + Sync + 'static,
{
    let (Ok(backup), Ok(copy)) = (std::fs::read(partial), std::fs::read(working)) else {
        return false;
    };
    if Sha256::digest(&backup) != Sha256::digest(&copy) {
        return false;
    }
    let mut name = partial.as_os_str().to_owned();
    name.push(CHECK_SUFFIX);
    let throwaway = PathBuf::from(name);
    let restored = match std::fs::write(&throwaway, &backup) {
        Ok(()) => counts(writer, &throwaway, wrapped).await.ok(),
        Err(_) => None,
    };
    remove_with_journal(&throwaway);
    let Some(restored) = restored else {
        return false;
    };
    counts(writer, working, wrapped)
        .await
        .is_ok_and(|counted| counted == restored)
}

/// R35's counts of one collection.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    /// Cards.
    pub cards: i64,
    /// Notes.
    pub notes: i64,
    /// Review-log rows.
    pub review_log_rows: i64,
    /// Review-log rows of type 4 with ease 0, the reschedule's own.
    pub reschedule_rows: i64,
    /// Cards per (queue, type), ascending by key.
    pub cards_by_queue_and_type: Vec<((i64, i64), i64)>,
    /// The cards the wrapped search selects.
    pub due: i64,
}

/// The name of the first count that moved other than as a reschedule of `moved` cards moves it
/// (R35): the review-log rows up by `moved`, each new row the reschedule's own, and the due count
/// down by `moved`; every other count equal. `None` when only those moved.
#[must_use]
pub fn moved_counts(before: &Counts, after: &Counts, moved: i64) -> Option<&'static str> {
    if after.cards != before.cards {
        return Some("cards");
    }
    if after.notes != before.notes {
        return Some("notes");
    }
    if after.cards_by_queue_and_type != before.cards_by_queue_and_type {
        return Some("cards_by_queue_and_type");
    }
    if after.review_log_rows != before.review_log_rows * moved {
        return Some("review_log_rows");
    }
    if after.reschedule_rows != before.reschedule_rows + moved {
        return Some("reschedule_rows");
    }
    if after.due != before.due - moved {
        return Some("due");
    }
    None
}

/// Removes every skip backup and partial backup in `directory`, the private copy's, and no other
/// file (A56), answering how many it removed.
///
/// # Errors
///
/// The first error listing the directory or removing a file.
pub fn erase_backups(directory: &Path) -> io::Result<usize> {
    let mut removed = 0;
    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        let named = entry.file_name().to_str().is_some_and(is_backup_name);
        if named && entry.file_type()?.is_file() {
            std::fs::remove_file(entry.path())?;
            removed += 1;
        }
    }
    Ok(removed)
}

/// The milliseconds of a day, a minute and an hour.
const DAY_MS: i64 = 86_400_000;
const MINUTE_MS: i64 = 60_000;
const HOUR_MS: i64 = 3_600_000;
/// How often the daylight-saving check reads the zone's offset over two calendar years, in
/// seconds: often enough to see a daylight period of one hour.
const DAYLIGHT_SAMPLE_SECS: usize = 1_800;

/// R3's checks of the collection at `path`, in their order: the pin, daylight saving, the
/// configured UTC offset read without a day computation (a missing one differs), and only then
/// the engine's day against the study day.
async fn day_and_zone<W>(writer: &W, path: &Path, request: &TakeRequest) -> Result<(), FailReason>
where
    W: CollectionWrite + Clone + Send + Sync + 'static,
{
    zone_checks(writer, path, request.day, request.rule, request.now).await
}

/// R3's checks, shared by the preview and the take.
async fn zone_checks<W>(
    writer: &W,
    path: &Path,
    day: StudyDay,
    rule: StudyDayRule,
    now: UtcMillis,
) -> Result<(), FailReason>
where
    W: CollectionWrite + Clone + Send + Sync + 'static,
{
    if !zone_pin() {
        return Err(FailReason::ZoneNotPinned);
    }
    if observes_daylight_saving(now) {
        return Err(FailReason::ZoneObservesDaylightSaving);
    }
    let facts = on_pool(writer, path, W::facts)
        .await
        .map_err(|_| FailReason::EngineFailed)?;
    if facts.utc_offset_west != Some(offset_west(now)) {
        return Err(FailReason::ZoneDiffers);
    }
    let engine = on_pool(writer, path, W::engine_day)
        .await
        .map_err(|_| FailReason::EngineFailed)?;
    if !is_the_study_day(engine, day, rule) {
        return Err(FailReason::EngineDayDiffers);
    }
    Ok(())
}

/// Whether the engine's day `engine` is the study day `day` under `rule`: both end at the same
/// instant, so both began at the same rollover.
fn is_the_study_day(engine: EngineDay, day: StudyDay, rule: StudyDayRule) -> bool {
    let end = day
        .epoch_day()
        .checked_add(1)
        .and_then(|next| next.checked_mul(DAY_MS))
        .and_then(|next| next.checked_sub(i64::from(rule.utc_offset().minutes()) * MINUTE_MS))
        .and_then(|next| next.checked_add(i64::from(rule.rollover_hour().get()) * HOUR_MS));
    end.is_some() && engine.next_day_at.checked_mul(1_000) == end
}

/// The process's zone offset at `now`, in minutes WEST of UTC as the engine stores the configured
/// UTC offset, read through chrono's `Local` as the engine reads it.
fn offset_west(now: UtcMillis) -> i32 {
    DateTime::from_timestamp_millis(now.epoch_millis()).map_or(i32::MIN, |at| {
        -(Local
            .offset_from_utc_datetime(&at.naive_utc())
            .local_minus_utc()
            / 60)
    })
}

/// Whether the process's zone observes daylight saving (R3): its offset, read through chrono's
/// `Local`, is not the same at every instant of the current and the next calendar year.
fn observes_daylight_saving(now: UtcMillis) -> bool {
    let Some(at) = DateTime::from_timestamp_millis(now.epoch_millis()) else {
        return true;
    };
    let year = at.year();
    let bound = |year: i32| {
        NaiveDate::from_ymd_opt(year, 1, 1)
            .and_then(|day| day.and_hms_opt(0, 0, 0))
            .map(|start| start.and_utc().timestamp())
    };
    let (Some(start), Some(end)) = (bound(year), bound(year + 2)) else {
        return true;
    };
    let offset = |secs: i64| {
        DateTime::from_timestamp(secs, 0).map(|at| {
            Local
                .offset_from_utc_datetime(&at.naive_utc())
                .local_minus_utc()
        })
    };
    let first = offset(start);
    (start..end)
        .step_by(DAYLIGHT_SAMPLE_SECS)
        .any(|secs| offset(secs) != first)
}

/// Whether the process's zone is pinned (R3, A44): `TZ` holds a POSIX rule that names no zone
/// file. The one reader of `TZ` and of the zone directories in this module.
fn zone_pin() -> bool {
    const ZONE_DIRECTORIES: [&str; 4] = [
        "/usr/share/zoneinfo",
        "/usr/lib/zoneinfo",
        "/usr/share/lib/zoneinfo",
        "/etc/zoneinfo",
    ];
    let Some(value) = std::env::var_os("TZ") else {
        return false;
    };
    let Some(value) = value.to_str() else {
        return false;
    };
    let shaped = !value.is_empty()
        && value != "localtime"
        && !value.starts_with(':')
        && !value.starts_with('/')
        && !value.split('/').any(|part| part == "..");
    shaped
        && !ZONE_DIRECTORIES
            .iter()
            .any(|directory| Path::new(directory).join(value).exists())
        && is_posix_rule(value)
}

/// Whether `rule` parses as a POSIX zone rule: a name, an offset, and optionally a daylight name,
/// its offset and the two transitions (`std offset [dst [offset] [,start[/time],end[/time]]]`).
fn is_posix_rule(rule: &str) -> bool {
    let Some(rest) = zone_name(rule).and_then(|rest| clock(rest, 24)) else {
        return false;
    };
    if rest.is_empty() {
        return true;
    }
    let Some(rest) = zone_name(rest) else {
        return false;
    };
    let rest = clock(rest, 24).unwrap_or(rest);
    if rest.is_empty() {
        return true;
    }
    rest.strip_prefix(',')
        .and_then(|rest| rest.split_once(','))
        .is_some_and(|(start, end)| transition(start) && transition(end))
}

/// A rule's zone name: three or more letters, or three or more of letters, digits, `+` and `-`
/// between `<` and `>`. Answers the text after it.
fn zone_name(text: &str) -> Option<&str> {
    if let Some(quoted) = text.strip_prefix('<') {
        let (name, rest) = quoted.split_once('>')?;
        let named = name.len() >= 3
            && name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'+' || byte == b'-');
        return named.then_some(rest);
    }
    let letters = text.bytes().take_while(u8::is_ascii_alphabetic).count();
    (letters >= 3).then(|| &text[letters..])
}

/// A rule's clock, `[+-]hh[:mm[:ss]]`, its hours at most `hours`. Answers the text after it.
fn clock(text: &str, hours: u32) -> Option<&str> {
    let mut rest = text.strip_prefix(['+', '-']).unwrap_or(text);
    for part in 0..3 {
        if part > 0 {
            match rest.strip_prefix(':') {
                Some(after) => rest = after,
                None => break,
            }
        }
        let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
        let limit = if part == 0 { hours } else { 59 };
        if !(1..=2).contains(&digits) || !in_range(&rest[..digits], 0, limit) {
            return None;
        }
        rest = &rest[digits..];
    }
    Some(rest)
}

/// A rule's transition: `Jn` (1 to 365), `n` (0 to 365) or `Mm.w.d`, then optionally `/time`.
fn transition(text: &str) -> bool {
    let (date, time) = text
        .split_once('/')
        .map_or((text, None), |(date, time)| (date, Some(time)));
    let dated = if let Some(day) = date.strip_prefix('J') {
        in_range(day, 1, 365)
    } else if let Some(month) = date.strip_prefix('M') {
        let parts: Vec<&str> = month.split('.').collect();
        parts.len() == 3
            && in_range(parts[0], 1, 12)
            && in_range(parts[1], 1, 5)
            && in_range(parts[2], 0, 6)
    } else {
        in_range(date, 0, 365)
    };
    dated && time.is_none_or(|time| clock(time, 167) == Some(""))
}

/// Whether `text` is decimal digits whose value lies in `low..=high`.
fn in_range(text: &str, low: u32, high: u32) -> bool {
    !text.is_empty()
        && text.bytes().all(|byte| byte.is_ascii_digit())
        && text
            .parse::<u32>()
            .is_ok_and(|value| (low..=high.max(99)).contains(&value))
}
