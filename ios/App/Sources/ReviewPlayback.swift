/// The review's two playback gestures, out of `perform`'s arms (SPEC-358 R5): replay plays the
/// shown face's replay clips again, and stop stops what plays. Neither moves the review, so neither
/// pairs with a phase: `perform` hands every gesture its own arms do not take on to here through
/// `default:`, and the rest are no playback.
@MainActor
enum ReviewPlayback {
    /// Plays or stops `face`'s clips on `player` for `action`; any other action plays nothing.
    static func perform(_ action: ReviewAction, face: ReviewFace, player: ClipPlayer) {
        switch action {
        case .replay:
            player.play(face.replay)
        case .stop:
            player.stop()
        default:
            break
        }
    }
}
