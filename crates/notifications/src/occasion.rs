//! The occasion the router decides on (SPEC-041 R3; ADR-041): its kind, which only a kind the policy
//! declares can produce, its opaque dedupe key, the surface it was raised on, the tier it asks for,
//! its text, its study day, and the lapse context its caller supplies.
//!
//! The vocabulary the policy file and the ledger share lives here too: the celebration tier, the
//! class of a kind, the surface and the dedupe scope, each with the one spelling the file uses.

use deck_streak_kernel::StudyDay;
use serde::{Deserialize, Serialize};

/// The longest dedupe key, in bytes: an opaque token, never a message.
pub const DEDUPE_KEY_MAX_LEN: usize = 128;

/// The budget whose kinds exist only inside a lapse, capped per lapse id (the policy's `comeback`).
pub const COMEBACK_BUDGET: &str = "comeback";

/// A celebration tier, from T0 (silent) to T5 (dice and a pin). It is the notifications context's
/// tier, never a Bloom tier (docs/CONTEXT-MAP.md), and its order is its loudness.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Tier {
    /// Silent: the decision is recorded and nothing is sent.
    T0,
    /// A reaction on the owner's last message.
    T1,
    /// A line.
    T2,
    /// A two-beat reveal.
    T3,
    /// Dice.
    T4,
    /// Dice and a pin.
    T5,
}

impl Tier {
    /// The tier as the policy and the ledger spell it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::T0 => "T0",
            Self::T1 => "T1",
            Self::T2 => "T2",
            Self::T3 => "T3",
            Self::T4 => "T4",
            Self::T5 => "T5",
        }
    }

    /// The tier the ledger spelled `text`, or `None` for any other text.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        [Self::T0, Self::T1, Self::T2, Self::T3, Self::T4, Self::T5]
            .into_iter()
            .find(|tier| tier.as_str() == text)
    }
}

/// What a kind is to the policy: it decides how quiet hours, a lapse and a failed send treat it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Class {
    /// A celebration: deferred in quiet hours, held after a failed send.
    Celebration,
    /// A nudge: it asks for study, and is withheld in quiet hours and in a lapse.
    Nudge,
    /// A digest.
    Digest,
    /// An alert: nothing silences it, quiet hours included.
    Alert,
}

/// A surface: where an occasion was raised, and where it is delivered.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Surface {
    /// The Telegram bot.
    Bot,
    /// The Mini App.
    MiniApp,
}

impl Surface {
    /// The surface as the policy and the ledger spell it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Bot => "bot",
            Self::MiniApp => "mini-app",
        }
    }

    /// The surface the ledger spelled `text`, or `None` for any other text.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        [Self::Bot, Self::MiniApp]
            .into_iter()
            .find(|surface| surface.as_str() == text)
    }
}

/// How long a delivered key stays delivered, as the kind declares.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DedupeScope {
    /// Once, ever.
    OnceEver,
    /// Once per study day.
    PerStudyDay,
    /// Once per study day of one lapse episode.
    PerEpisodeDay,
    /// Once per incident, which the key names.
    PerIncident,
}

/// A kind the policy declares, with its declaration. Only the policy makes one
/// ([`crate::policy::Policy::kind`]), so an occasion can never carry an undeclared kind.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Kind {
    name: String,
    class: Class,
    tiers: Vec<Tier>,
    budget: Option<String>,
    dedupe: DedupeScope,
    setting: Option<String>,
}

impl Kind {
    /// The kind `name`, declared with these values.
    pub(crate) const fn declared(
        name: String,
        class: Class,
        tiers: Vec<Tier>,
        budget: Option<String>,
        dedupe: DedupeScope,
        setting: Option<String>,
    ) -> Self {
        Self {
            name,
            class,
            tiers,
            budget,
            dedupe,
            setting,
        }
    }

    /// The kind's name, as the policy declares it.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The kind's class.
    #[must_use]
    pub const fn class(&self) -> Class {
        self.class
    }

    /// The tiers an occasion of the kind may ask for.
    #[must_use]
    pub fn tiers(&self) -> &[Tier] {
        &self.tiers
    }

    /// The budget the kind spends, if any.
    #[must_use]
    pub fn budget(&self) -> Option<&str> {
        self.budget.as_deref()
    }

    /// How long a delivered key of the kind stays delivered.
    #[must_use]
    pub const fn dedupe(&self) -> DedupeScope {
        self.dedupe
    }

    /// The owner's switch for the kind; an alert has none.
    #[must_use]
    pub fn setting(&self) -> Option<&str> {
        self.setting.as_deref()
    }

    /// Whether the kind spends the comeback budget, which exists only inside a lapse.
    #[must_use]
    pub fn is_comeback(&self) -> bool {
        self.budget() == Some(COMEBACK_BUDGET)
    }
}

/// An occasion's dedupe key: an opaque token of the grammar `^[a-z0-9][a-z0-9:._-]*$`, at most
/// [`DEDUPE_KEY_MAX_LEN`] bytes, that holds no calendar date.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DedupeKey(String);

impl DedupeKey {
    /// The key `text`.
    ///
    /// # Errors
    ///
    /// [`OccasionError::KeyNotAToken`] outside the grammar or the length, and
    /// [`OccasionError::KeyHoldsDate`] when it holds a calendar date.
    pub fn new(text: &str) -> Result<Self, OccasionError> {
        if !is_token(text) {
            return Err(OccasionError::KeyNotAToken);
        }
        if holds_calendar_date(text.as_bytes()) {
            return Err(OccasionError::KeyHoldsDate);
        }
        Ok(Self(text.to_owned()))
    }

    /// The key.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Whether `text` is a token of the grammar, within the length.
fn is_token(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.len() <= DEDUPE_KEY_MAX_LEN
        && bytes
            .first()
            .is_some_and(|first| first.is_ascii_lowercase() || first.is_ascii_digit())
        && bytes.iter().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || b":._-".contains(byte)
        })
}

/// Whether `text` holds a calendar date as the notifications-policy pack reads one: a year from
/// 1900 to 2099, a month and a day, each optionally after a `-`; or one or two digits, a `.` or
/// `/`, one or two digits, the same, and two digits more.
fn holds_calendar_date(text: &[u8]) -> bool {
    (0..text.len()).any(|at| iso_date_at(text, at) || dotted_date_at(text, at))
}

/// The digit at `at`, if there is one.
fn digit(text: &[u8], at: usize) -> Option<u8> {
    text.get(at)
        .filter(|byte| byte.is_ascii_digit())
        .map(|byte| byte - b'0')
}

/// Two digits at `at`, as a number.
fn two_digits(text: &[u8], at: usize) -> Option<u8> {
    Some(digit(text, at)? * 10 + digit(text, at + 1)?)
}

/// `at`, moved past a `-` there.
fn past_dash(text: &[u8], at: usize) -> usize {
    if text.get(at) == Some(&b'-') {
        at + 1
    } else {
        at
    }
}

/// Whether `(19|20)YY-?MM-?DD` starts at `at`.
fn iso_date_at(text: &[u8], at: usize) -> bool {
    let century = two_digits(text, at);
    if !matches!(century, Some(19 | 20)) || two_digits(text, at + 2).is_none() {
        return false;
    }
    let month_at = past_dash(text, at + 4);
    let Some(month) = two_digits(text, month_at) else {
        return false;
    };
    let day_at = past_dash(text, month_at + 2);
    let Some(day) = two_digits(text, day_at) else {
        return false;
    };
    (1..=12).contains(&month) && (1..=31).contains(&day)
}

/// Whether `D{1,2}[./]D{1,2}[./]D{2}` starts at `at`.
fn dotted_date_at(text: &[u8], at: usize) -> bool {
    let separator = |at: usize| matches!(text.get(at), Some(b'.' | b'/'));
    [1, 2].into_iter().any(|first| {
        [1, 2].into_iter().any(|second| {
            let second_at = at + first + 1;
            let third_at = second_at + second + 1;
            (0..first).all(|offset| digit(text, at + offset).is_some())
                && separator(at + first)
                && (0..second).all(|offset| digit(text, second_at + offset).is_some())
                && separator(second_at + second)
                && two_digits(text, third_at).is_some()
        })
    })
}

/// The lapse context an occasion carries: none, or the open lapse's id, which the governor owns and
/// the caller passes in (ADR-041).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LapseContext {
    /// No lapse is open.
    NoLapse,
    /// A lapse is open, with this id: the study day its episode is anchored on.
    Open(StudyDay),
}

/// One occasion to route.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Occasion {
    kind: Kind,
    key: DedupeKey,
    origin: Surface,
    tier: Tier,
    text: String,
    study_day: StudyDay,
    lapse: LapseContext,
}

impl Occasion {
    /// An occasion of `kind`, keyed `key`, raised on `origin`, asking for `tier`, carrying `text`
    /// (the bot's HTML), for `study_day`, in the `lapse` context.
    ///
    /// # Errors
    ///
    /// [`OccasionError::TierNotDeclared`] when the kind does not declare `tier`, and
    /// [`OccasionError::ComebackWithoutLapse`] for a comeback-budget kind outside an open lapse.
    pub fn new(
        kind: Kind,
        key: DedupeKey,
        origin: Surface,
        tier: Tier,
        text: impl Into<String>,
        study_day: StudyDay,
        lapse: LapseContext,
    ) -> Result<Self, OccasionError> {
        if !kind.tiers().contains(&tier) {
            return Err(OccasionError::TierNotDeclared);
        }
        if kind.is_comeback() && lapse == LapseContext::NoLapse {
            return Err(OccasionError::ComebackWithoutLapse);
        }
        Ok(Self {
            kind,
            key,
            origin,
            tier,
            text: text.into(),
            study_day,
            lapse,
        })
    }

    /// The kind.
    #[must_use]
    pub const fn kind(&self) -> &Kind {
        &self.kind
    }

    /// The dedupe key.
    #[must_use]
    pub const fn key(&self) -> &DedupeKey {
        &self.key
    }

    /// The surface the occasion was raised on.
    #[must_use]
    pub const fn origin(&self) -> Surface {
        self.origin
    }

    /// The tier asked for.
    #[must_use]
    pub const fn tier(&self) -> Tier {
        self.tier
    }

    /// The text, as the bot's HTML.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The study day the occasion belongs to.
    #[must_use]
    pub const fn study_day(&self) -> StudyDay {
        self.study_day
    }

    /// The lapse context.
    #[must_use]
    pub const fn lapse(&self) -> LapseContext {
        self.lapse
    }
}

/// Why an occasion could not be made. A refusal names the rule and never the value, because a
/// refused text may carry what no log line should.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum OccasionError {
    /// The key is outside the token grammar or longer than [`DEDUPE_KEY_MAX_LEN`].
    #[error(
        "a dedupe key is an opaque token matching ^[a-z0-9][a-z0-9:._-]*$ of at most 128 bytes"
    )]
    KeyNotAToken,
    /// The key holds a calendar date.
    #[error("a dedupe key never holds a calendar date")]
    KeyHoldsDate,
    /// The tier is not one the kind declares.
    #[error("an occasion asks for a tier its kind declares")]
    TierNotDeclared,
    /// A comeback-budget kind was raised outside an open lapse.
    #[error("a kind on the comeback budget is raised only inside an open lapse")]
    ComebackWithoutLapse,
}
