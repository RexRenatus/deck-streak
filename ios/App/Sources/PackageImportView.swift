import SwiftUI
import UniformTypeIdentifiers

extension UTType {
    /// An Anki deck package, `.apkg`, which the app's property list declares as an imported type
    /// (SPEC-681 R1).
    static let ankiDeckPackage = UTType(importedAs: "net.ankiweb.apkg")
}

/// The deck list's import on iPhone and iPad (SPEC-681 R1, R5, R6): the toolbar's Import control,
/// the system file importer it opens, the offer the empty list shows, and a refused package's
/// words. A picked file is imported through the model, and the list takes the decks the
/// collection then holds.
struct PackageImportControls: ViewModifier {
    let model: AppModel
    @Bindable var importer: PackageImportModel

    func body(content: Content) -> some View {
        content
            .overlay { PackageImportOffer(model: model, importer: importer) }
            .safeAreaInset(edge: .bottom) { PackageImportRefusal(importer: importer) }
            .toolbar {
                Button("Import") { importer.showingImporter = true }
                    .accessibilityIdentifier("import-package")
            }
            .fileImporter(
                isPresented: $importer.showingImporter, allowedContentTypes: [.ankiDeckPackage]
            ) { result in
                Task { model.decks = await importer.picked(result, keeping: model.decks) }
            }
    }
}

/// The offer the deck list shows while it holds no deck (R6).
struct PackageImportOffer: View {
    let model: AppModel
    let importer: PackageImportModel

    var body: some View {
        if model.decks.isEmpty {
            VStack(spacing: 16) {
                Text("No decks yet. Import an Anki deck package from Files.")
                    .foregroundStyle(Color.tideMuted)
                Button("Import a deck") { importer.showingImporter = true }
                    .accessibilityIdentifier("import-deck")
            }
            .padding()
        }
    }
}

/// A refused package's words, which say that nothing was imported (R5).
struct PackageImportRefusal: View {
    let importer: PackageImportModel

    var body: some View {
        if let refusal = importer.refusal {
            Text(refusal).accessibilityIdentifier("import-refusal")
                .foregroundStyle(Color.tideInk)
                .padding()
                .frame(maxWidth: .infinity)
                .background(Color.tideBackground)
        }
    }
}
