#!/usr/bin/env python3
"""locale_census: a stub that reads the web catalogs and refuses nothing (SPEC-571, ADR-590)."""

import argparse
import json
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
SETTINGS = "web/app/project.inlang/settings.json"


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=REPO)
    root = parser.parse_args(argv).root
    path = root / SETTINGS
    settings = json.loads(path.read_text(encoding="utf-8")) if path.is_file() else {}
    catalogs = {}
    for locale in settings.get("locales", []):
        catalog = path.parent.parent / "messages" / f"{locale}.json"
        if catalog.is_file():
            try:
                read = json.loads(catalog.read_text(encoding="utf-8"))
            except ValueError:
                continue
            if isinstance(read, dict):
                catalogs[locale] = read
    keys = [key for key in catalogs.get("en", {}) if key != "$schema"]
    print(f"examined {len(catalogs)} locales, {len(keys)} keys")
    return 0


if __name__ == "__main__":
    sys.exit(main())
