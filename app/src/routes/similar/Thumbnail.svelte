<script lang="ts">
  import { getImageThumbnail } from '$lib/api';

  let { path, alt = '' }: { path: string; alt?: string } = $props();

  let src = $state<string | null>(null);
  let failed = $state(false);

  // Fetch (and re-fetch when the path changes). The backend decodes and
  // shrinks the image into a data: URL we can use directly as <img src>.
  $effect(() => {
    const target = path;
    let cancelled = false;
    src = null;
    failed = false;

    getImageThumbnail(target, 160)
      .then((url) => {
        if (!cancelled) src = url;
      })
      .catch(() => {
        if (!cancelled) failed = true;
      });

    return () => {
      cancelled = true;
    };
  });
</script>

<div class="aspect-square bg-gray-100 rounded mb-2 flex items-center justify-center overflow-hidden">
  {#if src}
    <img {src} {alt} class="w-full h-full object-cover" />
  {:else if failed}
    <!-- Could not generate a preview (missing/unreadable image) -->
    <svg class="w-10 h-10 text-gray-300" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-label="No preview">
      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 3l18 18M9.88 9.88A3 3 0 0014.12 14.12M21 12a9 9 0 01-9 9m0-18a9 9 0 019 9" />
    </svg>
  {:else}
    <div class="w-full h-full animate-pulse bg-gray-200"></div>
  {/if}
</div>
