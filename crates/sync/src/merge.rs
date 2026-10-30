//! Concurrent edits of one register: the greater key wins, and every loser is kept (SPEC-509 R7;
//! ADR-527 D5).
//!
//! A register is one note field, one deck's name, one notetype part or one card's deck. An edit
//! carries the key of the operation that wrote it. The server is the one judge: an edit made on
//! the register's current value moves it forward, and any other edit is concurrent with it, so
//! the greater of the two wins and the other is recorded as a conflict. A set of concurrent edits
//! sharing one base therefore ends at the edit with the greatest key, with every other edit kept,
//! whatever order they arrive in.

use std::cmp::Ordering;

use crate::key::{Key, compare};

/// An edit of one register: the key of the operation that wrote it, and the value it writes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Edit<V> {
    /// The key of the operation that wrote the value.
    pub key: Key,
    /// The value written.
    pub value: V,
}

/// The edit order: by key, then by value, so two edits compare equal only when they are equal.
fn edit_order<V: Ord>(a: &Edit<V>, b: &Edit<V>) -> Ordering {
    compare(&a.key, &b.key).then_with(|| a.value.cmp(&b.value))
}

/// The greater of two edits first, then the other: the winner and the loser.
fn pick<V: Ord>(a: Edit<V>, b: Edit<V>) -> (Edit<V>, Edit<V>) {
    (b, a)
}

/// Merges two registers: the greater edit wins, and an empty register yields to the other.
#[must_use]
pub fn merge<V: Ord>(a: Option<Edit<V>>, b: Option<Edit<V>>) -> Option<Edit<V>> {
    match (a, b) {
        (None, other) | (other, None) => other,
        (Some(a), Some(b)) => Some(pick(a, b).0),
    }
}

/// Folds a register's edits into it from the last to the first: the same edit for every order of
/// the same edits.
#[must_use]
pub fn apply<V: Ord>(edits: Vec<Edit<V>>) -> Option<Edit<V>> {
    edits
        .into_iter()
        .rev()
        .fold(None, |register, edit| merge(Some(edit), register))
}

/// A conflict record: an edit that lost to a concurrent one, kept with the winner's key, so
/// nothing is lost silently.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Conflict<V> {
    /// The edit that lost, value and key.
    pub loser: Edit<V>,
    /// The key of the edit it lost to.
    pub winner: Key,
}

/// One register as the judge holds it: its current edit and every conflict it recorded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Register<V> {
    current: Option<Edit<V>>,
    conflicts: Vec<Conflict<V>>,
}

impl<V> Default for Register<V> {
    fn default() -> Self {
        Self {
            current: None,
            conflicts: Vec::new(),
        }
    }
}

impl<V: Ord> Register<V> {
    /// A register never written.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The register's current edit, if it was ever written.
    #[must_use]
    pub const fn current(&self) -> Option<&Edit<V>> {
        self.current.as_ref()
    }

    /// Every conflict recorded, in the order it was recorded.
    #[must_use]
    pub fn conflicts(&self) -> &[Conflict<V>] {
        &self.conflicts
    }

    /// Takes `edit`, made on `base`: the key of the register's edit the device had applied, or
    /// `None` for a register it never saw written. An edit made on the current edit moves the
    /// register forward; any other is concurrent with it, and the greater wins with the loser
    /// recorded.
    pub fn receive(&mut self, edit: Edit<V>, base: Option<Key>) {
        if base == self.current.as_ref().map(|held| held.key) {
            self.current = Some(edit);
            return;
        }
        match self.current.take() {
            None => self.current = Some(edit),
            Some(held) => {
                let (winner, loser) = pick(held, edit);
                self.conflicts.push(Conflict {
                    loser,
                    winner: winner.key,
                });
                self.current = Some(winner);
            }
        }
    }
}
