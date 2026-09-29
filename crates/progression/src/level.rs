//! Level info and titles (SPEC-072 R13): the predecessor's `gamification/xp.py:level_info` and
//! `level_title` at `27ee2bc`, over the curve of [`crate::xp`].

use crate::xp::{Level, XpTotal, level_for, xp_to_reach};

/// The titles a level earns, highest threshold first: the level from which it holds, the title and
/// its emoji (the predecessor's `LEVEL_TITLES`).
pub const LEVEL_TITLES: [(u32, &str, &str); 12] = [
    (100, "Grandmaster of Memory", "\u{1f451}"),
    (90, "Mind Cosmonaut", "\u{1f30c}"),
    (80, "Synapse Dragon", "\u{1f409}"),
    (70, "Retention Lord", "\u{1f6e1}\u{fe0f}"),
    (60, "Recall Knight", "\u{2694}\u{fe0f}"),
    (50, "Memory Mage", "\u{1f9e0}"),
    (40, "Graduate", "\u{1f393}"),
    (30, "Adept", "\u{1f4d9}"),
    (20, "Scholar", "\u{1f4d8}"),
    (10, "Apprentice", "\u{1f4d7}"),
    (5, "Seedling", "\u{1f331}"),
    (1, "Sprout", "\u{1f423}"),
];

/// The title and emoji of a level below every threshold.
const FALLBACK: (&str, &str) = ("Sprout", "\u{1f423}");

/// The title and emoji `level` holds: the first title whose threshold it reaches.
#[must_use]
pub fn level_title(level: Level) -> (&'static str, &'static str) {
    let _ = level;
    ("", "")
}

/// A total's place on the level curve.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LevelInfo {
    /// The total the info is for.
    pub total_xp: u64,
    /// The level the total reaches.
    pub level: Level,
    /// The level's title.
    pub title: &'static str,
    /// The title's emoji.
    pub emoji: &'static str,
    /// The XP the total is past the level's start.
    pub xp_into_level: u64,
    /// The XP the level spans, from its start to the next level's.
    pub xp_for_next: u64,
}

/// The level info of `total`.
#[must_use]
pub fn level_info(total: XpTotal) -> LevelInfo {
    let _ = total;
    LevelInfo {
        total_xp: 0,
        level: level_for(XpTotal::new(0)),
        title: "",
        emoji: "",
        xp_into_level: 0,
        xp_for_next: 0,
    }
}
