"""SPEC-046's registrations: a note's anchor and whether the anchor is long enough to verify.

`preread.py:anchor_for_note` reads no deck constant, and neither does `preread.py:is_anchor_usable`,
so both are plain functions' goldens. The cases are synthetic note texts covering the classes a port
gets wrong: markup with and without a closing bracket, named, numeric and legacy semicolonless
entities, full-width and compatibility characters that fold to a rail character after NFKC, the
rail's own characters, a tilde run, a `javascript:` token in mixed case, the whitespace Python's
`split` reads beyond a space, text longer than the anchor and text shorter than a usable one, and
code points outside the basic plane, where a slice counts code points and not bytes.

The case builders draw only from the `random.Random` the generator seeds.
"""

#: The longest an anchor is; a case is built around this bound and the usable one.
ANCHOR_MAX = 48
#: The shortest a usable anchor is.
ANCHOR_MIN = 8

FIXED_NOTES = (
    ("plain", "A bonus for speed"),
    ("plain", "Relevance is a low bar"),
    ("empty", ""),
    ("blank", "   \t\n  "),
    ("markup", "<b>Bold</b> and <i>slanted</i> text of a note"),
    ("markup", "<div class=\"x\">Inside a div</div><br/>after the break"),
    ("markup", "text with a lone < bracket and more words after it"),
    ("markup", "text with an unclosed <span tag that never ends"),
    ("markup", "<>empty tag then a long enough remainder of text"),
    ("markup", "a <tag> b <tag> c <tag> d <tag> e <tag> f <tag> g <tag> h"),
    ("entity", "Tom &amp; Jerry &lt;b&gt;bold&lt;/b&gt; &quot;quoted&quot; &#39;single&#39;"),
    ("entity", "numeric &#65;&#x42;&#x63; and hex &#X44; entities in a note"),
    ("entity", "non-breaking&nbsp;space&nbsp;and&nbsp;more&nbsp;words&nbsp;here"),
    ("legacy", "legacy semicolonless &amp and &lt and &gt and &copy 2024 entities"),
    ("legacy", "café &eacute and &Eacute and &szlig and &frac12 and &nbsp done"),
    ("legacy", "&notit; and &notin; and &not and &ampx are different names"),
    ("legacy", "&#65 and &#x42 numeric without a semicolon then text follows it"),
    ("entity", "unknown &nosuchentity; and &amp;amp; double escaped and &;"),
    ("entity", "an escaped tag &lt;script&gt;alert&lt;/script&gt; decodes then strips"),
    ("fullwidth", "ＦＵＬＬＷＩＤＴＨ　ｔｅｘｔ　１２３"),
    ("fullwidth", "［ｂｒａｃｋｅｔ］　ａｎｄ　｀ｔｉｃｋ｀　ｆｏｌｄ　ｔｏ　ｒａｉｌ"),
    ("fullwidth", "ｊａｖａｓｃｒｉｐｔ：alert and more words after the token"),
    ("compat", "ﬁnal ﬂow ① ② ㎏ ℃ ½ text of a note with ligatures"),
    ("rail", "code `inline` and [link](target) and more text following"),
    ("rail", "a NUL \x00 byte inside the text of a long enough note"),
    ("rail", "tilde ~~~ fence and ~~ pair and ~~~~ run of four inside text"),
    ("rail", "JavaScript:alert(1) and JAVASCRIPT: and javascript : spaced"),
    ("rail", "[[wiki]] ]reversed[ and `` double ticks `` and more text"),
    ("space", "tab\tseparated\tfields\tin\ta\tnote\twith\tmany\tof\tthem"),
    ("space", "line\nbreaks\r\nand\rcarriage\x0bvertical\x0cfeeds\x1cand\x1dseparators\x1e\x1f"),
    ("space", "ideographic　space em thin nbsp narrow line"),
    ("space", "​zero width​ and ﻿bom kept or not by split"),
    ("space", "  leading and trailing   spaces   collapse   "),
    ("boundary", "x" * (ANCHOR_MAX - 1)),
    ("boundary", "x" * ANCHOR_MAX),
    ("boundary", "x" * (ANCHOR_MAX + 1)),
    ("boundary", "x" * 200),
    ("boundary", "y" * (ANCHOR_MIN - 1)),
    ("boundary", "y" * ANCHOR_MIN),
    ("boundary", "z" * (ANCHOR_MIN + 1)),
    ("short", "Hi"),
    ("short", "<b>Short</b>"),
    ("short", "&amp;&amp;&amp;"),
    ("short", "`````"),
    ("astral", "😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀"),
    ("astral", "𝔘𝔫𝔦𝔠𝔬𝔡𝔢 fraktur folds under NFKC to plain letters"),
    ("cjk", "日本語のテキストとその読み方を説明する長い文章がここに入ります。二文目もあります。"),
    ("cjk", "中文文本和它的拼音说明在这里出现，还有更多的字来超过四十八个字符的界限。"),
    ("combining", "é combining accent and å ring folds to composed forms"),
    ("cut", "a" * 45 + "&amp;" + "tail after the cut"),
    ("cut", "b" * 47 + "<i>italic</i>"),
    ("cut", "c" * 46 + "javascript:"),
)

WORDS = (
    "relevance", "hearsay", "privilege", "&amp;", "&lt;", "<b>", "</b>", "`", "[", "]", "~~~",
    "javascript:", "Ａ", "１", "é", "日本", "😀", "&eacute", "&#65;", "&nbsp;", "\t", "\n", " ",
)


def note_cases(rng):
    """The fixed notes, then drawn notes built from words that stress the normaliser."""
    drawn = list(FIXED_NOTES)
    for _ in range(60):
        pieces = [rng.choice(WORDS) for _ in range(rng.randrange(1, 14))]
        joiner = rng.choice((" ", "", "  ", "\n"))
        drawn.append((None, joiner.join(pieces)))
    return [(label, {"text": text}) for label, text in drawn]


def anchor_cases(rng):
    """Anchors at and around the usable bound, then anchors the predecessor derives from notes."""
    drawn = [
        ("empty", ""),
        ("below", "a" * (ANCHOR_MIN - 1)),
        ("bound", "a" * ANCHOR_MIN),
        ("above", "a" * (ANCHOR_MIN + 1)),
        ("full", "a" * ANCHOR_MAX),
        ("astral", "😀" * (ANCHOR_MIN - 1)),
        ("astral", "😀" * ANCHOR_MIN),
        ("wide", "日本語のテキスト"),
        ("wide", "日本語のテキス"),
        ("combining", "é" * 4),
        ("combining", "é" * 3),
        ("space", " " * 12),
    ]
    for _ in range(20):
        drawn.append((None, "".join(rng.choice("ab é日😀") for _ in range(rng.randrange(0, 14)))))
    return [(label, {"anchor": text}) for label, text in drawn]


FUNCTIONS = {
    "anchor_for_note": {
        "kind": "function",
        "function": "preread.anchor_for_note",
        "cases": note_cases,
    },
    "is_anchor_usable": {
        "kind": "function",
        "function": "preread.is_anchor_usable",
        "cases": anchor_cases,
    },
}
