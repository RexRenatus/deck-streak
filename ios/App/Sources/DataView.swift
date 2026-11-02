import SwiftUI
import UniformTypeIdentifiers

/// The data screen (SPEC-516 R5, R6, R8): where the JSON copy of your data is, this device's
/// backups with back up now and an export of each through the system's save dialog, and the
/// server's backup age. Every string it shows is the words table's (`DataWords`).
struct DataView: View {
    @Bindable var model: DataModel

    var body: some View {
        Form {
            Section(DataWords.jsonTitle) {
                Text(model.jsonCopy).accessibilityIdentifier("data-json")
            }
            Section(DataWords.backupsTitle) {
                Text(DataWords.backupsAbout)
                Button(DataWords.backUpNow) {
                    Task { await model.backUpNow() }
                }
                .accessibilityIdentifier("data-back-up-now")
                .disabled(model.working)
                if model.working {
                    Text(DataWords.working).accessibilityIdentifier("data-working")
                }
                if model.backups.isEmpty {
                    Text(DataWords.backupsNone).accessibilityIdentifier("data-backups-none")
                }
                ForEach(model.backups) { copy in
                    HStack {
                        Text(DataWords.line(copy))
                        Spacer()
                        Button(DataWords.export) {
                            Task { await model.export(copy) }
                        }
                        .accessibilityIdentifier("data-export-\(copy.id)")
                    }
                }
                Text(model.message).accessibilityIdentifier("data-message")
            }
            Section(DataWords.serverTitle) {
                Text(model.serverAge).accessibilityIdentifier("data-server-age")
            }
        }
        .navigationTitle(DataWords.title)
        .task { await model.list() }
        .fileExporter(
            isPresented: $model.exporterShown, document: model.exporting, contentType: .data,
            defaultFilename: model.exportName
        ) { result in
            model.exported(result)
        }
    }
}
