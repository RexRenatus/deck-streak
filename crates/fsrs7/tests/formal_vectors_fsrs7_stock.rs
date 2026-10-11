//! #823: the stock projection and the pinned scheduler's forgetting curve hold the claims
//! `formal/lean/Formal/Fsrs7Stock.lean` proves, on every vector its Lean writer prints in
//! `formal/vectors/fsrs7-stock.jsonl`. A difficulty line carries a replayed difficulty and the
//! stock difficulty the port gives it, which `stock::project` must give bit for bit. A gap line
//! carries a gap, the gap the curve reads, and whether that reading is the review itself; the port
//! leaves the curve's components abstract, so the line carries no retrievability, and this test
//! reads the pinned crate's curve at each gap, under the scheduler's defaults and four parameter
//! sets at the clipper's bounds, for states across the scheduler's range: the curve reads the same
//! at a gap as at its reading, lies between its floor and its ceiling, reads its ceiling at the
//! review, and never rises with the gap. The crate's one dependency stays the pinned scheduler,
//! so the vectors are read by a small parser of integer arrays.

#![allow(
    clippy::expect_used,
    clippy::print_stdout,
    reason = "an integration test panics on a malformed vector, and prints its examined count on \
              purpose"
)]

use deck_streak_fsrs7::stock;
use fsrs7::{FSRS, MemoryState};

/// The vectors, as the Lean writer printed them.
const VECTORS: &str = include_str!("../../../formal/vectors/fsrs7-stock.jsonl");

/// The number of FSRS-7 parameters.
const PARAMETERS: usize = 34;

/// The states the curve is read at: three first-review states and the S90 state of the pinned
/// revision's own tests, and the four corners of the scheduler's range for the two stabilities.
const STATES: [MemoryState; 8] = [
    state(0.1104, 6.1686, 0.08832),
    state(3.9221, 3.530_724_3, 3.13768),
    state(11.7841, 1.0, 9.427_279),
    state(10.0, 5.0, 8.0),
    state(0.0001, 1.0, 0.0001),
    state(36_500.0, 10.0, 36_500.0),
    state(0.0001, 10.0, 36_500.0),
    state(36_500.0, 1.0, 0.0001),
];

const fn state(stability: f32, difficulty: f32, stability_fast: f32) -> MemoryState {
    MemoryState {
        stability,
        difficulty,
        stability_fast,
    }
}

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

/// The value a line holds under `key`, if it holds one.
fn field(line: &str, key: &str) -> Option<Value> {
    let marker = format!("\"{key}\":");
    line.split_once(&marker).map(|(_, after)| value(after).0)
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

/// A difficulty line: `project` gives the port's stock difficulty.
fn difficulty_holds(model: &FSRS, line: &str, replayed: &Value, stock: &Value) {
    let projected = stock::project(model, state(10.0, float(replayed), 8.0), 0.9).difficulty;
    let want = float(stock);
    assert!(
        projected.to_bits() == want.to_bits() || (projected.is_nan() && want.is_nan()),
        "{line}: the stock difficulty is {projected}, not {want}"
    );
}

#[test]
fn the_stock_projection_and_the_curve_hold_the_lean_claims_on_every_vector() {
    let mut lines = VECTORS.lines();
    let header = lines.next().expect("a header line");
    for named in [
        "\"schema\":\"phx.formal.vectors.v1\"",
        "\"entry\":\"Fsrs7Stock\"",
        "\"covers\":\"crates/fsrs7/src/stock.rs\"",
        "\"anchor\":\"project\"",
    ] {
        assert!(header.contains(named), "the header {header} names {named}");
    }
    let declared = field(header, "vectors").expect("a count").int();
    let models = models();
    let floor = 0.0_f32.mul_add(1.0 - 2e-5, 1e-5);
    let ceiling = 1.0_f32.mul_add(1.0 - 2e-5, 1e-5);

    let mut examined = 0_i64;
    let mut gaps: Vec<(f32, f32, bool)> = Vec::new();
    for line in lines {
        if let (Some(replayed), Some(stock)) = (field(line, "difficulty"), field(line, "stock")) {
            difficulty_holds(&models[0].1, line, &replayed, &stock);
        } else {
            let gap = float(&field(line, "gap").expect("a gap"));
            let read = float(&field(line, "read").expect("a reading"));
            let review = field(line, "review").expect("a review flag").int() == 1;
            gaps.push((gap, read, review));
        }
        examined += 1;
    }

    let mut readings = 0_usize;
    for (name, model) in &models {
        for memory in STATES {
            let mut curve: Vec<(f32, f32)> = Vec::new();
            for &(gap, read, review) in &gaps {
                let at_gap = model.current_retrievability(memory, gap);
                let at_read = model.current_retrievability(memory, read);
                assert_eq!(
                    at_gap.to_bits(),
                    at_read.to_bits(),
                    "{name} {memory:?}: the curve at {gap} reads as at {read}"
                );
                assert!(
                    (floor..=ceiling).contains(&at_gap),
                    "{name} {memory:?}: the curve at {gap} reads {at_gap}, outside the band"
                );
                if review {
                    assert_eq!(
                        at_gap.to_bits(),
                        ceiling.to_bits(),
                        "{name} {memory:?}: the curve at {gap} is the review, and reads {at_gap}"
                    );
                }
                curve.push((read, at_gap));
                readings += 1;
            }
            curve.sort_by(|left, right| left.0.total_cmp(&right.0));
            for pair in curve.windows(2) {
                let [(earlier, before), (later, after)] = pair else {
                    unreachable!("a window of two");
                };
                assert!(
                    after <= before,
                    "{name} {memory:?}: the curve rises from {before} at {earlier} to {after} at \
                     {later}"
                );
            }
        }
    }

    assert_eq!(
        examined, declared,
        "the header declares {declared} vector(s)"
    );
    println!("examined {examined} vector(s), {readings} curve reading(s)");
    assert!(
        examined > 0,
        "examined 0 vectors: the population is empty, so nothing was judged"
    );
}
