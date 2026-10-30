"""SPEC-302's registrations: CPython's own numbers the kernel ports once (ADR-090).

Each adapter reaches CPython's function through a module of the predecessor that imports it, and
CALLS it; none computes a rule.

* `through_the_builtins` drives `sum`, `statistics.median`, `statistics.mean` and `round` on the
  case's own floats, the statistics functions reached through the predecessor's `velocity` module.
* `through_the_generator` seeds `random.Random` (reached through the predecessor's `tilt` module)
  with the case's integer, draws the case's count of `random()` values, then draws `choices` of
  the case's count over `range(length)` from the same generator, and returns both.
* `through_lgamma` calls `math.lgamma` (reached through the predecessor's `velocity` module) on the
  case's float.

`percentile` is a function golden: `gamification/adaptive.py:percentile`, called with the case's
values and its pct.
"""

import math


def through_the_builtins(median, predecessor, *, op, values=None, x=None, ndigits=None):
    if op == "sum":
        return sum(values)
    if op == "median":
        return median(values)
    if op == "mean":
        return predecessor("velocity.statistics.mean")(values)
    return round(x, ndigits)


def through_the_generator(random_class, predecessor, *, seed, draws, length, k):
    generator = random_class(seed)
    randoms = [generator.random() for _ in range(draws)]
    picks = generator.choices(range(length), k=k)
    return {"random": randoms, "choices": picks}


def through_lgamma(lgamma, predecessor, *, x):
    return lgamma(x)


def floats(rng, count, low, high):
    return [rng.uniform(low, high) for _ in range(count)]


def basics_cases(rng):
    drawn = []
    # sum: cancellation, the empty sum, a negative zero, and runs of a float that is not exact.
    sums = [
        ("empty", []),
        ("cancellation", [1e16, 1.0, -1e16]),
        ("cancellation", [1e100, 1.0, -1e100, 1.0]),
        ("cancellation", [0.1, 0.2, 0.3]),
        ("negative_zero", [-0.0]),
        ("negative_zero", [-0.0, -0.0]),
        ("inexact", [0.1] * 10),
        ("inexact", [0.1] * 1000),
        ("single", [3.5]),
    ]
    for edge, values in sums:
        drawn.append((edge, {"op": "sum", "values": values}))
    for _ in range(12):
        values = floats(rng, rng.randrange(2, 40), -1000.0, 1000.0)
        values += [-v for v in values[: len(values) // 2]]
        rng.shuffle(values)
        drawn.append((None, {"op": "sum", "values": values}))
    # median: odd and even counts, one value, equal middles, unsorted input.
    medians = [
        ("single", [2.5]),
        ("odd", [3.0, 1.0, 2.0]),
        ("even", [4.0, 1.0, 3.0, 2.0]),
        ("even", [0.1, 0.2]),
        ("even", [1e308, -1e308]),
        ("equal", [2.0, 2.0, 2.0, 2.0]),
    ]
    for edge, values in medians:
        drawn.append((edge, {"op": "median", "values": values}))
    for count in (5, 6, 7, 8, 21, 22):
        drawn.append((None, {"op": "median", "values": floats(rng, count, -50.0, 50.0)}))
    # mean: lists whose running sum differs from the exact mean, and one rounded once.
    means = [
        ("running_sum_differs", [1e16, 1.0, 1.0, -1e16]),
        ("running_sum_differs", [1e16, 1.0, -1e16]),
        ("running_sum_differs", [0.1] * 10),
        ("running_sum_differs", [0.1, 0.2, 0.3]),
        ("single", [7.25]),
        ("subnormal", [5e-324, 5e-324, 5e-324]),
        ("subnormal", [5e-324, 1e-323]),
        ("large", [1.7976931348623157e308, 1.7976931348623157e308]),
        ("mixed_scale", [1e-300, 1e300, -1e300, 3.0]),
    ]
    for edge, values in means:
        drawn.append((edge, {"op": "mean", "values": values}))
    for _ in range(14):
        values = floats(rng, rng.randrange(1, 30), -1e6, 1e6)
        values += [rng.uniform(-1e-3, 1e-3) for _ in range(3)]
        drawn.append((None, {"op": "mean", "values": values}))
    # round: the binary value of a float, a tie to even, negative digits and a negative zero.
    rounds = [
        ("shortest_decimal_tie", 2.675, 2),
        ("shortest_decimal_tie", 1.005, 2),
        ("shortest_decimal_tie", 0.285, 2),
        ("shortest_decimal_tie", 1.15, 1),
        ("tie", 0.5, 0),
        ("tie", 1.5, 0),
        ("tie", 2.5, 0),
        ("tie", -0.5, 0),
        ("tie", -2.5, 0),
        ("tie", 0.125, 2),
        ("tie", 0.375, 2),
        ("tie", 0.625, 2),
        ("negative_digits", 123.456, -1),
        ("negative_digits", 1234.5, -2),
        ("negative_digits", 15.0, -1),
        ("negative_digits", 25.0, -1),
        ("negative_digits", 5e5, -6),
        ("negative_zero", -0.4, 0),
        ("beyond", 1e300, 400),
        ("beyond", 1e-300, 5),
        ("beyond", 12345.678, -400),
    ]
    for edge, x, ndigits in rounds:
        drawn.append((edge, {"op": "round", "x": x, "ndigits": ndigits}))
    for digits in range(5):
        for _ in range(3):
            half = (rng.randrange(0, 200) + 0.5) / 10**digits
            drawn.append(("halves", {"op": "round", "x": half, "ndigits": digits}))
    for _ in range(20):
        drawn.append((None, {"op": "round", "x": rng.uniform(-1000.0, 1000.0), "ndigits": rng.randrange(0, 6)}))
    return drawn


def percentile_cases(rng):
    drawn = [
        ("empty", {"values": [], "pct": 0.9}),
        ("single", {"values": [5], "pct": 0.9}),
    ]
    seventy = [rng.randrange(0, 200) for _ in range(70)]
    drawn.append(("float_edge", {"values": seventy, "pct": 0.9}))
    drawn.append((None, {"values": seventy, "pct": 0.2}))
    drawn.append((None, {"values": seventy, "pct": 1.0}))
    drawn.append(("rank_zero", {"values": seventy, "pct": 0.0}))
    distinct = rng.sample(range(100), 7)
    drawn.append(("fractional_rank", {"values": distinct, "pct": 0.9}))
    drawn.append(("float_edge", {"values": [rng.randrange(0, 50) for _ in range(10)], "pct": 0.7}))
    drawn.append(("float_edge", {"values": [rng.randrange(0, 50) for _ in range(100)], "pct": 0.29}))
    for _ in range(20):
        count = rng.randrange(1, 101)
        pct = rng.choice([0.1, 0.25, 0.5, 0.7, 0.75, 0.9, 0.95, 0.99, 1.0, 0.0, 0.29, 0.58])
        drawn.append((None, {"values": [rng.randrange(0, 400) for _ in range(count)], "pct": pct}))
    return drawn


def generator_cases(rng):
    drawn = []
    # Zero and one are one-word keys; 20260803 is one word; 1099511627783 is two words (past 32 bits).
    for seed in (0, 1, 20260803, 1099511627783):
        drawn.append(("long_run", {"seed": seed, "draws": 1000, "length": 7, "k": 40}))
        for length in (1, 2, 3, 10, 100, 1000):
            drawn.append((None, {"seed": seed, "draws": 5, "length": length, "k": 12}))
    return drawn


def lgamma_cases(rng):
    drawn = []
    for x in (1, 2, 3, 4, 5, 6, 10, 20, 50, 100, 1000, 10**5, 10**6):
        drawn.append(("integral", {"x": float(x)}))
    for x in (0.5, 1.5, 2.5, 3.7, 4.999, 5.0, 5.0000001, 10.25, 100.5, 12345.678, 999999.5):
        drawn.append(("non_integral", {"x": x}))
    for x in (1e-25, 1e-5, 0.1, 1.0000001):
        drawn.append(("small", {"x": x}))
    for _ in range(30):
        drawn.append((None, {"x": rng.uniform(1.0, 50.0)}))
    for _ in range(40):
        drawn.append((None, {"x": math.exp(rng.uniform(0.0, math.log(10**6)))}))
    for _ in range(20):
        drawn.append(("integral", {"x": float(rng.randrange(3, 10**6))}))
    return drawn


FUNCTIONS = {
    "pynum_basics": {
        "kind": "adapter",
        "function": "velocity.statistics.median",
        "adapter": through_the_builtins,
        "note": (
            "Calls CPython's sum, statistics.median, statistics.mean or round on the case's "
            "own floats, as its op names."
        ),
        "cases": basics_cases,
    },
    "percentile": {
        "kind": "function",
        "function": "gamification.adaptive.percentile",
        "cases": percentile_cases,
    },
    "pynum_random": {
        "kind": "adapter",
        "function": "tilt.random.Random",
        "adapter": through_the_generator,
        "note": (
            "Seeds random.Random with the case's integer, draws that many random() values, "
            "then draws choices of range(length) of k from the same generator."
        ),
        "cases": generator_cases,
    },
    "pynum_lgamma": {
        "kind": "adapter",
        "function": "velocity.math.lgamma",
        "adapter": through_lgamma,
        "note": "Calls CPython's math.lgamma on the case's float.",
        "cases": lgamma_cases,
    },
}
