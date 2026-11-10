import Foundation

struct TallyView: Decodable {
    let points: Int
    let streak: Int
}

enum TallyRoute {
    static let path = "/api/tally"
}
