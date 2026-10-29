<script lang="ts">
  // SPEC-516 R4 to R8; ADR-534 D1, D4 to D6. The data screen.
  import type { DataApi, DataClient } from './data';

  let {
    client,
    data,
    platform,
    save
  }: {
    client: () => Promise<DataClient>;
    data: DataApi;
    platform: string;
    save?: (blob: Blob, name: string) => void;
  } = $props();

  void client;
  void data;
  void platform;
  void save;
</script>

<main></main>
