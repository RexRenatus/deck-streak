<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { Envelope } from './insights';

  // SPEC-094 R19: one instrument's section. Inert until the green commit.
  let {
    title,
    envelope,
    children
  }: { title: string; envelope: Envelope; children: Snippet } = $props();
  void title;
  void envelope;
  void children;
</script>
