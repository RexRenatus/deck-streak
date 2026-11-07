import SwiftUI

/// The six Tide colours (SPEC-593 R11, ADR-615 D4): each member reads its colour set from the
/// app's asset catalog by name. A set holds a light and a dark entry and the system picks the one
/// for the appearance at render time, so no code reads the colour scheme and the app follows the
/// system appearance (ADR-616 D2). Every view reads its colours through these members alone.
extension Color {
    /// The root of a screen.
    static var tideBackground: Color { Color("TideBackground") }
    /// Text.
    static var tideInk: Color { Color("TideInk") }
    /// The tint of the controls.
    static var tideAccent: Color { Color("TideAccent") }
    /// A panel over the root.
    static var tideCard: Color { Color("TideCard") }
    /// Text that matters less.
    static var tideMuted: Color { Color("TideMuted") }
    /// A rule, the edge of a panel.
    static var tideBorder: Color { Color("TideBorder") }
}
