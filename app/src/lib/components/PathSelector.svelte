<script lang="ts">
  import { appState } from '$lib/stores/app';

  let inputPath = $state('');
  let isExpanded = $state(true);
  let showFilters = $state(false);
  
  // Filter options - bind to local state, sync with store
  let minSize = $state('');
  let maxSize = $state('');
  let extensions = $state('');
  let filePattern = $state('');

  // Sync filter changes to global store
  $effect(() => {
    appState.setFilterConfig({
      minSize: minSize ? parseFloat(minSize) * 1024 * 1024 : undefined,
      maxSize: maxSize ? parseFloat(maxSize) * 1024 * 1024 : undefined,
      extensions: extensions ? extensions.split(',').map(e => e.trim()).filter(e => e) : undefined,
      filePattern: filePattern || undefined
    });
  });

  function toggleExpanded() {
    isExpanded = !isExpanded;
  }

  function toggleFilters() {
    showFilters = !showFilters;
  }

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

  async function selectFolder() {
    // In Tauri mode, use native dialog
    if ('__TAURI_INTERNALS__' in window) {
      try {
        const { open } = await import('@tauri-apps/plugin-dialog');
        const selected = await open({
          directory: true,
          multiple: true,
        });
        
        if (selected) {
          if (Array.isArray(selected)) {
            // Multiple paths selected
            selected.forEach(path => appState.addScanPath(path));
            inputPath = '';
          } else {
            // Single path selected
            appState.addScanPath(selected);
            inputPath = '';
          }
        }
      } catch (e) {
        console.error('Failed to open dialog:', e);
      }
    }
  }

  function handleAddPath() {
    if (inputPath.trim()) {
      appState.addScanPath(inputPath.trim());
      inputPath = '';
    }
  }

  function removePath(path: string) {
    appState.removeScanPath(path);
  }

  function clearAllPaths() {
    appState.clearScanPaths();
  }
</script>

<div class="bg-white rounded-lg shadow-sm">
  <!-- Header with collapse toggle -->
  <div class="flex items-center justify-between p-4 pb-3 border-b border-gray-100">
    <h3 class="text-sm font-semibold text-gray-700">Scan Paths</h3>
    <div class="flex items-center gap-2">
      <button
        onclick={toggleFilters}
        class="relative px-2 py-1 text-xs font-medium rounded hover:bg-gray-100 transition-colors flex items-center gap-1"
        class:text-blue-600={activeFiltersCount > 0}
        class:text-gray-600={activeFiltersCount === 0}
        title="File Filters"
      >
        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 4a1 1 0 011-1h16a1 1 0 011 1v2.586a1 1 0 01-.293.707l-6.414 6.414a1 1 0 00-.293.707V17l-4 4v-6.586a1 1 0 00-.293-.707L3.293 7.293A1 1 0 013 6.586V4z"></path>
        </svg>
        Filters
        {#if activeFiltersCount > 0}
          <span class="ml-0.5 px-1.5 py-0.5 bg-blue-600 text-white rounded-full text-xs">
            {activeFiltersCount}
          </span>
        {/if}
      </button>
      <button
        onclick={toggleExpanded}
        class="p-1 hover:bg-gray-100 rounded transition-colors"
        aria-label={isExpanded ? 'Collapse' : 'Expand'}
        title={isExpanded ? 'Collapse' : 'Expand'}
      >
        <svg 
          class="w-5 h-5 text-gray-500 transition-transform duration-200"
          class:rotate-180={!isExpanded}
          fill="none" 
          stroke="currentColor" 
          viewBox="0 0 24 24"
        >
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7"></path>
        </svg>
      </button>
    </div>
  </div>

  <!-- Collapsible content -->
  {#if isExpanded}
    <div class="p-4 pt-3 space-y-3">
      <!-- Input Row -->
      <div class="flex gap-2">
      <input
        type="text"
        bind:value={inputPath}
        onkeydown={(e) => e.key === 'Enter' && handleAddPath()}
        placeholder="Add directory path..."
        class="flex-1 px-3 py-2 text-sm border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
      />
      <button
        onclick={selectFolder}
        class="px-3 py-2 border border-gray-300 rounded-lg hover:bg-gray-50 flex items-center gap-1"
        title="Browse for folder"
      >
        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"></path>
        </svg>
        Browse
      </button>
      <button
        onclick={handleAddPath}
        disabled={!inputPath.trim()}
        class="px-3 py-2 bg-blue-600 text-white text-sm rounded-lg hover:bg-blue-700 disabled:bg-gray-300 disabled:cursor-not-allowed"
      >
        Add
      </button>
    </div>

    <!-- File Filters Section -->
    {#if showFilters}
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
                onclick={() => { extensions = 'jpg,jpeg,png,gif,bmp,webp,svg'; showFilters = true; }}
                class="px-2 py-1 text-xs bg-white border border-gray-300 rounded hover:bg-gray-50"
              >
                Images
              </button>
              <button
                onclick={() => { extensions = 'mp4,avi,mkv,mov,wmv,flv,webm'; showFilters = true; }}
                class="px-2 py-1 text-xs bg-white border border-gray-300 rounded hover:bg-gray-50"
              >
                Videos
              </button>
              <button
                onclick={() => { extensions = 'pdf,doc,docx,txt,xls,xlsx,ppt,pptx'; showFilters = true; }}
                class="px-2 py-1 text-xs bg-white border border-gray-300 rounded hover:bg-gray-50"
              >
                Documents
              </button>
              <button
                onclick={() => { extensions = 'zip,rar,7z,tar,gz,bz2'; showFilters = true; }}
                class="px-2 py-1 text-xs bg-white border border-gray-300 rounded hover:bg-gray-50"
              >
                Archives
              </button>
              <button
                onclick={() => { minSize = '100'; showFilters = true; }}
                class="px-2 py-1 text-xs bg-white border border-gray-300 rounded hover:bg-gray-50"
              >
                Large (>100MB)
              </button>
            </div>
          </div>
        </div>
      </div>
    {/if}

    <!-- Multiple Paths Display -->
    {#if $appState.scanPaths.length > 0}
      <div class="space-y-2">
        <div class="flex items-center justify-between">
          <span class="text-xs font-medium text-gray-600">
            {$appState.scanPaths.length} path{$appState.scanPaths.length !== 1 ? 's' : ''} selected
          </span>
          <button
            onclick={clearAllPaths}
            class="text-xs text-red-600 hover:text-red-700"
          >
            Clear all
          </button>
        </div>
        <div class="flex flex-wrap gap-2">
          {#each $appState.scanPaths as path}
            <div class="flex items-center gap-1 px-2 py-1 bg-blue-100 text-blue-800 rounded-md text-xs">
              <span class="max-w-xs truncate" title={path}>{path}</span>
              <button
                onclick={() => removePath(path)}
                class="hover:bg-blue-200 rounded p-0.5"
                aria-label="Remove path"
                title="Remove"
              >
                <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
                </svg>
              </button>
            </div>
          {/each}
        </div>
      </div>
    {/if}
    </div>
  {/if}
</div>
