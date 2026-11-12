//! The declared write class's stop (SPEC-083 R36; ADR-301 (c), ADR-321 D16): one ledger row,
//! `write_class_stop` (`migrations/008303_ingest_write_class_stop.sql`), saying whether every
//! declared write class is stopped, who set it, why and since when. While it is set nothing writes
//! to the collection: the preview says so, and the take and the undo refuse before any request or
//! write.
//!
//! The read is fail-closed: a missing row, a row that does not hold a setter, or a read that fails
//! reads as stopped, because a stop nobody can read must not let a write through. A count that
//! moved sets the stop here; only the owner's command clears it (E4c), so this module has no clear
//! path, and the data-rights erase leaves the row alone (it is exempt).

use deck_streak_kernel::{Db, KernelError, UtcMillis};

/// Who set the class's stop: the take's own counts, or the owner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StopSetter {
    /// A count the take checks moved where only the moved cards may move it (R35).
    Counts,
    /// The owner, by the bot's owner-only command (E4c).
    Owner,
}

impl StopSetter {
    /// The setter as the row stores it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Counts => "counts",
            Self::Owner => "owner",
        }
    }

    /// The setter the row's text names, if any.
    fn from_column(text: &str) -> Option<Self> {
        match text {
            "counts" => Some(Self::Counts),
            "owner" => Some(Self::Owner),
            _ => None,
        }
    }
}

/// The class's stop, as one read found it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClassStop {
    /// Not stopped: a declared write class may write.
    Running,
    /// Stopped by `set_by`, for `reason`, since `since`.
    Stopped {
        /// Who set the stop.
        set_by: StopSetter,
        /// Why: the name of the count that moved, or the owner's reason.
        reason: String,
        /// When the stop was set.
        since: UtcMillis,
    },
    /// The row is missing, does not hold a setter, or could not be read: read as stopped.
    Unread,
}

impl ClassStop {
    /// Whether this read stops every write: only [`ClassStop::Running`] lets one through.
    #[must_use]
    pub const fn is_stopped(&self) -> bool {
        !matches!(self, Self::Running)
    }
}

/// The stop's one row in the ledger.
#[derive(Clone, Debug)]
pub struct WriteClassStop {
    db: Db,
}

impl WriteClassStop {
    /// The stop's row in `db`.
    #[must_use]
    pub fn new(db: Db) -> Self {
        Self { db }
    }

    /// Reads the class's stop, fail-closed: a missing row, a stopped row with no setter, or a read
    /// that fails is [`ClassStop::Unread`], which stops every write as a set stop does.
    pub async fn read_stop(&self) -> ClassStop {
        let read = sqlx::query!(
            r#"SELECT stopped, set_by, reason, set_at FROM write_class_stop WHERE id = 1"#
        )
        .fetch_optional(self.db.reader())
        .await;
        let row = match read {
            Ok(Some(row)) => row,
            Ok(None) => {
                tracing::warn!("the write class's stop row is missing; reading it as stopped");
                return ClassStop::Unread;
            }
            Err(_) => {
                tracing::warn!("the write class's stop could not be read; reading it as stopped");
                return ClassStop::Unread;
            }
        };
        if row.stopped == 0 {
            return ClassStop::Running;
        }
        let setter = row.set_by.as_deref().and_then(StopSetter::from_column);
        match (setter, row.reason, row.set_at) {
            (Some(set_by), Some(reason), Some(since)) => ClassStop::Stopped {
                set_by,
                reason,
                since: UtcMillis::from_epoch_millis(since),
            },
            _ => ClassStop::Unread,
        }
    }

    /// Sets the class's stop because the count named `count` moved (R35, R36): `set_by` is
    /// `counts`, `reason` the count's name, `set_at` is `now`. A stop already set keeps who set it,
    /// why and since when. Nothing here clears the stop.
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when the write fails, including a `count` the row's check refuses
    /// (1 to 40 lowercase letters or underscores).
    pub async fn set_by_counts(&self, count: &str, now: UtcMillis) -> Result<(), KernelError> {
        let set_by = StopSetter::Counts.as_str();
        let set_at = now.epoch_millis();
        let mut write = self.db.write().await?;
        sqlx::query!(
            "UPDATE write_class_stop SET stopped = 1, set_by = ?1, reason = ?2, set_at = ?3
             WHERE id = 1 AND stopped = 0",
            set_by,
            count,
            set_at
        )
        .execute(&mut *write)
        .await?;
        write.commit().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::StopSetter;

    #[test]
    fn from_column_names_each_setter_the_row_stores() {
        assert_eq!(StopSetter::from_column("owner"), Some(StopSetter::Owner));
        assert_eq!(StopSetter::from_column("counts"), Some(StopSetter::Counts));
        assert_eq!(StopSetter::from_column("other"), None);
        for setter in [StopSetter::Owner, StopSetter::Counts] {
            assert_eq!(StopSetter::from_column(setter.as_str()), Some(setter));
        }
    }
}
