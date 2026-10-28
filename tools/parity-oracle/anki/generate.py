#!/usr/bin/env python3
"""Write the second old system's goldens from one run of a pinned harness (SPEC-498, ADR-516).

Red-first stub: every name the tests reach exists, and none of the behaviour does.
"""

from __future__ import annotations

import hashlib
import importlib.util
import random
import sys
from pathlib import Path

HERE = Path(__file__).resolve()
REGISTRY = HERE.parent / "registry"
SCHEMA = "phx.parity-golden.v1"
GENERATOR = "tools/parity-oracle/anki/generate.py"
PUBLISHED_REGISTRY = "tools/parity-oracle/anki/registry"
ENGINE_COMMIT = "65e053b829825f534803b2b772c507a73ba936a3"
HARNESS_PACKAGE = "parity-oracle-anki-harness"
HARNESS_PATHS = ("tools/parity-oracle/anki/harness", "tools/render-oracle")
HARNESS_SOURCE_SHA256 = None


class Refused(Exception):
    """Refused before the harness starts (exit 2)."""


class RunFailed(Exception):
    """The run failed after the harness started (exit 3)."""


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def tree_digest(root):
    return hashlib.sha256(b"").hexdigest()


def load_module(path):
    name = f"anki_registry_{path.stem}"
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


def load_registry(directory):
    registered = {}
    for path in sorted(Path(directory).glob("*.py")):
        for name, registration in getattr(load_module(path), "FUNCTIONS", {}).items():
            registered[name] = (path, registration)
    return registered


def covered(name, registration, seed):
    return registration["cases"](random.Random(seed))


def writable_refusals(paths, writable=None):
    return []


def stock_hashes(texts):
    return set()


def census_refusals(name, golden, hashes):
    return []


def strict_refusals(name, golden):
    return []


def provenance_refusals(name, golden, root):
    return []


def workspace_refusals(root):
    return []


def run(harness, registered, seed_of, generator_digest):
    return {}, set()


def main(argv=None, pin=None):
    return 0


if __name__ == "__main__":
    sys.exit(main())
