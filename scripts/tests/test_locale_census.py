"""The web app's message catalogs are whole in all seven locales (SPEC-571 A1 to A11; ADR-590).

`scripts/locale_census.py` reads `web/app/project.inlang/settings.json` and the catalogs it names,
and prints one line per finding, then `examined <n> locales, <m> keys`; it exits 0 when green, 1 on
a finding and 3 when it examined no catalog.

`TheWebCensus` runs it over trees each test plants in a temporary directory: the settings file and
seven catalogs of four messages each, one with two placeholders, one with a composed accented
letter, one in each Chinese script. Each refusal is planted into that complete tree and asserted by
the census's exact lines and exit code, so a census that refused nothing, or that crashed, reads red.
The complete tree reads green as the control. `TheWebTree` runs it over this repository, by its
default root, and asserts no finding and an examined key count above 0, never a pinned count, since
other changes add messages."""

import copy
import json
import re
import subprocess
import sys
import tempfile
import unicodedata
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
CENSUS = REPO / "scripts" / "locale_census.py"
SETTINGS = "web/app/project.inlang/settings.json"
MESSAGES = "web/app/messages"
LISTED = ["en", "zh-Hans", "zh-Hant", "ja", "ko", "fr", "es"]
EXAMINED = re.compile(r"examined 7 locales, (\d+) keys")
#: The planted catalogs: four messages each, in English's order; `progress` holds two placeholders,
#: `summary` a composed accented letter in English and French, `title` each Chinese script.
PLANTED = {
    "en": {
        "title": "Study deck",
        "progress": "{done} of {total} cards",
        "summary": "Résumé",
        "close": "Close",
    },
    "es": {
        "title": "Mazo de estudio",
        "progress": "{done} de {total} tarjetas",
        "summary": "Reseña",
        "close": "Cerrar",
    },
    "fr": {
        "title": "Paquet d’étude",
        "progress": "{done} sur {total} cartes",
        "summary": "Résumé",
        "close": "Fermer",
    },
    "ja": {
        "title": "学習デッキ",
        "progress": "{total} 枚中 {done} 枚",
        "summary": "概要",
        "close": "閉じる",
    },
    "ko": {
        "title": "학습 덱",
        "progress": "{total}장 중 {done}장",
        "summary": "요약",
        "close": "닫기",
    },
    "zh-Hans": {
        "title": "学习卡组",
        "progress": "{total} 张中的 {done} 张",
        "summary": "摘要",
        "close": "关闭",
    },
    "zh-Hant": {
        "title": "學習卡組",
        "progress": "{total} 張中的 {done} 張",
        "summary": "摘要",
        "close": "關閉",
    },
}
GREEN = "examined 7 locales, 4 keys"


def census(root=None):
    """One run of the census, over `root` or by its default root: (exit code, stdout lines,
    stderr)."""
    argv = [sys.executable, str(CENSUS)]
    if root is not None:
        argv += ["--root", str(root)]
    done = subprocess.run(argv, capture_output=True, encoding="utf-8", check=False)
    return done.returncode, done.stdout.splitlines(), done.stderr


def settings_text(locales=None, base="en", pattern="./messages/{locale}.json"):
    """The project settings, shaped as the web app's own."""
    settings = {
        "$schema": "https://inlang.com/schema/project-settings",
        "plugin.inlang.messageFormat": {"pathPattern": pattern},
        "baseLocale": base,
        "locales": LISTED if locales is None else locales,
    }
    return json.dumps(settings, ensure_ascii=False, indent=2) + "\n"


def catalog_text(messages):
    """A catalog as the web app writes one: its `$schema` member, then its messages."""
    catalog = {"$schema": "https://inlang.com/schema/inlang-message-format", **messages}
    return json.dumps(catalog, ensure_ascii=False, indent=2) + "\n"


class Tree:
    """A planted tree in a temporary directory: the settings and one catalog per locale."""

    def __init__(self, case, catalogs=None, settings=None):
        holder = tempfile.TemporaryDirectory()
        case.addCleanup(holder.cleanup)
        self.root = Path(holder.name)
        if settings is not False:
            self.write(SETTINGS, settings_text() if settings is None else settings)
        for locale, messages in (PLANTED if catalogs is None else catalogs).items():
            self.write(f"{MESSAGES}/{locale}.json", catalog_text(messages))

    def write(self, relative, text):
        path = self.root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")

    def drop(self, relative):
        (self.root / relative).unlink()


def planted(**changes):
    """The planted catalogs with each `locale={key: value}` change applied; a value of None removes
    the key."""
    catalogs = copy.deepcopy(PLANTED)
    for locale, edits in changes.items():
        locale = locale.replace("_", "-")
        for key, value in edits.items():
            if value is None:
                del catalogs[locale][key]
            else:
                catalogs[locale][key] = value
    return catalogs


class TheWebCensus(unittest.TestCase):
    maxDiff = None

    def assertCensus(self, tree, code, lines):
        """The census over `tree` exits `code` and prints exactly `lines`, and nothing on stderr."""
        self.assertEqual(census(tree.root), (code, lines, ""))

    def test_a_key_removed_from_one_locale_is_refused(self):
        """A1"""
        cases = {
            "one key": (planted(zh_Hans={"summary": None}), ["missing zh-Hans summary"]),
            "two keys, in English's order": (
                planted(ja={"close": None, "title": None}),
                ["missing ja title", "missing ja close"],
            ),
        }
        for name, (catalogs, lines) in cases.items():
            with self.subTest(name):
                self.assertCensus(Tree(self, catalogs), 1, [*lines, GREEN])

    def test_a_blank_value_is_refused_in_any_locale(self):
        """A2"""
        cases = {
            "empty": (planted(fr={"title": ""}), ["empty fr title"]),
            "spaces": (planted(es={"summary": "   "}), ["empty es summary"]),
            "an ideographic space": (planted(ja={"close": chr(0x3000)}), ["empty ja close"]),
            "English": (planted(en={"close": ""}), ["empty en close"]),
        }
        for name, (catalogs, lines) in cases.items():
            with self.subTest(name):
                self.assertCensus(Tree(self, catalogs), 1, [*lines, GREEN])

    def test_a_placeholder_that_differs_from_english_is_refused(self):
        """A3"""
        cases = {
            "renamed": (
                planted(fr={"progress": "{done} sur {totals} cartes"}),
                ["placeholder fr progress"],
            ),
            "dropped": (planted(ko={"progress": "{done}장"}), ["placeholder ko progress"]),
            "added": (
                planted(zh_Hant={"close": "關閉 {name}"}),
                ["placeholder zh-Hant close"],
            ),
            "counted twice": (
                planted(es={"progress": "{done} de {total} tarjetas, {total}"}),
                ["placeholder es progress"],
            ),
        }
        for name, (catalogs, lines) in cases.items():
            with self.subTest(name):
                self.assertCensus(Tree(self, catalogs), 1, [*lines, GREEN])
        with self.subTest("reordered"):
            reordered = planted(fr={"progress": "{total} : {done} cartes"})
            self.assertCensus(Tree(self, reordered), 0, [GREEN])

    def test_a_key_english_lacks_is_refused_as_stale(self):
        """A4"""
        cases = {
            "one": (planted(ko={"extra": "추가"}), ["stale ko extra"]),
            "two": (
                planted(es={"older": "Antiguo"}, zh_Hans={"newer": "较新"}),
                ["stale es older", "stale zh-Hans newer"],
            ),
        }
        for name, (catalogs, lines) in cases.items():
            with self.subTest(name):
                self.assertCensus(Tree(self, catalogs), 1, [*lines, GREEN])

    def test_a_value_out_of_nfc_is_refused(self):
        """A5"""
        cases = {
            "a decomposed accent": ("fr", "summary"),
            "decomposed Hangul": ("ko", "title"),
            "English": ("en", "summary"),
        }
        for name, (locale, key) in cases.items():
            with self.subTest(name):
                decomposed = unicodedata.normalize("NFD", PLANTED[locale][key])
                self.assertNotEqual(decomposed, PLANTED[locale][key])
                catalogs = planted(**{locale: {key: decomposed}})
                self.assertCensus(Tree(self, catalogs), 1, [f"not-nfc {locale} {key}", GREEN])

    def test_a_locale_set_other_than_the_seven_is_refused(self):
        """A6"""
        six = [locale for locale in LISTED if locale != "ko"]
        without_ko = {k: v for k, v in PLANTED.items() if k != "ko"}
        with self.subTest("six locales"):
            tree = Tree(self, without_ko, settings_text(locales=six))
            self.assertCensus(
                tree,
                1,
                [
                    'locale-set locales ["en", "zh-Hans", "zh-Hant", "ja", "fr", "es"]',
                    "examined 6 locales, 4 keys",
                ],
            )
        with self.subTest("an eighth locale"):
            eight = {**PLANTED, "de": PLANTED["en"]}
            tree = Tree(self, eight, settings_text(locales=[*LISTED, "de"]))
            self.assertCensus(
                tree,
                1,
                [
                    'locale-set locales ["en", "zh-Hans", "zh-Hant", "ja", "ko", "fr", "es", "de"]',
                    "examined 8 locales, 4 keys",
                ],
            )
        with self.subTest("another base"):
            tree = Tree(self, settings=settings_text(base="fr"))
            self.assertCensus(tree, 1, ['locale-set base "fr"', GREEN])
        with self.subTest("another pattern"):
            tree = Tree(self, settings=settings_text(pattern="./i18n/{locale}.json"))
            self.assertCensus(tree, 1, ['locale-set pattern "./i18n/{locale}.json"', GREEN])

    def test_an_unreadable_or_repeating_catalog_is_refused(self):
        """A7"""
        six = "examined 6 locales, 4 keys"
        with self.subTest("a missing catalog"):
            tree = Tree(self)
            tree.drop(f"{MESSAGES}/fr.json")
            self.assertCensus(tree, 1, ["file-missing fr", six])
        with self.subTest("a stray catalog"):
            tree = Tree(self)
            tree.write(f"{MESSAGES}/de.json", catalog_text(PLANTED["en"]))
            self.assertCensus(tree, 1, ["file-stray de.json", GREEN])
        with self.subTest("a catalog that is not an object"):
            tree = Tree(self)
            tree.write(f"{MESSAGES}/ja.json", '["not", "an", "object"]\n')
            self.assertCensus(tree, 1, ["unreadable ja not-an-object", six])
        with self.subTest("a catalog that is not JSON"):
            tree = Tree(self)
            tree.write(f"{MESSAGES}/ko.json", '{\n  "title": "학습 덱",\n')
            self.assertCensus(tree, 1, ["unreadable ko not-json", six])
        with self.subTest("a repeated key"):
            tree = Tree(self)
            text = catalog_text(PLANTED["es"]).replace(
                '  "close": "Cerrar"\n', '  "close": "Cerrar",\n  "close": "Cerrar"\n'
            )
            self.assertEqual(text.count('"close"'), 2)
            tree.write(f"{MESSAGES}/es.json", text)
            self.assertCensus(tree, 1, ["unreadable es repeats close", six])

    def test_a_value_that_is_not_text_is_refused(self):
        """A8"""
        cases = {
            "a number": (planted(fr={"close": 5}), ["not-text fr close"]),
            "variants": (
                planted(zh_Hant={"title": {"one": "學習卡組"}}),
                ["not-text zh-Hant title"],
            ),
            "a list in English": (
                planted(en={"progress": ["{done}", "{total}"]}),
                ["not-text en progress"],
            ),
        }
        for name, (catalogs, lines) in cases.items():
            with self.subTest(name):
                self.assertCensus(Tree(self, catalogs), 1, [*lines, GREEN])
        with self.subTest("a JSON null"):
            tree = Tree(self)
            tree.write(
                f"{MESSAGES}/es.json",
                catalog_text(PLANTED["es"]).replace('"Reseña"', "null"),
            )
            self.assertCensus(tree, 1, ["not-text es summary", GREEN])

    def test_a_planted_complete_tree_reads_green(self):
        """A9 (the control)"""
        self.assertCensus(Tree(self), 0, [GREEN])

    def test_a_tree_with_no_catalog_is_void(self):
        """A10"""
        with self.subTest("the settings and no catalog"):
            tree = Tree(self, catalogs={})
            missing = [f"file-missing {locale}" for locale in sorted(LISTED)]
            self.assertCensus(tree, 3, [*missing, "examined 0 locales, 0 keys"])
        with self.subTest("an empty tree"):
            tree = Tree(self, catalogs={}, settings=False)
            self.assertCensus(
                tree,
                3,
                [
                    "locale-set locales []",
                    "locale-set base null",
                    "locale-set pattern null",
                    "examined 0 locales, 0 keys",
                ],
            )


class TheWebTree(unittest.TestCase):
    def test_the_web_apps_catalogs_are_whole(self):
        """A11"""
        code, lines, err = census()
        self.assertEqual(err, "")
        self.assertGreater(len(lines), 0, "the census printed nothing")
        *findings, last = lines
        print(last)
        self.assertEqual(
            findings, [], f"{len(findings)} finding lines, the first three: {findings[:3]}"
        )
        self.assertEqual(code, 0)
        counted = EXAMINED.fullmatch(last)
        self.assertIsNotNone(counted, last)
        self.assertGreater(int(counted.group(1)), 0)


if __name__ == "__main__":
    unittest.main()
