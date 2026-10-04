//! SPEC-342 A10 to A13 (R7, R8): a cell's fixed line, the grid that writes them, and the report
//! over both targets' lines. Every expected line and refusal is typed by hand from R7 and R8.

use deck_streak_fsrs7::measure::report::{self, Summary};
use deck_streak_fsrs7::measure::{self, MEANS, REVIEWS, RUNS, WARMUPS};
use deck_streak_fsrs7::replay::Method;

/// The grid's twelve cells as the report orders them, typed by hand: method, review rows, mean.
const CELLS: [(&str, usize, usize); 12] = [
    ("single", 10_000, 8),
    ("single", 10_000, 32),
    ("single", 100_000, 8),
    ("single", 100_000, 32),
    ("single", 1_000_000, 8),
    ("single", 1_000_000, 32),
    ("batch", 10_000, 8),
    ("batch", 10_000, 32),
    ("batch", 100_000, 8),
    ("batch", 100_000, 32),
    ("batch", 1_000_000, 8),
    ("batch", 1_000_000, 32),
];

/// One cell's line in the fixed form, every timing fixed and the checksum given.
fn line(target: &str, (method, reviews, mean): (&str, usize, usize), checksum: f64) -> String {
    format!(
        "fsrs7-replay target={target} method={method} reviews={reviews} cards=7 mean={mean} \
         runs=5 median_ms=2.000 min_ms=1.000 max_ms=3.000 per_review_us=0.2000 \
         checksum={checksum:.3}"
    )
}

/// A target's log: one line per cell of [`CELLS`], each with its checksum.
fn log(target: &str, checksums: &[f64; 12]) -> Vec<String> {
    CELLS
        .iter()
        .zip(checksums)
        .map(|(&cell, &checksum)| line(target, cell, checksum))
        .collect()
}

fn text(lines: &[String]) -> String {
    lines
        .iter()
        .flat_map(|line| [line.as_str(), "\n"])
        .collect()
}

fn refused(reasons: &[&str]) -> Result<String, Vec<String>> {
    Err(reasons.iter().map(|&reason| reason.to_owned()).collect())
}

#[test]
fn a_cells_line_carries_its_median_min_and_max_in_the_fixed_form() {
    let native = Summary {
        target: "native".to_owned(),
        method: Method::Single,
        reviews: 10_000,
        cards: 1_250,
        mean: 8,
        runs_ms: vec![5.0, 1.0, 3.0, 2.0, 4.0],
        checksum: 12_345.678_9,
    };
    assert_eq!(
        native.line(),
        "fsrs7-replay target=native method=single reviews=10000 cards=1250 mean=8 runs=5 \
         median_ms=3.000 min_ms=1.000 max_ms=5.000 per_review_us=0.3000 checksum=12345.679"
    );

    let wasm = Summary {
        target: "wasm32-wasip1".to_owned(),
        method: Method::Batch,
        reviews: 100_000,
        cards: 3_125,
        mean: 32,
        runs_ms: vec![2.5, 0.5, 1.5, 1.0],
        checksum: 0.5,
    };
    assert_eq!(
        wasm.line(),
        "fsrs7-replay target=wasm32-wasip1 method=batch reviews=100000 cards=3125 mean=32 runs=4 \
         median_ms=1.500 min_ms=0.500 max_ms=2.500 per_review_us=0.0150 checksum=0.500"
    );
}

#[test]
fn the_grid_is_three_sizes_by_two_lengths_by_two_methods_five_runs_each() {
    // A fake clock that advances one millisecond per read: each timed run reads it twice, so
    // every run takes exactly 1 ms, and a warm-up, which never reads it, takes none.
    let mut now = 0.0;
    let mut clock = || {
        now += 1.0;
        now
    };
    let mut out = Vec::new();
    measure::run_grid("native", &[10, 20, 30], &[2, 3], &mut clock, &mut out)
        .expect("the grid runs over a tiny history");
    let written = String::from_utf8(out).expect("the lines are UTF-8");
    let lines: Vec<&str> = written.lines().collect();

    let mut expected = Vec::new();
    for reviews in [10, 20, 30] {
        for mean in [2, 3] {
            for method in ["single", "batch"] {
                expected.push((method, reviews, mean));
            }
        }
    }
    assert_eq!(lines.len(), expected.len(), "one line per cell: {written}");
    for (line, (method, reviews, mean)) in lines.iter().zip(&expected) {
        let head = format!("fsrs7-replay target=native method={method} reviews={reviews} cards=");
        let timed = format!(" mean={mean} runs=5 median_ms=1.000 min_ms=1.000 max_ms=1.000 ");
        assert!(
            line.starts_with(&head) && line.contains(&timed),
            "the line {line:?} is not the cell {method} reviews={reviews} mean={mean}, timed \
             five times"
        );
    }

    assert_eq!(REVIEWS, [10_000, 100_000, 1_000_000]);
    assert_eq!(MEANS, [8, 32]);
    assert_eq!(RUNS, 5);
    assert_eq!(WARMUPS, 1);
}

#[test]
fn a_report_missing_or_doubling_a_cell_is_refused_by_name() {
    let native = log("native", &[1_000.0; 12]);
    let wasm = log("wasm32-wasip1", &[1_000.0; 12]);

    assert_eq!(
        report::render(&text(&native), &text(&wasm[..11]), "pass"),
        refused(&["wasm32-wasip1: cell batch reviews=1000000 mean=32 is missing"])
    );

    let mut doubled = native.clone();
    doubled.push(native[0].clone());
    assert_eq!(
        report::render(&text(&doubled), &text(&wasm), "pass"),
        refused(&["native: cell single reviews=10000 mean=8 is reported twice"])
    );

    let malformed = "fsrs7-replay target=native method=single reviews=10000";
    let mut broken = native.clone();
    broken[0] = malformed.to_owned();
    assert_eq!(
        report::render(&text(&broken), &text(&wasm), "pass"),
        refused(&[
            &format!("native line 1: not in the fixed form: {malformed}"),
            "native: cell single reviews=10000 mean=8 is missing",
        ])
    );

    let mut outside = native.clone();
    outside.push(native[0].replace(" reviews=10000 ", " reviews=20000 "));
    assert_eq!(
        report::render(&text(&outside), &text(&wasm), "pass"),
        refused(&["native: cell single reviews=20000 mean=8 is outside the grid"])
    );

    let complete = report::render(&text(&native), &text(&wasm), "pass")
        .expect("a report holding every cell once is accepted");
    let rows = complete
        .lines()
        .filter(|row| row.starts_with("| single |") || row.starts_with("| batch |"))
        .count();
    assert_eq!(rows, 12, "one table row per cell: {complete}");
    assert!(
        complete.contains(
            "| single | 10000 | 8 | 7 | 2.000 (1.000 to 3.000), 0.2000 µs | 2.000 (1.000 to \
             3.000), 0.2000 µs | 1000.000 |"
        ),
        "{complete}"
    );
    assert!(
        complete.contains("web target (`wasm32-unknown-unknown`) check: pass"),
        "{complete}"
    );
}

#[test]
fn disagreeing_checksums_and_a_missing_web_check_are_refused() {
    let native = text(&log("native", &[10_000.0; 12]));
    let wasm = |first: f64| {
        let mut checksums = [10_000.0; 12];
        checksums[0] = first;
        text(&log("wasm32-wasip1", &checksums))
    };

    assert_eq!(
        report::render(&native, &wasm(10_002.0), "pass"),
        refused(&[
            "cell single reviews=10000 mean=8: checksums disagree: native 10000.000 and \
             wasm32-wasip1 10002.000"
        ])
    );
    assert!(
        report::render(&native, &wasm(10_000.5), "pass").is_ok(),
        "half a part in 10,000 is within the tolerance"
    );
    assert!(
        report::render(&native, &wasm(10_001.0), "pass").is_ok(),
        "exactly one part in 10,000 is within the tolerance"
    );

    assert_eq!(
        report::render(&native, &wasm(10_000.0), ""),
        refused(&["no web-target check line"])
    );
    assert_eq!(
        report::render(&native, &wasm(10_000.0), "maybe"),
        refused(&["the web-target check reads \"maybe\", not pass or fail crates=<names>"])
    );
    let failing = report::render(&native, &wasm(10_000.0), "fail crates=getrandom,ring")
        .expect("a failing web-target check that names its crates is recorded, not refused");
    assert!(
        failing.contains("check: fail (getrandom,ring)"),
        "{failing}"
    );
}
