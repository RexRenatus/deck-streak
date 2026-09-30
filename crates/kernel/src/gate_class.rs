//! The gate class: the name of a check an output gate runs, and the class a gate's failure reports
//! (SPEC-043 R9, SPEC-046 R7).
//!
//! It lives in the kernel because the agent's gate reports a failure's class and the readings
//! context ranks and names it, and neither context may depend on the other (docs/CONTEXT-MAP.md).
//!
//! The type is closed. One declaration below yields every variant, its name and [`GateClass::ALL`],
//! so a class is always a value `ALL` lists, and a test that walks `ALL` walks every class a failure
//! can carry. A name is a `const fn` of a variant that holds no data, so no class carries text read
//! while the engine runs. The declaration takes a doc comment, a variant and a name, and nothing
//! else: an attribute that would leave a variant out of one build does not compile.

/// Declares the classes, their names and `ALL` from one list.
macro_rules! gate_classes {
    ($($(#[doc = $doc:literal])+ $variant:ident => $name:literal,)+) => {
        /// A check an output gate runs, and the class a gate's failure reports.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum GateClass {
            $($(#[doc = $doc])+ $variant,)+
        }

        impl GateClass {
            /// Every class, in the order of the declaration.
            pub const ALL: &'static [Self] = &[$(Self::$variant,)+];

            /// The class's name, as the packs and the probe protocol spell it.
            #[must_use]
            pub const fn name(self) -> &'static str {
                match self {
                    $(Self::$variant => $name,)+
                }
            }
        }
    };
}

gate_classes! {
    /// A gate that could not run: an unproven output is never delivered.
    Void => "void",
    /// A probe report that examined nothing.
    ExaminedNothing => "examined-nothing",
    /// ai-content-safety's `output-links` check.
    OutputLinks => "output-links",
    /// ai-content-safety's `output-invisible` check.
    OutputInvisible => "output-invisible",
    /// ai-content-safety's `output-marked` check.
    OutputMarked => "output-marked",
    /// persona-core's `output-contract` check.
    OutputContract => "output-contract",
    /// persona-core's `no-dates` check.
    NoDates => "no-dates",
    /// persona-core's `scrubber` check.
    Scrubber => "scrubber",
    /// persona-core's `no-human-claim` check.
    NoHumanClaim => "no-human-claim",
    /// persona-core's `memory-scope` check.
    MemoryScope => "memory-scope",
    /// law-professors' `rule-cites-corpus` check.
    RuleCitesCorpus => "rule-cites-corpus",
    /// law-professors' `citations-resolve` check.
    CitationsResolve => "citations-resolve",
    /// law-professors' `quotes-grounded` check.
    QuotesGrounded => "quotes-grounded",
    /// law-professors' `authority-grounded` check.
    AuthorityGrounded => "authority-grounded",
    /// language-mentors' `i1-glosses` check.
    I1Glosses => "i1-glosses",
    /// study-duties' `reading-length` check.
    ReadingLength => "reading-length",
}

/// A name that no gate class carries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnknownGateClass;

impl std::str::FromStr for GateClass {
    type Err = UnknownGateClass;

    /// The class named `name`: text becomes a class only when it is a declared name, exactly.
    fn from_str(name: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .iter()
            .copied()
            .find(|class| class.name() == name)
            .ok_or(UnknownGateClass)
    }
}
