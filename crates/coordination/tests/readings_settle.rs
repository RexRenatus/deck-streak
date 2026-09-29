//! The settle step (SPEC-047 A6, A7, A8; R4, R5, R7): a settle after a sync stores the studied
//! count, grants the studied XP and stamps the Studied line once on crossing the threshold, a
//! retired reading turns studied on late reviews made inside its window, and a read and studied
//! reading earns exactly 100 XP however often its grants are replayed.
//!
//! Every card, review and vault call here is synthetic and in memory.

// An integration test is test code: its helpers panic on a failed fixture, and counts are printed
// on purpose.
#![allow(clippy::expect_used, clippy::panic, clippy::print_stdout)]

use std::collections::BTreeSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use deck_streak_coordination::readings::generate::{PortFuture, VaultWriteFailed};
use deck_streak_coordination::readings::read_tap::{ReadTap, ReadTick};
use deck_streak_coordination::readings::settle::{
    ReviewsByCard, ReviewsUnreadable, Settle, StudiedStamp,
};
use deck_streak_ingest::reader::Review;
use deck_streak_kernel::{
    Db, Hour, KernelError, ManualClock, StudyDay, StudyDayRule, Track, UtcMillis, UtcOffset,
};
use deck_streak_progression::grant::{
    GrantAnswer, GrantPort, GrantRequest, GrantScope, GrantSource,
};
use deck_streak_progression::ledger::SqliteXpLedger;
use deck_streak_progression::xp::{XpAmount, XpTotal};
use deck_streak_readings::reading::ReadingId;
use deck_streak_readings::store::{NewReading, SqliteReadings, VaultStatus};
use deck_streak_readings::studied::Verdict;
use deck_streak_readings::topic::TopicKey;
use deck_streak_readings::xp::{read_source, studied_source};

const DAY_MS: i64 = 86_400_000;
const HOUR_MS: i64 = 3_600_000;
/// 05:00 UTC of epoch day 20 000: an hour into study day 20 000 under the default rule.
const START: i64 = 20_000 * DAY_MS + 5 * HOUR_MS;
/// The rollover that starts study day 20 002: the window's close.
const CLOSE: i64 = 20_002 * DAY_MS + 4 * HOUR_MS;
const TOPIC: &str = "law/synthetic-contracts";

#[derive(Clone, Default)]
struct Calls(Arc<Mutex<u32>>);

impl Calls {
    fn count(&self) -> u32 {
        *self.0.lock().expect("a counter")
    }
    fn bump(&self) {
        *self.0.lock().expect("a counter") += 1;
    }
}

/// The study days a port was handed, as epoch days, in call order.
#[derive(Clone, Default)]
struct Days(Arc<Mutex<Vec<i64>>>);

impl Days {
    fn push(&self, day: StudyDay) {
        self.0.lock().expect("days").push(day.epoch_day());
    }
    fn all(&self) -> Vec<i64> {
        self.0.lock().expect("days").clone()
    }
}

/// The XP ledger, recording the study day of every request it is asked to grant.
struct RecordingGrant {
    inner: SqliteXpLedger,
    days: Days,
}

impl GrantPort for RecordingGrant {
    async fn grant(
        &self,
        request: &GrantRequest,
        at: UtcMillis,
    ) -> Result<GrantAnswer, KernelError> {
        self.days.push(request.study_day);
        self.inner.grant(request, at).await
    }
}

#[derive(Clone, Default)]
struct FakeStamp {
    calls: Calls,
    days: Days,
    failing: Arc<AtomicBool>,
}

impl StudiedStamp for FakeStamp {
    fn stamp_studied<'a>(
        &'a self,
        _topic: &'a TopicKey,
        today: StudyDay,
    ) -> PortFuture<'a, Result<(), VaultWriteFailed>> {
        self.calls.bump();
        self.days.push(today);
        let failing = self.failing.load(Ordering::SeqCst);
        Box::pin(async move {
            if failing {
                Err(VaultWriteFailed)
            } else {
                Ok(())
            }
        })
    }
}

#[derive(Clone, Default)]
struct FakeTick(Calls, Days);

impl ReadTick for FakeTick {
    fn tick_read<'a>(
        &'a self,
        _topic: &'a TopicKey,
        today: StudyDay,
    ) -> PortFuture<'a, Result<(), VaultWriteFailed>> {
        self.0.bump();
        self.1.push(today);
        Box::pin(async { Ok(()) })
    }
}

#[derive(Clone, Default)]
struct FakeReviews(Arc<Mutex<Vec<Review>>>);

impl FakeReviews {
    /// A learn review of `card` answered at `at`.
    fn add(&self, card: i64, at: i64) {
        self.0.lock().expect("reviews").push(Review {
            id: at,
            card_id: card,
            ease: 3,
            interval: 1,
            last_interval: 0,
            factor: 2500,
            taken_ms: 4000,
            kind: 0,
        });
    }
}

impl ReviewsByCard for FakeReviews {
    fn reviews<'a>(
        &'a self,
        card_ids: &'a [i64],
        since: UtcMillis,
    ) -> PortFuture<'a, Result<Vec<Review>, ReviewsUnreadable>> {
        let found: Vec<Review> = self
            .0
            .lock()
            .expect("reviews")
            .iter()
            .filter(|review| {
                card_ids.contains(&review.card_id) && review.id >= since.epoch_millis()
            })
            .copied()
            .collect();
        Box::pin(async move { Ok(found) })
    }
}

struct Rig {
    _scratch: tempfile::TempDir,
    readings: SqliteReadings,
    ledger: SqliteXpLedger,
    stamp: FakeStamp,
    tick: FakeTick,
    reviews: FakeReviews,
    clock: Arc<ManualClock>,
    settle: Settle<RecordingGrant, FakeStamp, FakeReviews>,
    tap: ReadTap<RecordingGrant, FakeTick>,
    /// The study days the settle's and the tap's XP requests carried.
    settle_grant_days: Days,
    tap_grant_days: Days,
    id: ReadingId,
}

impl Rig {
    async fn new() -> Self {
        Self::with(StudyDayRule::default(), START).await
    }

    /// The rig under the configured `rule`, its reading generated at `generated`.
    async fn with(rule: StudyDayRule, generated: i64) -> Self {
        let scratch = tempfile::tempdir().expect("a scratch directory");
        let db = Db::open(&scratch.path().join("deckstreak.db"))
            .await
            .expect("a database");
        let readings = SqliteReadings::new(db.clone());
        let ledger = SqliteXpLedger::new(db.clone());
        let stamp = FakeStamp::default();
        let tick = FakeTick::default();
        let reviews = FakeReviews::default();
        let clock = Arc::new(ManualClock::new(UtcMillis::from_epoch_millis(generated)));
        let id = ReadingId::of(TOPIC, 20_000, &"c".repeat(64));
        readings
            .store_reading(&NewReading {
                id: id.clone(),
                topic: TopicKey::parse(TOPIC).expect("a topic"),
                study_day: rule.study_day(UtcMillis::from_epoch_millis(generated)),
                digest: "c".repeat(64),
                persona: "law-synthetic".to_owned(),
                text: "a synthetic reading".to_owned(),
                word_count: 900,
                minutes: 5,
                card_ids: vec![1, 2, 3, 4, 5],
                note_count: 2,
                generated_at: UtcMillis::from_epoch_millis(generated),
                vault: VaultStatus::Written("readings/20000/synthetic.md".to_owned()),
            })
            .await
            .expect("a stored reading");
        let settle_grant_days = Days::default();
        let tap_grant_days = Days::default();
        let settle = Settle::new(
            readings.clone(),
            RecordingGrant {
                inner: SqliteXpLedger::new(db.clone()),
                days: settle_grant_days.clone(),
            },
            stamp.clone(),
            reviews.clone(),
            clock.clone(),
            rule,
        );
        let tap = ReadTap::new(
            readings.clone(),
            RecordingGrant {
                inner: SqliteXpLedger::new(db),
                days: tap_grant_days.clone(),
            },
            tick.clone(),
            clock.clone(),
            rule,
        );
        Self {
            _scratch: scratch,
            readings,
            ledger,
            stamp,
            tick,
            reviews,
            clock,
            settle,
            tap,
            settle_grant_days,
            tap_grant_days,
            id,
        }
    }

    fn set_clock(&self, at: i64) {
        self.clock.set(UtcMillis::from_epoch_millis(at));
    }

    async fn total(&self) -> XpTotal {
        self.ledger.total().await.expect("a total")
    }

    async fn progress(&self) -> deck_streak_readings::store::ReadingProgress {
        let row = self.readings.progress(&self.id).await.expect("a read");
        assert!(row.is_some(), "the reading has a row");
        row.expect("a row")
    }
}

#[tokio::test]
async fn a_settle_grants_and_stamps_once_on_crossing_the_threshold() {
    let rig = Rig::new().await;
    // Three of five cards studied: below the majority, so the window stays open.
    for card in 1..=3 {
        rig.reviews.add(card, START + HOUR_MS);
    }
    rig.set_clock(START + 2 * HOUR_MS);
    let first = rig.settle.run().await.expect("a settle");
    assert_eq!(first.measured, 1, "the open reading is measured");
    assert_eq!(first.studied, 0);
    let open = rig.progress().await;
    assert_eq!(open.studied_count, 3, "the count is stored");
    assert_eq!(open.verdict, Verdict::Open);
    assert_eq!(
        rig.total().await,
        XpTotal::new(0),
        "nothing earned below the line"
    );
    assert_eq!(rig.stamp.calls.count(), 0);

    // A fourth card crosses four of five.
    rig.reviews.add(4, START + 3 * HOUR_MS);
    rig.set_clock(START + 4 * HOUR_MS);
    let crossing = rig.settle.run().await.expect("a settle");
    assert_eq!(crossing.studied, 1);
    let studied = rig.progress().await;
    assert_eq!(studied.studied_count, 4);
    assert_eq!(studied.verdict, Verdict::Studied);
    assert_eq!(
        studied.studied_at,
        Some(UtcMillis::from_epoch_millis(START + 4 * HOUR_MS))
    );
    assert_eq!(rig.total().await, XpTotal::new(60), "60 XP on the crossing");
    assert_eq!(
        rig.ledger.track_total(Track::Law).await.expect("a total"),
        XpTotal::new(60),
        "on the reading's own track"
    );
    assert_eq!(
        rig.stamp.calls.count(),
        1,
        "the Studied line is stamped once"
    );
    assert_eq!(
        rig.tick.0.count(),
        0,
        "the settle never ticks the read line"
    );

    // A later settle changes nothing.
    rig.reviews.add(5, START + 5 * HOUR_MS);
    rig.set_clock(START + 6 * HOUR_MS);
    rig.settle.run().await.expect("a settle");
    assert_eq!(rig.total().await, XpTotal::new(60), "granted once");
    assert_eq!(rig.stamp.calls.count(), 1, "stamped once");
    assert_eq!(rig.progress().await.verdict, Verdict::Studied);
}

#[tokio::test]
async fn a_retired_reading_turns_studied_on_late_reviews_inside_its_window() {
    let rig = Rig::new().await;
    rig.reviews.add(1, START + HOUR_MS);
    rig.reviews.add(2, START + 2 * HOUR_MS);
    // One instant before the rollover that starts d + 2 the window is still open.
    rig.set_clock(CLOSE - 1);
    rig.settle.run().await.expect("a settle");
    assert_eq!(rig.progress().await.verdict, Verdict::Open);

    // At the rollover it retires below the line.
    rig.set_clock(CLOSE);
    let retiring = rig.settle.run().await.expect("a settle");
    assert_eq!(retiring.retired, 1);
    let retired = rig.progress().await;
    assert_eq!(retired.verdict, Verdict::Retired);
    assert_eq!(retired.studied_count, 2);
    assert_eq!(rig.total().await, XpTotal::new(0));

    // A late sync brings two reviews made inside the window, and a review made after it.
    rig.reviews.add(3, START + 30 * HOUR_MS);
    rig.reviews.add(4, START + 40 * HOUR_MS);
    rig.reviews.add(5, CLOSE + HOUR_MS);
    rig.set_clock(CLOSE + 2 * HOUR_MS);
    let late = rig.settle.run().await.expect("a settle");
    assert_eq!(late.studied, 1, "the retired reading turns studied");
    let turned = rig.progress().await;
    assert_eq!(turned.verdict, Verdict::Studied);
    assert_eq!(
        turned.studied_count, 4,
        "the review after the window is not counted"
    );
    assert_eq!(rig.total().await, XpTotal::new(60));
    assert_eq!(rig.stamp.calls.count(), 1);
}

#[tokio::test]
async fn only_a_reading_that_turns_retired_this_pass_is_counted_retired() {
    let rig = Rig::new().await;
    rig.reviews.add(1, START + HOUR_MS);
    rig.set_clock(CLOSE - 1);
    let open = rig.settle.run().await.expect("a settle");
    assert_eq!(open.retired, 0, "an open reading that stays open");
    rig.set_clock(CLOSE);
    let retiring = rig.settle.run().await.expect("a settle");
    assert_eq!(retiring.retired, 1, "the pass that retires it");
    rig.set_clock(CLOSE + HOUR_MS);
    let again = rig.settle.run().await.expect("a settle");
    assert_eq!(again.retired, 0, "a reading that was already retired");
}

#[tokio::test]
async fn a_read_and_studied_reading_earns_exactly_100_xp_once() {
    let rig = Rig::new().await;
    let tapped = rig.tap.tap_reading(rig.id.as_str()).await;
    assert!(tapped.is_ok(), "the tap answered: {tapped:?}");
    let tapped = tapped.expect("a tap");
    assert!(tapped.first);
    for card in 1..=4 {
        rig.reviews.add(card, START + HOUR_MS);
    }
    rig.set_clock(START + 2 * HOUR_MS);
    rig.settle.run().await.expect("a settle");
    assert_eq!(
        rig.total().await,
        XpTotal::new(100),
        "40 for the tap and 60 for the studying"
    );

    // Replaying both grants on later study days writes nothing.
    let read = read_source(&rig.id);
    let studied = studied_source(&rig.id);
    assert!(
        !read.is_empty() && !studied.is_empty(),
        "the sources are named"
    );
    assert_ne!(read, studied, "the two grants are two sources");
    for (day, source, amount) in [(20_003, read, 40), (20_004, studied, 60)] {
        let request = GrantRequest {
            study_day: StudyDay::from_epoch_day(day),
            source: GrantSource::new(&source).expect("a source"),
            track: Track::Law,
            amount: XpAmount::new(amount),
            scope: GrantScope::Once,
        };
        let answer = rig
            .ledger
            .grant(&request, UtcMillis::from_epoch_millis(START + 3 * DAY_MS))
            .await
            .expect("a replay");
        assert!(
            matches!(answer, GrantAnswer::AlreadyGranted(_)),
            "{source} is granted once"
        );
    }
    assert_eq!(rig.total().await, XpTotal::new(100), "exactly 100 XP");
}

#[tokio::test]
async fn a_failed_stamp_leaves_the_reading_open_for_the_next_pass() {
    let rig = Rig::new().await;
    for card in 1..=4 {
        rig.reviews.add(card, START + HOUR_MS);
    }
    rig.set_clock(START + 2 * HOUR_MS);
    rig.stamp.failing.store(true, Ordering::SeqCst);
    let failed = rig.settle.run().await.expect("a settle");
    assert_eq!(
        failed.studied, 0,
        "a reading whose stamp failed is not counted studied"
    );
    let open = rig.progress().await;
    assert_eq!(
        open.verdict,
        Verdict::Open,
        "the verdict waits for the stamp"
    );
    assert_eq!(open.studied_at, None);
    assert_eq!(open.studied_count, 4);
    assert_eq!(
        rig.total().await,
        XpTotal::new(60),
        "the once-scoped grant stands"
    );

    // The next pass stamps it, counts it and grants nothing again.
    rig.stamp.failing.store(false, Ordering::SeqCst);
    rig.set_clock(START + 3 * HOUR_MS);
    let retried = rig.settle.run().await.expect("a settle");
    assert_eq!(retried.studied, 1);
    let studied = rig.progress().await;
    assert_eq!(studied.verdict, Verdict::Studied);
    assert_eq!(
        studied.studied_at,
        Some(UtcMillis::from_epoch_millis(START + 3 * HOUR_MS))
    );
    assert_eq!(rig.stamp.calls.count(), 2, "the stamp is retried once");
    assert_eq!(rig.total().await, XpTotal::new(60), "exactly 60 XP");
}

/// The rollover hours the population takes.
const HOURS: [i64; 2] = [0, 4];
/// The offsets, in minutes east of UTC: west, on the half hour, on the quarter hour and east.
const OFFSETS: [i64; 8] = [-720, -300, -210, 0, 330, 345, 540, 840];

/// The UTC instant study day `day` begins at, with the rollover at `hour` local to `offset` minutes
/// east of UTC: written from the definition, never through the rule under test.
fn begins(day: i64, hour: i64, offset: i64) -> i64 {
    day * DAY_MS + hour * HOUR_MS - offset * 60_000
}

/// The study day `instant` falls in, from the same definition.
fn day_of(instant: i64, hour: i64, offset: i64) -> i64 {
    (instant + offset * 60_000 - hour * HOUR_MS).div_euclid(DAY_MS)
}

/// Every configured rule of the population: the product of the offsets and the rollover hours,
/// generated here and never listed.
fn rules() -> Vec<(i64, i64, StudyDayRule)> {
    let mut rules = Vec::new();
    for offset in OFFSETS {
        for hour in HOURS {
            let rule = StudyDayRule::new(
                Hour::new(u8::try_from(hour).expect("an hour")).expect("an hour"),
                UtcOffset::from_minutes(i16::try_from(offset).expect("minutes"))
                    .expect("an offset"),
            );
            rules.push((offset, hour, rule));
        }
    }
    rules
}

#[tokio::test]
async fn the_settle_and_the_tap_read_the_configured_study_day() {
    let mut examined = 0;
    let mut instants_examined = 0;
    for (offset, hour, rule) in rules() {
        let generated = begins(20_000, hour, offset) + HOUR_MS;
        let close = begins(20_002, hour, offset);

        // The window: four of five cards reviewed at the last instant of d + 1 count.
        let inside = Rig::with(rule, generated).await;
        for card in 1..=4 {
            inside.reviews.add(card, close - 1);
        }
        inside.set_clock(close - 1);
        let counted = inside.settle.run().await.expect("a settle");
        assert_eq!(
            counted.studied, 1,
            "offset {offset}, hour {hour}: the last instant of d + 1 counts"
        );

        // The same reviews at the rollover that starts d + 2 do not; the reading stays open until
        // that instant and retires at it.
        let outside = Rig::with(rule, generated).await;
        for card in 1..=4 {
            outside.reviews.add(card, close);
        }
        outside.set_clock(close - 1);
        let open = outside.settle.run().await.expect("a settle");
        assert_eq!(
            (open.studied, open.retired),
            (0, 0),
            "offset {offset}, hour {hour}: open before the close"
        );
        assert_eq!(outside.progress().await.verdict, Verdict::Open);
        outside.set_clock(close);
        let closed = outside.settle.run().await.expect("a settle");
        assert_eq!(
            (closed.studied, closed.retired),
            (0, 1),
            "offset {offset}, hour {hour}: retired at the rollover that starts d + 2"
        );

        // The study day: at each instant either side of the configured rollover, and either side
        // of the default rule's, the day the grant and the stamp carry is the configured day.
        let mut instants = BTreeSet::new();
        for day in [20_001, 20_002] {
            let own = begins(day, hour, offset);
            let default = day * DAY_MS + 4 * HOUR_MS;
            instants.extend([own - 1, own, default - 1, default]);
        }
        for instant in instants {
            let rig = Rig::with(rule, generated).await;
            for card in 1..=4 {
                rig.reviews.add(card, generated);
            }
            rig.set_clock(instant);
            let day = day_of(instant, hour, offset);
            let settled = rig.settle.run().await.expect("a settle");
            assert_eq!(settled.studied, 1, "the reading crosses at {instant}");
            assert_eq!(
                rig.settle_grant_days.all(),
                [day],
                "offset {offset}, hour {hour}, instant {instant}: the settle's XP is dated by the configured day"
            );
            assert_eq!(
                rig.stamp.days.all(),
                [day],
                "offset {offset}, hour {hour}, instant {instant}: the stamp looks the note up by the configured day"
            );
            let tapped = rig.tap.tap_reading(rig.id.as_str()).await.expect("a tap");
            assert!(tapped.first, "the first tap");
            assert_eq!(
                rig.tap_grant_days.all(),
                [day],
                "offset {offset}, hour {hour}, instant {instant}: the tap's XP is dated by the configured day"
            );
            assert_eq!(
                rig.tick.1.all(),
                [day],
                "offset {offset}, hour {hour}, instant {instant}: the tick looks the note up by the configured day"
            );
            instants_examined += 1;
        }
        examined += 1;
    }
    println!("examined {examined} configured rule(s), {instants_examined} instant(s)");
    assert_eq!(
        examined,
        OFFSETS.len() * HOURS.len(),
        "every offset by every rollover hour"
    );
    assert_eq!(examined, 16, "eight offsets by two rollover hours");
    assert!(instants_examined >= 16 * 4, "each rule is read at its instants");
}
