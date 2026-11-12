//! The readings date tree (SPEC-042 R5 to R11): where a reading's note lives, and every operation
//! the service's own code performs on it.
//!
//! Today's note is `<readings>/<YYYY-MM-DD>/<topic file>`, the day being the study day. A note the
//! next study day carries is rolled into that day's folder with three keys patched; every other note
//! of a prior day is archived unchanged to `<readings>/<archive>/<YYYY>/<MM>/W<WW>/<YYYY-MM-DD>/`, by
//! calendar year and month and zero-padded ISO week, as the predecessor's `_archive_dir` names it.
//!
//! Every write passes the rails, lands atomically, stays inside the readings folder after `..` and
//! symbolic links are resolved, and never overwrites a note (compared case-insensitively). Nothing
//! here deletes a note: a roll removes its source only after the destination reads back as written,
//! an archive is a rename, and only an emptied day folder is removed. A symbolic link in the tree is
//! never followed: a linked note or day folder is refused or reported, and never read or written.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::{Path, PathBuf};

use deck_streak_kernel::{StudyDay, Verdict};

use crate::config::{FolderName, VaultPaths, VaultSettings};
use crate::fs::{EntryKind, VaultFs};
use crate::note::{self, BodyHash, BoxLine, Malformed, TopicKey};
use crate::rails::{RailRow, Rails};
use crate::{VaultError, atomic};

/// Days from the proleptic Gregorian 0000-03-01 to 1970-01-01: the calendar below counts eras of
/// 400 years from a March 1st, so that a leap day is the last day of its year.
const EPOCH_FROM_MARCH_ZERO: i64 = 719_468;
/// Days in 400 Gregorian years, one era.
const DAYS_PER_ERA: i64 = 146_097;

/// The folder a study day's archived notes go to, relative to the vault root:
/// `<readings>/<archive>/<YYYY>/<MM>/W<WW>/<YYYY-MM-DD>`, where the year and the month are the
/// day's calendar year and month and the week is its ISO week, zero-padded. At a year boundary the
/// two disagree (the last days of December can be week 1), and that is the predecessor's rule.
#[must_use]
pub fn archive_folder(readings: &FolderName, archive: &FolderName, day: StudyDay) -> PathBuf {
    let (year, month, _) = civil_from_days(day.epoch_day());
    [
        readings.as_str().to_owned(),
        archive.as_str().to_owned(),
        format!("{year:04}"),
        format!("{month:02}"),
        format!("W{:02}", iso_week(day.epoch_day())),
        day.to_string(),
    ]
    .iter()
    .collect()
}

/// The ISO week of an epoch day: the week, Monday to Sunday, counted in the year its Thursday falls
/// in (ISO 8601).
fn iso_week(epoch_day: i64) -> i64 {
    // The epoch's day, 1970-01-01, was a Thursday: Monday is 0.
    let weekday = (epoch_day + 3).rem_euclid(7);
    let thursday = epoch_day - weekday + 3;
    let (year, _, _) = civil_from_days(thursday);
    (thursday - days_from_civil(year, 1, 1)) / 7 + 1
}

/// The civil date of an epoch day (Howard Hinnant's `civil_from_days`).
fn civil_from_days(epoch_day: i64) -> (i64, i64, i64) {
    let from_march_zero = epoch_day + EPOCH_FROM_MARCH_ZERO;
    let era = from_march_zero.div_euclid(DAYS_PER_ERA);
    let day_of_era = from_march_zero.rem_euclid(DAYS_PER_ERA);
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_from_march = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_from_march + 2) / 5 + 1;
    let month = if month_from_march < 10 {
        month_from_march + 3
    } else {
        month_from_march - 9
    };
    (year_of_era + era * 400 + i64::from(month <= 2), month, day)
}

/// The epoch day of a civil date (Howard Hinnant's `days_from_civil`).
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = year - i64::from(month <= 2);
    let era = year.div_euclid(400);
    let year_of_era = year.rem_euclid(400);
    let month_from_march = if month > 2 { month - 3 } else { month + 9 };
    let day_of_year = (153 * month_from_march + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * DAYS_PER_ERA + day_of_era - EPOCH_FROM_MARCH_ZERO
}

/// A note the adapter created: where it is, relative to the vault root, and the hash of the body it
/// wrote, which a later body replacement proves unchanged (R10).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Created {
    /// The note's path, relative to the vault root.
    pub path: PathBuf,
    /// The hash of the body the adapter wrote.
    pub body: BodyHash,
}

/// What a box write did.
#[must_use = "a box write reports whether it wrote"]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoxOutcome {
    /// The box was unticked, and is now ticked.
    Ticked,
    /// The box was already ticked, so nothing was written.
    AlreadyTicked,
}

/// Why one note of a roll-forward was not relocated. Its siblings still were (R8).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RollFailureReason {
    /// The note is malformed, so it cannot be rolled without risking its bytes.
    Malformed(Malformed),
    /// A note is already at the roll's destination, so the roll is refused (R7).
    DestinationTaken,
    /// The rolled note would not pass the rails, so it is not written.
    Rails(RailRow),
    /// The entry is a symbolic link or not a regular file, which the adapter never follows.
    NotARegularFile,
}

impl fmt::Display for RollFailureReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Malformed(why) => write!(f, "{why}"),
            Self::DestinationTaken => f.write_str("a note is already at the roll's destination"),
            Self::Rails(row) => write!(f, "the rolled note would fail the rail {row}"),
            Self::NotARegularFile => f.write_str("it is not a regular file"),
        }
    }
}

/// One note a roll-forward could not relocate: its path relative to the vault root, and a bounded
/// reason that never quotes it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RollFailure {
    /// The note's path, relative to the vault root.
    pub path: PathBuf,
    /// Why it stayed where it is.
    pub reason: RollFailureReason,
}

/// What a roll-forward did: the notes it rolled into today's folder, the notes it archived, and the
/// notes it reported. Paths are relative to the vault root.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RollReport {
    /// The notes rolled into today's folder, at their new paths.
    pub rolled: Vec<PathBuf>,
    /// The notes archived, at their archive paths.
    pub archived: Vec<PathBuf>,
    /// The notes that stayed, each with its reason.
    pub failed: Vec<RollFailure>,
}

/// The readings date tree, over a file system `F`.
#[derive(Debug)]
pub struct ReadingsTree<F: VaultFs> {
    fs: F,
    rails: Rails,
    paths: VaultPaths,
}

impl<F: VaultFs> ReadingsTree<F> {
    /// The tree of the configured vault, after the start check (R1).
    ///
    /// # Errors
    ///
    /// [`VaultError::Start`] when the vault features refuse to start, and [`VaultError::Io`] when
    /// the check could not run.
    pub fn open(settings: &VaultSettings, fs: F, rails: Rails) -> Result<Self, VaultError> {
        let paths = VaultPaths::check(settings, &fs)?;
        Ok(Self { fs, rails, paths })
    }

    /// The file system the tree works on.
    pub fn fs(&self) -> &F {
        &self.fs
    }

    /// The resolved vault paths.
    pub fn paths(&self) -> &VaultPaths {
        &self.paths
    }

    /// The path of `topic`'s note for `day`, relative to the vault root (R5).
    #[must_use]
    pub fn note_path(&self, day: StudyDay, topic: &TopicKey) -> PathBuf {
        self.day_path(day).join(topic.file_name())
    }

    /// Writes a new reading's note for `day` (R6), creating the day's folder inside the readings
    /// folder when it is missing.
    ///
    /// # Errors
    ///
    /// [`VaultError::NoteExists`] when a note is there already (compared case-insensitively),
    /// [`VaultError::Rails`] when the note would fail the rails, [`VaultError::OutsideConfinement`]
    /// when the day's folder resolves outside the readings folder, the refusals of
    /// [`note::render`], and [`VaultError::Io`].
    pub fn create(
        &self,
        day: StudyDay,
        topic: &TopicKey,
        digest: &str,
        body: &str,
    ) -> Result<Created, VaultError> {
        let text = note::render(topic, day, digest, body)?;
        self.pass_rails(&text)?;
        let folder = self.day_folder(day, true)?.ok_or(VaultError::NoteMissing)?;
        let name = topic.file_name();
        if self.taken(&folder)?.contains(&name.to_lowercase()) {
            return Err(VaultError::NoteExists);
        }
        atomic::write(&self.fs, &folder.join(&name), text.as_bytes())?;
        Ok(Created {
            path: self.note_path(day, topic),
            body: BodyHash::of(body),
        })
    }

    /// Rolls the readings forward to `today` (R7, R8): from the single most recent prior-day folder
    /// the notes of the `carried` topics are rolled into today's folder, and every other note of
    /// any prior-day folder is archived unchanged. Each emptied prior-day folder is removed. A
    /// malformed note is reported and its siblings still move, and a second call on the same day
    /// changes nothing.
    ///
    /// # Errors
    ///
    /// [`VaultError::ReadBack`] when a rolled note did not read back as written (its source stays,
    /// and the roll stops), and [`VaultError::Io`] or a confinement refusal when a step could not
    /// run.
    pub fn roll_forward(
        &self,
        today: StudyDay,
        carried: &[TopicKey],
    ) -> Result<RollReport, VaultError> {
        let mut report = RollReport::default();
        let mut prior = Vec::new();
        for (name, kind) in self.entries(self.paths.readings())? {
            let Ok(day) = name.parse::<StudyDay>() else {
                continue;
            };
            if day >= today {
                continue;
            }
            if kind == EntryKind::Dir {
                prior.push(day);
            } else {
                report.failed.push(RollFailure {
                    path: self.day_path(day),
                    reason: RollFailureReason::NotARegularFile,
                });
            }
        }
        prior.sort();
        let Some(&most_recent) = prior.last() else {
            return Ok(report);
        };
        let carried: BTreeSet<String> = carried.iter().map(TopicKey::file_name).collect();
        for day in prior {
            let folder = self.paths.readings().join(day.to_string());
            for (name, kind) in self.entries(&folder)? {
                if name.strip_suffix(".md").is_none() {
                    continue;
                }
                let path = self.day_path(day).join(&name);
                if kind != EntryKind::File {
                    report.failed.push(RollFailure {
                        path,
                        reason: RollFailureReason::NotARegularFile,
                    });
                    continue;
                }
                let source = folder.join(&name);
                if day == most_recent && carried.contains(&name) {
                    match self.roll_note(&source, today, &name)? {
                        Ok(rolled) => report.rolled.push(rolled),
                        Err(reason) => report.failed.push(RollFailure { path, reason }),
                    }
                } else {
                    report
                        .archived
                        .push(self.archive_file(&source, day, &name)?);
                }
            }
            self.remove_if_empty(&folder)?;
        }
        Ok(report)
    }

    /// Archives `topic`'s note of `day` unchanged, as a superseded reading (R8, SPEC-048): a name
    /// already taken in the archive folder gets `-2`, `-3` and so on before `.md`, and nothing is
    /// overwritten. Returns the archive path, relative to the vault root.
    ///
    /// # Errors
    ///
    /// [`VaultError::NoteMissing`] when there is no such note, and [`VaultError::Io`] or a
    /// confinement refusal when a step could not run.
    pub fn archive(&self, day: StudyDay, topic: &TopicKey) -> Result<PathBuf, VaultError> {
        let source = self.note_file(day, topic)?;
        self.archive_file(&source, day, &topic.file_name())
    }

    /// The day of the folder holding `topic`'s live note: today's folder first, then every other
    /// day folder, most recent first. The archive is never live.
    ///
    /// # Errors
    ///
    /// [`VaultError::Io`] when the readings folder could not be listed.
    pub fn find_live(
        &self,
        topic: &TopicKey,
        today: StudyDay,
    ) -> Result<Option<StudyDay>, VaultError> {
        let mut days: Vec<StudyDay> = self
            .entries(self.paths.readings())?
            .into_iter()
            .filter(|(_, kind)| *kind == EntryKind::Dir)
            .filter_map(|(name, _)| name.parse().ok())
            .collect();
        days.sort_by_key(|day| (*day != today, std::cmp::Reverse(*day)));
        let name = topic.file_name();
        for day in days {
            let path = self.paths.readings().join(day.to_string()).join(&name);
            if self.kind(&path)? == Some(EntryKind::File) {
                return Ok(Some(day));
            }
        }
        Ok(None)
    }

    /// Stamps the Studied box of `topic`'s note in `day`'s folder (R9).
    ///
    /// # Errors
    ///
    /// [`VaultError::BoxAnchorMissing`], [`VaultError::BoxAnchorAmbiguous`], the note's absence
    /// and the write's refusals, each with no write.
    pub fn stamp_studied(&self, day: StudyDay, topic: &TopicKey) -> Result<BoxOutcome, VaultError> {
        self.write_box(day, topic, BoxLine::Studied)
    }

    /// Ticks the owner's `I read it` box of `topic`'s note in `day`'s folder (R9). Its one caller is
    /// the owner's read-tap use case (SPEC-047).
    ///
    /// # Errors
    ///
    /// As [`ReadingsTree::stamp_studied`].
    pub fn tick_read(&self, day: StudyDay, topic: &TopicKey) -> Result<BoxOutcome, VaultError> {
        self.write_box(day, topic, BoxLine::Read)
    }

    /// Replaces the body of `topic`'s note in `day`'s folder with `body` (R10), keeping the
    /// frontmatter and both box lines byte for byte. `last_written` is the hash of the body the
    /// adapter last wrote; returns the hash of the new body.
    ///
    /// # Errors
    ///
    /// [`VaultError::NoteEdited`] (`vault_note_edited`) when the note's body is not the one the
    /// adapter last wrote, with no write; the refusals of [`note::check_body`]; and the write's.
    pub fn replace_body(
        &self,
        day: StudyDay,
        topic: &TopicKey,
        last_written: BodyHash,
        body: &str,
    ) -> Result<BodyHash, VaultError> {
        note::check_body(body)?;
        let path = self.note_file(day, topic)?;
        let text = self.read_note(&path)?;
        let current = note::body(&text).ok_or(VaultError::NoteEdited)?;
        if BodyHash::of(current) != last_written {
            return Err(VaultError::NoteEdited);
        }
        let replaced = note::with_body(&text, body).ok_or(VaultError::NoteEdited)?;
        self.pass_rails(&replaced)?;
        atomic::write(&self.fs, &path, replaced.as_bytes())?;
        Ok(BodyHash::of(body))
    }

    /// The day folder's path, relative to the vault root.
    fn day_path(&self, day: StudyDay) -> PathBuf {
        Path::new(self.paths.readings_name().as_str()).join(day.to_string())
    }

    /// `day`'s folder, resolved and proved inside the readings folder; created first when `create`
    /// and it is missing, and `None` when it is missing otherwise.
    fn day_folder(&self, day: StudyDay, create: bool) -> Result<Option<PathBuf>, VaultError> {
        let folder = self.paths.readings().join(day.to_string());
        if self.kind(&folder)?.is_none() {
            if !create {
                return Ok(None);
            }
            self.fs
                .create_dir(&folder)
                .map_err(VaultError::io("create a day folder"))?;
        }
        self.confined(&folder).map(Some)
    }

    /// `folder` resolved, when it is a folder inside the readings folder after `..` and links.
    fn confined(&self, folder: &Path) -> Result<PathBuf, VaultError> {
        let resolved = self
            .fs
            .canonicalize(folder)
            .map_err(VaultError::io("resolve a folder"))?;
        if !resolved.starts_with(self.paths.readings()) {
            return Err(VaultError::OutsideConfinement);
        }
        if self.kind(&resolved)? != Some(EntryKind::Dir) {
            return Err(VaultError::NotAFolder);
        }
        Ok(resolved)
    }

    /// The archive folder of `day`, each missing folder created inside the readings folder, and
    /// each proved inside it before the next is made.
    fn archive_dir(&self, day: StudyDay) -> Result<PathBuf, VaultError> {
        let relative = archive_folder(self.paths.readings_name(), self.paths.archive_name(), day);
        let mut folder = self.paths.readings().to_path_buf();
        for part in relative.iter().skip(1) {
            let next = folder.join(part);
            if self.kind(&next)?.is_none() {
                self.fs
                    .create_dir(&next)
                    .map_err(VaultError::io("create an archive folder"))?;
            }
            folder = self.confined(&next)?;
        }
        Ok(folder)
    }

    /// Moves `source`, a note of `day` named `name`, unchanged into `day`'s archive folder, under
    /// the first of `name`, `<stem>-2.md`, `<stem>-3.md`, ... that no entry there holds in any case.
    fn archive_file(
        &self,
        source: &Path,
        day: StudyDay,
        name: &str,
    ) -> Result<PathBuf, VaultError> {
        let folder = self.archive_dir(day)?;
        let taken = self.taken(&folder)?;
        let stem = name.strip_suffix(".md").unwrap_or(name);
        let free = std::iter::once(name.to_owned())
            .chain((2..=u32::MAX).map(|number| format!("{stem}-{number}.md")))
            .find(|candidate| !taken.contains(&candidate.to_lowercase()))
            .ok_or(VaultError::NoteExists)?;
        self.fs
            .rename(source, &folder.join(&free))
            .map_err(VaultError::io("archive a note"))?;
        self.fs
            .sync_dir(&folder)
            .map_err(VaultError::io("sync an archive folder"))?;
        if let Some(from) = source.parent() {
            self.fs
                .sync_dir(from)
                .map_err(VaultError::io("sync a day folder"))?;
        }
        Ok(archive_folder(self.paths.readings_name(), self.paths.archive_name(), day).join(free))
    }

    /// Rolls `source`, named `name`, into `today`'s folder: the rolled note is written, read back
    /// and compared before the source is removed. The inner error is a note's own refusal, which
    /// its siblings do not share.
    fn roll_note(
        &self,
        source: &Path,
        today: StudyDay,
        name: &str,
    ) -> Result<Result<PathBuf, RollFailureReason>, VaultError> {
        let bytes = self
            .fs
            .read(source)
            .map_err(VaultError::io("read a note to roll"))?;
        let Ok(text) = String::from_utf8(bytes) else {
            return Ok(Err(RollFailureReason::Malformed(Malformed::NotUtf8)));
        };
        let rolled = match note::roll(&text, today) {
            Ok(rolled) => rolled,
            Err(why) => return Ok(Err(RollFailureReason::Malformed(why))),
        };
        if let Verdict::Refuse(refusal) = self.rails.check(&rolled) {
            return Ok(Err(RollFailureReason::Rails(refusal.row)));
        }
        let folder = self
            .day_folder(today, true)?
            .ok_or(VaultError::NoteMissing)?;
        if self.taken(&folder)?.contains(&name.to_lowercase()) {
            return Ok(Err(RollFailureReason::DestinationTaken));
        }
        let destination = folder.join(name);
        atomic::write(&self.fs, &destination, rolled.as_bytes())?;
        if self.fs.read(&destination).ok().as_deref() != Some(rolled.as_bytes()) {
            // Only this call's own write is removed: the source, the one other copy, stays.
            let _removed = self.fs.remove_file(&destination);
            let _emptied = self.remove_if_empty(&folder);
            return Err(VaultError::ReadBack);
        }
        self.fs
            .remove_file(source)
            .map_err(VaultError::io("remove a rolled note's source"))?;
        if let Some(from) = source.parent() {
            self.fs
                .sync_dir(from)
                .map_err(VaultError::io("sync a day folder"))?;
        }
        Ok(Ok(self.day_path(today).join(name)))
    }

    /// Ticks the box `which` of `topic`'s note in `day`'s folder.
    fn write_box(
        &self,
        day: StudyDay,
        topic: &TopicKey,
        which: BoxLine,
    ) -> Result<BoxOutcome, VaultError> {
        let path = self.note_file(day, topic)?;
        let text = self.read_note(&path)?;
        match note::tick(&text, which)? {
            None => Ok(BoxOutcome::AlreadyTicked),
            Some(ticked) => {
                self.pass_rails(&ticked)?;
                atomic::write(&self.fs, &path, ticked.as_bytes())?;
                Ok(BoxOutcome::Ticked)
            }
        }
    }

    /// The path of `topic`'s note in `day`'s folder, when it is a regular file there.
    fn note_file(&self, day: StudyDay, topic: &TopicKey) -> Result<PathBuf, VaultError> {
        let folder = self
            .day_folder(day, false)?
            .ok_or(VaultError::NoteMissing)?;
        let path = folder.join(topic.file_name());
        match self.kind(&path)? {
            Some(EntryKind::File) => Ok(path),
            Some(_) => Err(VaultError::NotARegularFile),
            None => Err(VaultError::NoteMissing),
        }
    }

    /// The UTF-8 text of the note at `path`.
    fn read_note(&self, path: &Path) -> Result<String, VaultError> {
        let bytes = self.fs.read(path).map_err(VaultError::io("read a note"))?;
        String::from_utf8(bytes).map_err(|_| VaultError::Malformed(Malformed::NotUtf8))
    }

    /// Refuses `text` when a rail refuses it.
    fn pass_rails(&self, text: &str) -> Result<(), VaultError> {
        self.rails
            .check(text)
            .into_result()
            .map_err(VaultError::from)
    }

    /// Every entry of `folder` whose name is UTF-8, sorted by name, with its kind.
    fn entries(&self, folder: &Path) -> Result<BTreeMap<String, EntryKind>, VaultError> {
        Ok(self
            .fs
            .list(folder)
            .map_err(VaultError::io("list a folder"))?
            .into_iter()
            .filter_map(|entry| {
                let name = entry.name.into_string().ok()?;
                Some((name, entry.kind))
            })
            .collect())
    }

    /// The names in `folder`, lowercased, so a name is taken whatever its case.
    fn taken(&self, folder: &Path) -> Result<BTreeSet<String>, VaultError> {
        Ok(self
            .fs
            .list(folder)
            .map_err(VaultError::io("list a folder"))?
            .into_iter()
            .map(|entry| entry.name.to_string_lossy().to_lowercase())
            .collect())
    }

    /// What is at `path`, without following a link.
    fn kind(&self, path: &Path) -> Result<Option<EntryKind>, VaultError> {
        self.fs.kind(path).map_err(VaultError::io("read an entry"))
    }

    /// Removes `folder` when it holds nothing.
    fn remove_if_empty(&self, folder: &Path) -> Result<(), VaultError> {
        let empty = self
            .fs
            .list(folder)
            .map_err(VaultError::io("list a folder"))?
            .is_empty();
        if empty {
            self.fs
                .remove_dir(folder)
                .map_err(VaultError::io("remove an emptied day folder"))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{civil_from_days, days_from_civil};

    /// Epoch days and their civil dates, as Python's `datetime.date` counts them from 1970-01-01:
    /// the days either side of the epoch, a leap day and the day after it, a century's last day of
    /// February that is no leap day, and a day in each quarter of a year.
    const DATES: [(i64, (i64, i64, i64)); 10] = [
        (0, (1970, 1, 1)),
        (-1, (1969, 12, 31)),
        (11_016, (2000, 2, 29)),
        (11_017, (2000, 3, 1)),
        (19_753, (2024, 1, 31)),
        (19_843, (2024, 4, 30)),
        (19_908, (2024, 7, 4)),
        (20_088, (2024, 12, 31)),
        (-25_508, (1900, 3, 1)),
        (-25_509, (1900, 2, 28)),
    ];

    #[test]
    fn an_epoch_day_reads_as_its_civil_date_and_the_date_as_its_epoch_day() {
        for (epoch_day, (year, month, day)) in DATES {
            assert_eq!(
                civil_from_days(epoch_day),
                (year, month, day),
                "epoch day {epoch_day}"
            );
            assert_eq!(
                days_from_civil(year, month, day),
                epoch_day,
                "{year}-{month:02}-{day:02}"
            );
        }
    }
}
