/// The protobuf wire format, as far as the harness's six calls need it (SPEC-339 R9, ADR-350).
/// Each failure to read a response is one of these.
public enum WireError: Error, Equatable, Sendable {
    /// The bytes end inside a field.
    case truncated
    /// A varint runs past ten bytes.
    case malformedVarint
    /// A field's wire type is one protobuf does not define, or one the harness never reads.
    case unknownWireType(UInt8)
    /// A string field is not UTF-8.
    case invalidUTF8
}

/// Writes a message's fields in the order the encoder calls them. It writes every field it is
/// handed, a zero or an empty one included; proto3's habit of omitting defaults is the encoders'
/// choice, made by not handing the field over.
struct WireWriter {
    private(set) var bytes: [UInt8] = []

    /// A base-128 varint: seven bits a byte, the low group first, and the high bit set on every
    /// byte but the last, so a reader knows another byte follows.
    mutating func varint(_ value: UInt64) {
        var rest = value
        while rest >= 0x80 {
            bytes.append(UInt8(truncatingIfNeeded: rest & 0x7f) | 0x80)
            rest >>= 7
        }
        bytes.append(UInt8(truncatingIfNeeded: rest))
    }

    /// A varint field: its key (wire type 0), then its value.
    mutating func varintField(_ number: UInt64, _ value: UInt64) {
        varint(number << 3)
        varint(value)
    }

    /// An `int64` field: the value's two's complement as a varint, so a negative takes ten bytes.
    mutating func int64Field(_ number: UInt64, _ value: Int64) {
        varintField(number, UInt64(bitPattern: value))
    }

    /// A length-delimited field (wire type 2): its key, its length as a varint, then its bytes.
    mutating func bytesField(_ number: UInt64, _ value: [UInt8]) {
        varint((number << 3) | 2)
        varint(UInt64(value.count))
        bytes.append(contentsOf: value)
    }

    /// A string field, as its UTF-8 bytes.
    mutating func stringField(_ number: UInt64, _ value: String) {
        bytesField(number, Array(value.utf8))
    }
}

/// One field's value, by its wire type. The harness reads no 32-bit or 64-bit field, so it keeps
/// only that one was there.
enum WireValue: Equatable {
    case varint(UInt64)
    case lengthDelimited([UInt8])
    case fixed64
    case fixed32
}

/// A message's fields, in the order its bytes hold them. Every field is parsed past by its wire
/// type, so a field the harness does not read is skipped whole and the next one is read from where
/// it starts. A field that occurs twice keeps both occurrences; a reader of one value takes the
/// last, as protobuf does.
struct WireMessage {
    private(set) var fields: [(number: UInt64, value: WireValue)] = []

    init(_ bytes: [UInt8]) throws {
        var at = 0
        while at < bytes.count {
            let key = try Self.varint(bytes, &at)
            let value: WireValue
            switch UInt8(truncatingIfNeeded: key & 7) {
            case 0:
                value = .varint(try Self.varint(bytes, &at))
            case 1:
                try Self.skip(8, of: bytes, from: &at)
                value = .fixed64
            case 2:
                let length = try Self.varint(bytes, &at)
                guard length <= UInt64(bytes.count - at) else { throw WireError.truncated }
                let end = at + Int(length)
                value = .lengthDelimited(Array(bytes[at..<end]))
                at = end
            case 5:
                try Self.skip(4, of: bytes, from: &at)
                value = .fixed32
            case let other:
                throw WireError.unknownWireType(other)
            }
            fields.append((number: key >> 3, value: value))
        }
    }

    /// Reads a varint at `at` and moves past it.
    private static func varint(_ bytes: [UInt8], _ at: inout Int) throws -> UInt64 {
        var value: UInt64 = 0
        var shift: UInt64 = 0
        while true {
            guard at < bytes.count else { throw WireError.truncated }
            guard shift < 64 else { throw WireError.malformedVarint }
            let byte = bytes[at]
            at += 1
            value |= UInt64(byte & 0x7f) << shift
            if byte & 0x80 == 0 {
                return value
            }
            shift += 7
        }
    }

    /// Moves past a fixed-width value.
    private static func skip(_ width: Int, of bytes: [UInt8], from at: inout Int) throws {
        guard width <= bytes.count - at else { throw WireError.truncated }
        at += width
    }

    /// A varint field's last value, or zero when the message omits it (proto3's default).
    func varint(_ number: UInt64) -> UInt64 {
        var found: UInt64 = 0
        for field in fields where field.number == number {
            if case .varint(let value) = field.value {
                found = value
            }
        }
        return found
    }

    /// Every occurrence of a length-delimited field, in order.
    func repeated(_ number: UInt64) -> [[UInt8]] {
        fields.compactMap { field in
            guard field.number == number, case .lengthDelimited(let value) = field.value else {
                return nil
            }
            return value
        }
    }

    /// A length-delimited field's last occurrence, or `nil` when the message omits it.
    func lengthDelimited(_ number: UInt64) -> [UInt8]? {
        repeated(number).last
    }

    /// A string field's value, or the empty string when the message omits it.
    func string(_ number: UInt64) throws -> String {
        let bytes = lengthDelimited(number) ?? []
        let text = String(decoding: bytes, as: UTF8.self)
        // Decoding replaces an invalid sequence, so a string that does not re-encode to the same
        // bytes was not UTF-8.
        guard Array(text.utf8) == bytes else { throw WireError.invalidUTF8 }
        return text
    }
}
