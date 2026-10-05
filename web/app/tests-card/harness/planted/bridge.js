// SPEC-341 R10, A11 (SEC01-F15): the scripts-on measurement's bridge card, loaded as the frame's one
// admitted script. It reaches for everything the planted `bridge` card's inline script does
// (tests-card/planted.ts, BRIDGE_STEPS): the page's bridge, a message to the parent and the top, a
// BroadcastChannel, the page's storage and the engine's port. Each step that throws is passed over.
for (const step of [
  () => parent.bridge(),
  () => parent.postMessage('card', '*'),
  () => top.postMessage('card', '*'),
  () => new BroadcastChannel('deck-streak').postMessage('card'),
  () => localStorage.setItem('deck-streak-card', 'card'),
  () => parent.enginePort.postMessage('card')
]) {
  try {
    step();
  } catch {
    // the step is closed to this frame
  }
}
