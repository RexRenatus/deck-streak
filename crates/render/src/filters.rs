//! The filters a replacement applies to its field's text (SPEC-603 R6, ADR-624 D3): `text`,
//! `hint`, the `type` marker, `furigana`, `kana`, `kanji` and `tts`. A filter this delivery does
//! not know, the cloze filters among them, leaves the text as it is.

/// Escapes `text` for HTML: `&`, `<`, `>` and `"`.
pub(crate) fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            other => out.push(other),
        }
    }
    out
}

/// Applies `filters`, the nearest the name first, to the field `name` whose text is `value`.
///
/// A `type` filter applied last marks where the typed answer goes and drops the field's text; a
/// `cloze` or `nc` filter just before it is kept in the marker.
pub(crate) fn apply(name: &str, value: &str, filters: &[String]) -> String {
    if filters.last().is_some_and(|last| last == "type") {
        let kind = match filters {
            [first, _] if first == "cloze" || first == "nc" => format!("{first}:"),
            _ => String::new(),
        };
        return format!("[[type:{kind}{name}]]");
    }
    let mut text = value.to_owned();
    for filter in filters {
        text = match filter.as_str() {
            "text" => strip_html(&text),
            "hint" => hint(name, text),
            "furigana" => ruby(&text, Ruby::Furigana),
            "kana" => ruby(&text, Ruby::Kana),
            "kanji" => ruby(&text, Ruby::Kanji),
            other => match other.strip_prefix("tts ") {
                Some(options) => format!("[anki:tts lang={options}]{text}[/anki:tts]"),
                None => text,
            },
        };
    }
    text
}

/// The hint filter: the text behind a `details` element whose summary is the field's name, or
/// the text as it is when it holds nothing but whitespace.
fn hint(name: &str, text: String) -> String {
    if text.trim().is_empty() {
        return text;
    }
    format!(
        "<details class=\"hint\"><summary>{}</summary>{text}</details>",
        escape(name)
    )
}

/// The text filter: tags, comments, styles and scripts removed, then entities decoded.
pub(crate) fn strip_html(html: &str) -> String {
    decode_entities(&remove_markup(html))
}

/// `html` with every comment, style element, script element and tag removed.
fn remove_markup(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    while let Some(open) = rest.find('<') {
        out.push_str(rest.get(..open).unwrap_or_default());
        let from = rest.get(open..).unwrap_or_default();
        if let Some(length) = markup_length(from) {
            rest = from.get(length..).unwrap_or_default();
        } else {
            out.push('<');
            rest = from.get(1..).unwrap_or_default();
        }
    }
    out.push_str(rest);
    out
}

/// The length of the comment, style or script element, or tag that starts `from`, if one does.
fn markup_length(from: &str) -> Option<usize> {
    let comment = from
        .strip_prefix("<!--")
        .and_then(|inside| inside.find("-->"))
        .map(|end| end + 7);
    comment
        .or_else(|| element_length(from, "style"))
        .or_else(|| element_length(from, "script"))
        .or_else(|| from.find('>').map(|end| end + 1))
}

/// The length of the element `name` (any case) that starts `from`, up to its first closing tag.
fn element_length(from: &str, name: &str) -> Option<usize> {
    let after = from.get(1 + name.len()..)?;
    if !from.get(1..=name.len())?.eq_ignore_ascii_case(name) {
        return None;
    }
    let body = after.find('>')?;
    let closer = format!("</{name}>");
    let close = body + find_ascii_case(after.get(body..)?, &closer)?;
    Some(1 + name.len() + close + closer.len())
}

/// The first byte where `needle`, ASCII letters in any case, occurs in `text`.
fn find_ascii_case(text: &str, needle: &str) -> Option<usize> {
    text.as_bytes()
        .windows(needle.len())
        .position(|window| window.eq_ignore_ascii_case(needle.as_bytes()))
}

/// `text` with its character references decoded, all or none: when one is malformed or unknown,
/// the text is returned as it is. A decoded text shows every no-break space as a space.
pub(crate) fn decode_entities(text: &str) -> String {
    if !text.contains('&') {
        return text.to_owned();
    }
    match decode_all(text) {
        Some(decoded) => decoded.replace('\u{a0}', " "),
        None => text.to_owned(),
    }
}

/// Every reference in `text` decoded, or nothing when any one cannot be.
fn decode_all(text: &str) -> Option<String> {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(amp) = rest.find('&') {
        out.push_str(rest.get(..amp)?);
        let after = rest.get(amp + 1..)?;
        let semicolon = after.find(';')?;
        out.push(reference(after.get(..semicolon)?)?);
        rest = after.get(semicolon + 1..)?;
    }
    out.push_str(rest);
    Some(out)
}

/// The character a reference's body (between `&` and `;`) names.
fn reference(body: &str) -> Option<char> {
    let Some(number) = body.strip_prefix('#') else {
        return ENTITIES
            .iter()
            .find(|(entity, _)| *entity == body)
            .and_then(|&(_, code)| char::from_u32(code));
    };
    let (digits, radix) = match number.strip_prefix('x') {
        Some(hex) => (hex, 16),
        None => (number, 10),
    };
    if digits.is_empty() || !digits.chars().all(|c| c.is_digit(radix)) {
        return None;
    }
    char::from_u32(u32::from_str_radix(digits, radix).ok()?)
}

/// Which part of a reading the ruby filters keep.
#[derive(Clone, Copy)]
enum Ruby {
    /// The base text with its reading above it.
    Furigana,
    /// The reading alone.
    Kana,
    /// The base text alone.
    Kanji,
}

/// One `base[reading]` the ruby filters found.
struct Annotated<'a> {
    /// The whole match, with its leading space if it had one.
    whole: &'a str,
    /// The base text.
    base: &'a str,
    /// The reading, between the brackets.
    reading: &'a str,
}

/// The ruby filters: every `base[reading]`, after an optional space, drawn as `form` asks. A
/// reading that is a sound reference is kept as written.
fn ruby(text: &str, form: Ruby) -> String {
    let text = text.replace("&nbsp;", " ");
    let mut out = String::with_capacity(text.len());
    let mut rest = text.as_str();
    while let Some(first) = rest.chars().next() {
        let Some(found) = reading_at(rest) else {
            out.push(first);
            rest = rest.get(first.len_utf8()..).unwrap_or_default();
            continue;
        };
        if found.reading.starts_with("sound:") {
            out.push_str(found.whole);
        } else {
            match form {
                Ruby::Furigana => {
                    out.push_str("<ruby><rb>");
                    out.push_str(found.base);
                    out.push_str("</rb><rt>");
                    out.push_str(found.reading);
                    out.push_str("</rt></ruby>");
                }
                Ruby::Kana => out.push_str(found.reading),
                Ruby::Kanji => out.push_str(found.base),
            }
        }
        rest = rest.get(found.whole.len()..).unwrap_or_default();
    }
    out
}

/// The reading that starts `text`, if one does: an optional space, the shortest base of one or
/// more characters other than a space or `>` that a bracketed reading follows, and the reading,
/// one or more characters on one line, up to the first `]`.
fn reading_at(text: &str) -> Option<Annotated<'_>> {
    let lead = usize::from(text.starts_with(' '));
    let body = text.get(lead..)?;
    for (open, c) in body.char_indices() {
        if c == ' ' || c == '>' {
            return None;
        }
        if c != '[' || open == 0 {
            continue;
        }
        let after = body.get(open + 1..)?;
        if let Some(length) = bracketed(after) {
            return Some(Annotated {
                whole: text.get(..lead + open + 1 + length + 1)?,
                base: body.get(..open)?,
                reading: after.get(..length)?,
            });
        }
    }
    None
}

/// The length of a reading that `after` (the text after a `[`) holds: at least one character,
/// none a line break, up to the first `]` after the first character.
fn bracketed(after: &str) -> Option<usize> {
    let line = after.split('\n').next()?;
    let first = line.chars().next()?.len_utf8();
    line.get(first..)?.find(']').map(|end| first + end)
}

/// The named character references of HTML 4, each with its code point.
const ENTITIES: &[(&str, u32)] = &[
    ("quot", 34),
    ("amp", 38),
    ("lt", 60),
    ("gt", 62),
    ("nbsp", 160),
    ("iexcl", 161),
    ("cent", 162),
    ("pound", 163),
    ("curren", 164),
    ("yen", 165),
    ("brvbar", 166),
    ("sect", 167),
    ("uml", 168),
    ("copy", 169),
    ("ordf", 170),
    ("laquo", 171),
    ("not", 172),
    ("shy", 173),
    ("reg", 174),
    ("macr", 175),
    ("deg", 176),
    ("plusmn", 177),
    ("sup2", 178),
    ("sup3", 179),
    ("acute", 180),
    ("micro", 181),
    ("para", 182),
    ("middot", 183),
    ("cedil", 184),
    ("sup1", 185),
    ("ordm", 186),
    ("raquo", 187),
    ("frac14", 188),
    ("frac12", 189),
    ("frac34", 190),
    ("iquest", 191),
    ("Agrave", 192),
    ("Aacute", 193),
    ("Acirc", 194),
    ("Atilde", 195),
    ("Auml", 196),
    ("Aring", 197),
    ("AElig", 198),
    ("Ccedil", 199),
    ("Egrave", 200),
    ("Eacute", 201),
    ("Ecirc", 202),
    ("Euml", 203),
    ("Igrave", 204),
    ("Iacute", 205),
    ("Icirc", 206),
    ("Iuml", 207),
    ("ETH", 208),
    ("Ntilde", 209),
    ("Ograve", 210),
    ("Oacute", 211),
    ("Ocirc", 212),
    ("Otilde", 213),
    ("Ouml", 214),
    ("times", 215),
    ("Oslash", 216),
    ("Ugrave", 217),
    ("Uacute", 218),
    ("Ucirc", 219),
    ("Uuml", 220),
    ("Yacute", 221),
    ("THORN", 222),
    ("szlig", 223),
    ("agrave", 224),
    ("aacute", 225),
    ("acirc", 226),
    ("atilde", 227),
    ("auml", 228),
    ("aring", 229),
    ("aelig", 230),
    ("ccedil", 231),
    ("egrave", 232),
    ("eacute", 233),
    ("ecirc", 234),
    ("euml", 235),
    ("igrave", 236),
    ("iacute", 237),
    ("icirc", 238),
    ("iuml", 239),
    ("eth", 240),
    ("ntilde", 241),
    ("ograve", 242),
    ("oacute", 243),
    ("ocirc", 244),
    ("otilde", 245),
    ("ouml", 246),
    ("divide", 247),
    ("oslash", 248),
    ("ugrave", 249),
    ("uacute", 250),
    ("ucirc", 251),
    ("uuml", 252),
    ("yacute", 253),
    ("thorn", 254),
    ("yuml", 255),
    ("OElig", 338),
    ("oelig", 339),
    ("Scaron", 352),
    ("scaron", 353),
    ("Yuml", 376),
    ("fnof", 402),
    ("circ", 710),
    ("tilde", 732),
    ("Alpha", 913),
    ("Beta", 914),
    ("Gamma", 915),
    ("Delta", 916),
    ("Epsilon", 917),
    ("Zeta", 918),
    ("Eta", 919),
    ("Theta", 920),
    ("Iota", 921),
    ("Kappa", 922),
    ("Lambda", 923),
    ("Mu", 924),
    ("Nu", 925),
    ("Xi", 926),
    ("Omicron", 927),
    ("Pi", 928),
    ("Rho", 929),
    ("Sigma", 931),
    ("Tau", 932),
    ("Upsilon", 933),
    ("Phi", 934),
    ("Chi", 935),
    ("Psi", 936),
    ("Omega", 937),
    ("alpha", 945),
    ("beta", 946),
    ("gamma", 947),
    ("delta", 948),
    ("epsilon", 949),
    ("zeta", 950),
    ("eta", 951),
    ("theta", 952),
    ("iota", 953),
    ("kappa", 954),
    ("lambda", 955),
    ("mu", 956),
    ("nu", 957),
    ("xi", 958),
    ("omicron", 959),
    ("pi", 960),
    ("rho", 961),
    ("sigmaf", 962),
    ("sigma", 963),
    ("tau", 964),
    ("upsilon", 965),
    ("phi", 966),
    ("chi", 967),
    ("psi", 968),
    ("omega", 969),
    ("thetasym", 977),
    ("upsih", 978),
    ("piv", 982),
    ("ensp", 8194),
    ("emsp", 8195),
    ("thinsp", 8201),
    ("zwnj", 8204),
    ("zwj", 8205),
    ("lrm", 8206),
    ("rlm", 8207),
    ("ndash", 8211),
    ("mdash", 8212),
    ("lsquo", 8216),
    ("rsquo", 8217),
    ("sbquo", 8218),
    ("ldquo", 8220),
    ("rdquo", 8221),
    ("bdquo", 8222),
    ("dagger", 8224),
    ("Dagger", 8225),
    ("bull", 8226),
    ("hellip", 8230),
    ("permil", 8240),
    ("prime", 8242),
    ("Prime", 8243),
    ("lsaquo", 8249),
    ("rsaquo", 8250),
    ("oline", 8254),
    ("frasl", 8260),
    ("euro", 8364),
    ("image", 8465),
    ("weierp", 8472),
    ("real", 8476),
    ("trade", 8482),
    ("alefsym", 8501),
    ("larr", 8592),
    ("uarr", 8593),
    ("rarr", 8594),
    ("darr", 8595),
    ("harr", 8596),
    ("crarr", 8629),
    ("lArr", 8656),
    ("uArr", 8657),
    ("rArr", 8658),
    ("dArr", 8659),
    ("hArr", 8660),
    ("forall", 8704),
    ("part", 8706),
    ("exist", 8707),
    ("empty", 8709),
    ("nabla", 8711),
    ("isin", 8712),
    ("notin", 8713),
    ("ni", 8715),
    ("prod", 8719),
    ("sum", 8721),
    ("minus", 8722),
    ("lowast", 8727),
    ("radic", 8730),
    ("prop", 8733),
    ("infin", 8734),
    ("ang", 8736),
    ("and", 8743),
    ("or", 8744),
    ("cap", 8745),
    ("cup", 8746),
    ("int", 8747),
    ("there4", 8756),
    ("sim", 8764),
    ("cong", 8773),
    ("asymp", 8776),
    ("ne", 8800),
    ("equiv", 8801),
    ("le", 8804),
    ("ge", 8805),
    ("sub", 8834),
    ("sup", 8835),
    ("nsub", 8836),
    ("sube", 8838),
    ("supe", 8839),
    ("oplus", 8853),
    ("otimes", 8855),
    ("perp", 8869),
    ("sdot", 8901),
    ("lceil", 8968),
    ("rceil", 8969),
    ("lfloor", 8970),
    ("rfloor", 8971),
    ("lang", 9001),
    ("rang", 9002),
    ("loz", 9674),
    ("spades", 9824),
    ("clubs", 9827),
    ("hearts", 9829),
    ("diams", 9830),
];
