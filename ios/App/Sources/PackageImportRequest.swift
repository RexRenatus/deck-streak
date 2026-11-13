import Foundation

/// The deck package import's request and its copy of the picked file (SPEC-681 R2). The request is
/// `ImportAnkiPackageRequest` with the package's path alone, field 1: the native adapter reads the
/// package under its bound and writes the import's options itself (ADR-703). The copy sits in the
/// app's temporary folder, made while the picked file's security-scoped access is held, so the
/// engine reads a file the app owns.
enum PackageImportRequest {
    /// The request's bytes: field 1, length-delimited, holding the path's UTF-8 bytes.
    static func bytes(path: String) -> [UInt8] {
        let text = Array(path.utf8)
        var out: [UInt8] = [0x0A]
        var length = UInt64(text.count)
        while length >= 0x80 {
            out.append(UInt8(length & 0x7F) | 0x80)
            length >>= 7
        }
        out.append(UInt8(length))
        return out + text
    }

    /// Copies the picked file into the temporary folder while its security-scoped access is held,
    /// and answers the copy. A file the app reaches without that access, as a test's planted file,
    /// is copied the same way.
    static func copy(_ picked: URL) throws -> URL {
        let scoped = picked.startAccessingSecurityScopedResource()
        defer {
            if scoped {
                picked.stopAccessingSecurityScopedResource()
            }
        }
        let copy = URL.temporaryDirectory.appending(
            path: "import-\(UUID().uuidString).apkg", directoryHint: .notDirectory)
        try FileManager.default.copyItem(at: picked, to: copy)
        return copy
    }
}
