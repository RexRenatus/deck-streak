<script lang="ts">
  import ColourModeSetting from '#lib/settings/ColourModeSetting.svelte';
  import { m } from '#lib/paraglide/messages.js';

  // The settings screen (SPEC-593 R8; ADR-616 D1): the colour mode this device keeps.
</script>

<main class="mx-auto max-w-prose px-6 py-12">
  <h1 class="text-3xl font-semibold tracking-tight">{m.settings_link()}</h1>

  <div class="mt-8 rounded-lg border bg-card p-4 text-card-foreground">
    <ColourModeSetting />
  </div>

  <nav class="mt-8">
    <a href="/">{m.back_to_today()}</a>
  </nav>
</main>
