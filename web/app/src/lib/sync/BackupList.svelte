<script lang="ts">
  // SPEC-377 R15 to R17; ADR-388 D3, D5, D14. The browser's backups, as the Worker listed them after
  // its retention: each by its kind and age, newest first, with an export that saves its bytes as a
  // file named by kind and age, and each backup retention removed named once.
  import type { BackupsListed } from '$lib/engine/protocol';
  import type { BackupsClient } from '$lib/engine/backups';

  let { listed, client }: { listed: BackupsListed; client: () => Promise<BackupsClient> } = $props();

  // red stub: the list is absent from the screen
</script>
