<script lang="ts">
  // SPEC-341 R1, R4, R6; ADR-352 D1, D2. A card face on the web: one frame, sandboxed with no
  // token, whose document is the frame document. A card the frame document refuses renders a frame
  // with no document, marked as refused, so a card that escaped shows nothing rather than itself.
  // SPEC-350 R7; ADR-361: the card's classes reach the frame's body through its document, and the
  // frame gains no attribute and no style of its own, so its size is the parent's to give.
  import { frameDocument } from './frame-document';
  import { FRAME_SANDBOX } from './policy.js';

  let { html, css, title, classes }: { html: string; css: string; title: string; classes?: string } =
    $props();
  const doc = $derived(frameDocument(html, css, classes));
</script>

<iframe sandbox={FRAME_SANDBOX} {title} srcdoc={doc.srcdoc} data-card-refused={doc.refused}></iframe>
