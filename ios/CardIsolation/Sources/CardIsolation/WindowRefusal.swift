import WebKit

/// The card view's UI delegate, layer L6 (SPEC-349 R2, ADR-360 D1).
@MainActor
public final class WindowRefusal: NSObject, WKUIDelegate {
    override public init() {
        super.init()
    }
}
