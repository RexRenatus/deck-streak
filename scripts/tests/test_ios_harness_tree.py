"""The harness tree carries the seam the internal build lane inherits, and no signing material
(SPEC-339 R13 and A10, ADR-350 D9).

The lane signs, numbers and uploads the harness from its own environment, so the committed tree
holds the settings it reads and nothing private: the Info.plist's package type, its version keys
read from build settings and its export answer; a privacy manifest; the bundle id read from one
setting whose committed value is a reserved placeholder (RFC 2606's `.invalid`, written reversed);
and no development team, signing identity, signing switch, key, certificate or profile anywhere
under `ios/`. The checker reads one root, so it judges the real tree and planted trees alike: each
planted tree breaks one rule and must be refused by that rule's name, which is the positive
control the absence census needs.

The secret-shaped plants are assembled at run time from their parts, so this file holds none of
them as one literal.
"""

import plistlib
import re
import tempfile
import unittest
from pathlib import Path

from _support import REPO, examined

INFO_PLIST = Path("ios/Harness/Info.plist")
PRIVACY_MANIFEST = Path("ios/Harness/PrivacyInfo.xcprivacy")
# The Info.plist keys the seam fixes, each with the one value it must hold.
SEAM = {
    "CFBundlePackageType": "APPL",
    "CFBundleVersion": "$(CURRENT_PROJECT_VERSION)",
    "CFBundleShortVersionString": "$(MARKETING_VERSION)",
    "CFBundleIdentifier": "$(PRODUCT_BUNDLE_IDENTIFIER)",
    "ITSAppUsesNonExemptEncryption": False,
}
# The build settings only the lane may set, from its own environment or on its command line.
SIGNING_SETTINGS = ("DEVELOPMENT_TEAM", "CODE_SIGN_IDENTITY", "CODE_SIGNING_ALLOWED")
# A key, a certificate, a profile or a keychain: none is ever committed.
SIGNING_SUFFIXES = (".p8", ".p12", ".pfx", ".mobileprovision", ".keychain", ".keychain-db")
PEM_PRIVATE_KEY = re.compile("-----BEGIN [A-Z ]*" + "PRIVATE" + " KEY-----")
SETTING = re.compile(r"^\s*([A-Za-z_][A-Za-z0-9_]*)(?:\[[^\]]*\])*\s*=\s*(.*?)\s*;?\s*$")


def xcconfig_settings(text):
    """Each `NAME = value` line of an xcconfig, in order, its `//` comment dropped; an `#include`
    line sets nothing."""
    settings = []
    for line in text.splitlines():
        line = line.split("//", 1)[0]
        if line.lstrip().startswith("#"):
            continue
        match = SETTING.match(line)
        if match:
            settings.append((match.group(1), match.group(2)))
    return settings


def plist_at(root, relative, problems):
    """The property list at `relative` under `root`, or None with the reason in `problems`."""
    path = root / relative
    if not path.is_file():
        problems.append(f"{relative.as_posix()}: missing")
        return None
    try:
        value = plistlib.loads(path.read_bytes())
    except (plistlib.InvalidFileException, ValueError) as error:
        problems.append(f"{relative.as_posix()}: not a property list ({error})")
        return None
    if not isinstance(value, dict):
        problems.append(f"{relative.as_posix()}: not a dictionary")
        return None
    return value


def harness_problems(root):
    """Every rule of the seam the tree at `root` breaks, each named, and how much was examined."""
    root = Path(root)
    problems = []
    judged = {"files": [], "plist keys": [], "bundle id settings": []}
    files = sorted(path for path in (root / "ios").rglob("*") if path.is_file())
    for path in files:
        relative = path.relative_to(root).as_posix()
        judged["files"].append(relative)
        if path.name.lower().endswith(SIGNING_SUFFIXES):
            problems.append(f"{relative}: signing material ({path.suffix}) is never committed")
        text = path.read_bytes().decode("utf-8", errors="replace")
        for setting in SIGNING_SETTINGS:
            if re.search(rf"(?<![A-Za-z0-9_]){setting}(?![A-Za-z0-9_])", text):
                problems.append(f"{relative}: sets {setting}, which only the lane sets")
        if PEM_PRIVATE_KEY.search(text):
            problems.append(f"{relative}: holds a private key block")
    info = plist_at(root, INFO_PLIST, problems)
    for key, wanted in SEAM.items():
        if info is None:
            break
        judged["plist keys"].append(key)
        held = info.get(key, "absent")
        # `False == 0` in Python, so the export answer is compared by identity: the boolean false.
        wrong = (held is not wanted) if isinstance(wanted, bool) else (held != wanted)
        if wrong:
            problems.append(f"{INFO_PLIST.as_posix()}: {key} is {held!r}, not {wanted!r}")
    manifest = plist_at(root, PRIVACY_MANIFEST, problems)
    if manifest is not None and manifest.get("NSPrivacyTracking") is not False:
        problems.append(
            f"{PRIVACY_MANIFEST.as_posix()}: NSPrivacyTracking is not the boolean false"
        )
    bundle_ids, app_ids = [], []
    for path in sorted((root / "ios").rglob("*.xcconfig")):
        relative = path.relative_to(root).as_posix()
        for name, value in xcconfig_settings(path.read_text(encoding="utf-8")):
            if name == "PRODUCT_BUNDLE_IDENTIFIER":
                bundle_ids.append((relative, value))
            elif name == "DS_APP_ID":
                app_ids.append((relative, value))
    judged["bundle id settings"] = bundle_ids + app_ids
    if not bundle_ids:
        problems.append("ios: no xcconfig sets PRODUCT_BUNDLE_IDENTIFIER")
    for relative, value in bundle_ids:
        if value != "$(DS_APP_ID)":
            problems.append(
                f"{relative}: PRODUCT_BUNDLE_IDENTIFIER is {value!r}, not '$(DS_APP_ID)'"
            )
    if not app_ids:
        problems.append("ios: no xcconfig sets DS_APP_ID")
    for relative, value in app_ids:
        if value.split(".", 1)[0] != "invalid":
            problems.append(
                f"{relative}: DS_APP_ID is {value!r}, whose first label is not the reserved 'invalid'"
            )
    return problems, judged


# A tree that keeps every rule: the controls start from it and each breaks one rule.
GOOD_INFO = {
    "CFBundlePackageType": "APPL",
    "CFBundleVersion": "$(CURRENT_PROJECT_VERSION)",
    "CFBundleShortVersionString": "$(MARKETING_VERSION)",
    "CFBundleIdentifier": "$(PRODUCT_BUNDLE_IDENTIFIER)",
    "ITSAppUsesNonExemptEncryption": False,
}
GOOD_XCCONFIG = (
    "PRODUCT_BUNDLE_IDENTIFIER = $(DS_APP_ID)\n"
    "DS_APP_ID = invalid.planted.harness\n"
    '#include? "Signing.local.xcconfig"\n'
)


def plant(root, info=None, xcconfig=GOOD_XCCONFIG, extra=None):
    """Writes a harness tree under `root`: the good tree, with `info`, `xcconfig` and `extra`
    (relative path to text) in place of or beside its files."""
    harness = root / "ios" / "Harness"
    harness.mkdir(parents=True)
    (harness / "Info.plist").write_bytes(plistlib.dumps(GOOD_INFO if info is None else info))
    (harness / "PrivacyInfo.xcprivacy").write_bytes(
        plistlib.dumps({"NSPrivacyTracking": False, "NSPrivacyAccessedAPITypes": []})
    )
    (root / "ios" / "Config").mkdir()
    (root / "ios" / "Config" / "Harness.xcconfig").write_text(xcconfig, encoding="utf-8")
    for relative, text in (extra or {}).items():
        path = root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")


class TheHarnessTreeCarriesTheSeam(unittest.TestCase):
    def test_the_harness_tree_carries_the_seam_and_no_signing_material(self):
        # The behaviour first: the real tree holds the harness's Info.plist, and keeps every rule.
        self.assertTrue(
            (REPO / INFO_PLIST).is_file(), f"{INFO_PLIST.as_posix()} is not in the tree"
        )
        problems, judged = harness_problems(REPO)
        self.assertEqual(problems, [])
        examined("harness files", judged["files"])
        self.assertEqual(examined("Info.plist keys", judged["plist keys"]), list(SEAM))
        examined("bundle id settings", judged["bundle id settings"])

        # The controls: the good tree is accepted, and each plant is refused by its rule's name.
        team = "DEVELOPMENT" + "_TEAM"
        key_block = (
            "-----BEGIN " + "PRIVATE KEY-----\n" + "A" * 64 + "\n-----END " + "PRIVATE KEY-----\n"
        )
        plants = {
            "the good tree": ({}, []),
            "a literal team": (
                {"xcconfig": GOOD_XCCONFIG + f"{team} = " + "A1B2C" + "3D4E5\n"},
                [f"ios/Config/Harness.xcconfig: sets {team}, which only the lane sets"],
            ),
            "an upload key": (
                {"extra": {"ios/AuthKey.p" + "8": "planted"}},
                ["ios/AuthKey.p8: signing material (.p8) is never committed"],
            ),
            "a profile": (
                {"extra": {"ios/Harness/dev.mobile" + "provision": "planted"}},
                [
                    "ios/Harness/dev.mobileprovision: signing material (.mobileprovision) is "
                    "never committed"
                ],
            ),
            "a private key block": (
                {"extra": {"ios/Harness/notes.txt": key_block}},
                ["ios/Harness/notes.txt: holds a private key block"],
            ),
            "a real-looking app id": (
                {
                    "xcconfig": GOOD_XCCONFIG.replace(
                        "invalid.planted.harness", "com.planted.harness"
                    )
                },
                [
                    "ios/Config/Harness.xcconfig: DS_APP_ID is 'com.planted.harness', whose first "
                    "label is not the reserved 'invalid'"
                ],
            ),
            "signing off in the xcconfig": (
                {"xcconfig": GOOD_XCCONFIG + "CODE_SIGNING_ALLOWED = NO\n"},
                [
                    "ios/Config/Harness.xcconfig: sets CODE_SIGNING_ALLOWED, which only the "
                    "lane sets"
                ],
            ),
            "a bundle id set past the placeholder": (
                {"xcconfig": GOOD_XCCONFIG.replace("= $(DS_APP_ID)", "= invalid.planted.harness")},
                [
                    "ios/Config/Harness.xcconfig: PRODUCT_BUNDLE_IDENTIFIER is "
                    "'invalid.planted.harness', not '$(DS_APP_ID)'"
                ],
            ),
            "an export answer of true": (
                {"info": {**GOOD_INFO, "ITSAppUsesNonExemptEncryption": True}},
                ["ios/Harness/Info.plist: ITSAppUsesNonExemptEncryption is True, not False"],
            ),
            "an export answer of zero": (
                {"info": {**GOOD_INFO, "ITSAppUsesNonExemptEncryption": 0}},
                ["ios/Harness/Info.plist: ITSAppUsesNonExemptEncryption is 0, not False"],
            ),
            "a literal build number": (
                {"info": {**GOOD_INFO, "CFBundleVersion": "7"}},
                [
                    "ios/Harness/Info.plist: CFBundleVersion is '7', not "
                    "'$(CURRENT_PROJECT_VERSION)'"
                ],
            ),
        }
        for name, (shape, wanted) in examined("planted trees", list(plants.items())):
            with self.subTest(plant=name), tempfile.TemporaryDirectory() as scratch:
                plant(Path(scratch), **shape)
                self.assertEqual(harness_problems(Path(scratch))[0], wanted, name)


if __name__ == "__main__":
    unittest.main()
