<script lang="ts">
  import './layout.css';
  import { onMount } from 'svelte';
  import { telegram } from '$lib/telegram.svelte';

  let { children } = $props();

  // Telegram shows its own placeholder until the Mini App says it is ready. The root layout mounts
  // after the first screen has rendered, so this is the moment (SPEC-028 R2).
  onMount(() => telegram.ready());
</script>

{@render children()}
