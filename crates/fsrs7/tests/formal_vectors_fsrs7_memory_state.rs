//! #823: the replay holds the three claims `formal/lean/Formal/Fsrs7MemoryState.lean` proves over
//! every value the pinned scheduler's arithmetic can produce, on every vector its Lean writer
//! prints in `formal/vectors/fsrs7-memory-state.jsonl`, under the scheduler's defaults and four
//! parameter sets at the clipper's bounds. A line carries histories and the same histories as the
//! port reads them, and no state, because the port leaves the arithmetic abstract: every state
//! the replay returns lies in the scheduler's bounds, the batch returns what the card-by-card
//! replay returns bit for bit, and the histories as read replay to the same states as the
//! histories as they arrive. The crate's one dependency stays the pinned scheduler, so the vectors
//! are read by a small parser of integer arrays.

#![allow(
    clippy::expect_used,
    clippy::print_stdout,
    reason = "an integration test panics on a malformed vector, and prints its examined count on \
              purpose"
)]

use deck_streak_fsrs7::replay::{self, Method};
use fsrs7::{FSRS, FSRSItem, FSRSReview, MemoryState};

/// The vectors, as the Lean writer printed them.
const VECTORS: &str = include_str!("../../../formal/vectors/fsrs7-memory-state.jsonl");

/// The scheduler's bounds, typed from the pinned revision's `S_MIN`, `S_MAX`, `D_MIN` and `D_MAX`
/// (src/simulation.rs:199-202), never read from the crate.
const S_MIN: f32 = 0.0001;
const S_MAX: f32 = 36_500.0;
const D_MIN: f32 = 1.0;
const D_MAX: f32 = 10.0;

/// The number of FSRS-7 parameters.
const PARAMETERS: usize = 34;

/// One value of a vector line: an integer, or an array of values.
#[derive(Debug)]
enum Value {
    Int(i64),
    List(Vec<Value>),
}

impl Value {
    fn int(&self) -> i64 {
        match self {
            Self::Int(number) => *number,
            Self::List(_) => panic!("an integer, not {self:?}"),
        }
    }

    fn list(&self) -> &[Self] {
        match self {
            Self::List(items) => items,
            Self::Int(_) => panic!("an array, not {self:?}"),
        }
    }
}

/// Reads the value at the start of `text`, and returns it with the text after it.
fn value(text: &str) -> (Value, &str) {
    if let Some(mut rest) = text.strip_prefix('[') {
        let mut items = Vec::new();
        if let Some(after) = rest.strip_prefix(']') {
            return (Value::List(items), after);
        }
        loop {
            let (item, after) = value(rest);
            items.push(item);
            if let Some(next) = after.strip_prefix(',') {
                rest = next;
            } else if let Some(end) = after.strip_prefix(']') {
                return (Value::List(items), end);
            } else {
                panic!("a ',' or a ']' before {after:?}");
            }
        }
    }
    let digits = text
        .find(|c: char| c != '-' && !c.is_ascii_digit())
        .unwrap_or(text.len());
    let (number, after) = text.split_at(digits);
    (Value::Int(number.parse().expect("an integer")), after)
}

/// The value a line holds under `key`.
fn field(line: &str, key: &str) -> Value {
    let marker = format!("\"{key}\":");
    let (_, after) = line
        .split_once(&marker)
        .unwrap_or_else(|| panic!("{line} has no {marker}"));
    value(after).0
}

/// An `f32` as the writer tags it: `[0]` NaN, `[1]` and `[2]` the infinities, `[3,n,d]` the value
/// `n / d`.
#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    reason = "the writer's numerators are under 2^53 and its denominators powers of two, so the \
              quotient is exact in an f64 and in the f32 it names"
)]
fn float(value: &Value) -> f32 {
    match value.list() {
        [tag] if tag.int() == 0 => f32::NAN,
        [tag] if tag.int() == 1 => f32::NEG_INFINITY,
        [tag] if tag.int() == 2 => f32::INFINITY,
        [tag, numerator, denominator] if tag.int() == 3 => {
            (numerator.int() as f64 / denominator.int() as f64) as f32
        }
        _ => panic!("a tagged f32, not {value:?}"),
    }
}

/// A list of histories as the scheduler's items.
fn items(value: &Value) -> Vec<FSRSItem> {
    value
        .list()
        .iter()
        .map(|item| FSRSItem {
            reviews: item
                .list()
                .iter()
                .map(|review| {
                    let [rating, gap] = review.list() else {
                        panic!("a review of a rating and a gap, not {review:?}");
                    };
                    FSRSReview {
                        rating: u32::try_from(rating.int()).expect("a rating"),
                        delta_t: float(gap),
                    }
                })
                .collect(),
        })
        .collect()
}

/// A state as the bits of its three values.
type Bits = (u32, u32, u32);

fn bits(state: &MemoryState) -> Bits {
    (
        state.stability.to_bits(),
        state.difficulty.to_bits(),
        state.stability_fast.to_bits(),
    )
}

/// A replay's answer as bits: the states, or `None` for the scheduler's refusal.
fn replayed(method: Method, model: &FSRS, items: Vec<FSRSItem>) -> Option<Vec<Bits>> {
    replay::replay(method, model, items)
        .ok()
        .map(|states| states.iter().map(bits).collect())
}

/// The scheduler's defaults, and four parameter sets the clipper holds at its bounds.
fn models() -> Vec<(&'static str, FSRS)> {
    let alternate: [f32; PARAMETERS] =
        core::array::from_fn(|index| if index % 2 == 0 { 1e30 } else { -1e30 });
    vec![
        ("defaults", FSRS::new(&[]).expect("the defaults")),
        ("all high", FSRS::new(&[1e30; PARAMETERS]).expect("high")),
        ("all low", FSRS::new(&[-1e30; PARAMETERS]).expect("low")),
        ("all zero", FSRS::new(&[0.0; PARAMETERS]).expect("zero")),
        ("alternate", FSRS::new(&alternate).expect("alternate")),
    ]
}

/// A state in the scheduler's bounds, every value finite.
fn bounded(state: &MemoryState) -> bool {
    (S_MIN..=S_MAX).contains(&state.stability)
        && (D_MIN..=D_MAX).contains(&state.difficulty)
        && (S_MIN..=S_MAX).contains(&state.stability_fast)
}

#[test]
fn the_replay_holds_the_lean_claims_on_every_vector() {
    let mut lines = VECTORS.lines();
    let header = lines.next().expect("a header line");
    for named in [
        "\"schema\":\"phx.formal.vectors.v1\"",
        "\"entry\":\"Fsrs7MemoryState\"",
        "\"covers\":\"crates/fsrs7/src/replay.rs\"",
        "\"anchor\":\"replay\"",
    ] {
        assert!(header.contains(named), "the header {header} names {named}");
    }
    let declared = field(header, "vectors").int();
    let models = models();

    let mut examined = 0_i64;
    let mut states = 0_usize;
    for line in lines {
        let arriving = field(line, "items");
        let read = field(line, "normal");
        for (name, model) in &models {
            let single = replayed(Method::Single, model, items(&arriving));
            let batch = replayed(Method::Batch, model, items(&arriving));
            assert_eq!(
                batch, single,
                "{name}: the batch replays {line} as each card alone"
            );

            let as_read = replayed(Method::Single, model, items(&read));
            assert_eq!(
                as_read, single,
                "{name}: {line} replays as the port reads it"
            );

            for item in items(&arriving) {
                let state = model
                    .memory_state(item.clone(), None)
                    .expect("every history replays");
                if item.reviews.is_empty() {
                    assert_eq!(
                        bits(&state),
                        (0, 0, 0),
                        "{name}: the empty history replays to the zero state the port gives"
                    );
                } else {
                    assert!(
                        bounded(&state),
                        "{name}: {item:?} replays to {state:?}, outside the bounds"
                    );
                    states += 1;
                }
            }
        }
        examined += 1;
    }

    assert_eq!(
        examined, declared,
        "the header declares {declared} vector(s)"
    );
    println!("examined {examined} vector(s), {states} state(s) in bounds");
    assert!(
        examined > 0,
        "examined 0 vectors: the population is empty, so nothing was judged"
    );
}
