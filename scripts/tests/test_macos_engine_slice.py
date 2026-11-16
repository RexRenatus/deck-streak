"""SPEC-679 section 9a: the engine's Swift package declares macOS at the SPEC's minimum, and the
XCFramework workflow builds a macOS library beside the device and simulator ones, joins the three
into one XCFramework, checks that each slice holds the one Rust library and typechecks the
generated Swift against each slice's headers. Every file is read through the workflow tests' one
reader."""

import re
import unittest
from pathlib import Path

from test_ci_workflows import workflow_file_text

REPO = Path(__file__).resolve().parents[2]
PACKAGE = "ios/EnginePackage/Package.swift"
WORKFLOW = ".github/workflows/xcframework.yml"
SPEC = "docs/specs/SPEC-679-a-native-macos-destination-for-the-one-app-target.md"
MINIMUM = re.compile(r"macOS (\d+(?:\.\d+)+) as the destination's minimum version")
LIBRARY = "libdeck_streak_ffi.a"
# The three slices: each one's Rust target, its library folder in the XCFramework and its SDK.
SLICES = (
    ("aarch64-apple-ios", "ios-arm64", "iphoneos"),
    ("aarch64-apple-ios-sim", "ios-arm64-simulator", "iphonesimulator"),
    ("aarch64-apple-darwin", "macos-arm64", "macosx"),
)


def examined(what, items):
    """Print how many items a check examined, and refuse an empty population."""
    items = list(items)
    print(f"examined {len(items)} {what}")
    if not items:
        raise AssertionError(f"examined 0 {what}: the population is empty, so nothing was judged")
    return items


def text_of(case, relative):
    """Assert that a committed file is present, then return its text."""
    path = REPO / relative
    case.assertTrue(path.is_file(), f"{relative} is not a file")
    return workflow_file_text(path)


def spec_minimum(case):
    """The one minimum macOS version SPEC-679 names, read across line breaks."""
    named = set(MINIMUM.findall(" ".join(text_of(case, SPEC).split())))
    case.assertEqual(len(named), 1, f"SPEC-679 names {sorted(named)} as its minimum macOS version")
    return named.pop()


def names(line, target):
    """Whether the line names the Rust target as a whole word."""
    return re.search(rf"(?<![\w-]){re.escape(target)}(?![\w-])", line) is not None


def lines_holding(lines, marker):
    return [line.strip() for line in lines if marker in line]


class TheEngineCarriesAMacosSlice(unittest.TestCase):
    def test_the_package_declares_macos(self):
        """SPEC-679 section 9a: the engine package's platforms name macOS at the SPEC's minimum."""
        major = spec_minimum(self).split(".")[0]
        platforms = examined(
            "platforms lines", lines_holding(text_of(self, PACKAGE).splitlines(), "platforms:")
        )
        self.assertEqual(len(platforms), 1, f"{PACKAGE} holds one platforms line")
        self.assertIn(
            f".macOS(.v{major})", platforms[0], "the engine package declares macOS at the minimum"
        )

    def test_the_workflow_builds_checks_and_typechecks_three_slices(self):
        """SPEC-679 section 9a: the workflow adds each slice's Rust target, builds each library,
        joins all three into the XCFramework, lists all three in its slice check and its per-slice
        typecheck, and builds the macOS library for the SPEC's minimum."""
        lines = examined("workflow lines", text_of(self, WORKFLOW).splitlines())
        rustup = lines_holding(lines, "rustup target add")
        loops = lines_holding(lines, "for target in")
        sliced = lines_holding(lines, f"/{LIBRARY} ")
        consumer = lines_holding(lines, "for slice in")
        for target, folder, sdk in examined("slices", SLICES):
            self.assertTrue(any(names(line, target) for line in rustup), f"rustup adds {target}")
            self.assertTrue(
                any(names(line, target) for line in loops),
                f"the library loop builds {target}",
            )
            self.assertIn(
                f"-library target/{target}/release/{LIBRARY}",
                " ".join(lines),
                f"the XCFramework joins the {folder} library",
            )
            self.assertTrue(
                any(f"{folder}/{LIBRARY} " in f"{line} " for line in sliced),
                f"the slice check lists {folder}",
            )
            self.assertTrue(
                any(f"{folder}:{sdk}:" in line for line in consumer),
                f"the per-slice typecheck lists {folder}",
            )
        self.assertIn(
            f'MACOSX_DEPLOYMENT_TARGET: "{spec_minimum(self)}"',
            [line.strip() for line in lines],
            "the macOS library is built for SPEC-679's minimum",
        )


if __name__ == "__main__":
    unittest.main()
