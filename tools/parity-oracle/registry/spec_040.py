"""SPEC-040's registration: the level of an XP total, from the predecessor's own level curve.

`gamification/xp.py:level_for_xp` maps a total to the largest level whose threshold it reaches,
through an integer square root and floor division. The case builder picks where to ask: on the
curve's thresholds for levels 1 to 100 and one XP below each (level 1 begins at 0 XP, below which
no unsigned total lies); on eight seeded levels between 2 x 10^7 and 6 x 10^8, where a
floating-point square root places the total one XP below a threshold a level too high; and on the
widest total a 64-bit unsigned ledger holds. A threshold is only where a case sits: every output is
what the predecessor's function returned, and a case whose input missed a threshold would show no
level rise between it and the case below it, which the port's test reads.
"""

#: The widest total an unsigned 64-bit ledger holds: the port computes its level without overflow.
WIDEST_TOTAL = 2**64 - 1

#: The seeded levels lie past the first level where a double-precision root misplaces the total one
#: below its threshold, and below the last level whose threshold an unsigned 64-bit total holds.
LARGE_LEVELS = (20_000_000, 600_000_000)


def threshold(level):
    """The total the curve's `level` begins at, 50L^2 - 50L: an input a case sits on."""
    return 50 * level * level - 50 * level


def cases(rng):
    found = []
    for level in range(1, 101):
        found.append(("threshold", {"total_xp": threshold(level)}))
        if level > 1:
            found.append(("below", {"total_xp": threshold(level) - 1}))
    for level in sorted(rng.randrange(*LARGE_LEVELS) for _ in range(8)):
        found.append(("large-threshold", {"total_xp": threshold(level)}))
        found.append(("large-below", {"total_xp": threshold(level) - 1}))
    found.append(("widest", {"total_xp": WIDEST_TOTAL}))
    return found


FUNCTIONS = {
    "level_for_xp": {
        "kind": "function",
        "function": "gamification.xp.level_for_xp",
        "cases": cases,
    },
}
