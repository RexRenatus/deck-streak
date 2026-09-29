"""SPEC-047's registration: whether a reading's covered cards are studied.

`preread_tracking.py:is_studied` reads only its two arguments and a module constant, so it is a plain
function's golden. The cases are synthetic counts covering what a port gets wrong: an empty covered
set, every pair of small counts (the 80 percent line falls between 4 of 5 and 3 of 5), the
boundaries around one hundred covered cards, and more studied than covered.

The case builders draw only from the `random.Random` the generator seeds.
"""


def studied_cases(rng):
    """Every pair up to twelve covered, the one-hundred boundaries and a seeded spread."""
    drawn = [("empty", {"studied": 0, "covered": 0}), ("empty", {"studied": 3, "covered": 0})]
    for covered in range(1, 13):
        for studied in range(0, covered + 2):
            drawn.append(("small", {"studied": studied, "covered": covered}))
    for studied in (78, 79, 80, 81, 100):
        drawn.append(("hundred", {"studied": studied, "covered": 100}))
    for _ in range(40):
        covered = rng.randrange(1, 400)
        drawn.append(("seeded", {"studied": rng.randrange(0, covered + 5), "covered": covered}))
    return drawn


FUNCTIONS = {
    "is_studied": {
        "kind": "function",
        "function": "preread_tracking.is_studied",
        "cases": studied_cases,
    },
}
