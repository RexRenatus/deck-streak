//! The badge catalog (SPEC-073 R2): the predecessor's 40 badges, each with its key, name, emoji and
//! description at tier 0, equal to `goldens/badge_catalog.json`.
//!
//! Five descriptions name courses of the predecessor's own owner; here they are rendered from the
//! configured courses (ADR-087), so a catalog never carries a course the owner does not study.

use deck_streak_kernel::Courses;
use deck_streak_kernel::courses::Course;

/// The family a badge belongs to: which context decides it is earned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Family {
    /// The 24 study badges, decided by the recompute's badge step (R5 to R8).
    Study,
    /// The eight habit badges (reading and writing), decided by the habit context.
    Habit,
    /// The eight focus badges, decided by the focus context.
    Focus,
}

/// One catalog badge.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Badge {
    /// The key, unique across the catalog.
    pub key: String,
    /// The name a screen shows.
    pub name: String,
    /// The emoji a screen shows.
    pub emoji: String,
    /// The criterion in words.
    pub description: String,
    /// The tier, 0 for every catalog badge.
    pub tier: u32,
    /// Which context decides the badge.
    pub family: Family,
}

/// A catalog row: key, name, emoji, description and family, in the predecessor's order.
type Row = (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    Family,
);

/// The 40 rows, in the predecessor's order: 24 study, eight habit, eight focus.
const ROWS: [Row; 40] = [
    (
        "first_steps",
        "First Steps",
        "👟",
        "Your first ever review",
        Family::Study,
    ),
    (
        "week_warrior",
        "Week Warrior",
        "🔥",
        "7-day streak",
        Family::Study,
    ),
    (
        "monthly_monk",
        "Monthly Monk",
        "🧘",
        "30-day streak",
        Family::Study,
    ),
    (
        "century_flame",
        "Century Flame",
        "💯🔥",
        "100-day streak",
        Family::Study,
    ),
    (
        "year_of_iron",
        "Year of Iron",
        "🗓️👑",
        "365-day streak",
        Family::Study,
    ),
    (
        "grinder",
        "Grinder",
        "⚙️",
        "1,000 lifetime reviews",
        Family::Study,
    ),
    (
        "marathoner",
        "Marathoner",
        "🏃",
        "10,000 lifetime reviews",
        Family::Study,
    ),
    (
        "centurion_day",
        "Centurion Day",
        "🛡️",
        "100+ reviews in one day",
        Family::Study,
    ),
    (
        "sharpshooter",
        "Sharpshooter",
        "🎯",
        "90%+ true retention over 7 days",
        Family::Study,
    ),
    (
        "sniper_elite",
        "Sniper Elite",
        "🎯✨",
        "95%+ mature retention over 30 days",
        Family::Study,
    ),
    (
        "inbox_zero",
        "Inbox Zero",
        "📭",
        "Cleared the due queue to zero",
        Family::Study,
    ),
    (
        "backlog_slayer",
        "Backlog Slayer",
        "🐉⚔️",
        "Cleared a 200+ card backlog in a day",
        Family::Study,
    ),
    (
        "night_owl",
        "Night Owl",
        "🦉",
        "50+ reviews between midnight and rollover",
        Family::Study,
    ),
    (
        "early_bird",
        "Early Bird",
        "🐦",
        "50+ reviews in the early morning",
        Family::Study,
    ),
    (
        "comeback_kid",
        "Comeback Kid",
        "💪",
        "Returned after breaking a 7+ day streak",
        Family::Study,
    ),
    (
        "maturity_milestone",
        "Maturity Milestone",
        "🌳",
        "100 cards reached mature",
        Family::Study,
    ),
    (
        "forest_guardian",
        "Forest Guardian",
        "🌲🌲",
        "1,000 mature cards",
        Family::Study,
    ),
    (
        "leech_tamer",
        "Leech Tamer",
        "🪱✅",
        "Zero unhandled leeches",
        Family::Study,
    ),
    (
        "polyglot",
        "Polyglot",
        "🌍",
        "Studied 3+ decks in one day",
        Family::Study,
    ),
    (
        "globetrotter",
        "Globetrotter",
        "🗺️",
        "Reviewed 5+ decks in a week",
        Family::Study,
    ),
    (
        "perfect_week",
        "Perfect Week",
        "✅",
        "7/7 days scoring 75+",
        Family::Study,
    ),
    (
        "legendary_day",
        "Legendary Day",
        "🏆",
        "A perfect daily score of 100",
        Family::Study,
    ),
    (
        "speed_demon",
        "Speed Demon",
        "⚡",
        "100+ reviews averaging under 6 seconds",
        Family::Study,
    ),
    (
        "iron_will",
        "Iron Will",
        "🦾",
        "30 days, no misses, no freezes used",
        Family::Study,
    ),
    (
        "first_page",
        "First Page",
        "📖",
        "Logged your first reading session",
        Family::Habit,
    ),
    (
        "quill_initiate",
        "Quill Initiate",
        "✍️",
        "Logged your first writing day",
        Family::Habit,
    ),
    (
        "ink_week",
        "Ink Week",
        "✍️🔥",
        "7-day all-CJK writing streak",
        Family::Habit,
    ),
    (
        "ink_month",
        "Ink Month",
        "✍️🌙",
        "30-day all-CJK writing streak",
        Family::Habit,
    ),
    (
        "ink_century",
        "Ink Century",
        "✍️💯",
        "100-day all-CJK writing streak",
        Family::Habit,
    ),
    (
        "bookworm_week",
        "Bookworm Week",
        "📚✅",
        "Hit the 3.5h reading goal in every language",
        Family::Habit,
    ),
    (
        "polyglot_reader",
        "Polyglot Reader",
        "📚🌍",
        "Read in all 5 languages in one week",
        Family::Habit,
    ),
    (
        "marathon_reader",
        "Marathon Reader",
        "📖🏃",
        "Logged 10 hours of reading in one week",
        Family::Habit,
    ),
    (
        "focus_initiate",
        "Focus Initiate",
        "⏳",
        "Completed your first focus block",
        Family::Focus,
    ),
    (
        "deep_work_day",
        "Deep Work Day",
        "🧘‍♂️",
        "Hit the 50-min focus goal in a day",
        Family::Focus,
    ),
    (
        "deep_diver",
        "Deep Diver",
        "🌊",
        "Completed a single 90-minute focus block",
        Family::Focus,
    ),
    (
        "focus_week",
        "Focus Week",
        "⏳🔥",
        "7-day focus streak",
        Family::Focus,
    ),
    (
        "focus_month",
        "Focus Month",
        "⏳🌙",
        "30-day focus streak",
        Family::Focus,
    ),
    (
        "monk_mode",
        "Monk Mode",
        "🛕",
        "10 hours of focus in one week",
        Family::Focus,
    ),
    (
        "deep_work_centurion",
        "Deep Work Centurion",
        "🏛️",
        "100 completed focus blocks",
        Family::Focus,
    ),
    (
        "subject_devotee",
        "Subject Devotee",
        "🎓🔥",
        "50 focus blocks on one subject",
        Family::Focus,
    ),
];

/// The names of the courses that carry the writing habit, as the ink badges' descriptions list
/// them; an owner with no writing course gets the generic wording.
fn writing_courses(courses: &Courses) -> Vec<&Course> {
    courses
        .courses()
        .iter()
        .filter(|course| course.writing)
        .collect()
}

/// `names` as a list in words: `A`, `A and B`, `A, B and C`.
fn listed(names: &[&str]) -> String {
    match names {
        [] => String::new(),
        [only] => (*only).to_owned(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

/// The description of `key`, rendered from the configured `courses` when it names courses (R2).
fn describe(key: &str, fixed: &str, courses: &Courses) -> String {
    let writing = writing_courses(courses);
    let writing_names: Vec<&str> = writing.iter().map(|course| course.name.as_str()).collect();
    let all: Vec<&str> = courses
        .courses()
        .iter()
        .map(|course| course.name.as_str())
        .collect();
    let streak = |days: &str| {
        if writing_names.is_empty() {
            format!("{days}-day writing streak")
        } else {
            format!("{days}-day {} writing streak", listed(&writing_names))
        }
    };
    match key {
        "ink_week" => streak("7"),
        "ink_month" => streak("30"),
        "ink_century" => streak("100"),
        "bookworm_week" if all.is_empty() => "Hit the reading goal in every course".to_owned(),
        "bookworm_week" => format!("Hit the reading goal in {}", listed(&all)),
        "polyglot_reader" if all.is_empty() => "Read in every course in one week".to_owned(),
        "polyglot_reader" => format!("Read in all {} courses in one week", all.len()),
        _ => fixed.to_owned(),
    }
}

/// The catalog under `courses`: 40 badges, each at tier 0, in the predecessor's order.
#[must_use]
pub fn catalog(courses: &Courses) -> Vec<Badge> {
    ROWS.iter()
        .map(|(key, name, emoji, description, family)| Badge {
            key: (*key).to_owned(),
            name: (*name).to_owned(),
            emoji: (*emoji).to_owned(),
            description: describe(key, description, courses),
            tier: 0,
            family: *family,
        })
        .collect()
}

/// Whether `key` names a catalog badge.
#[must_use]
pub fn is_catalog_key(key: &str) -> bool {
    ROWS.iter().any(|row| row.0 == key)
}
