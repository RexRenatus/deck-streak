<script lang="ts">
  import { m } from '$lib/paraglide/messages.js';
  import SignInMethods from '$lib/settings/SignInMethods.svelte';

  // The sign-in methods screen (SPEC-385 R12), which Today links to; the settings screen embeds the
  // same component later (#57).
</script>

<main class="mx-auto max-w-prose px-6 py-12">
  <h1 class="text-3xl font-semibold tracking-tight">{m.methods_title()}</h1>
  <SignInMethods />

  <nav class="mt-8">
    <a href="/">{m.back_to_today()}</a>
  </nav>
</main>
