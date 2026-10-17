"""The app's tree carries the seam the lane renders, and keeps the sync credential in one Keychain
item on this device (SPEC-347 R12 and A13, ADR-358).

The lane builds the app from the committed tree and its own settings, so the tree holds what the
lane reads and nothing it must not: the Info.plist's seven keys, each read from a build setting but
the package type and the boolean false export answer, and no App Transport Security key; a sync
endpoint that is an `https` URL on a reserved `.invalid` host, and a sync user that is a
placeholder; a privacy manifest with tracking off and no collected data; no entitlements file;
and a credential store whose one file asks the Keychain for a generic password that stays on this
device, is never synchronized, and names no access group. Every generator spec under `ios/` is
read through `test_one_static_library`'s own readers, the app's included, so #624's reading and
this one cannot drift apart.

The checker reads one root and the paths it is handed, so it judges the real tree and planted trees
alike: each planted tree breaks one rule and must be refused by that rule's name.
"""

import plistlib
import re
import tempfile
import unittest
from pathlib import Path
from urllib.parse import urlsplit
from xml.parsers.expat import ExpatError

from _support import REPO, examined
from test_ios_harness_tree import xcconfig_settings
from test_ios_thin_swift import code_of
from test_one_static_library import generator_specs, spec_problems, tracked_paths

INFO_PLIST = "ios/App/Info.plist"
PRIVACY_MANIFEST = "ios/App/PrivacyInfo.xcprivacy"
SETTINGS = "ios/Config/App.xcconfig"
CREDENTIAL_STORE = "ios/App/Sources/SyncCredentialStore.swift"
APP_SPEC = "ios/app.yml"
# R5's seven keys, each with the one value it must hold: the harness's five, and the two sync
# settings the app reads through its config door.
SEAM = {
    "CFBundlePackageType": "APPL",
    "CFBundleVersion": "$(CURRENT_PROJECT_VERSION)",
    "CFBundleShortVersionString": "$(MARKETING_VERSION)",
    "CFBundleIdentifier": "$(PRODUCT_BUNDLE_IDENTIFIER)",
    "ITSAppUsesNonExemptEncryption": False,
    "DSSyncEndpoint": "$(DS_SYNC_ENDPOINT)",
    "DSSyncUser": "$(DS_SYNC_USER)",
}
# The committed sync user: a placeholder the lane replaces, never an account.
SYNC_USER = "invalid-sync-user"
# R9's accessibility: readable after the first unlock, and never restored to another device.
ACCESSIBLE = "kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly"
# The names the store must use: the item's class and its accessibility.
REQUIRED = ("kSecClassGenericPassword", ACCESSIBLE)
# Any accessibility constant, or an access control, which carries an accessibility of its own.
ACCESSIBILITY = re.compile(r"\bkSecAttrAccessible[A-Z]\w*|\bkSecAttrAccessControl\w*")
ACCESS_GROUP = re.compile(r"\bkSecAttrAccessGroup\w*")
SYNCHRONIZABLE = re.compile(r"\bkSecAttrSynchronizable\w*")
NOT_SYNCHRONIZED = re.compile(
    r"\bkSecAttrSynchronizable(?:\s+as\s+String)?\s*:\s*kCFBooleanFalse\b"
)


def plist_of(root, paths, relative, rule, problems):
    """The property list at `relative`, or None with the reason, under `rule`, in `problems`."""
    if relative not in paths:
        problems.append(f"{relative}: {rule}: missing")
        return None
    try:
        value = plistlib.loads((root / relative).read_bytes())
    except (plistlib.InvalidFileException, ValueError, ExpatError) as error:
        problems.append(f"{relative}: {rule}: not a property list ({type(error).__name__})")
        return None
    if not isinstance(value, dict):
        problems.append(f"{relative}: {rule}: not a dictionary")
        return None
    return value


def info_problems(root, paths, judged):
    """The Info.plist's seven keys and values, and no App Transport Security key."""
    problems = []
    info = plist_of(root, paths, INFO_PLIST, "the property list", problems)
    if info is None:
        return problems
    for key, wanted in SEAM.items():
        judged["plist keys"].append(key)
        held = info.get(key, "absent")
        # `False == 0` in Python, so the export answer is compared by identity: the boolean false.
        wrong = (held is not wanted) if isinstance(wanted, bool) else (held != wanted)
        if wrong:
            problems.append(f"{INFO_PLIST}: the property list: {key} is {held!r}, not {wanted!r}")
    if "NSAppTransportSecurity" in info:
        problems.append(f"{INFO_PLIST}: the property list: holds NSAppTransportSecurity")
    return problems


def settings_problems(root, paths, judged):
    """The sync endpoint, `https` on a host under `.invalid`, and the placeholder sync user. Every
    setting of either name is judged, so a later line cannot replace a good one unread."""
    if SETTINGS not in paths:
        return [f"{SETTINGS}: the settings: missing"]
    settings = xcconfig_settings((root / SETTINGS).read_text(encoding="utf-8"))
    endpoints = [value for name, value in settings if name == "DS_SYNC_ENDPOINT"]
    users = [value for name, value in settings if name == "DS_SYNC_USER"]
    problems = []
    if not endpoints:
        problems.append(f"{SETTINGS}: the settings: sets no DS_SYNC_ENDPOINT")
    for value in endpoints:
        judged["settings"].append(f"DS_SYNC_ENDPOINT = {value}")
        # `$()` expands to nothing: it is how an xcconfig writes `//` without opening a comment.
        url = urlsplit(value.replace("$()", ""))
        if url.scheme != "https":
            problems.append(f"{SETTINGS}: the settings: DS_SYNC_ENDPOINT {value!r} is not https")
        host = url.hostname or ""
        if not host.endswith(".invalid"):
            problems.append(
                f"{SETTINGS}: the settings: DS_SYNC_ENDPOINT {value!r} is not on a host under "
                ".invalid"
            )
    if not users:
        problems.append(f"{SETTINGS}: the settings: sets no DS_SYNC_USER")
    for value in users:
        judged["settings"].append(f"DS_SYNC_USER = {value}")
        if value != SYNC_USER:
            problems.append(
                f"{SETTINGS}: the settings: DS_SYNC_USER is {value!r}, not the placeholder "
                f"{SYNC_USER!r}"
            )
    return problems


def privacy_problems(root, paths, judged):
    """The privacy manifest: tracking the boolean false, and no collected data type."""
    problems = []
    manifest = plist_of(root, paths, PRIVACY_MANIFEST, "the privacy manifest", problems)
    if manifest is None:
        return problems
    judged["privacy keys"].extend(["NSPrivacyTracking", "NSPrivacyCollectedDataTypes"])
    if manifest.get("NSPrivacyTracking") is not False:
        problems.append(
            f"{PRIVACY_MANIFEST}: the privacy manifest: NSPrivacyTracking is not the boolean false"
        )
    if manifest.get("NSPrivacyCollectedDataTypes", []) != []:
        problems.append(f"{PRIVACY_MANIFEST}: the privacy manifest: declares a collected data type")
    return problems


def credential_problems(root, paths, judged):
    """The store's one file, read as code (its comments and string text removed): a generic
    password, R9's accessibility and no other, never synchronizable, and no access group."""
    if CREDENTIAL_STORE not in paths:
        return [f"{CREDENTIAL_STORE}: the credential store: missing"]
    code = code_of((root / CREDENTIAL_STORE).read_text(encoding="utf-8"))
    problems = []
    for name in REQUIRED:
        judged["credential names"].append(name)
        if not re.search(rf"\b{name}\b", code):
            problems.append(f"{CREDENTIAL_STORE}: the credential store: names no {name}")
    for token in sorted(set(ACCESSIBILITY.findall(code))):
        judged["credential names"].append(token)
        if token != ACCESSIBLE:
            problems.append(
                f"{CREDENTIAL_STORE}: the credential store: names {token}, another accessibility"
            )
    for token in sorted(set(ACCESS_GROUP.findall(code))):
        judged["credential names"].append(token)
        problems.append(f"{CREDENTIAL_STORE}: the credential store: names {token}, an access group")
    named = SYNCHRONIZABLE.findall(code)
    judged["credential names"].extend(named)
    if not named:
        problems.append(
            f"{CREDENTIAL_STORE}: the credential store: names no kSecAttrSynchronizable"
        )
    elif len(NOT_SYNCHRONIZED.findall(code)) != len(named):
        problems.append(
            f"{CREDENTIAL_STORE}: the credential store: kSecAttrSynchronizable is not paired with "
            "kCFBooleanFalse"
        )
    return problems


def app_tree_problems(root, paths):
    """Every rule of R12 the tree at `root` breaks, each named, and what was examined. `paths` are
    the files judged, relative to `root`: what git tracks, for the live tree."""
    root, paths = Path(root), set(paths)
    judged = {
        "plist keys": [],
        "settings": [],
        "privacy keys": [],
        "credential names": [],
        "generator specs": [],
    }
    problems = info_problems(root, paths, judged)
    problems += settings_problems(root, paths, judged)
    problems += privacy_problems(root, paths, judged)
    for path in sorted(paths):
        if path.startswith("ios/") and path.lower().endswith(".entitlements"):
            problems.append(f"{path}: the entitlements: an entitlements file is never committed")
    problems += credential_problems(root, paths, judged)
    specs = generator_specs(sorted(paths))
    judged["generator specs"] = specs
    if APP_SPEC not in specs:
        problems.append(f"{APP_SPEC}: the generator specs: missing")
    for spec in specs:
        for problem in spec_problems(spec, (root / spec).read_text(encoding="utf-8")):
            problems.append(f"{spec}: the generator specs: {problem.removeprefix(spec + ' ')}")
    return problems, judged


# A tree that keeps every rule: the controls start from it and each breaks one rule. The store
# names a refused constant in a comment and an access group's value in a string, so a census that
# read comments or string text would refuse it.
GOOD_INFO = {**SEAM, "UILaunchScreen": {}}
GOOD_SETTINGS = (
    "PRODUCT_BUNDLE_IDENTIFIER = $(DS_APP_ID)\n"
    "DS_APP_ID = invalid.planted.app\n"
    "DS_SYNC_ENDPOINT = https:/$()/sync.planted.invalid/\n"
    "DS_SYNC_USER = invalid-sync-user\n"
    '#include? "Signing.local.xcconfig"\n'
)
GOOD_PRIVACY = {
    "NSPrivacyTracking": False,
    "NSPrivacyCollectedDataTypes": [],
    "NSPrivacyAccessedAPITypes": [],
}
GOOD_STORE = (
    "import Security\n\n"
    "// Never kSecAttrAccessibleWhenUnlocked, which a restore carries to another device.\n"
    "enum SyncCredentialStore {\n"
    "    static func query(service: String, account: String) -> [CFString: Any] {\n"
    "        [\n"
    "            kSecClass: kSecClassGenericPassword,\n"
    '            kSecAttrLabel: "kSecAttrAccessGroup",\n'
    "            kSecAttrService: service,\n"
    "            kSecAttrAccount: account,\n"
    f"            kSecAttrAccessible: {ACCESSIBLE},\n"
    "            kSecAttrSynchronizable: kCFBooleanFalse as Any,\n"
    "        ]\n"
    "    }\n"
    "}\n"
)
GOOD_APP_SPEC = (
    "name: DeckStreak\n"
    "packages:\n"
    "  EnginePackage:\n"
    "    path: EnginePackage\n"
    "targets:\n"
    "  DeckStreak:\n"
    "    type: application\n"
    "    dependencies:\n"
    "      - package: EnginePackage\n"
    "        product: DeckStreakFFI\n"
)
GOOD_HARNESS_SPEC = GOOD_APP_SPEC.replace("DeckStreak:\n", "Harness:\n")
KEEP = object()


def plant(
    root,
    info=KEEP,
    settings=KEEP,
    privacy=KEEP,
    store=KEEP,
    spec=KEEP,
    harness_spec=KEEP,
    extra=None,
):
    """Writes an app tree under `root`: the good tree, with each argument in place of its file
    (None leaves the file out), and `extra` (relative path to text) beside them."""
    files = {
        INFO_PLIST: plistlib.dumps(GOOD_INFO if info is KEEP else info)
        if info is not None
        else None,
        PRIVACY_MANIFEST: (
            plistlib.dumps(GOOD_PRIVACY if privacy is KEEP else privacy)
            if privacy is not None
            else None
        ),
        SETTINGS: GOOD_SETTINGS if settings is KEEP else settings,
        CREDENTIAL_STORE: GOOD_STORE if store is KEEP else store,
        APP_SPEC: GOOD_APP_SPEC if spec is KEEP else spec,
        "ios/project.yml": GOOD_HARNESS_SPEC if harness_spec is KEEP else harness_spec,
        **(extra or {}),
    }
    for relative, content in files.items():
        if content is None:
            continue
        path = root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        if isinstance(content, bytes):
            path.write_bytes(content)
        else:
            path.write_text(content, encoding="utf-8")


def files_under(root):
    """Every file under `root`, relative to it, as git would list them."""
    return sorted(path.relative_to(root).as_posix() for path in root.rglob("*") if path.is_file())


def store_with(old, new):
    """The good store, with `old` replaced once by `new`."""
    assert GOOD_STORE.count(old) == 1, f"{old!r} is not in the planted store once"
    return GOOD_STORE.replace(old, new)


def settings_with(old, new):
    """The good settings, with `old` replaced once by `new`."""
    assert GOOD_SETTINGS.count(old) == 1, f"{old!r} is not in the planted settings once"
    return GOOD_SETTINGS.replace(old, new)


ENDPOINT = "https:/$()/sync.planted.invalid/"
INFO_RULE = f"{INFO_PLIST}: the property list:"
SETTINGS_RULE = f"{SETTINGS}: the settings:"
PRIVACY_RULE = f"{PRIVACY_MANIFEST}: the privacy manifest:"
STORE_RULE = f"{CREDENTIAL_STORE}: the credential store:"
PLANTS = {
    "the good tree": ({}, []),
    "no property list": ({"info": None}, [f"{INFO_RULE} missing"]),
    "a property list that does not parse": (
        {"extra": {INFO_PLIST: "<plist><dict><key>planted</key></plist>"}},
        [f"{INFO_RULE} not a property list (ExpatError)"],
    ),
    "an App Transport Security key": (
        {"info": {**GOOD_INFO, "NSAppTransportSecurity": {}}},
        [f"{INFO_RULE} holds NSAppTransportSecurity"],
    ),
    "an export answer of zero": (
        {"info": {**GOOD_INFO, "ITSAppUsesNonExemptEncryption": 0}},
        [f"{INFO_RULE} ITSAppUsesNonExemptEncryption is 0, not False"],
    ),
    "a literal sync user in the property list": (
        {"info": {**GOOD_INFO, "DSSyncUser": "learner"}},
        [f"{INFO_RULE} DSSyncUser is 'learner', not '$(DS_SYNC_USER)'"],
    ),
    "no sync endpoint key": (
        {"info": {key: value for key, value in GOOD_INFO.items() if key != "DSSyncEndpoint"}},
        [f"{INFO_RULE} DSSyncEndpoint is 'absent', not '$(DS_SYNC_ENDPOINT)'"],
    ),
    "no settings": ({"settings": None}, [f"{SETTINGS_RULE} missing"]),
    "an endpoint on a real host": (
        {"settings": settings_with(ENDPOINT, "https:/$()/sync.planted.example/")},
        [
            f"{SETTINGS_RULE} DS_SYNC_ENDPOINT 'https:/$()/sync.planted.example/' is not on a host "
            "under .invalid"
        ],
    ),
    "an endpoint whose host only holds the label": (
        {"settings": settings_with(ENDPOINT, "https:/$()/sync.invalid.example/")},
        [
            f"{SETTINGS_RULE} DS_SYNC_ENDPOINT 'https:/$()/sync.invalid.example/' is not on a host "
            "under .invalid"
        ],
    ),
    "a plain http endpoint": (
        {"settings": settings_with(ENDPOINT, "http:/$()/sync.planted.invalid/")},
        [f"{SETTINGS_RULE} DS_SYNC_ENDPOINT 'http:/$()/sync.planted.invalid/' is not https"],
    ),
    "an endpoint an xcconfig comment cuts": (
        {"settings": settings_with(ENDPOINT, "https:" + "//sync.planted.invalid/")},
        [f"{SETTINGS_RULE} DS_SYNC_ENDPOINT 'https:' is not on a host under .invalid"],
    ),
    "a second endpoint after a good one": (
        {"settings": GOOD_SETTINGS + "DS_SYNC_ENDPOINT = https:/$()/sync.planted.example/\n"},
        [
            f"{SETTINGS_RULE} DS_SYNC_ENDPOINT 'https:/$()/sync.planted.example/' is not on a host "
            "under .invalid"
        ],
    ),
    "no sync endpoint setting": (
        {"settings": settings_with(f"DS_SYNC_ENDPOINT = {ENDPOINT}\n", "")},
        [f"{SETTINGS_RULE} sets no DS_SYNC_ENDPOINT"],
    ),
    "a real-looking sync user": (
        {"settings": settings_with("DS_SYNC_USER = invalid-sync-user", "DS_SYNC_USER = learner")},
        [f"{SETTINGS_RULE} DS_SYNC_USER is 'learner', not the placeholder 'invalid-sync-user'"],
    ),
    "no sync user setting": (
        {"settings": settings_with("DS_SYNC_USER = invalid-sync-user\n", "")},
        [f"{SETTINGS_RULE} sets no DS_SYNC_USER"],
    ),
    "no privacy manifest": ({"privacy": None}, [f"{PRIVACY_RULE} missing"]),
    "tracking on": (
        {"privacy": {**GOOD_PRIVACY, "NSPrivacyTracking": True}},
        [f"{PRIVACY_RULE} NSPrivacyTracking is not the boolean false"],
    ),
    "a collected data type": (
        {
            "privacy": {
                **GOOD_PRIVACY,
                "NSPrivacyCollectedDataTypes": [
                    {"NSPrivacyCollectedDataType": "NSPrivacyCollectedDataTypeEmailAddress"}
                ],
            }
        },
        [f"{PRIVACY_RULE} declares a collected data type"],
    ),
    "an entitlements file": (
        {"extra": {"ios/App/DeckStreak.entitlements": "<plist/>\n"}},
        [
            "ios/App/DeckStreak.entitlements: the entitlements: an entitlements file is never "
            "committed"
        ],
    ),
    "no credential store": ({"store": None}, [f"{STORE_RULE} missing"]),
    "an accessibility that leaves the device": (
        {"store": store_with(ACCESSIBLE, "kSecAttrAccessibleAfterFirstUnlock")},
        [
            f"{STORE_RULE} names no {ACCESSIBLE}",
            f"{STORE_RULE} names kSecAttrAccessibleAfterFirstUnlock, another accessibility",
        ],
    ),
    "a second accessibility beside the first": (
        {
            "store": store_with(
                "            kSecAttrService: service,\n",
                "            kSecAttrService: service,\n"
                "            kSecAttrAccessible: kSecAttrAccessibleWhenUnlocked,\n",
            )
        },
        [f"{STORE_RULE} names kSecAttrAccessibleWhenUnlocked, another accessibility"],
    ),
    "an access control": (
        {
            "store": store_with(
                "            kSecAttrService: service,\n",
                "            kSecAttrService: service,\n            kSecAttrAccessControl: control,\n",
            )
        },
        [f"{STORE_RULE} names kSecAttrAccessControl, another accessibility"],
    ),
    "the accessibility only in a comment": (
        {
            "store": store_with(
                f"            kSecAttrAccessible: {ACCESSIBLE},\n",
                f"            // kSecAttrAccessible: {ACCESSIBLE},\n",
            )
        },
        [f"{STORE_RULE} names no {ACCESSIBLE}"],
    ),
    "an access group": (
        {
            "store": store_with(
                "            kSecAttrService: service,\n",
                '            kSecAttrService: service,\n            kSecAttrAccessGroup: "planted",\n',
            )
        },
        [f"{STORE_RULE} names kSecAttrAccessGroup, an access group"],
    ),
    "a synchronizable item": (
        {"store": store_with("kCFBooleanFalse", "kCFBooleanTrue")},
        [f"{STORE_RULE} kSecAttrSynchronizable is not paired with kCFBooleanFalse"],
    ),
    "any synchronization": (
        {
            "store": store_with(
                "kSecAttrSynchronizable: kCFBooleanFalse",
                "kSecAttrSynchronizable: kSecAttrSynchronizableAny",
            )
        },
        [f"{STORE_RULE} kSecAttrSynchronizable is not paired with kCFBooleanFalse"],
    ),
    "no synchronizable attribute": (
        {"store": store_with("            kSecAttrSynchronizable: kCFBooleanFalse as Any,\n", "")},
        [f"{STORE_RULE} names no kSecAttrSynchronizable"],
    ),
    "an internet password": (
        {"store": store_with("kSecClassGenericPassword", "kSecClassInternetPassword")},
        [f"{STORE_RULE} names no kSecClassGenericPassword"],
    ),
    "no app spec": ({"spec": None}, [f"{APP_SPEC}: the generator specs: missing"]),
    "a framework in the app's spec": (
        {"spec": GOOD_APP_SPEC + "      - framework: Planted.framework\n"},
        [f"{APP_SPEC}: the generator specs: declares a framework dependency"],
    ),
    "a remote package in the app's spec": (
        {
            "spec": GOOD_APP_SPEC.replace(
                "packages:\n", "packages:\n  Remote:\n    url: https://planted.invalid/remote\n"
            )
        },
        [
            f"{APP_SPEC}: the generator specs: declares a remote package: url: "
            "https://planted.invalid/remote"
        ],
    ),
    "a framework in the harness's spec": (
        {"harness_spec": GOOD_HARNESS_SPEC + "      - framework: Planted.framework\n"},
        ["ios/project.yml: the generator specs: declares a framework dependency"],
    ),
}


class TheAppTreeCarriesTheSeam(unittest.TestCase):
    def test_the_app_tree_carries_the_seam_and_keeps_the_credential_in_the_keychain(self):
        # The behaviour first: the tracked tree keeps every rule of R12.
        problems, judged = app_tree_problems(REPO, tracked_paths(REPO))
        counts = ", ".join(f"{len(items)} {what}" for what, items in judged.items())
        with self.subTest(tree="the live tree"):
            self.assertEqual(problems, [], f"examined {counts}")

        # The controls: the good tree is accepted, and each plant is refused by its rule's name.
        for name, (shape, wanted) in examined("planted trees", list(PLANTS.items())):
            with self.subTest(plant=name), tempfile.TemporaryDirectory() as scratch:
                root = Path(scratch)
                plant(root, **shape)
                got = app_tree_problems(root, files_under(root))[0]
                print(f"planted {name}: {'; '.join(got) if got else 'accepted'}")
                self.assertEqual(got, wanted, name)

        for what, items in judged.items():
            with self.subTest(examined=what):
                examined(what, items)


if __name__ == "__main__":
    unittest.main()
