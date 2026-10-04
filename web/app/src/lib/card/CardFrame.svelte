<script lang="ts">
  // SPEC-341 R1, R4, R6; ADR-352 D1, D2. A card face on the web: one frame, sandboxed with no
  // token, whose document is the frame document. A card the frame document refuses renders a frame
  // with no document, marked as refused, so a card that escaped shows nothing rather than itself.
  import { frameDocument } from './frame-document';
  import { FRAME_SANDBOX } from './policy.js';

  let { html, css, title }: { html: string; css: string; title: string } = $props();
  const doc = $derived(frameDocument(html, css));
</script>

<iframe sandbox={FRAME_SANDBOX} {title} srcdoc={doc.srcdoc} data-card-refused={doc.refused}></iframe>
