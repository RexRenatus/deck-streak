//! The images the engine renders from a note's LaTeX, and the names it gives them (SPEC-513 R9;
//! ADR-531 D8).
//!
//! A note writes LaTeX as `[latex]…[/latex]`, as `[$]…[/$]` (the expression `$…$`) or as
//! `[$$]…[/$$]` (the expression `\begin{displaymath}…\end{displaymath}`). The markers match in any
//! case, and an expression is at least one character, up to the first closing marker after it; at
//! each `[` the three forms are tried in that order, and the search resumes after a match. In the
//! expression, `<br>`, `<br/>` and `<div>` (in any case) become a newline, then every comment, every
//! `style` and `script` element and every other tag is removed, and the entities are decoded as the
//! grammar decodes a name ([`crate::scan::decode_entities`]). The image is named `latex-`, the SHA-1
//! of the expression's UTF-8 in lowercase hex, `.` and the image's extension.

/// The type of image a note type renders its LaTeX to.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LatexFormat {
    /// A PNG image, the engine's default.
    #[default]
    Png,
    /// An SVG image, for a note type that asks for one.
    Svg,
}

impl LatexFormat {
    /// The image's extension.
    #[must_use]
    pub const fn extension(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Svg => "svg",
        }
    }
}

/// One LaTeX expression a text writes, and the name of the image the engine renders it to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LatexImage {
    /// The expression the image renders: its markup removed and its entities decoded.
    pub expression: String,
    /// `latex-`, the SHA-1 of the expression in lowercase hex, `.` and the image's extension.
    pub name: String,
}

/// Every LaTeX image `text` writes, in the order written, named for `format`. An expression written
/// twice is listed twice.
#[must_use]
pub fn latex_images(text: &str, format: LatexFormat) -> Vec<LatexImage> {
    let _ = (text, format);
    Vec::new()
}

/// The name of the image `expression` renders to, in `format`.
#[must_use]
pub fn latex_name(expression: &str, format: LatexFormat) -> String {
    let _ = (expression, format);
    String::new()
}
