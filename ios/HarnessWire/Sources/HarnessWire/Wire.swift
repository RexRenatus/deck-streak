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
