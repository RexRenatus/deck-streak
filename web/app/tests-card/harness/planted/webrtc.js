// SPEC-341 R10, A11 (SEC01-F14): the scripts-on measurement's peer-connection card, loaded as the
// frame's one admitted script. It opens a peer connection whose STUN server is the suite's UDP
// listener, as the planted `webrtc` card's inline script does (tests-card/planted.ts), and the
// listener counts the datagrams gathering sends.
const { host, port } = document.currentScript.dataset;
const peer = new RTCPeerConnection({ iceServers: [{ urls: `stun:${host}:${port}` }] });
peer.createDataChannel('card');
peer.createOffer().then((offer) => peer.setLocalDescription(offer));
