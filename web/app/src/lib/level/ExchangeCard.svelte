<script lang="ts">
  import type { ExchangeView } from './exchange';

  // The XP exchange readout card (SPEC-075 R9), inert: it renders none of the view yet.
  let { view }: { view: ExchangeView } = $props();
  void view;
</script>
