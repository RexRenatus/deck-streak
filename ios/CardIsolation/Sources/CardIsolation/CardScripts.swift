/// The one switch that lets a card's own scripts run on iPhone and iPad (SPEC-355 R1, ADR-366
/// D1). Scripts run only when the switch is on and every control in `required` is read back as
/// present in the view the factory built; otherwise the card view is the scripts-off view, still
/// carrying every control it has.
public enum CardScripts {
    /// The switch. It is defined here and nowhere else under `ios/` (SPEC-355 A5).
    public static let switchedOn = false

    /// The controls a scripted card view must carry, each read back from the view built: every
    /// layer but L2, which is the verdict itself, and L14, the link strip, which is no control.
    public static let required: Set<CardLayer> = [
        .L1,
        .L3,
        .L4,
        .L5,
        .L6,
        .L7,
        .L8,
        .L10,
        .L11,
        .L12,
        .L13,
    ]

    /// What the switch decided for one built view.
    public enum Verdict: Equatable, Sendable {
        /// Page JavaScript on: the switch is on and every required control is present.
        case run
        /// Page JavaScript off, naming the required controls that were missing (none when only
        /// the switch was off).
        case off(missing: Set<CardLayer>)
    }

    /// The decision, pure: `.run` exactly when `switchedOn` and `present` holds every control in
    /// `required`; otherwise `.off`, naming the controls `present` lacks.
    public static func decide(switchedOn: Bool, present: Set<CardLayer>) -> Verdict {
        let missing = required.subtracting(present)
        return switchedOn && missing.isEmpty ? .run : .off(missing: missing)
    }
}
