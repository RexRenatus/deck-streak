"""SPEC-498's registrations (ADR-516).

Red-first stub: the corpus is loaded and nothing is registered yet.
"""

import importlib.util
import sys
from pathlib import Path

_CORPUS_PATH = Path(__file__).resolve().parent.parent / "corpus.py"
_spec = importlib.util.spec_from_file_location("anki_corpus", _CORPUS_PATH)
corpus = importlib.util.module_from_spec(_spec)
sys.modules["anki_corpus"] = corpus
_spec.loader.exec_module(corpus)

CORPUS = corpus
CORPUS_SHA256 = ""
SEED = corpus.SEED
FUNCTIONS: dict = {}
