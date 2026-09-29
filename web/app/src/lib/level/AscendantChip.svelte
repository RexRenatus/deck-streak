<script lang="ts">
  import { m } from '$lib/paraglide/messages.js';

  // The Ascendant chip (SPEC-072 R26): shown on an Ascendant day only, as text, never as colour
  // alone.
</script>

<p class="mt-4 inline-block rounded-full bg-muted px-3 py-1 text-sm" data-ascendant>
  ✨ {m.level_ascendant()}
</p>
