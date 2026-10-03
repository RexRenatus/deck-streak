"""SPEC-091's horizon registrations: the forward-obligation scan, its readout, and the constants the
port uses verbatim.

Each adapter builds what JSON cannot carry and CALLS the predecessor; none computes a rule.

* `with_card_rows` drives `horizon.py:compute_horizon` on `Card` objects built from the case's rows
  `[queue, ctype, due, odid, copies]`, each row repeated `copies` times, at the case's day number.
  It returns every `HorizonScan` field.
* `with_card_rows_and_a_dial` drives `horizon.py:build_readout` on the same rows and day number
  with the case's desired retention as its input. It returns every `HorizonReadout` field.

The case builders draw only from the `random.Random` the generator seeds. Every card is synthetic,
every day is a day number, and a row's copy count stands for that many identical cards.
"""

import dataclasses

#: A synthetic day number near the present, and the seconds an intraday card's `due` holds.
TODAY = 2770
EPOCH_SECONDS = 1_780_000_000


def with_card_rows(compute_horizon, predecessor, *, today, cards):
    """Call the scan on `Card` objects built from the case's rows at the day number."""
    return dataclasses.asdict(compute_horizon(built(predecessor, cards), today))


def with_card_rows_and_a_dial(build_readout, predecessor, *, today, cards, desired_retention_pct):
    """Call the readout on the same cards, with the case's desired retention as its input."""
    readout = build_readout(
        built(predecessor, cards), today, desired_retention_pct=desired_retention_pct
    )
    return dataclasses.asdict(readout)


def built(predecessor, rows):
    """The cards of the rows `[queue, ctype, due, odid, copies]`: one `Card` per row, listed as
    many times as the row's copy count (the scan only reads a card, so the copies may be one)."""
    card = predecessor("types.Card")
    cards = []
    for index, (queue, ctype, due, odid, copies) in enumerate(rows):
        one = card(
            id=index + 1,
            nid=index + 1,
            did=1,
            queue=queue,
            ctype=ctype,
            due=due,
            ivl=0,
            factor=2500,
            reps=0,
            lapses=0,
            odid=odid,
        )
        cards.extend([one] * copies)
    return cards


def row(queue, due, copies=1, ctype=2, odid=0):
    """A review-typed row unless the case says otherwise."""
    return [queue, ctype, due, odid, copies]


def edge_cases(dial):
    """The classes a port gets wrong, as `(class, input)` pairs; `dial` adds the readout's input."""

    def case(edge, today, rows):
        arguments = {"today": today, "cards": rows}
        if dial is not None:
            arguments["desired_retention_pct"] = dial
        return (edge, arguments)

    return [
        case("empty", 100, []),
        case("excluded", TODAY, [row(-1, TODAY + 3), row(-2, TODAY + 3), row(-3, TODAY + 3)]),
        case("type-new", TODAY, [row(2, TODAY + 5, 3, ctype=0), row(1, EPOCH_SECONDS, ctype=0)]),
        case("queue-new", TODAY, [row(0, TODAY + 5, 2, ctype=1), row(0, 7, 4, ctype=2)]),
        case("new", TODAY, [row(0, 4000, 5, ctype=0)]),
        case("intraday-1", TODAY, [row(1, EPOCH_SECONDS, 2, ctype=1)]),
        case("intraday-4", TODAY, [row(4, EPOCH_SECONDS, 3, ctype=2)]),
        case("day-relearn", TODAY, [row(3, TODAY + 5, 2, ctype=3), row(3, TODAY - 9, ctype=3)]),
        case("legacy-borrowed", TODAY, [row(2, -40, 3, odid=9), row(3, -1, odid=9)]),
        case("modern-borrowed", TODAY, [row(2, TODAY + 3, 2, odid=9), row(2, TODAY - 3, odid=9)]),
        case("unknown-queue", TODAY, [row(5, TODAY + 9, 2), row(-4, EPOCH_SECONDS), row(7, 3)]),
        case("negative-today", -10, [row(2, -5, 2, odid=9), row(2, -5, 3), row(2, 20)]),
        case("overdue", TODAY, [row(2, TODAY - 1, 2), row(2, TODAY - 400), row(2, TODAY, 4)]),
        case("edge-364", TODAY, [row(2, TODAY + 364, 2), row(2, TODAY + 363)]),
        case("edge-365", TODAY, [row(2, TODAY + 365, 3), row(2, TODAY + 366), row(2, TODAY + 364)]),
        case("window-29", TODAY, [row(2, TODAY + 29, 6), row(2, TODAY + 30, 7)]),
        case("window-30", TODAY, [row(2, TODAY + 30, 7), row(2, TODAY + 31, 1)]),
        case("peak-25", TODAY, [row(2, TODAY + 10, 25), row(2, TODAY + 11, 3)]),
        case("peak-26", TODAY, [row(2, TODAY + 10, 26), row(2, TODAY + 11, 3)]),
        case("peak-tie", TODAY, [row(2, TODAY + 9, 30), row(2, TODAY + 5, 30)]),
        case("peak-late-tie", TODAY, [row(2, TODAY + 5, 30), row(2, TODAY + 9, 31)]),
        case("peak-day-1", TODAY, [row(2, TODAY + 1, 40), row(2, TODAY + 2, 40)]),
        case("peak-day-364", TODAY, [row(2, TODAY + 364, 40), row(2, TODAY + 200, 39)]),
        case("arrears-only", TODAY, [row(2, TODAY - 50, 500), row(1, EPOCH_SECONDS, 20)]),
        case("grouped-1000", TODAY, [row(0, 5, 1000, ctype=0), row(-1, 5, 999)]),
        case(
            "grouped-12345",
            TODAY,
            [row(-2, 5, 12_345), row(2, TODAY + 40, 12_345), row(2, TODAY + 3, 26)],
        ),
        case("grouped-million", TODAY, [row(0, 5, 1_234_567, ctype=0), row(2, TODAY + 2)]),
    ]


def random_rows(rng, count):
    """Rows over every queue, type and window the scan reads, with small copy counts."""
    rows = []
    for _ in range(count):
        queue = rng.choice((-3, -2, -1, 0, 1, 2, 2, 2, 3, 4, 6))
        ctype = rng.choice((0, 1, 2, 2, 3))
        due = rng.choice((rng.randrange(-30, 450), rng.randrange(0, 900), EPOCH_SECONDS))
        odid = rng.choice((0, 0, 0, 7))
        rows.append([queue, ctype, TODAY + due, odid, rng.randrange(1, 40)])
    return rows


def horizon_cases(rng):
    """The scan's classes, then seeded mixes over every queue and both sides of each edge."""
    drawn = edge_cases(None)
    for _ in range(8):
        drawn.append(
            (None, {"today": rng.choice((TODAY, 0, -3, 5000)), "cards": random_rows(rng, 30)})
        )
    return drawn


def readout_cases(rng):
    """Every class at a plain dial, the dials at half-way ties over a small population, then
    seeded mixes."""
    dials = (92.5, 93.5, 0.5, 90.0, 87.4, 100.0, 1.5, 2.5, 0.0, 89.5, 90.49999999999999)
    drawn = [(edge, arguments) for edge, arguments in edge_cases(90.0)]
    base = [row(2, TODAY + 4, 2), row(0, 3, 1, ctype=0), row(-1, 3)]
    for dial in dials:
        drawn.append(("dial", {"today": TODAY, "cards": base, "desired_retention_pct": dial}))
    for _ in range(8):
        drawn.append(
            (
                None,
                {
                    "today": rng.choice((TODAY, 0, -3, 5000)),
                    "cards": random_rows(rng, 30),
                    "desired_retention_pct": rng.choice(dials),
                },
            )
        )
    return drawn


FUNCTIONS = {
    "horizon": {
        "kind": "adapter",
        "function": "horizon.compute_horizon",
        "adapter": with_card_rows,
        "note": (
            "Builds the predecessor's Card from each row [queue, ctype, due, odid, copies], "
            "repeated copies times, calls compute_horizon at the case's day number and returns "
            "the HorizonScan's fields."
        ),
        "cases": horizon_cases,
    },
    "horizon_readout": {
        "kind": "adapter",
        "function": "horizon.build_readout",
        "adapter": with_card_rows_and_a_dial,
        "note": (
            "Builds the same Card rows, calls build_readout at the case's day number with the "
            "case's desired retention as its input and returns the HorizonReadout's fields."
        ),
        "cases": readout_cases,
    },
    "horizon.constants": {
        "kind": "constants",
        "names": [
            "horizon.HORIZON_DAYS",
            "horizon.WINDOW_DAYS",
            "horizon.FLAT_PEAK_REVIEWS",
            "divest.NEW_CARD_LIFETIME_REVIEWS",
        ],
    },
}
