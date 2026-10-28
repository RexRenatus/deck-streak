<script lang="ts">
  // SPEC-507 R2 to R9; ADR-525. The card health panel (stub: the dialog's shape only).
  import type { CardHealth } from '$lib/engine/protocol';

  let { health, loading, onclose }: { health: CardHealth | null; loading: boolean; onclose: () => void } =
    $props();
  void health;
  void loading;
</script>

<div role="dialog" aria-modal="true" aria-label="Card info">
  <button type="button" onclick={onclose}>Close</button>
</div>
