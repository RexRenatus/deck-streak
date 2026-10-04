//! The synthetic notes `seed` writes into an empty collection, for the browser tests and the
//! measurements (SPEC-338 R12): the same on every target, so the native tests judge exactly the
//! fields the `wasm32` module stores.
//!
//! Each note has the shape of ADR-022's measured collection, two fields of 200 characters of
//! seeded words, and its front starts with its own number, so no two notes share a front.

/// The characters in each of a note's two fields (ADR-022).
const FIELD_CHARACTERS: usize = 200;

/// The syllables the fields' words are drawn from, as the ingest spike's collection draws them.
const SYLLABLES: [&str; 16] = [
    "ka", "lo", "mi", "ne", "ru", "sa", "te", "vi", "zo", "pa", "de", "fu", "gi", "ho", "ja", "be",
];

/// `SplitMix64`, seeded by the note's number, so a note's fields are the same on every run and
/// every target.
struct Draw(u64);

impl Draw {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn syllable(&mut self) -> &'static str {
        let at = usize::try_from(self.next() % 16).unwrap_or(0);
        SYLLABLES[at]
    }
}

/// A field of exactly [`FIELD_CHARACTERS`] ASCII characters: its label and the note's number, then
/// words of one to three syllables.
fn field(label: &str, number: u32, draw: &mut Draw) -> String {
    let mut text = format!("{label} {number}");
    while text.len() < FIELD_CHARACTERS {
        text.push(' ');
        for _ in 0..=draw.next() % 3 {
            text.push_str(draw.syllable());
        }
    }
    text.truncate(FIELD_CHARACTERS);
    text
}

/// The two fields of synthetic note `number`: its front and its back.
#[must_use]
pub fn fields(number: u32) -> [String; 2] {
    let mut draw = Draw(u64::from(number));
    let front = field("front", number, &mut draw);
    let back = field("back", number, &mut draw);
    [front, back]
}
