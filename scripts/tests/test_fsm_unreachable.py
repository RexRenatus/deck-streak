"""FSM-UNREACHABLE (SPEC-433 R8 and R11; ADR-447 D3): a listed-set ratchet on match arms whose
value is a panicking macro, in product source.

A10 reads this repository's own tree and prints the rule's lines into the job's log. A11 reads a
fixture tree written under a temporary directory, and prints nothing."""

import tempfile
import unittest
from pathlib import Path

import _fsm_lints
from _support import REPO, examined

#: The register's seeded `listed` entries (SPEC-433 section 4), frozen here: none.
SEED = ()
SUMMARY = (
    r"(?m)^FSM-LINT FSM-UNREACHABLE level=(warn|fail) examined=[1-9][0-9]* arms"
    r" hits=[0-9]+ new=[0-9]+ listed=[0-9]+ stale=[0-9]+$"
)
PLANT = "crates/plant/src/lib.rs"

#: A11: each macro as an arm's bare value, and each as its block's first statement, beside the
#: same macros in a doc comment, under `#[cfg(test)]` and outside any arm.
PANICKING_TREE = {
    PLANT: """\
/// The light's arms.
///
/// ```
/// match light { Light::Red => unreachable!(), _ => {} }
/// ```
pub enum Light {
    Red,
    Amber,
    Green,
    Off,
}

pub fn bare(light: Light) -> u8 {
    match light {
        Light::Red => unreachable!("never red"),
        Light::Amber => panic!("never amber"),
        Light::Green => todo!(),
        Light::Off => unimplemented!(),
    }
}

pub fn blocked(light: Light) -> u8 {
    match light {
        Light::Red => {
            unreachable!("never red")
        }
        Light::Amber => {
            panic!("never amber");
        }
        Light::Green => { todo!() }
        Light::Off => {
            unimplemented!()
        }
    }
}

pub fn outside(light: Light) -> u8 {
    if matches!(light, Light::Off) {
        panic!("outside any arm");
    }
    match light {
        Light::Red => 1,
        _ => 2,
    }
}

#[cfg(test)]
mod tests {
    fn tested(value: u8) -> u8 {
        match value {
            1 => panic!("under test"),
            _ => unreachable!(),
        }
    }
}
""",
}
PANICKING_HITS = {
    ("new", PLANT, ("fn=bare", "macro=unreachable")),
    ("new", PLANT, ("fn=bare", "macro=panic")),
    ("new", PLANT, ("fn=bare", "macro=todo")),
    ("new", PLANT, ("fn=bare", "macro=unimplemented")),
    ("new", PLANT, ("fn=blocked", "macro=unreachable")),
    ("new", PLANT, ("fn=blocked", "macro=panic")),
    ("new", PLANT, ("fn=blocked", "macro=todo")),
    ("new", PLANT, ("fn=blocked", "macro=unimplemented")),
}


def plant(root, files):
    """Write a fixture tree: each repository-relative path with its text."""
    for path, text in files.items():
        target = Path(root) / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(text, encoding="utf-8")


class UnreachableLint(unittest.TestCase):
    def test_the_tree_reading_reads_every_product_file(self):
        body = _fsm_lints.read_register(REPO)["rules"]["FSM-UNREACHABLE"]
        reading = _fsm_lints.unreachable(REPO, body, SEED)
        crates = REPO / "crates"
        listing = examined(
            "product files",
            sorted(
                p
                for p in crates.rglob("*.rs")
                if p.is_file() and p.relative_to(crates).parts[1:2] == ("src",)
            ),
        )
        self.assertEqual(reading.files, len(listing))
        printed = _fsm_lints.report(reading)
        self.assertRegex("\n".join(printed), SUMMARY)
        self.assertEqual(len(printed), 1 + len(reading.hits))

    def test_each_panicking_arm_is_a_hit_and_nothing_else_is(self):
        with tempfile.TemporaryDirectory() as scratch:
            plant(scratch, PANICKING_TREE)
            reading = _fsm_lints.unreachable(Path(scratch), {"level": "warn", "listed": []}, ())
        self.assertEqual({(hit.kind, hit.path, hit.key) for hit in reading.hits}, PANICKING_HITS)
        self.assertEqual(reading.examined, 10)


if __name__ == "__main__":
    unittest.main()
