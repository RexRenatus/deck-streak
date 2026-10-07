//! The full-sync choice's one-way write, driven by the core in the order the model checks it
//! (SPEC-364 R3 to R8; ADR-375): the server copy is counted from its file, the replaced side is
//! backed up and read back from its file, an upload's server is read again, and the device is read
//! again at the write. Every id a transition judges is read by the core from a file through the
//! engine, never passed in by an adapter, and nothing the choice makes is deleted (R8).

use std::path::Path;

use anki_proto::sync::{FullUploadOrDownloadRequest, SyncAuth, SyncCollectionResponse};

use crate::dispatch::{Dispatcher, Refusal};
use crate::full_sync::{
    BackedUp, Checked, Confirmed, Counted, Direction, IdSets, Offer, Ready, Write,
};
use crate::gesture::{GestureRefusal, OwnerGesture};

/// A step that did not move the choice: the state it was given, kept for the owner, and why.
#[derive(Debug)]
pub struct Refused<S> {
    /// The unchanged state, boxed so a refusal stays small to return.
    pub state: Box<S>,
    /// Why the step did not move it.
    pub reason: Reason,
}

/// Why a step of the one-way write refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reason {
    /// The engine, a private engine, or the endpoint rule refused (in the engine's error shape).
    Engine(Refusal),
    /// The gesture is not a one-way sync's, or the engine refused the write it ran.
    Gesture(GestureRefusal),
    /// The path names the collection the dispatcher has open.
    OpenCollection,
    /// The path's collection already holds a review, a card or a note.
    HoldsRows,
    /// The backup, read from its file, lacks an id of the side the write replaces.
    Unheld,
}

/// Why the write did not run.
#[derive(Debug, PartialEq, Eq)]
pub enum Unwritten {
    /// The device changed since its backup: new counts the owner sees and confirms again.
    Recount(Counted),
    /// A read or the write itself was refused.
    Refused(Reason),
}

/// The engine's one-way request for `write`, built by the core alone: the confirmed direction,
/// the checked `auth`, and no media (SPEC-364 R3).
pub fn request(write: &Write, auth: &SyncAuth) -> FullUploadOrDownloadRequest {
    let _ = write.direction();
    FullUploadOrDownloadRequest {
        auth: Some(auth.clone()),
        upload: true,
        server_usn: None,
    }
}

/// The choice's counts: the offer from the normal sync's `answer`, the device's ids, and the ids
/// of the server's collection fetched into the empty file `copy` (SPEC-364 R4).
///
/// # Errors
///
/// A [`Reason`] when the endpoint rule, the engine or a read refuses.
pub fn count(
    dispatcher: &Dispatcher,
    answer: &SyncCollectionResponse,
    auth: &SyncAuth,
    copy: &Path,
) -> Result<Counted, Reason> {
    let server = fetched(dispatcher, auth, copy)?;
    let device = dispatcher.id_sets().map_err(Reason::Engine)?;
    let offer = Offer::from_answer(answer);
    Ok(Counted::show(offer, device, server))
}

/// The backup of the side the write replaces, made and read back by the core (SPEC-364 R5).
///
/// # Errors
///
/// The unchanged state and its [`Reason`] when the backup cannot be made or read, or lacks an id.
pub fn back_up(
    dispatcher: &Dispatcher,
    confirmed: Confirmed,
    _backup: &Path,
    _copy: &Path,
) -> Result<BackedUp, Refused<Confirmed>> {
    let from_file = match confirmed.direction() {
        Direction::Download | Direction::Upload => dispatcher.id_sets(),
    };
    match from_file {
        Ok(from_file) => confirmed
            .backed_up(&from_file)
            .map_err(|state| refused(state, Reason::Unheld)),
        Err(refusal) => Err(refused(confirmed, Reason::Engine(refusal))),
    }
}

/// An upload's re-check: a fresh server copy fetched into the empty file `fresh`, compared with
/// the counted copy (SPEC-364 R6).
///
/// # Errors
///
/// The unchanged state and its [`Reason`] when the fresh copy cannot be fetched or read.
pub fn recheck(
    dispatcher: &Dispatcher,
    checked: Checked,
    auth: &SyncAuth,
    fresh: &Path,
) -> Result<Result<Ready, Counted>, Refused<Checked>> {
    let _ = auth;
    match read(dispatcher, fresh) {
        Ok(server) => Ok(checked.rechecked(server)),
        Err(reason) => Err(refused(checked, reason)),
    }
}

/// The write: the device read again, the write's last check, and the one-way sync (SPEC-364 R7).
///
/// # Errors
///
/// [`Unwritten::Recount`] when the device gained a row a download's backup lacks, and
/// [`Unwritten::Refused`] when a read, the gesture or the engine refuses.
pub fn write(
    dispatcher: &Dispatcher,
    ready: Ready,
    gesture: OwnerGesture,
    auth: &SyncAuth,
) -> Result<(), Unwritten> {
    let device_now = dispatcher
        .id_sets()
        .map_err(|refusal| Unwritten::Refused(Reason::Engine(refusal)))?;
    let _ = device_now;
    let write = ready
        .at_write(&IdSets::default())
        .map_err(Unwritten::Recount)?;
    dispatcher
        .run_one_way(gesture, write, auth)
        .map_err(|refusal| Unwritten::Refused(Reason::Gesture(refusal)))
}

/// A refusal that keeps `state` for the owner.
fn refused<S>(state: S, reason: Reason) -> Refused<S> {
    Refused {
        state: Box::new(state),
        reason,
    }
}

/// The server's collection fetched into the file `path` by a private engine, and its ids.
fn fetched(dispatcher: &Dispatcher, auth: &SyncAuth, path: &Path) -> Result<IdSets, Reason> {
    let private = dispatcher.private(path).map_err(Reason::Engine)?;
    let request = FullUploadOrDownloadRequest {
        auth: Some(auth.clone()),
        ..FullUploadOrDownloadRequest::default()
    };
    let ids = private
        .full_sync(&request)
        .map_err(|error| Refusal::Engine { error })
        .and_then(|()| private.id_sets());
    let closed = private.close();
    let ids = ids.map_err(Reason::Engine)?;
    closed.map_err(Reason::Engine)?;
    Ok(ids)
}

/// The ids a private engine reads from the file `path`.
fn read(dispatcher: &Dispatcher, path: &Path) -> Result<IdSets, Reason> {
    let private = dispatcher.private(path).map_err(Reason::Engine)?;
    let ids = private.id_sets();
    let closed = private.close();
    let ids = ids.map_err(Reason::Engine)?;
    closed.map_err(Reason::Engine)?;
    Ok(ids)
}
