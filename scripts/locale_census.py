#!/usr/bin/env python3
"""locale_census: every message of the web app exists in all seven locales (SPEC-571, ADR-590).

    python3 scripts/locale_census.py [--root ROOT]

WHY. The web app's message compiler does not refuse a gap: a message missing from a locale is
compiled as English's own, so a reader of another locale meets an English line inside a translated
screen. This census reads `web/app/project.inlang/settings.json` and every catalog it names, and
refuses each gap where it is made, by a check CI's Python stage already runs.

It refuses, one line per finding:
- `locale-set locales <list>`, `locale-set base <base>`, `locale-set pattern <pattern>`: a locale
  set other than the seven, a base locale other than English, or another path pattern;
- `file-missing <locale>`: a listed locale with no catalog;
- `file-stray <file>`: a catalog under the catalogs' folder that no listed locale names;
- `unreadable <locale> not-json`, `unreadable <locale> not-an-object`,
  `unreadable <locale> repeats <key>`: a catalog that is not a JSON object, or repeats a key; such
  a catalog is not examined;
- `missing <locale> <key>`: a key English holds that the locale lacks;
- `stale <locale> <key>`: a key the locale holds that English lacks;
- `not-text <locale> <key>`, `empty <locale> <key>`: a value that is not text, or is blank after
  trimming, in any locale, English included;
- `not-nfc <locale> <key>`: a value not in Unicode NFC;
- `placeholder <locale> <key>`: a value whose `{name}` placeholders, counted as a multiset, differ
  from English's.

Then it prints `examined <n> locales, <m> keys`: the catalogs it examined and English's keys. The
`$schema` member is not a message. It exits 0 when green, 1 on a finding, and 3 when it examined no
catalog: a run that examined nothing is void, never a pass. It reads presence and shape, never
meaning, and pins no key count.
"""

import argparse
import collections
import json
import re
import sys
import unicodedata
from pathlib import Path

SEVEN = ("en", "es", "fr", "ja", "ko", "zh-Hans", "zh-Hant")
BASE = "en"
PATTERN = "./messages/{locale}.json"
PLACEHOLDER = re.compile(r"\{[^{}]*\}")
FORM = "NFC"
VOID = 3

REPO = Path(__file__).resolve().parents[1]
SETTINGS = "web/app/project.inlang/settings.json"


def placeholders(value):
    """The `{name}` placeholders of a message, counted as a multiset."""
    return collections.Counter(PLACEHOLDER.findall(value))


def catalog_of(path, locale, findings):
    """The messages of one catalog, or None, with a finding, when it cannot be examined."""
    repeated = []

    def pairs(members):
        counted = collections.Counter(key for key, _ in members)
        repeated.extend(key for key, times in counted.items() if times > 1)
        return dict(members)

    try:
        catalog = json.loads(path.read_text(encoding="utf-8"), object_pairs_hook=pairs)
    except ValueError:
        findings.append(f"unreadable {locale} not-json")
        return None
    if not isinstance(catalog, dict):
        findings.append(f"unreadable {locale} not-an-object")
        return None
    findings.extend(f"unreadable {locale} repeats {key}" for key in repeated)
    if repeated:
        return None
    catalog.pop("$schema", None)
    return catalog


def judged(locale, key, value, reference):
    """The findings of one message of one locale, against English's catalog."""
    if key not in reference:
        return [f"stale {locale} {key}"]
    if not isinstance(value, str):
        return [f"not-text {locale} {key}"]
    if not value.strip():
        return [f"empty {locale} {key}"]
    found = []
    if unicodedata.normalize(FORM, value) != value:
        found.append(f"not-nfc {locale} {key}")
    english = reference[key]
    if isinstance(english, str) and placeholders(value) != placeholders(english):
        found.append(f"placeholder {locale} {key}")
    return found


def census(root):
    """(findings, catalogs examined, English keys examined) over the tree at `root`."""
    findings = []
    path = root / SETTINGS
    settings = json.loads(path.read_text(encoding="utf-8")) if path.is_file() else {}
    listed = settings.get("locales", [])
    if sorted(listed) != sorted(SEVEN):
        findings.append(f"locale-set locales {json.dumps(listed)}")
    base = settings.get("baseLocale")
    if base != BASE:
        findings.append(f"locale-set base {json.dumps(base)}")
    pattern = settings.get("plugin.inlang.messageFormat", {}).get("pathPattern")
    if pattern != PATTERN:
        findings.append(f"locale-set pattern {json.dumps(pattern)}")
    project = path.parent.parent
    catalogs = {}
    for locale in sorted(listed):
        catalog_path = project / PATTERN.format(locale=locale)
        if not catalog_path.is_file():
            findings.append(f"file-missing {locale}")
            continue
        catalog = catalog_of(catalog_path, locale, findings)
        if catalog is not None:
            catalogs[locale] = catalog
    findings.extend(
        f"file-stray {stray.name}"
        for stray in sorted((project / PATTERN).parent.glob("*.json"))
        if stray.stem not in listed
    )
    reference = catalogs.get(BASE, {})
    for locale, catalog in catalogs.items():
        findings.extend(f"missing {locale} {key}" for key in reference if key not in catalog)
        for key, value in catalog.items():
            findings.extend(judged(locale, key, value, reference))
    return findings, len(catalogs), len(reference)


def main(argv=None):
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    parser.add_argument("--root", type=Path, default=REPO)
    findings, locales, keys = census(parser.parse_args(argv).root)
    print("\n".join([*findings, f"examined {locales} locales, {keys} keys"]))
    if not locales:
        return VOID
    return 1 if findings else 0


if __name__ == "__main__":
    sys.exit(main())
