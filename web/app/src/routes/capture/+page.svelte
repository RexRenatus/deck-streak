<script lang="ts">
  import { api } from '$lib/api';
  import QuickCapture from '$lib/capture/QuickCapture.svelte';
  import { m } from '$lib/paraglide/messages.js';

  // The quick capture screen (SPEC-118 R11): a note or a journal line, saved into the vault's
  // inbox through the app's one API client.
</script>

<main class="mx-auto max-w-prose px-6 py-12">
  <h1 class="text-3xl font-semibold tracking-tight">DeckStreak</h1>

  <div class="mt-8 rounded-lg bg-card p-4 text-card-foreground">
    <QuickCapture send={api.capture} />
  </div>

  <nav class="mt-8">
    <a href="/">{m.back_to_today()}</a>
  </nav>
</main>
