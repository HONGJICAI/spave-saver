<script lang="ts">
  import { appState } from '$lib/stores/app';
  import type { PathValidationResult } from '$lib/utils/path';
  import PathInput from './PathInput.svelte';
  import FileFilters from './FileFilters.svelte';
  import PathList from './PathList.svelte';
  import InvalidPathList from './InvalidPathList.svelte';

  let isExpanded = $state(true);
  let showFilters = $state(false);
  
  // Local state for invalid/rejected paths from browse dialog
  interface InvalidPath {
    path: string;
    validation: PathValidationResult;
  }
  let invalidPaths = $state<InvalidPath[]>([]);

  function toggleExpanded() {
    isExpanded = !isExpanded;
  }

  function toggleFilters() {
    showFilters = !showFilters;
  }

  function handlePathAdded(path: string, hasSubpaths: string[]) {
    const validation = appState.validatePath(path);
    
    if (validation.isValid || (validation.hasSubpaths.length > 0 && !validation.isDuplicate && validation.isSubpathOf.length === 0)) {
      // Build new paths list: current paths minus any subpaths, plus new path
      const newPaths = [...$appState.scanPaths.filter(p => !validation.hasSubpaths.includes(p)), path];
      
      // Move removed subpaths to invalid
      validation.hasSubpaths.forEach(subpath => {
        const subpathValidation = appState.validatePath(subpath, newPaths);
        if (!invalidPaths.some(ip => ip.path === subpath)) {
          invalidPaths = [...invalidPaths, { path: subpath, validation: subpathValidation }];
        }
      });
      
      // Set the new paths all at once
      appState.setScanPaths(newPaths);
    }
    else {
      // Invalid path - add to invalid list
      if (!invalidPaths.some(ip => ip.path === path)) {
        invalidPaths = [...invalidPaths, { path, validation }];
      }
    }
  }

  function removePath(path: string) {
    appState.removeScanPath(path);
  }

  function removeInvalidPath(path: string) {
    invalidPaths = invalidPaths.filter(ip => ip.path !== path);
  }

  function clearAllPaths() {
    appState.clearScanPaths();
    invalidPaths = [];
  }
  
  function clearInvalidPaths() {
    invalidPaths = [];
  }

  // Count active filters
  let activeFiltersCount = $derived.by(() => {
    let count = 0;
    const filter = $appState.filterConfig;
    if (filter.minSize) count++;
    if (filter.maxSize) count++;
    if (filter.extensions && filter.extensions.length > 0) count++;
    if (filter.filePattern) count++;
    return count;
  });
</script>

<div class="bg-white rounded-lg shadow-sm">
  <!-- Header with collapse toggle -->
  <div class="flex items-center justify-between p-4 pb-3 border-b border-gray-100">
    <div class="flex items-center gap-2">
      <h3 class="text-sm font-semibold text-gray-700">Scan Paths</h3>
      {#if $appState.scanPaths.length > 0}
        <span class="px-1.5 py-0.5 bg-blue-600 text-white rounded-full text-xs font-medium">
          {$appState.scanPaths.length}
        </span>
      {/if}
    </div>
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
      <!-- Path Input -->
      <PathInput onPathAdded={handlePathAdded} />

      <!-- File Filters Section -->
      <FileFilters show={showFilters} />

      <!-- Valid Paths Display -->
      <PathList 
        paths={$appState.scanPaths}
        onRemove={removePath}
        onClearAll={clearAllPaths}
      />

      <!-- Invalid Paths Display -->
      <InvalidPathList 
        {invalidPaths}
        onRemove={removeInvalidPath}
        onClearAll={clearInvalidPaths}
      />
    </div>
  {/if}
</div>
