//! The open lapse coordination hands on is counted from the study reviews its caller read
//! (SPEC-049 A15; R15; ADR-088). Every review is synthetic and every instant is set by hand.

use std::collections::{BTreeMap, BTreeSet};

use deck_streak_coordination::lapse::open_lapse;
use deck_streak_coordination::recompute::RecomputeFacts;
use deck_streak_ingest::reader::{CollectionData, Review, is_study_event};
use deck_streak_kernel::{Hour, StudyDay, StudyDayRule, UtcMillis, UtcOffset};
use deck_streak_streaks::lapse::LAPSE_AFTER_SILENT_DAYS;

const DAY_MS: i64 = 86_400_000;
const HOUR_MS: i64 = 3_600_000;
/// A study day near the present.
const T: i64 = 20_000;

/// Ten o'clock UTC of study day `day` under the default rule (rollover at 04:00 UTC).
const fn at(day: i64) -> i64 {
    day * DAY_MS + 10 * HOUR_MS
}

/// A review on `day` of type `kind` answered with `ease`.
const fn review(day: i64, kind: i64, ease: i64) -> Review {
    Review {
        id: at(day),
        card_id: 1,
        ease,
        interval: 25,
        last_interval: 15,
        factor: 2500,
        taken_ms: 9_000,
        kind,
    }
}

/// The lapse open on study day `today`, over the window `reviews`.
fn open(today: i64, reviews: Vec<Review>) -> Option<i64> {
    let data = CollectionData {
        reviews,
        cards: Vec::new(),
        created_at: UtcMillis::from_epoch_millis(at(T - 1_000)),
        deck_names: BTreeMap::new(),
    };
    let facts = RecomputeFacts::new(
        &data,
        StudyDayRule::default(),
        UtcMillis::from_epoch_millis(at(today)),
        None,
    );
    open_lapse(&facts).map(StudyDay::epoch_day)
}

#[test]
fn the_open_lapse_is_counted_from_the_study_reviews_read() {
    let studied = [review(T - 20, 1, 3), review(T - 4, 1, 3)];
    // Three silent study days after a study day open a lapse whose id is the first of them.
    assert_eq!(open(T - 1, studied.to_vec()), Some(T - 3));
    assert_eq!(open(T, studied.to_vec()), Some(T - 3));
    // Two do not.
    assert_eq!(open(T - 2, studied.to_vec()), None);
    // A study review closes it.
    let closed = [studied[0], studied[1], review(T, 1, 4)];
    assert_eq!(open(T, closed.to_vec()), None);
    assert_eq!(open(T - 1, closed.to_vec()), Some(T - 3));
    // Every kind of study review counts: a learn, a review, a relearn and a filtered answer.
    for kind in 0..=3 {
        let with_kind = [studied[0], studied[1], review(T, kind, 2)];
        assert_eq!(open(T, with_kind.to_vec()), None, "kind {kind}");
    }
    // A manual or rescheduling entry, and an answer of ease 0, are no study review: they close
    // nothing.
    for (kind, ease) in [(4, 3), (5, 3), (1, 0)] {
        let quiet = [studied[0], studied[1], review(T, kind, ease)];
        assert_eq!(
            open(T, quiet.to_vec()),
            Some(T - 3),
            "kind {kind} ease {ease}"
        );
    }
    // An empty window holds no lapse.
    assert_eq!(open(T, Vec::new()), None);
}

const MINUTE_MS: i64 = 60_000;

/// The instant study day `d` begins under an offset of `offset` minutes east and a rollover at
/// `hour`: the day is the local calendar day of the offset, shifted to start at the rollover hour,
/// so it begins `hour` hours after local midnight, which is `offset` minutes before UTC midnight.
const fn begins(offset: i64, hour: i64, d: i64) -> i64 {
    d * DAY_MS - offset * MINUTE_MS + hour * HOUR_MS
}

/// The study day of instant `t` from the definition alone: the day whose span, from its beginning
/// to the next day's, holds `t`.
fn day_of(offset: i64, hour: i64, t: i64) -> i64 {
    let mut d = t.div_euclid(DAY_MS);
    while begins(offset, hour, d) > t {
        d -= 1;
    }
    while begins(offset, hour, d + 1) <= t {
        d += 1;
    }
    d
}

/// A study review answered at instant `t`.
const fn review_at(t: i64) -> Review {
    Review {
        id: t,
        card_id: 1,
        ease: 3,
        interval: 25,
        last_interval: 15,
        factor: 2500,
        taken_ms: 9_000,
        kind: 1,
    }
}

// A helper of test code: a constant outside the configurable range is a malformed population.
#[allow(clippy::expect_used)]
fn rule(offset: i64, hour: i64) -> StudyDayRule {
    StudyDayRule::new(
        Hour::new(u8::try_from(hour).expect("an hour")).expect("an hour of the day"),
        UtcOffset::from_minutes(i16::try_from(offset).expect("minutes"))
            .expect("an offset in range"),
    )
}

/// The lapse open at instant `now` over `reviews`, under `rule`.
fn open_under(rule: StudyDayRule, now: i64, reviews: Vec<Review>) -> Option<i64> {
    let data = CollectionData {
        reviews,
        cards: Vec::new(),
        created_at: UtcMillis::from_epoch_millis(now - 1_000 * DAY_MS),
        deck_names: BTreeMap::new(),
    };
    let facts = RecomputeFacts::new(&data, rule, UtcMillis::from_epoch_millis(now), None);
    open_lapse(&facts).map(StudyDay::epoch_day)
}

/// A rule as the judges read it: its offset in minutes and its rollover hour.
type RuleKey = (i16, u8);
/// A lapse member: the rule, the instant of now, and each review's instant, kind and ease.
type LapseMember = (RuleKey, i64, Vec<(i64, i64, i64)>);

/// How many members the lapse judge is handed: 448 over the rollover and 576 at a study
/// event's edges.
const EXAMINED: u64 = 1024;
/// The distinct members the lapse is judged at: every one of them is its own.
const DISTINCT_ROLLOVER_MEMBERS: usize = 1024;
/// The distinct members the rule's day is judged at: sixteen rules by seven instants.
const DISTINCT_DAY_MEMBERS: usize = 112;
/// How many study days of the members handed to the lapse judge hold exactly one review. The walk
/// asks whether a day holds more than none, so one review is the day at that boundary; a fold that
/// keeps every member distinct can still stack the reviews on one day, and this count shows it.
const ONE_REVIEW_DAYS: usize = 2048;
/// How many members sit at each other boundary a judge compares, read with the test's own day from
/// the members each judge was handed. The kernel's day turns at the rollover, so an instant at a
/// day's first millisecond and one at its last are the two sides of it, for the day judge's
/// instant, each review the lapse judge is handed and its `now`. The walk opens a lapse when its run
/// of silent days reaches the threshold, so a run of exactly the threshold and one a day short are
/// the two sides of that. A fold that keeps every member distinct can still move every member off
/// one side, and these counts show it.
const BOUNDARY_MEMBERS: [(&str, usize); 8] = [
    ("day instants at a rollover", 16),
    ("day instants a millisecond before one", 16),
    ("reviews at a rollover", 640),
    ("reviews a millisecond before one", 64),
    ("nows at a rollover", 800),
    ("nows a millisecond before one", 224),
    ("runs of silent days at the threshold", 352),
    ("runs of silent days one short of it", 352),
];

/// Where instant `t` sits against the rollover under the rule: at a day's first millisecond, or at
/// its last.
fn rollover_sides(offset: i64, hour: i64, t: i64) -> (bool, bool) {
    let d = day_of(offset, hour, t);
    (
        t == begins(offset, hour, d),
        t + 1 == begins(offset, hour, d + 1),
    )
}

/// The run of silent study days that ends on `now`'s day, by the test's own day: the days after
/// the latest day holding a study review (kind 0 to 3, ease at least 1), or from the first day any
/// review is on when none holds one. Each review is its instant, kind and ease.
fn silent_run(offset: i64, hour: i64, now: i64, reviews: &[(i64, i64, i64)]) -> i64 {
    let today = day_of(offset, hour, now);
    let Some(first) = reviews.iter().map(|r| day_of(offset, hour, r.0)).min() else {
        return 0;
    };
    let latest = reviews
        .iter()
        .filter(|r| (0..=3).contains(&r.1) && r.2 >= 1)
        .map(|r| day_of(offset, hour, r.0))
        .filter(|&n| n <= today)
        .max();
    (today - latest.unwrap_or(first - 1)).max(0)
}

/// The members at each boundary of [`BOUNDARY_MEMBERS`], from the members each judge was handed.
fn boundary_sides(
    days: &BTreeSet<(RuleKey, i64)>,
    lapses: &BTreeSet<LapseMember>,
) -> BTreeMap<&'static str, usize> {
    let mut sides: BTreeMap<&'static str, usize> =
        BOUNDARY_MEMBERS.iter().map(|&(k, _)| (k, 0)).collect();
    let mut add = |name: &'static str, on: bool| *sides.entry(name).or_default() += usize::from(on);
    for &((offset, hour), t) in days {
        let (at, before) = rollover_sides(i64::from(offset), i64::from(hour), t);
        add("day instants at a rollover", at);
        add("day instants a millisecond before one", before);
    }
    let threshold = i64::from(LAPSE_AFTER_SILENT_DAYS);
    for &((offset, hour), now, ref handed) in lapses {
        let (o, h) = (i64::from(offset), i64::from(hour));
        for &(t, _, _) in handed {
            let (at, before) = rollover_sides(o, h, t);
            add("reviews at a rollover", at);
            add("reviews a millisecond before one", before);
        }
        let (at, before) = rollover_sides(o, h, now);
        add("nows at a rollover", at);
        add("nows a millisecond before one", before);
        let run = silent_run(o, h, now, handed);
        add("runs of silent days at the threshold", run == threshold);
        add("runs of silent days one short of it", run + 1 == threshold);
    }
    sides
}

/// The population's counts: the members examined, its spread over each judge, the study days
/// holding one review, both answers, the members at each boundary, and the members each move of
/// [`LAPSE_MOVES`] decides.
// A helper of test code: it prints the counts it asserts, as the test prints its own.
#[allow(clippy::print_stdout)]
fn assert_population(
    (examined, opened): (u64, u64),
    one_review_days: usize,
    distinct: &BTreeSet<LapseMember>,
    days: &BTreeSet<(RuleKey, i64)>,
) {
    println!(
        "examined {examined} rollover member(s), {} distinct, over {} distinct day member(s), \
         {one_review_days} study day(s) holding one review",
        distinct.len(),
        days.len()
    );
    let sides = boundary_sides(days, distinct);
    println!("boundary members: {sides:?}");
    assert_eq!(
        examined, EXAMINED,
        "the population is generated: {examined}"
    );
    assert_eq!(
        distinct.len(),
        DISTINCT_ROLLOVER_MEMBERS,
        "the population's spread: {} distinct of {examined}",
        distinct.len()
    );
    assert_eq!(
        days.len(),
        DISTINCT_DAY_MEMBERS,
        "the day judge's spread: {} distinct",
        days.len()
    );
    assert_eq!(
        one_review_days, ONE_REVIEW_DAYS,
        "the walk's boundary: {one_review_days} study day(s) holding one review"
    );
    assert!(
        opened > 0 && opened < examined,
        "both answers occur: {opened} open of {examined}"
    );
    assert_eq!(
        sides,
        BTreeMap::from(BOUNDARY_MEMBERS),
        "the boundaries' members: {sides:?}"
    );
    let decided = decided_by_each_move(distinct);
    println!("members each move decides: {decided:?}");
    let pinned: BTreeMap<&str, usize> = LAPSE_MOVES.iter().map(|&(k, _, n)| (k, n)).collect();
    assert!(
        pinned.values().all(|&n| n > 0),
        "every move decides a member: {pinned:?}"
    );
    assert_eq!(
        decided, pinned,
        "the members each move decides: {decided:?}"
    );
}

/// Every kind from one below a study event's first to one above its last, at ease 0, 1 and 2.
fn study_edges() -> impl Iterator<Item = (i64, i64)> {
    (-1..=4_i64).flat_map(|kind| (0..=2_i64).map(move |ease| (kind, ease)))
}

/// One move of a boundary the lapse judge compares, or one line its rows replace, as the judge
/// with that one line changed would read its member.
#[derive(Clone, Copy, Debug)]
enum Move {
    /// Every review read `ms` milliseconds, plus `offsets` times the rule's offset and `hours`
    /// times its rollover hour, later: the study day of each review is the rule's day of that
    /// instant.
    Reviews { ms: i64, offsets: i64, hours: i64 },
    /// Now read `n` milliseconds later.
    Now(i64),
    /// A review of kind `.0` read as kind `.1`.
    Kind(i64, i64),
    /// A review of ease `.0` read as ease `.1`.
    Ease(i64, i64),
    /// Every review read as a study review.
    AllStudy,
    /// The walk over the study days' counts, with one of its lines edited.
    Walk(WalkEdit),
}

/// Each move and how many distinct members of the population it decides: the members whose answer
/// the judge with that line changed differs from the judge's. The rule's day turns at the rollover,
/// so each review and now is moved a millisecond each way, and a study event is an interval of
/// kinds and a floor of ease, so each edge is moved one step each way; the walk's own boundaries
/// are moved as the walk's test moves them. The other moves are the lines this crate's rows
/// replace. A count of members at a boundary cannot tell a member whose side decides its answer
/// from one parked there; a member counted here is one on which the moved judge and the judge
/// disagree.
const LAPSE_MOVES: [(&str, Move, usize); 28] = [
    (
        "a review a millisecond earlier",
        Move::Reviews {
            ms: -1,
            offsets: 0,
            hours: 0,
        },
        320,
    ),
    (
        "a review a millisecond later",
        Move::Reviews {
            ms: 1,
            offsets: 0,
            hours: 0,
        },
        32,
    ),
    (
        "a review eight hours earlier",
        Move::Reviews {
            ms: -8 * HOUR_MS,
            offsets: 0,
            hours: 0,
        },
        832,
    ),
    (
        "a review an hour later",
        Move::Reviews {
            ms: HOUR_MS,
            offsets: 0,
            hours: 0,
        },
        64,
    ),
    (
        "a review on its UTC date",
        Move::Reviews {
            ms: 0,
            offsets: -1,
            hours: 1,
        },
        458,
    ),
    (
        "a review by the default rule",
        Move::Reviews {
            ms: -4 * HOUR_MS,
            offsets: -1,
            hours: 1,
        },
        524,
    ),
    (
        "a review with the offset twice",
        Move::Reviews {
            ms: 0,
            offsets: -1,
            hours: 0,
        },
        448,
    ),
    ("now a millisecond earlier", Move::Now(-1), 240),
    ("now a millisecond later", Move::Now(1), 112),
    ("kind -1 a study kind", Move::Kind(-1, 0), 64),
    ("kind 0 no study kind", Move::Kind(0, 4), 64),
    ("kind 3 no study kind", Move::Kind(3, 4), 64),
    ("kind 4 a study kind", Move::Kind(4, 3), 64),
    ("ease 0 a study answer", Move::Ease(0, 1), 128),
    ("ease 1 no study answer", Move::Ease(1, 0), 128),
    ("every review a study review", Move::AllStudy, 320),
    (
        "the window begun a day later",
        Move::Walk(WalkEdit::Reach(1)),
        320,
    ),
    (
        "the window begun a day earlier",
        Move::Walk(WalkEdit::Reach(-1)),
        320,
    ),
    (
        "the window begun at its latest day",
        Move::Walk(WalkEdit::LatestKey),
        320,
    ),
    (
        "the walk begun a day before today",
        Move::Walk(WalkEdit::Start(-1)),
        352,
    ),
    (
        "the walk begun a day after today",
        Move::Walk(WalkEdit::Start(1)),
        352,
    ),
    (
        "a day closing the run at two reviews",
        Move::Walk(WalkEdit::ClosesAbove(1)),
        704,
    ),
    (
        "every day closing the run",
        Move::Walk(WalkEdit::ClosesAbove(-1)),
        672,
    ),
    (
        "a day closing the run at 101 reviews",
        Move::Walk(WalkEdit::ClosesAbove(100)),
        704,
    ),
    (
        "one more silent day to open",
        Move::Walk(WalkEdit::Threshold(1)),
        352,
    ),
    (
        "one fewer silent day to open",
        Move::Walk(WalkEdit::Threshold(-1)),
        352,
    ),
    (
        "the walk passing two days a step",
        Move::Walk(WalkEdit::Stride(2)),
        512,
    ),
    (
        "the id the first silent day met",
        Move::Walk(WalkEdit::FirstMet),
        672,
    ),
];

/// One edit of one line of streaks' walk, as the walk's own test names it.
#[derive(Clone, Copy, Debug)]
enum WalkEdit {
    /// No edit: the walk as it stands.
    Held,
    /// The window's first day read `n` days later.
    Reach(i64),
    /// The window's first day taken from its latest key.
    LatestKey,
    /// The walk begun `n` days after today.
    Start(i64),
    /// A day closing the run only when its count is more than `n`.
    ClosesAbove(i64),
    /// The threshold `n` days more.
    Threshold(i64),
    /// The walk passing `n` days a step.
    Stride(i64),
    /// The id the first silent day the walk meets, not the last.
    FirstMet,
}

/// Streaks' `open_lapse` line for line over day numbers, with the one edit `edit` makes, and the
/// empty skip set this crate hands it.
fn walk_edited(
    today: i64,
    rows: &BTreeMap<i64, u32>,
    threshold: u32,
    edit: WalkEdit,
) -> Option<i64> {
    let mut window_start = match edit {
        WalkEdit::LatestKey => *rows.keys().next_back()?,
        _ => *rows.keys().next()?,
    };
    let (mut silent, mut first_silent) = (0_i64, None);
    let mut number = today;
    match edit {
        WalkEdit::Reach(n) => window_start += n,
        WalkEdit::Start(n) => number += n,
        _ => {}
    }
    let (closes_above, stride, more) = match edit {
        WalkEdit::ClosesAbove(n) => (n, 1, 0),
        WalkEdit::Stride(n) => (0, n, 0),
        WalkEdit::Threshold(n) => (0, 1, n),
        _ => (0, 1, 0),
    };
    while number >= window_start {
        if i64::from(rows.get(&number).copied().unwrap_or(0)) > closes_above {
            break;
        }
        silent += 1;
        first_silent = match edit {
            WalkEdit::FirstMet => first_silent.or(Some(number)),
            _ => Some(number),
        };
        number -= stride;
    }
    if silent >= i64::from(threshold) + more {
        first_silent
    } else {
        None
    }
}

/// The distinct members each move of [`LAPSE_MOVES`] decides. On every member the judge's own
/// answer is first asserted equal to the copy of its walk over the study days' counts, so a member
/// counted for a move is one the judge with that line changed answers differently: a move of the
/// reviews, of now, or of a kind or an ease is the judge itself handed the member so moved.
// A helper of test code: a rule outside the configurable range is a malformed population.
#[allow(clippy::expect_used)]
fn decided_by_each_move(lapses: &BTreeSet<LapseMember>) -> BTreeMap<&'static str, usize> {
    let mut decided: BTreeMap<&'static str, usize> =
        LAPSE_MOVES.iter().map(|&(k, _, _)| (k, 0)).collect();
    let threshold = LAPSE_AFTER_SILENT_DAYS;
    for &((offset, hour), now, ref handed) in lapses {
        let (o, h) = (i64::from(offset), i64::from(hour));
        let the_rule = rule(o, h);
        let judged = |now: i64, handed: &[(i64, i64, i64)]| {
            let reviews = handed
                .iter()
                .map(|&(id, kind, ease)| Review {
                    kind,
                    ease,
                    ..review_at(id)
                })
                .collect();
            open_under(the_rule, now, reviews)
        };
        let day = |t: i64| {
            the_rule
                .study_day(UtcMillis::from_epoch_millis(t))
                .epoch_day()
        };
        let mut counts: BTreeMap<i64, u32> = BTreeMap::new();
        for &(t, kind, ease) in handed {
            let count: &mut u32 = counts.entry(day(t)).or_insert(0);
            *count = count.saturating_add(u32::from(is_study_event(kind, ease)));
        }
        let held = judged(now, handed);
        assert_eq!(
            held,
            walk_edited(day(now), &counts, threshold, WalkEdit::Held),
            "offset {offset}, rollover {hour}, now {now}, reviews {handed:?}: the judge differs \
             from the copy of its walk"
        );
        let each = |f: &dyn Fn(i64, i64, i64) -> (i64, i64, i64)| -> Vec<(i64, i64, i64)> {
            handed.iter().map(|&(t, k, e)| f(t, k, e)).collect()
        };
        for &(name, step, _) in &LAPSE_MOVES {
            let moved = match step {
                Move::Reviews { ms, offsets, hours } => {
                    let by = ms + offsets * o * MINUTE_MS + hours * h * HOUR_MS;
                    judged(now, &each(&|t, k, e| (t + by, k, e)))
                }
                Move::Now(ms) => judged(now + ms, handed),
                Move::Kind(from, to) => judged(
                    now,
                    &each(&|t, k, e| (t, if k == from { to } else { k }, e)),
                ),
                Move::Ease(from, to) => judged(
                    now,
                    &each(&|t, k, e| (t, k, if e == from { to } else { e })),
                ),
                Move::AllStudy => judged(now, &each(&|t, _, _| (t, 1, 1))),
                Move::Walk(edit) => walk_edited(day(now), &counts, threshold, edit),
            };
            *decided.entry(name).or_default() += usize::from(moved != held);
        }
    }
    decided
}

const fn rule_key(rule: StudyDayRule) -> RuleKey {
    (rule.utc_offset().minutes(), rule.rollover_hour().get())
}

#[test]
fn a_review_counts_on_the_study_day_the_rule_gives_at_every_boundary() {
    let mut examined: u64 = 0;
    let mut opened: u64 = 0;
    // Each judge records its member from the arguments it is handed, so a fold that merges one
    // judge's members shows in that judge's count while the other judge still reads the value.
    let mut distinct: BTreeSet<LapseMember> = BTreeSet::new();
    let mut days: BTreeSet<(RuleKey, i64)> = BTreeSet::new();
    let mut one_review_days: usize = 0;
    let mut day_judge = |rule: StudyDayRule, t: i64| {
        days.insert((rule_key(rule), t));
        rule.study_day(UtcMillis::from_epoch_millis(t)).epoch_day()
    };
    let mut lapse_judge = |rule: StudyDayRule, now: i64, reviews: &[Review]| {
        let (offset, hour) = rule_key(rule);
        let mut per_day: BTreeMap<i64, usize> = BTreeMap::new();
        for r in reviews {
            *per_day
                .entry(day_of(i64::from(offset), i64::from(hour), r.id))
                .or_default() += 1;
        }
        one_review_days += per_day.values().filter(|&&n| n == 1).count();
        let handed = reviews.iter().map(|r| (r.id, r.kind, r.ease)).collect();
        distinct.insert((rule_key(rule), now, handed));
        open_under(rule, now, reviews.to_vec())
    };
    for offset in [-720_i64, -300, -210, 0, 330, 345, 540, 840] {
        for hour in [0_i64, 4] {
            let the_rule = rule(offset, hour);
            // Local midnight of local calendar date `T` in this offset.
            let midnight = T * DAY_MS - offset * MINUTE_MS;
            let mut instants = vec![
                begins(offset, hour, T) - 1,
                begins(offset, hour, T),
                begins(offset, hour, T) + 1,
            ];
            for (h, m) in [(23, 59), (0, 1), (3, 59), (4, 1)] {
                instants.push(midnight + h * HOUR_MS + m * MINUTE_MS);
            }
            for t in instants {
                let d = day_of(offset, hour, t);
                assert_eq!(
                    day_judge(the_rule, t),
                    d,
                    "offset {offset}, rollover {hour}, instant {t}: the rule's day"
                );
                // One study review long ago and one at `t`; the run after `t` is what the day
                // decides: silent on d+1, d+2, d+3 opens a lapse with id d+1.
                let reviews = vec![
                    review_at(begins(offset, hour, d - 10) + HOUR_MS),
                    review_at(t),
                ];
                for (now, want) in [
                    (begins(offset, hour, d + 3), Some(d + 1)),
                    (begins(offset, hour, d + 4) - 1, Some(d + 1)),
                    (begins(offset, hour, d + 3) - 1, None),
                    (begins(offset, hour, d + 2), None),
                ] {
                    examined += 1;
                    opened += u64::from(want.is_some());
                    assert_eq!(
                        lapse_judge(the_rule, now, &reviews),
                        want,
                        "offset {offset}, rollover {hour}, review at {t} (day {d}), now {now}"
                    );
                }
            }
            // Both reviews at each edge of a study event, the later on the day `T` begins: a study
            // event closes the run, and without one the run begins on the window's first day.
            for (kind, ease) in study_edges() {
                let edge = |instant: i64| Review {
                    kind,
                    ease,
                    ..review_at(instant)
                };
                let edges = [
                    edge(begins(offset, hour, T - 10) + HOUR_MS),
                    edge(begins(offset, hour, T)),
                ];
                let study = (0..=3).contains(&kind) && ease >= 1;
                for (days_on, want) in [(3, Some(T + 1)), (2, None)] {
                    let want = if study { want } else { Some(T - 10) };
                    examined += 1;
                    opened += u64::from(want.is_some());
                    assert_eq!(
                        lapse_judge(the_rule, begins(offset, hour, T + days_on), &edges),
                        want,
                        "offset {offset}, rollover {hour}, kind {kind}, ease {ease}, day {days_on}"
                    );
                }
            }
        }
    }
    assert_population((examined, opened), one_review_days, &distinct, &days);
    // The zone west of UTC with a 04:00 rollover: 03:59 local is still the day before, 04:01 is
    // the day itself.
    let west = rule(-300, 4);
    let midnight = T * DAY_MS + 300 * MINUTE_MS;
    let local =
        |h: i64, m: i64| UtcMillis::from_epoch_millis(midnight + h * HOUR_MS + m * MINUTE_MS);
    assert_eq!(west.study_day(local(23, 59)).epoch_day(), T);
    assert_eq!(west.study_day(local(0, 1)).epoch_day(), T - 1);
    assert_eq!(west.study_day(local(3, 59)).epoch_day(), T - 1);
    assert_eq!(west.study_day(local(4, 1)).epoch_day(), T);
}
