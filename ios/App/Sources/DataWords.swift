import Foundation

/// Every string the data screen shows, in DeckStreak's words (SPEC-516 R8): the view reads these and
/// nothing else, so a locale's table is one file.
enum DataWords {
    /// The screen's title, and the account sheet's entry to it.
    static let title = "Your data"
    /// The JSON copy's section.
    static let jsonTitle = "A copy of your data"
    /// Where the JSON copy is, while this app holds no signed-in session for the service (R5).
    static let jsonElsewhere =
        "This app holds no signed-in session for the service yet, so it cannot fetch the JSON copy "
        + "of your data here. Open DeckStreak's data screen on the web, or send /export to the bot: "
        + "it sends you the same document in the chat."
    /// The device's backups' section.
    static let backupsTitle = "This device's backups"
    /// What the device's backups are, and how many are kept.
    static let backupsAbout =
        "Copies of this device's collection, kept in the app's own folder on this device and never "
        + "sent to the service: the newest three of each kind."
    /// The list with no backup in it.
    static let backupsNone = "This device holds no backup yet."
    /// The button that backs this device up now.
    static let backUpNow = "Back up now"
    /// The button that saves one backup through the system's save dialog.
    static let export = "Export"
    /// While a backup is made.
    static let working = "Working…"
    /// A backup was made.
    static let made = "A backup of this device's collection was made."
    /// The save dialog saved the backup.
    static let saved = "The backup was saved."
    /// The save dialog saved nothing.
    static let unsaved = "Nothing was saved."
    /// The engine or the device's storage refused.
    static let refused = "Nothing was done: the engine or this device's storage refused. Try again."
    /// The server's backup's section.
    static let serverTitle = "The server's backup"
    /// The server's backup age, unknown while this app holds no signed-in session (R6).
    static let serverUnknown =
        "The server's backup age is unknown: this app holds no signed-in session for the service yet."

    /// One backup's line: whose collection it holds, and how long ago it was saved.
    static func line(_ copy: DeviceCopy) -> String {
        let age = RelativeDateTimeFormatter().localizedString(
            fromTimeInterval: -TimeInterval(copy.ageSeconds))
        let whose = copy.serverCopy ? "The server's collection" : "This device's collection"
        return "\(whose), saved \(age)"
    }

    /// The name the save dialog offers for `copy`.
    static func filename(_ copy: DeviceCopy) -> String {
        "deck-streak-\(copy.id).anki2"
    }
}
