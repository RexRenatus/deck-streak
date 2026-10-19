"""SPEC-483: the wallet model's later-day refund property is declared once, at ramp=block.

The formal checker, outside this repository, reads a property's ramp from its
`@phx property` header line and decides what the ramp does; this module reads only that
line's text, never the model. A1 reads the committed entry; A2 plants each fault in a
fixture tree and reads the reader refuse it. The ramp moves only on the streak the
checker counts (ADR-500 D2)."""

import re
import tempfile
import unittest
from pathlib import Path

from _support import REPO, examined

ENTRY = Path("formal") / "tla" / "WalletFloor" / "WalletFloor.tla"
PROPERTY = "RefundWritesItsOwnMovement"
WANTED = "block"
HEADER = re.compile(r"^\\\* @phx property (\S+) ramp=(\S+)$")

# The entry's four other property lines and the refund property's two spellings, written
# here and never read from the tree; each plant changes only the refund property's line.
OTHERS = (
    "\\* @phx property FloorHolds ramp=report",
    "\\* @phx property OneMovementPerKey ramp=report",
    "\\* @phx property DepositOnceEver ramp=report",
    "\\* @phx property SettledMintNeverFalls ramp=report",
)
BLOCK = "\\* @phx property RefundWritesItsOwnMovement ramp=block"
REPORT = "\\* @phx property RefundWritesItsOwnMovement ramp=report"
PLANTS = (
    (
        "block",
        (*OTHERS, BLOCK),
        "ok: RefundWritesItsOwnMovement reads ramp=block",
    ),
    (
        "report",
        (*OTHERS, REPORT),
        "refused: RefundWritesItsOwnMovement reads ramp=report",
    ),
    (
        "absent",
        OTHERS,
        "refused: 0 lines declare RefundWritesItsOwnMovement",
    ),
    (
        "twice",
        (*OTHERS, BLOCK, REPORT),
        "refused: 2 lines declare RefundWritesItsOwnMovement",
    ),
)


def verdict(root, name=PROPERTY):
    """The verdict on the one property line naming `name` in the entry under `root`."""
    text = (root / ENTRY).read_text(encoding="utf-8")
    matches = [HEADER.match(line) for line in text.splitlines()]
    found = [m.group(2) for m in matches if m and m.group(1) == name]
    if len(found) != 1:
        return f"refused: {len(found)} lines declare {name}"
    if found[0] != WANTED:
        return f"refused: {name} reads ramp={found[0]}"
    return f"ok: {name} reads ramp={found[0]}"


class WalletRefundRamp(unittest.TestCase):
    def test_the_refund_property_is_declared_once_at_block(self):
        """A1: the committed entry declares the refund property once, at ramp=block."""
        self.assertEqual(verdict(REPO), "ok: RefundWritesItsOwnMovement reads ramp=block")

    def test_the_reader_refuses_each_planted_ramp(self):
        """A2: each planted line reads its own verdict; the block line alone reads ok."""
        seen = 0
        for label, lines, expected in PLANTS:
            with self.subTest(plant=label), tempfile.TemporaryDirectory() as scratch:
                root = Path(scratch)
                (root / ENTRY).parent.mkdir(parents=True)
                (root / ENTRY).write_text("\n".join(lines) + "\n", encoding="utf-8")
                self.assertEqual(verdict(root), expected)
                seen += 1
        examined("plants", range(seen))
        self.assertEqual(seen, len(PLANTS))


if __name__ == "__main__":
    unittest.main()
