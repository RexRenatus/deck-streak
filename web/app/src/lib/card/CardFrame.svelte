<script lang="ts">
  // SPEC-341 R1, R4, R6; ADR-352 D1, D2. A card face on the web. A stub until the layers are
  // written: a plain srcdoc frame.
  import { frameDocument } from './frame-document';

  let { html, css, title }: { html: string; css: string; title: string } = $props();
</script>

<iframe {title} srcdoc={frameDocument(html, css).srcdoc}></iframe>
