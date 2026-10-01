//! The one router (SPEC-041 R1, R4 to R8, R13; ADR-041): every celebration, nudge, digest and alert
//! of either surface is decided here, in the policy's order, recorded in the decision ledger, and
//! delivered through the port's calls: the bot transport's, which take this module's [`Pass`], so a
//! call of one elsewhere does not compile (SPEC-041 A2), and [`push_in_app`], which is private to
//! it. A census (A15) reads the shipped sources of the kinds A15 names and refuses in them a
//! delivery around the port by a name it holds: the bot's own send, edit or command handler
//! outside their named call sites, and the handler's replies outside their named callers; one of
//! the send or delivery methods of the pinned client's table named anywhere but its named send,
//! and the Bot API's host named outside the bot's sources; the in-app feed or the held queue
//! named outside this module, the ledger and the data-rights port, which own their writes, or in
//! this crate the ledger named outside them, its root's declaration aside; and in this crate
//! `#[path]`, `#[macro_export]`, `#[macro_use]`, or a `pub use` of the ledger, its feed's and
//! queue's tables or its writes to the feed and the queue. It guards ordinary code, not code
//! written to evade it, such as a request or a table's name assembled from parts, or a `pub`
//! wrapper under another name, which review catches (#297).
//!
//! [`Router::route`] decides one occasion inside one `BEGIN IMMEDIATE` write: the kind's switch;
//! the claim of its key in its dedupe scope, which a later withhold releases; the lapse; the quiet
//! window; the comeback cap; and whether a transport answers. A bot send commits its claim before
//! the call and records what the call came to after it. [`Router::flush`] delivers what quiet hours
//! and failed sends held, at most two in full and the rest in one recap line that names every
//! celebration it rolls up or abandons.
//!
//! A celebration's tier is the ladder's (SPEC-084): the tier its event or rarity asks for, after the
//! owner's weekly budget and the streak-break cap, chosen in the same write before the quiet window
//! is read, so a celebration held in quiet hours is held at its tier. Each tier renders through the
//! port's calls of its own: a reaction, the line, the reveal, a dice, and a pinned message.

use std::cmp::Reverse;
use std::fmt;
use std::sync::{Arc, Mutex, PoisonError};

use deck_streak_kernel::{Clock, Db, KernelError, StudyDay, StudyDayRule, UtcMillis};
use sqlx::{Sqlite, SqliteConnection, Transaction};

use crate::ladder::{self, DICE_EMOJI, REACTION_EMOJI, REVEAL_PAUSE, REVEAL_PLACEHOLDER};
use crate::ledger::{self, Abandoned, ClaimRow, DecisionRow, HeldRow};
use crate::occasion::{Class, DedupeScope, LapseContext, Occasion, StreakFacts, Surface, Tier};
use crate::owner_message;
use crate::photo::{FileId, Photo, check_caption};
use crate::policy::Policy;
use crate::quiet::{in_quiet_hours, local_minute};
use crate::transport::{BotTransport, PhotoPushed, Prepared, Pushed};

/// The owner's override of the quiet window's start, in minutes of the day (the predecessor's key).
pub const QUIET_START_SETTING: &str = "quiet_start_min";
/// The owner's override of the quiet window's end, in minutes of the day (the predecessor's key).
pub const QUIET_END_SETTING: &str = "quiet_end_min";
/// What a withheld occasion's kind is recorded with appended, so it never stands for a delivery.
pub const WITHHELD_SUFFIX: &str = ":withheld";
/// The owner's celebration intensity, which picks the weekly budget (the predecessor's key).
pub const INTENSITY_SETTING: &str = "celebration_intensity";

/// One minute, in milliseconds.
const MINUTE_MS: i64 = 60_000;

/// The setting key a flush holds while it delivers, so two flushers over one queue never send one
/// item twice (#291). Its value is the instant the lease lapses, in epoch milliseconds.
pub const FLUSH_LEASE_SETTING: &str = "flush_lease";

/// How long a flush's lease lasts when the flush never releases it, so a crashed flush does not
/// block the queue for good.
const FLUSH_LEASE_MS: i64 = 10 * MINUTE_MS;

/// The router's pass. Every bot transport call takes one, and only this module can make one: its
/// field is private, and it derives neither `Default` nor `Clone`. So a call of the port outside
/// the router does not compile (SPEC-041 A2).
#[derive(Debug)]
pub struct Pass(());

/// The pass the router hands a transport for each call.
const PASS: Pass = Pass(());

/// Why an occasion was withheld: the policy's withhold reasons that the router records.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Reason {
    /// The owner switched the kind off.
    NudgesDisabled,
    /// A delivery already holds the key in its scope.
    AlreadyRecorded,
    /// A nudge in an open lapse.
    Lapse,
    /// Inside the quiet window.
    QuietHours,
    /// The comeback's cap or gap.
    BudgetSpent,
    /// No transport answers for the surface.
    NoNotifier,
    /// The joined transport has no photo call.
    PhotoUnsupported,
}

impl Reason {
    /// Every reason the router records; the policy must list each (R2).
    pub const ALL: [Self; 7] = [
        Self::NudgesDisabled,
        Self::AlreadyRecorded,
        Self::Lapse,
        Self::QuietHours,
        Self::BudgetSpent,
        Self::NoNotifier,
        Self::PhotoUnsupported,
    ];

    /// The reason as the policy and the ledger spell it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NudgesDisabled => "nudges_disabled",
            Self::AlreadyRecorded => "already_recorded",
            Self::Lapse => "lapse",
            Self::QuietHours => "quiet_hours",
            Self::BudgetSpent => "budget_spent",
            Self::NoNotifier => "no_notifier",
            Self::PhotoUnsupported => "photo_unsupported",
        }
    }
}

/// Why a celebration is held on the queue.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Hold {
    /// The quiet window deferred it, or a reaction waits for the owner's next message.
    Quiet,
    /// A send failed or a reaction was refused, or its breaker was open.
    Send,
}

impl Hold {
    /// The hold as the ledger spells it: the deferral's reason.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Quiet => "quiet",
            Self::Send => "send",
        }
    }

    /// The hold the ledger spelled `text`.
    fn parse(text: &str) -> Self {
        if text == Self::Send.as_str() {
            Self::Send
        } else {
            Self::Quiet
        }
    }

    /// The reason an abandoned celebration held this way is recorded with.
    const fn abandoned(self) -> Reason {
        match self {
            Self::Quiet => Reason::QuietHours,
            Self::Send => Reason::NoNotifier,
        }
    }
}

/// What [`Router::route`] decided, as the ledger recorded it.
#[must_use]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision {
    /// Delivered on `surface`, rendered at `tier`.
    Sent {
        /// The surface it went to.
        surface: Surface,
        /// The tier it rendered at.
        tier: Tier,
    },
    /// Held on the queue for a flush.
    Deferred {
        /// The surface it will go to.
        surface: Surface,
        /// Why it is held.
        hold: Hold,
    },
    /// Withheld, for `reason`.
    Withheld {
        /// The surface it would have gone to.
        surface: Surface,
        /// Why.
        reason: Reason,
    },
}

/// Why a photo was not sent now, though nothing forbids it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NotNow {
    /// Inside the quiet window: the caller tries again after it.
    QuietHours,
    /// The send failed or the outage breaker is open: the caller tries again later.
    SendFailed,
}

/// What [`Router::route_photo`] decided.
#[must_use]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PhotoDecision {
    /// Delivered; Telegram holds the photo under `file_id`.
    Sent {
        /// The file id of the largest size.
        file_id: FileId,
    },
    /// Not sent, held nowhere and recorded nowhere: the caller keeps the photo and asks again.
    NotNow {
        /// Why.
        reason: NotNow,
    },
    /// Withheld, for `reason`, and recorded.
    Withheld {
        /// Why.
        reason: Reason,
    },
}

/// What [`Router::flush`] did.
#[must_use]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Flushed {
    /// No bot transport is joined, so nothing can be delivered or named: the queue is left.
    NoNotifier,
    /// Inside the quiet window: the queue is left.
    QuietHours,
    /// The outage breaker is open: the queue is left.
    BreakerOpen,
    /// Another flush holds the queue's lease: this one delivers nothing, so no item is sent twice.
    Busy,
    /// It ran, and delivered this many messages: full renders and the recap line.
    Ran {
        /// Messages delivered.
        sends: u32,
    },
}

/// Takes the flush lease on `write`'s transaction: its token, or `None` when another flush holds
/// an unlapsed one. The token is the instant the lease lapses.
async fn take_lease(
    write: &mut SqliteConnection,
    now: UtcMillis,
) -> Result<Option<String>, KernelError> {
    if let Some(held) = ledger::setting(write, FLUSH_LEASE_SETTING).await?
        && held
            .parse::<i64>()
            .is_ok_and(|lapses| lapses > now.epoch_millis())
    {
        return Ok(None);
    }
    let token = lease_token(now);
    sqlx::query(
        "INSERT INTO notification_settings (key, value, created_at) VALUES (?, ?, ?) \
         ON CONFLICT (key) DO UPDATE SET value = excluded.value",
    )
    .bind(FLUSH_LEASE_SETTING)
    .bind(&token)
    .bind(now.epoch_millis())
    .execute(write)
    .await?;
    Ok(Some(token))
}

/// The token of the flush that starts at `now`: the instant its lease lapses. It is the lease's
/// value and the claim written on every row the flush takes, so a row's claim lapses with the
/// lease that took it.
fn lease_token(now: UtcMillis) -> String {
    (now.epoch_millis() + FLUSH_LEASE_MS).to_string()
}

/// What the rules decided before any delivery call.
enum Verdict {
    Withhold(Reason),
    Defer(Hold),
    SendInApp,
    SendBot(i64),
}

/// What one render on the bot came to.
enum Outcome {
    /// Delivered, rendered at this tier.
    Delivered(Tier),
    /// No message was sent: a reaction not attempted, or refused. The celebration is held for this
    /// reason, its failed sends unchanged.
    Held(Hold),
    /// The message failed, which opens the outage breaker.
    Failed,
}

/// What one decision is about: an occasion being routed, or a held celebration being flushed.
struct Subject<'a> {
    key: &'a str,
    kind: &'a str,
    surface: Surface,
    requested: Tier,
    study_day: i64,
}

impl<'a> Subject<'a> {
    fn routed(occasion: &'a Occasion, surface: Surface, requested: Tier) -> Self {
        Self {
            key: occasion.key().as_str(),
            kind: occasion.kind().name(),
            surface,
            requested,
            study_day: occasion.study_day().epoch_day(),
        }
    }

    fn held(row: &'a HeldRow) -> Self {
        Self {
            key: &row.dedupe_key,
            kind: &row.kind,
            surface: row.surface,
            requested: row.tier_requested,
            study_day: row.study_day,
        }
    }

    fn row(
        &self,
        arm: &'static str,
        reason: Option<&'static str>,
        rendered: Tier,
        now: UtcMillis,
    ) -> DecisionRow<'a> {
        DecisionRow {
            dedupe_key: self.key,
            kind: self.kind.to_owned(),
            surface: self.surface,
            arm,
            reason,
            tier_requested: self.requested,
            tier_rendered: rendered,
            study_day: self.study_day,
            created_at: now,
        }
    }

    fn sent(&self, rendered: Tier, now: UtcMillis) -> DecisionRow<'a> {
        self.row("send", None, rendered, now)
    }

    fn deferred(&self, hold: Hold, now: UtcMillis) -> DecisionRow<'a> {
        self.row("defer", Some(hold.as_str()), Tier::T0, now)
    }

    fn withheld(&self, reason: Reason, now: UtcMillis) -> DecisionRow<'a> {
        DecisionRow {
            kind: format!("{}{WITHHELD_SUFFIX}", self.kind),
            ..self.row("withhold", Some(reason.as_str()), Tier::T0, now)
        }
    }
}

/// The one router.
pub struct Router {
    policy: Arc<Policy>,
    db: Db,
    clock: Arc<dyn Clock>,
    rule: StudyDayRule,
    bot: Option<Arc<dyn BotTransport>>,
    /// When the last bot send failed: the outage breaker is open for the cooldown after it.
    failed_at: Mutex<Option<UtcMillis>>,
    /// When the last reaction was refused: the reaction breaker is open for the cooldown after it
    /// (SPEC-084 R9).
    reaction_failed_at: Mutex<Option<UtcMillis>>,
}

impl fmt::Debug for Router {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Router")
            .field("rule", &self.rule)
            .field("bot", &self.bot.is_some())
            .finish_non_exhaustive()
    }
}

impl Router {
    /// The router of `policy` over `db`, reading the time from `clock` and the local offset from
    /// `rule`, with no bot transport: until one is joined, a bot occasion is withheld with
    /// `no_notifier` and a flush does nothing.
    #[must_use]
    pub fn new(policy: Arc<Policy>, db: Db, clock: Arc<dyn Clock>, rule: StudyDayRule) -> Self {
        Self {
            policy,
            db,
            clock,
            rule,
            bot: None,
            failed_at: Mutex::new(None),
            reaction_failed_at: Mutex::new(None),
        }
    }

    /// This router, delivering the bot's occasions through `bot`.
    #[must_use]
    pub fn with_bot(mut self, bot: Arc<dyn BotTransport>) -> Self {
        self.bot = Some(bot);
        self
    }

    /// Routes `photo` for `occasion` to the owner's chat, in the rules' order (SPEC-132 R4): the
    /// kind's switch, the claim of its key, the quiet window and the breaker, then one
    /// `push_photo`. A photo asks T2 whatever its event asks, so it never spends a T4 or T5 of the
    /// week's budget (R5). It is held nowhere: in the quiet window or with the breaker open, or
    /// when its send fails, the answer is [`PhotoDecision::NotNow`], its claim is left free and
    /// nothing is recorded, so the caller keeps the image and asks again. A transport with no
    /// photo call records the photo withheld as `photo_unsupported` and sends nothing else (R6).
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when the ledger cannot be read or written.
    pub async fn route_photo(
        &self,
        occasion: &Occasion,
        photo: &Photo,
    ) -> Result<PhotoDecision, KernelError> {
        let now = self.clock.now();
        let subject = Subject::routed(occasion, Surface::Bot, Tier::T2);
        let mut write = self.db.write().await?;
        let claim = match self.decide(&mut write, occasion, Surface::Bot, now).await? {
            Verdict::Withhold(reason) => {
                ledger::record(&mut write, &subject.withheld(reason, now)).await?;
                write.commit().await?;
                return Ok(PhotoDecision::Withheld { reason });
            }
            Verdict::Defer(hold) => {
                // Dropping the write rolls the claim back: nothing is held or recorded.
                drop(write);
                return Ok(PhotoDecision::NotNow {
                    reason: match hold {
                        Hold::Quiet => NotNow::QuietHours,
                        Hold::Send => NotNow::SendFailed,
                    },
                });
            }
            Verdict::SendInApp => {
                drop(write);
                return Ok(PhotoDecision::Withheld {
                    reason: Reason::NoNotifier,
                });
            }
            Verdict::SendBot(claim) => claim,
        };
        write.commit().await?;
        let pushed = match &self.bot {
            Some(bot) => bot.push_photo(&PASS, photo, photo.caption()).await,
            None => PhotoPushed::Unsupported,
        };
        let now = self.clock.now();
        let mut write = self.db.write().await?;
        let decision = match pushed {
            PhotoPushed::Delivered { file_id } => {
                ledger::record(&mut write, &subject.sent(Tier::T2, now)).await?;
                PhotoDecision::Sent { file_id }
            }
            PhotoPushed::Failed => {
                self.trip(now);
                ledger::release(&mut write, claim).await?;
                PhotoDecision::NotNow {
                    reason: NotNow::SendFailed,
                }
            }
            PhotoPushed::Unsupported => {
                tracing::warn!(
                    call = "push_photo",
                    "the bot transport has no such call: the photo is withheld"
                );
                ledger::release(&mut write, claim).await?;
                let reason = Reason::PhotoUnsupported;
                ledger::record(&mut write, &subject.withheld(reason, now)).await?;
                PhotoDecision::Withheld { reason }
            }
        };
        write.commit().await?;
        Ok(decision)
    }

    /// Prepares the photo Telegram holds as `file`, captioned `caption`, for the owner to share
    /// (SPEC-132 R8): one `prepare_share`, no ledger row, no delivery. The transport's outcome is
    /// the answer, and with no transport joined it is [`Prepared::Unsupported`]. A caption over the
    /// Bot API's bound, the photo's own, is [`Prepared::Refused`] before any call is made.
    pub async fn prepare_share(&self, file: &FileId, caption: &str) -> Prepared {
        if let Err(refusal) = check_caption(caption) {
            return Prepared::Refused(refusal);
        }
        match &self.bot {
            Some(bot) => bot.prepare_share(&PASS, file, caption).await,
            None => Prepared::Unsupported,
        }
    }

    /// Decides `occasion`, delivers it when the decision is to send, and records the decision.
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when the ledger cannot be read or written.
    pub async fn route(&self, occasion: &Occasion) -> Result<Decision, KernelError> {
        let now = self.clock.now();
        let celebration = occasion.kind().class() == Class::Celebration;
        let surface = if celebration && occasion.origin() == Surface::MiniApp {
            Surface::MiniApp
        } else {
            Surface::Bot
        };
        let requested = self.requested(occasion);
        let subject = Subject::routed(occasion, surface, requested);
        let mut write = self.db.write().await?;
        let tier = if celebration {
            self.ladder_tier(&mut write, occasion, requested).await?
        } else {
            rendered(requested)
        };
        let decision = match self.decide(&mut write, occasion, surface, now).await? {
            Verdict::Withhold(reason) => {
                ledger::record(&mut write, &subject.withheld(reason, now)).await?;
                Decision::Withheld { surface, reason }
            }
            Verdict::Defer(hold) => {
                let row = held_row(&subject, occasion.text(), tier, hold, 0, now);
                self.hold(&mut write, &row, now).await?;
                ledger::record(&mut write, &subject.deferred(hold, now)).await?;
                Decision::Deferred { surface, hold }
            }
            Verdict::SendInApp => {
                push_in_app(&mut write, &subject, tier, occasion.text(), now).await?;
                ledger::record(&mut write, &subject.sent(tier, now)).await?;
                Decision::Sent { surface, tier }
            }
            Verdict::SendBot(claim) => {
                write.commit().await?;
                return self.send_bot(occasion, &subject, claim, tier).await;
            }
        };
        write.commit().await?;
        Ok(decision)
    }

    /// The tier `occasion` asks for: a celebration's event's or rarity's on the ladder (SPEC-084 R1),
    /// else the tier it was raised at.
    fn requested(&self, occasion: &Occasion) -> Tier {
        occasion.event().map_or_else(
            || occasion.tier(),
            |event| ladder::requested_tier(&self.policy, event, occasion.rarity()),
        )
    }

    /// The tier the ladder renders the celebration `occasion` at (SPEC-084 R2 to R5): `requested`
    /// after the owner's weekly budget, counted over the week of its study day, then the
    /// streak-break cap of that day.
    async fn ladder_tier(
        &self,
        write: &mut SqliteConnection,
        occasion: &Occasion,
        requested: Tier,
    ) -> Result<Tier, KernelError> {
        let intensity = ledger::setting(write, INTENSITY_SETTING)
            .await?
            .unwrap_or_default();
        let since = ladder::week_start(occasion.study_day());
        let at_or_above_5 = self.week_count(write, since, Tier::T5).await?;
        let at_or_above_4 = self.week_count(write, since, Tier::T4).await?;
        let used = (
            u32::try_from(at_or_above_4 - at_or_above_5).unwrap_or(u32::MAX),
            u32::try_from(at_or_above_5).unwrap_or(u32::MAX),
        );
        let event = occasion.event().unwrap_or_default();
        let budgeted = ladder::apply_budget(
            &self.policy,
            requested,
            used,
            ladder::weekly_budget(&self.policy, &intensity),
            ladder::budget_exempt(&self.policy, event),
            ladder::rare_floor(&self.policy, occasion.rarity()),
        );
        let broke = ladder::streak_broke_on(occasion.streak(), occasion.study_day());
        Ok(budgeted.min(ladder::outcome_cap(&self.policy, broke)))
    }

    /// Renders `occasion` on the bot at `tier` after its claim was committed, and records what that
    /// came to: a celebration by the ladder's renders (SPEC-084 R8, R9), any other occasion as its
    /// line.
    async fn send_bot(
        &self,
        occasion: &Occasion,
        subject: &Subject<'_>,
        claim: i64,
        tier: Tier,
    ) -> Result<Decision, KernelError> {
        let celebration = occasion.kind().class() == Class::Celebration;
        let outcome = match &self.bot {
            Some(bot) if celebration => {
                self.render(bot.as_ref(), occasion.text(), tier, true)
                    .await?
            }
            _ => delivered(self.push(occasion.text(), tier).await, tier),
        };
        let now = self.clock.now();
        let mut write = self.db.write().await?;
        let decision = match outcome {
            Outcome::Delivered(rendered) => {
                ledger::record(&mut write, &subject.sent(rendered, now)).await?;
                Decision::Sent {
                    surface: Surface::Bot,
                    tier: rendered,
                }
            }
            Outcome::Held(hold) => {
                let row = held_row(subject, occasion.text(), tier, hold, 0, now);
                self.hold(&mut write, &row, now).await?;
                ledger::record(&mut write, &subject.deferred(hold, now)).await?;
                Decision::Deferred {
                    surface: Surface::Bot,
                    hold,
                }
            }
            Outcome::Failed => {
                self.trip(now);
                if celebration {
                    let row = held_row(subject, occasion.text(), tier, Hold::Send, 1, now);
                    self.hold(&mut write, &row, now).await?;
                    ledger::record(&mut write, &subject.deferred(Hold::Send, now)).await?;
                    Decision::Deferred {
                        surface: Surface::Bot,
                        hold: Hold::Send,
                    }
                } else {
                    ledger::release(&mut write, claim).await?;
                    ledger::record(&mut write, &subject.withheld(Reason::NoNotifier, now)).await?;
                    Decision::Withheld {
                        surface: Surface::Bot,
                        reason: Reason::NoNotifier,
                    }
                }
            }
        };
        write.commit().await?;
        Ok(decision)
    }

    /// Renders `text` on the bot at `tier` (SPEC-084 R8): nothing at T0; a reaction at T1; the line
    /// at T2; the reveal at T3, or the line where `reveal` is false, as a flush renders a T3, since a
    /// reveal replayed hours later holds no suspense; a dice then the line at T4; and a dice then the
    /// pinned message at T5. A reveal or a pin the transport does not implement degrades to the
    /// line, at T2.
    async fn render(
        &self,
        bot: &dyn BotTransport,
        text: &str,
        tier: Tier,
        reveal: bool,
    ) -> Result<Outcome, KernelError> {
        Ok(match tier {
            Tier::T0 => Outcome::Delivered(Tier::T0),
            Tier::T1 => self.react(bot).await?,
            Tier::T2 => delivered(bot.push_message(&PASS, text).await, Tier::T2),
            Tier::T3 if !reveal => delivered(bot.push_message(&PASS, text).await, Tier::T3),
            Tier::T3 => {
                let pushed = bot
                    .push_reveal(&PASS, REVEAL_PLACEHOLDER, text, REVEAL_PAUSE)
                    .await;
                or_line(bot, pushed, "push_reveal", text, tier).await
            }
            Tier::T4 => {
                dice(bot).await;
                delivered(bot.push_message(&PASS, text).await, Tier::T4)
            }
            Tier::T5 => {
                dice(bot).await;
                let pushed = bot.push_pin(&PASS, text).await;
                or_line(bot, pushed, "push_pin", text, tier).await
            }
        })
    }

    /// The T1 render (SPEC-084 R9): a reaction to the owner's latest message. While the reaction
    /// breaker is open it is not attempted and the celebration is held for the send; without a
    /// message at most the ladder's age old it is held for the owner to write. A refused reaction
    /// opens the reaction breaker, never the outage breaker; a transport without reactions renders
    /// the celebration silent, since a line would exceed T1.
    async fn react(&self, bot: &dyn BotTransport) -> Result<Outcome, KernelError> {
        let now = self.clock.now();
        if self.reaction_breaker_open(now) {
            return Ok(Outcome::Held(Hold::Send));
        }
        let latest = owner_message::latest(&self.db).await?;
        let Some(latest) =
            latest.filter(|latest| ladder::reaction_fresh(&self.policy, latest.arrived_at, now))
        else {
            return Ok(Outcome::Held(Hold::Quiet));
        };
        Ok(
            match bot
                .push_reaction(&PASS, latest.message_id, REACTION_EMOJI)
                .await
            {
                Pushed::Delivered => Outcome::Delivered(Tier::T1),
                Pushed::Failed => {
                    stamp(&self.reaction_failed_at, now);
                    Outcome::Held(Hold::Send)
                }
                Pushed::Unsupported => {
                    tracing::warn!(
                        call = "push_reaction",
                        "the bot transport has no such call: the celebration renders silent"
                    );
                    Outcome::Delivered(Tier::T0)
                }
            },
        )
    }

    /// The flush's held T1s (SPEC-084 R11): each reacts in turn, and one delivered is settled and
    /// recorded at the tier it rendered; a reaction not made leaves its celebration held as it was.
    /// Answers how many were delivered.
    async fn flush_reactions(
        &self,
        bot: &dyn BotTransport,
        reactions: &[HeldRow],
    ) -> Result<u32, KernelError> {
        let mut sends = 0;
        for row in reactions {
            if let Outcome::Delivered(tier) = self.react(bot).await? {
                let mut write = self.db.write().await?;
                let claim = row.claim.as_deref().unwrap_or_default();
                if ledger::settle_claimed(&mut write, row.id, claim).await? {
                    let sent = Subject::held(row).sent(tier, self.clock.now());
                    ledger::record(&mut write, &sent).await?;
                    sends += 1;
                }
                write.commit().await?;
            }
        }
        Ok(sends)
    }

    /// The rules of R4 in order: the kind's switch, then the claim, then the rules after it; a
    /// withhold after the claim releases it.
    async fn decide(
        &self,
        write: &mut SqliteConnection,
        occasion: &Occasion,
        surface: Surface,
        now: UtcMillis,
    ) -> Result<Verdict, KernelError> {
        let kind = occasion.kind();
        if let Some(setting) = kind.setting()
            && ledger::setting(write, setting).await?.as_deref()
                == Some(self.policy.comeback.disable_value.as_str())
        {
            return Ok(Verdict::Withhold(Reason::NudgesDisabled));
        }
        let lapse_id = match occasion.lapse() {
            LapseContext::Open(id) => Some(id.epoch_day()),
            LapseContext::NoLapse => None,
        };
        let study_day = occasion.study_day().epoch_day();
        let scope = scope(kind.dedupe(), study_day, lapse_id);
        let row = ClaimRow {
            kind: kind.name(),
            dedupe_key: occasion.key().as_str(),
            scope: &scope,
            surface,
            study_day,
            lapse_id,
            created_at: now,
        };
        let Some(claim) = ledger::claim(write, &row).await? else {
            return Ok(Verdict::Withhold(Reason::AlreadyRecorded));
        };
        let verdict = self
            .after_claim(write, occasion, surface, now, (claim, lapse_id))
            .await?;
        if let Verdict::Withhold(_) = verdict {
            ledger::release(write, claim).await?;
        }
        Ok(verdict)
    }

    /// The rules after the claim: the lapse, the quiet window, the comeback cap and the transport.
    async fn after_claim(
        &self,
        write: &mut SqliteConnection,
        occasion: &Occasion,
        surface: Surface,
        now: UtcMillis,
        (claim, lapse_id): (i64, Option<i64>),
    ) -> Result<Verdict, KernelError> {
        let kind = occasion.kind();
        let celebration = kind.class() == Class::Celebration;
        if lapse_id.is_some()
            && self.policy.lapse.suppress_classes.contains(&kind.class())
            && !kind.is_comeback()
        {
            return Ok(Verdict::Withhold(Reason::Lapse));
        }
        if !self
            .policy
            .quiet_hours
            .exempt_classes
            .contains(&kind.class())
            && self.in_quiet_window(write, now).await?
        {
            return Ok(if celebration {
                Verdict::Defer(Hold::Quiet)
            } else {
                Verdict::Withhold(Reason::QuietHours)
            });
        }
        if let (true, Some(lapse_id)) = (kind.is_comeback(), lapse_id) {
            let (sends, last) = ledger::lapse_sends(write, kind.name(), lapse_id, claim).await?;
            let gap = i64::from(self.policy.comeback.min_gap_days);
            let too_soon = last.is_some_and(|last| occasion.study_day().epoch_day() - last < gap);
            if sends >= i64::from(self.policy.comeback.max_per_episode) || too_soon {
                return Ok(Verdict::Withhold(Reason::BudgetSpent));
            }
        }
        Ok(match surface {
            Surface::MiniApp => Verdict::SendInApp,
            Surface::Bot if self.bot.is_none() => Verdict::Withhold(Reason::NoNotifier),
            Surface::Bot if self.breaker_open(now) => {
                if celebration {
                    Verdict::Defer(Hold::Send)
                } else {
                    Verdict::Withhold(Reason::NoNotifier)
                }
            }
            Surface::Bot => Verdict::SendBot(claim),
        })
    }

    /// Delivers what quiet hours and failed sends held, when a bot transport is joined, the quiet
    /// window has ended and the breaker is closed (R7, R8). A celebration held longer than the
    /// policy's age is abandoned; at most the policy's flush count render in full, loudest first
    /// and in the order they were held; and one recap line names the rest and every abandoned
    /// celebration. The first failed send ends the flush.
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when the ledger cannot be read or written.
    pub async fn flush(&self) -> Result<Flushed, KernelError> {
        self.flush_with(None).await
    }

    /// [`Router::flush`], re-capping each held celebration for the streak's `facts` on the flush's
    /// study day (SPEC-084 R11). A held T1 first reacts, each in turn, and a reaction not made
    /// leaves it held as it was; a celebration held at T0, or past the policy's age or the queue's
    /// bound, is abandoned; of the rest, the two of highest held tier render in full at the smaller
    /// of that tier and the cap, the older first on a tie and in the order they were held, and the
    /// recap line settles every other one at most at T2.
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when the ledger cannot be read or written.
    pub async fn flush_with(&self, facts: Option<StreakFacts>) -> Result<Flushed, KernelError> {
        let Some(bot) = &self.bot else {
            return Ok(Flushed::NoNotifier);
        };
        let now = self.clock.now();
        let mut write = self.db.write().await?;
        if self.in_quiet_window(&mut write, now).await? {
            return Ok(Flushed::QuietHours);
        }
        if self.breaker_open(now) {
            return Ok(Flushed::BreakerOpen);
        }
        let Some(lease) = take_lease(&mut write, now).await? else {
            return Ok(Flushed::Busy);
        };
        let flushed = self.deliver(bot.as_ref(), facts, now, write).await;
        let mut release = self.db.write().await?;
        sqlx::query("DELETE FROM notification_settings WHERE key = ? AND value = ?")
            .bind(FLUSH_LEASE_SETTING)
            .bind(&lease)
            .execute(&mut *release)
            .await?;
        ledger::release_claims(&mut release, &lease).await?;
        release.commit().await?;
        flushed
    }

    /// The flush's work after its window, breaker and lease checks passed: `write` is the open
    /// transaction that holds the lease.
    async fn deliver(
        &self,
        bot: &dyn BotTransport,
        facts: Option<StreakFacts>,
        now: UtcMillis,
        mut write: Transaction<'static, Sqlite>,
    ) -> Result<Flushed, KernelError> {
        let broke = ladder::streak_broke_on(facts.as_ref(), self.rule.study_day(now));
        let cap = ladder::outcome_cap(&self.policy, broke);
        let max_age = i64::from(self.policy.deferral.max_age_minutes) * MINUTE_MS;
        let token = lease_token(now);
        for (id, key, claim) in ledger::lapsed_claims(&mut write, now.epoch_millis()).await? {
            if ledger::abandon_lapsed(&mut write, id).await? {
                lapsed_notice(&key, &claim);
            }
        }
        let (mut reactions, mut rest) = (Vec::new(), Vec::new());
        for row in ledger::claim_held(&mut write, &token).await? {
            if now.epoch_millis() - row.deferred_at > max_age || row.tier_pending == Tier::T0 {
                let reason = Hold::parse(&row.hold).abandoned();
                self.abandon(&mut write, &row, (reason, "expired"), now)
                    .await?;
            } else if row.tier_pending == Tier::T1 && row.surface == Surface::Bot {
                reactions.push(row);
            } else {
                rest.push(row);
            }
        }
        let bound = usize::try_from(self.policy.deferral.queue_max).unwrap_or(usize::MAX);
        let mut full = ranked(rest);
        for row in full.split_off(full.len().min(bound)) {
            let reason = Hold::parse(&row.hold).abandoned();
            self.abandon(&mut write, &row, (reason, "over the queue bound"), now)
                .await?;
        }
        write.commit().await?;
        let mut sends = self.flush_reactions(bot, &reactions).await?;
        let flush_max = usize::try_from(self.policy.deferral.flush_max).unwrap_or(usize::MAX);
        let rolled = full.split_off(full.len().min(flush_max));
        full.sort_by_key(|row| row.id);
        for row in &full {
            let tier = row.tier_pending.min(cap);
            let outcome = match row.surface {
                Surface::MiniApp => Outcome::Delivered(tier),
                Surface::Bot => self.render(bot, &row.text, tier, false).await?,
            };
            let now = self.clock.now();
            let mut write = self.db.write().await?;
            match outcome {
                Outcome::Delivered(rendered) => {
                    let claim = row.claim.as_deref().unwrap_or_default();
                    if ledger::settle_claimed(&mut write, row.id, claim).await? {
                        let subject = Subject::held(row);
                        if row.surface == Surface::MiniApp {
                            push_in_app(&mut write, &subject, rendered, &row.text, now).await?;
                        }
                        ledger::record(&mut write, &subject.sent(rendered, now)).await?;
                        sends += 1;
                    }
                }
                Outcome::Held(_) => {}
                Outcome::Failed => {
                    self.trip(now);
                    self.retry_or_abandon(&mut write, row, Hold::Send, now)
                        .await?;
                    write.commit().await?;
                    return Ok(Flushed::Ran { sends });
                }
            }
            write.commit().await?;
        }
        let mut read = self.db.write().await?;
        let abandoned = ledger::abandoned(&mut read).await?;
        drop(read);
        if rolled.is_empty() && abandoned.is_empty() {
            return Ok(Flushed::Ran { sends });
        }
        let pushed = bot.push_message(&PASS, &recap(&rolled, &abandoned)).await;
        let now = self.clock.now();
        let mut write = self.db.write().await?;
        if pushed == Pushed::Delivered {
            for row in &rolled {
                let tier = row.tier_pending.min(cap).min(Tier::T2);
                let claim = row.claim.as_deref().unwrap_or_default();
                if ledger::settle_claimed(&mut write, row.id, claim).await? {
                    ledger::record(&mut write, &Subject::held(row).sent(tier, now)).await?;
                }
            }
            for (id, _, _, _) in &abandoned {
                ledger::settle(&mut write, *id).await?;
            }
            sends += 1;
        } else {
            self.trip(now);
            for row in &rolled {
                let hold = Hold::parse(&row.hold);
                self.retry_or_abandon(&mut write, row, hold, now).await?;
            }
        }
        write.commit().await?;
        Ok(Flushed::Ran { sends })
    }

    /// How many celebrations of study day `since` or later were delivered at `min` or above, or are
    /// still held at `min` or above (SPEC-084 R4).
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when the ledger cannot be read.
    pub async fn celebrations_at_or_above(
        &self,
        since: StudyDay,
        min: Tier,
    ) -> Result<i64, KernelError> {
        let mut connection = self.db.reader().acquire().await?;
        self.week_count(&mut connection, since, min).await
    }

    /// [`Router::celebrations_at_or_above`], read on `connection`: the delivered ones and the held
    /// ones of every celebration kind, so a held celebration spends its week's budget (R4).
    async fn week_count(
        &self,
        connection: &mut SqliteConnection,
        since: StudyDay,
        min: Tier,
    ) -> Result<i64, KernelError> {
        let since = since.epoch_day();
        let mut count = 0;
        for kind in self.policy.celebration_kinds() {
            let delivered = ledger::delivered_at_or_above(connection, kind, since, min).await?;
            let held = ledger::held_at_or_above(connection, kind, since, min).await?;
            count += delivered + held;
        }
        Ok(count)
    }

    /// Holds `row` on the queue, then keeps the queue's bound: past the policy's count of held
    /// celebrations, the lowest-ranked is abandoned.
    async fn hold(
        &self,
        write: &mut SqliteConnection,
        row: &HeldRow,
        now: UtcMillis,
    ) -> Result<(), KernelError> {
        ledger::hold(write, row, now).await?;
        let bound = usize::try_from(self.policy.deferral.queue_max).unwrap_or(usize::MAX);
        for row in ranked(ledger::held(write).await?).iter().skip(bound) {
            let reason = Hold::parse(&row.hold).abandoned();
            self.abandon(write, row, (reason, "over the queue bound"), now)
                .await?;
        }
        Ok(())
    }

    /// Abandons the held `row`: it waits on the queue for a recap to name it, and its decision is
    /// a withhold for `reason`, what held it. A row this flush claimed is abandoned under its
    /// claim; the log line names the item, its claimant and `why` (SPEC-041 R14).
    async fn abandon(
        &self,
        write: &mut SqliteConnection,
        row: &HeldRow,
        (reason, why): (Reason, &str),
        now: UtcMillis,
    ) -> Result<(), KernelError> {
        let ours = match &row.claim {
            Some(claim) => ledger::abandon_claimed(write, row.id, claim, row.tries).await?,
            None => {
                ledger::abandon(write, row.id, row.tries).await?;
                true
            }
        };
        if !ours {
            return Ok(());
        }
        ledger::record(write, &Subject::held(row).withheld(reason, now)).await?;
        tracing::warn!(
            item = %row.dedupe_key,
            claimant = row.claim.as_deref().unwrap_or("none"),
            kind = %row.kind,
            reason = reason.as_str(),
            why,
            "a held celebration was abandoned"
        );
        Ok(())
    }

    /// Holds `row` again for `hold` after a failed send, its first deferral time kept, or abandons
    /// it once its retries are spent (R8). A row whose own message failed is held for the send; a
    /// row a failed recap only named keeps what held it (SPEC-084 R11). A row that is no longer
    /// this flush's claim is left alone.
    async fn retry_or_abandon(
        &self,
        write: &mut SqliteConnection,
        row: &HeldRow,
        hold: Hold,
        now: UtcMillis,
    ) -> Result<(), KernelError> {
        let tries = row.tries + 1;
        if tries > i64::from(self.policy.send_failure.retry_max) {
            let spent = HeldRow {
                tries,
                ..row.clone()
            };
            return self
                .abandon(write, &spent, (Hold::Send.abandoned(), "send failed"), now)
                .await;
        }
        let claim = row.claim.as_deref().unwrap_or_default();
        if ledger::relatch(write, row.id, claim, tries, hold.as_str()).await? {
            ledger::record(write, &Subject::held(row).deferred(hold, now)).await?;
        }
        Ok(())
    }

    /// Sends `text` to the bot at `rendered` as a line: nothing at T0, which delivers by sending
    /// nothing.
    async fn push(&self, text: &str, rendered: Tier) -> Pushed {
        match &self.bot {
            _ if rendered == Tier::T0 => Pushed::Delivered,
            Some(bot) => bot.push_message(&PASS, text).await,
            None => Pushed::Failed,
        }
    }

    /// Whether `now` falls in the owner's quiet window, or the policy's where the owner set none.
    async fn in_quiet_window(
        &self,
        write: &mut SqliteConnection,
        now: UtcMillis,
    ) -> Result<bool, KernelError> {
        let quiet = &self.policy.quiet_hours;
        let start = minutes_setting(write, QUIET_START_SETTING, quiet.start.minutes()).await?;
        let end = minutes_setting(write, QUIET_END_SETTING, quiet.end.minutes()).await?;
        Ok(in_quiet_hours(
            local_minute(now, self.rule.utc_offset()),
            start,
            end,
        ))
    }

    /// Whether the outage breaker is open: a bot send failed less than the cooldown before `now`.
    /// A clock that stepped back past the failure closes it.
    fn breaker_open(&self, now: UtcMillis) -> bool {
        self.cooling(&self.failed_at, now)
    }

    /// Whether the reaction breaker is open: a reaction was refused less than the cooldown before
    /// `now` (SPEC-084 R9).
    fn reaction_breaker_open(&self, now: UtcMillis) -> bool {
        self.cooling(&self.reaction_failed_at, now)
    }

    /// Whether the failure `failed_at` holds came less than the outage cooldown before `now`.
    fn cooling(&self, failed_at: &Mutex<Option<UtcMillis>>, now: UtcMillis) -> bool {
        let failed_at = *failed_at.lock().unwrap_or_else(PoisonError::into_inner);
        let cooldown = i64::from(self.policy.send_failure.outage_cooldown_ms);
        failed_at
            .is_some_and(|at| (0..cooldown).contains(&(now.epoch_millis() - at.epoch_millis())))
    }

    /// Opens the outage breaker at `now`.
    fn trip(&self, now: UtcMillis) {
        stamp(&self.failed_at, now);
    }
}

/// Records a failure at `now` in the breaker `failed_at`.
fn stamp(failed_at: &Mutex<Option<UtcMillis>>, now: UtcMillis) {
    *failed_at.lock().unwrap_or_else(PoisonError::into_inner) = Some(now);
}

/// What the push `pushed` came to, rendered at `tier`: a call the transport lacks delivers nothing.
const fn delivered(pushed: Pushed, tier: Tier) -> Outcome {
    match pushed {
        Pushed::Delivered => Outcome::Delivered(tier),
        Pushed::Failed | Pushed::Unsupported => Outcome::Failed,
    }
}

/// What the ladder's call `call` came to at `tier`, or, when the transport does not implement it,
/// what the line it degrades to came to, at T2, the call logged by its name (SPEC-084 R8).
async fn or_line(
    bot: &dyn BotTransport,
    pushed: Pushed,
    call: &'static str,
    text: &str,
    tier: Tier,
) -> Outcome {
    match pushed {
        Pushed::Unsupported => {
            tracing::warn!(
                call,
                tier = tier.as_str(),
                "the bot transport has no such call: the celebration renders degraded, as a line"
            );
            delivered(bot.push_message(&PASS, text).await, Tier::T2)
        }
        Pushed::Delivered | Pushed::Failed => delivered(pushed, tier),
    }
}

/// The dice a T4 and a T5 open with. Its outcome is never a delivery's (SPEC-084 R9); a transport
/// without it is logged by the call's name, and the tier renders on without it.
async fn dice(bot: &dyn BotTransport) {
    match bot.push_dice(&PASS, DICE_EMOJI).await {
        Pushed::Unsupported => tracing::warn!(
            call = "push_dice",
            "the bot transport has no such call: the celebration renders without its dice"
        ),
        Pushed::Delivered | Pushed::Failed => {}
    }
}

/// `rows` ranked for a flush and the queue's bound: the loudest held tier first, then the oldest
/// (SPEC-084 R11).
fn ranked(mut rows: Vec<HeldRow>) -> Vec<HeldRow> {
    rows.sort_by_key(|row| (Reverse(row.tier_pending), row.id));
    rows
}

/// The Mini App's delivery call: appends the item to the in-app feed the owner pulls, inside the
/// write that records its decision (R13).
async fn push_in_app(
    write: &mut SqliteConnection,
    subject: &Subject<'_>,
    rendered: Tier,
    text: &str,
    now: UtcMillis,
) -> Result<(), KernelError> {
    ledger::append_feed(write, subject.key, subject.kind, rendered, text, now).await
}

/// The owner's minutes for `key`, or `default` where the owner set none or set no number.
async fn minutes_setting(
    write: &mut SqliteConnection,
    key: &str,
    default: i64,
) -> Result<i64, KernelError> {
    Ok(ledger::setting(write, key)
        .await?
        .and_then(|value| value.parse().ok())
        .unwrap_or(default))
}

/// The tier a nudge, a digest or an alert renders at: nothing at T0, and a line at every other tier.
/// A celebration's is the ladder's (SPEC-084).
const fn rendered(requested: Tier) -> Tier {
    if matches!(requested, Tier::T0) {
        Tier::T0
    } else {
        Tier::T2
    }
}

/// The scope a delivery claims its key in: none for once-ever and per-incident, the study day, or
/// the lapse and the study day.
fn scope(dedupe: DedupeScope, study_day: i64, lapse_id: Option<i64>) -> String {
    match dedupe {
        DedupeScope::OnceEver | DedupeScope::PerIncident => String::new(),
        DedupeScope::PerStudyDay => format!("day:{study_day}"),
        DedupeScope::PerEpisodeDay => format!(
            "lapse:{}:day:{study_day}",
            lapse_id.map_or_else(String::new, |id| id.to_string())
        ),
    }
}

/// The queue's row for `subject`, its message `text`, held at `tier` for `hold` with `tries` failed
/// sends, deferred at `now`.
fn held_row(
    subject: &Subject<'_>,
    text: &str,
    tier: Tier,
    hold: Hold,
    tries: i64,
    now: UtcMillis,
) -> HeldRow {
    HeldRow {
        id: 0,
        kind: subject.kind.to_owned(),
        dedupe_key: subject.key.to_owned(),
        surface: subject.surface,
        tier_requested: subject.requested,
        tier_pending: tier,
        text: text.to_owned(),
        hold: hold.as_str().to_owned(),
        tries,
        deferred_at: now.epoch_millis(),
        study_day: subject.study_day,
        claim: None,
    }
}

/// Logs, in a line a person reads, that the claimed item `key` was abandoned because its claim
/// `claim` lapsed with its claimant gone or past its lease: the push may have reached the owner,
/// and it is never sent again (SPEC-041 R14).
fn lapsed_notice(key: &str, claim: &str) {
    tracing::warn!(
        item = key,
        claimant = claim,
        reason = "may have been sent",
        "a held celebration was abandoned"
    );
}

/// The recap line (the predecessor's `CelebrationsLayer._rollup_text`): a head that says what held
/// them, then each rolled-up celebration by its key, and each abandoned one with why it went unseen.
fn recap(rolled: &[HeldRow], abandoned: &[Abandoned]) -> String {
    let mut names = Vec::new();
    let (mut quiet, mut send) = (false, false);
    for row in rolled {
        names.push(row.dedupe_key.clone());
        match Hold::parse(&row.hold) {
            Hold::Quiet => quiet = true,
            Hold::Send => send = true,
        }
    }
    for (_, key, hold, claim) in abandoned {
        let hold = Hold::parse(hold);
        names.push(match (claim, hold) {
            (Some(_), _) => format!("{key} (may have been sent, unseen)"),
            (None, Hold::Quiet) => format!("{key} (expired, unseen)"),
            (None, Hold::Send) => format!("{key} (gave up retrying, unseen)"),
        });
        match hold {
            Hold::Quiet => quiet = true,
            Hold::Send => send = true,
        }
    }
    let head = match (quiet, send) {
        (false, true) => "\u{1f4e1} <b>{n} held celebration(s)</b> (send failures)",
        (true, true) => "\u{1f319} <b>{n} held celebration(s)</b> (quiet hours + send failures)",
        _ => "\u{1f319} <b>{n} held celebration(s)</b> (quiet hours)",
    }
    .replace("{n}", &names.len().to_string());
    let lines: Vec<String> = names
        .iter()
        .map(|name| format!("\u{2022} {name}"))
        .collect();
    format!("{head}\n{}", lines.join("\n"))
}
