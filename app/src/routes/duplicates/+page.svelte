<script lang="ts">
  import { findDuplicates, type DuplicateGroup, deleteFiles } from '$lib/api';
  import StatCard from '$lib/components/StatCard.svelte';
  import { formatSize } from '$lib/utils/format';
  import { appState } from '$lib/stores/app';
  
  let loading = false;
  let error = '';
  let duplicates: DuplicateGroup[] = [];
  let selectedForDeletion: Set<string> = new Set();
  let sortBy = 'default' as 'default' | 'size' | 'count';
  
  $: totalWasted = duplicates.reduce((sum, group) => sum + group.wasted_space, 0);
  $: totalGroups = duplicates.length;
  
  $: sortedDuplicates = (() => {
    const sorted = [...duplicates];
    switch (sortBy) {
      case 'size':
        return sorted.sort((a, b) => b.wasted_space - a.wasted_space);
      case 'count':
        return sorted.sort((a, b) => b.count - a.count);
      default:
        return sorted;
    }
  })();
  
  async function handleScan() {
    // Use scanPaths
    const paths = $appState.scanPaths;
    
    if (paths.length === 0) {
      error = 'Please enter at least one path to scan';
      return;
    }
    
    loading = true;
    error = '';
    duplicates = [];
    selectedForDeletion.clear();
    
    try {
      duplicates = await findDuplicates(paths, $appState.filterConfig);
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to find duplicates';
    } finally {
      loading = false;
    }
  }
  
  function toggleFileSelection(path: string) {
    if (selectedForDeletion.has(path)) {
      selectedForDeletion.delete(path);
    } else {
      selectedForDeletion.add(path);
    }
    selectedForDeletion = selectedForDeletion;
  }
  
  async function handleDelete() {
    if (selectedForDeletion.size === 0) {
      alert('No files selected for deletion');
      return;
    }
    
    if (!confirm(`Are you sure you want to delete ${selectedForDeletion.size} files?`)) {
      return;
    }
    
    try {
      const paths = Array.from(selectedForDeletion);
      await deleteFiles(paths);
      alert(`Successfully deleted ${paths.length} files`);
      selectedForDeletion.clear();
      await handleScan(); // Rescan
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to delete files';
    }
  }
</script>

<svelte:head>
  <title>Duplicates - Space-Saver</title>
</svelte:head>

<div class="max-w-7xl">
  <h1 class="text-3xl font-bold text-gray-900 mb-6">📋 Find Duplicate Files</h1>
  
  <div class="bg-white rounded-lg shadow p-6 mb-6">
    <button
      onclick={handleScan}
      disabled={loading}
      class="px-6 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:bg-gray-400 w-full"
    >
      {loading ? '⏳ Scanning...' : '🔍 Find Duplicates'}
    </button>
    
    {#if error}
      <div class="mt-4 p-4 bg-red-50 text-red-700 rounded-lg">
        ⚠️ {error}
      </div>
    {/if}
  </div>
  
  {#if duplicates.length > 0}
    <div class="grid grid-cols-1 md:grid-cols-3 gap-4 mb-6">
      <StatCard
        label="Duplicate Groups"
        value={totalGroups}
        icon="📋"
        color="yellow"
      />
      <StatCard
        label="Wasted Space"
        value={formatSize(totalWasted)}
        icon="💸"
        color="red"
      />
      <StatCard
        label="Selected"
        value={selectedForDeletion.size}
        icon="✓"
        color="blue"
      />
    </div>
    
    <div class="flex items-center justify-between mb-6">
      {#if selectedForDeletion.size > 0}
        <button
          onclick={handleDelete}
          class="px-6 py-2 bg-red-600 text-white rounded-lg hover:bg-red-700"
        >
          🗑️ Delete Selected ({selectedForDeletion.size} files)
        </button>
      {:else}
        <div></div>
      {/if}
      
      <div class="flex items-center gap-3">
        <label for="sort-select" class="text-sm font-medium text-gray-700">
          Sort by:
        </label>
        <select
          id="sort-select"
          bind:value={sortBy}
          class="px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
        >
          <option value="default">Default</option>
          <option value="size">Size (Largest First)</option>
          <option value="count">File Count (Most First)</option>
        </select>
      </div>
    </div>
    
    <div class="space-y-4">
      {#each sortedDuplicates as group, index}
        <div class="bg-white rounded-lg shadow p-6">
          <div class="flex items-center justify-between mb-4">
            <h3 class="text-lg font-bold text-gray-900">
              Group #{index + 1}
            </h3>
            <div class="text-sm text-gray-600">
              <span class="font-medium">{group.count} files</span>
              <span class="mx-2">•</span>
              <span class="text-red-600 font-medium">
                Wasted: {formatSize(group.wasted_space)}
              </span>
            </div>
          </div>
          
          <div class="space-y-2">
            {#each group.files as file}
              <div 
                class="flex items-center gap-3 p-3 rounded border hover:bg-gray-50 cursor-pointer"
                onclick={() => toggleFileSelection(file.path)}
              >
                <input
                  type="checkbox"
                  checked={selectedForDeletion.has(file.path)}
                  onclick={(e) => e.stopPropagation()}
                  class="rounded"
                />
                <div class="flex-1">
                  <p class="font-mono text-sm text-gray-900">{file.path}</p>
                  <p class="text-xs text-gray-500 mt-1">
                    {formatSize(file.size)} • {file.file_type}
                  </p>
                </div>
              </div>
            {/each}
          </div>
        </div>
      {/each}
    </div>
  {:else if !loading}
    <div class="bg-white rounded-lg shadow p-12 text-center text-gray-500">
      <p class="text-xl mb-2">No duplicates found</p>
      <p class="text-sm">Scan a directory to find duplicate files</p>
    </div>
  {/if}
</div>
