<script lang="ts">
  import { appState } from '$lib/stores/app';
  import { onMount } from 'svelte';

  interface Props {
    show?: boolean;
  }
  
  let { show = false }: Props = $props();

  let minSize = $state('');
  let maxSize = $state('');
  let extensions = $state('');
  let filePattern = $state('');

  // Load persisted filter values on mount
  onMount(() => {
    const currentFilter = $appState.filterConfig;
    if (currentFilter.minSize) {
      minSize = (currentFilter.minSize / (1024 * 1024)).toString();
    }
    if (currentFilter.maxSize) {
      maxSize = (currentFilter.maxSize / (1024 * 1024)).toString();
    }
    if (currentFilter.extensions && currentFilter.extensions.length > 0) {
      extensions = currentFilter.extensions.join(', ');
    }
    if (currentFilter.filePattern) {
      filePattern = currentFilter.filePattern;
    }
  });

  // Sync filter changes to global store
  $effect(() => {
    appState.setFilterConfig({
      minSize: minSize ? parseFloat(minSize) * 1024 * 1024 : undefined,
      maxSize: maxSize ? parseFloat(maxSize) * 1024 * 1024 : undefined,
      extensions: extensions ? extensions.split(',').map(e => e.trim()).filter(e => e) : undefined,
      filePattern: filePattern || undefined
    });
  });

  function clearFilters() {
    minSize = '';
    maxSize = '';
    extensions = '';
    filePattern = '';
    appState.clearFilters();
  }

  let activeFiltersCount = $derived.by(() => {
    let count = 0;
    if (minSize) count++;
    if (maxSize) count++;
    if (extensions) count++;
    if (filePattern) count++;
    return count;
  });
</script>

{#if show}
  <div class="mt-3 p-3 bg-gray-50 rounded-lg border border-gray-200">
    <div class="flex items-center justify-between mb-3">
      <h4 class="text-xs font-semibold text-gray-700 uppercase">File Filters</h4>
      {#if activeFiltersCount > 0}
        <button
          onclick={clearFilters}
          class="text-xs text-blue-600 hover:text-blue-700"
        >
          Clear all filters
        </button>
      {/if}
    </div>
    
    <div class="space-y-3">
      <!-- Size Filters -->
      <div class="grid grid-cols-2 gap-2">
        <div>
          <label for="minSize" class="block text-xs text-gray-600 mb-1">Min Size (MB)</label>
          <input
            id="minSize"
            type="number"
            bind:value={minSize}
            placeholder="0"
            min="0"
            step="0.1"
            class="w-full px-2 py-1.5 text-xs border border-gray-300 rounded focus:ring-1 focus:ring-blue-500 focus:border-transparent"
          />
        </div>
        <div>
          <label for="maxSize" class="block text-xs text-gray-600 mb-1">Max Size (MB)</label>
          <input
            id="maxSize"
            type="number"
            bind:value={maxSize}
            placeholder="∞"
            min="0"
            step="0.1"
            class="w-full px-2 py-1.5 text-xs border border-gray-300 rounded focus:ring-1 focus:ring-blue-500 focus:border-transparent"
          />
        </div>
      </div>

      <!-- Extensions Filter -->
      <div>
        <label for="extensions" class="block text-xs text-gray-600 mb-1">File Extensions (comma-separated)</label>
        <input
          id="extensions"
          type="text"
          bind:value={extensions}
          placeholder="e.g., jpg, png, pdf, mp4"
          class="w-full px-2 py-1.5 text-xs border border-gray-300 rounded focus:ring-1 focus:ring-blue-500 focus:border-transparent"
        />
        <p class="mt-1 text-xs text-gray-500">Leave empty to include all file types</p>
      </div>

      <!-- Pattern Filter -->
      <div>
        <label for="filePattern" class="block text-xs text-gray-600 mb-1">File Name Pattern</label>
        <input
          id="filePattern"
          type="text"
          bind:value={filePattern}
          placeholder="e.g., backup, temp, cache"
          class="w-full px-2 py-1.5 text-xs border border-gray-300 rounded focus:ring-1 focus:ring-blue-500 focus:border-transparent"
        />
        <p class="mt-1 text-xs text-gray-500">Match files containing this text</p>
      </div>

      <!-- Quick Filter Presets -->
      <div class="pt-2 border-t border-gray-300">
        <p class="text-xs text-gray-600 mb-2">Quick Presets:</p>
        <div class="flex flex-wrap gap-1.5">
          <button
            onclick={() => { extensions = 'jpg,jpeg,png,gif,bmp,webp,svg'; }}
            class="px-2 py-1 text-xs bg-white border border-gray-300 rounded hover:bg-gray-50"
          >
            Images
          </button>
          <button
            onclick={() => { extensions = 'mp4,avi,mkv,mov,wmv,flv,webm'; }}
            class="px-2 py-1 text-xs bg-white border border-gray-300 rounded hover:bg-gray-50"
          >
            Videos
          </button>
          <button
            onclick={() => { extensions = 'pdf,doc,docx,txt,xls,xlsx,ppt,pptx'; }}
            class="px-2 py-1 text-xs bg-white border border-gray-300 rounded hover:bg-gray-50"
          >
            Documents
          </button>
          <button
            onclick={() => { extensions = 'zip,rar,7z,tar,gz,bz2'; }}
            class="px-2 py-1 text-xs bg-white border border-gray-300 rounded hover:bg-gray-50"
          >
            Archives
          </button>
          <button
            onclick={() => { minSize = '100'; }}
            class="px-2 py-1 text-xs bg-white border border-gray-300 rounded hover:bg-gray-50"
          >
            Large (>100MB)
          </button>
        </div>
      </div>
    </div>
  </div>
{/if}
