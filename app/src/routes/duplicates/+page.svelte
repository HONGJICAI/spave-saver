<script lang="ts">
  import { findDuplicates, deleteFiles, type DuplicateGroup, type DeleteMode, type DeleteResult } from '$lib/api';
  import StatCard from '$lib/components/StatCard.svelte';
  import { formatSize } from '$lib/utils/format';
  import { appState } from '$lib/stores/app';
  import {
    selectDuplicates,
    fullySelectedGroups,
    keepOnePerGroup,
    applyDeletions,
    type KeepStrategy,
  } from '$lib/utils/duplicates';

  let loading = $state(false);
  let error = $state('');
  let duplicates = $state<DuplicateGroup[]>([]);
  let selected = $state<Set<string>>(new Set());
  let sortBy = $state<'default' | 'size' | 'count'>('size');
  let hasScanned = $state(false);

  // Delete flow
  let showConfirm = $state(false);
  let deleteMode = $state<DeleteMode>('trash');
  let allowFullGroups = $state(false);
  let deleting = $state(false);
  let lastResults = $state<DeleteResult[] | null>(null);

  let totalWasted = $derived(duplicates.reduce((sum, g) => sum + g.wasted_space, 0));

  let sizeByPath = $derived.by(() => {
    const map = new Map<string, number>();
    for (const group of duplicates) {
      for (const file of group.files) map.set(file.path, file.size);
    }
    return map;
  });

  let selectedSize = $derived(
    Array.from(selected).reduce((sum, path) => sum + (sizeByPath.get(path) ?? 0), 0)
  );

  let sortedDuplicates = $derived.by(() => {
    const sorted = [...duplicates];
    switch (sortBy) {
      case 'size':
        return sorted.sort((a, b) => b.wasted_space - a.wasted_space);
      case 'count':
        return sorted.sort((a, b) => b.count - a.count);
      default:
        return sorted;
    }
  });

  let endangeredGroups = $derived(fullySelectedGroups(duplicates, selected));

  async function handleScan() {
    if ($appState.scanPaths.length === 0) {
      error = 'Please add paths to scan first';
      return;
    }

    loading = true;
    error = '';
    duplicates = [];
    selected = new Set();
    lastResults = null;
    showConfirm = false;

    try {
      duplicates = await findDuplicates($appState.scanPaths, $appState.filterConfig);
      hasScanned = true;
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to find duplicates';
    } finally {
      loading = false;
    }
  }

  function toggleFile(path: string) {
    const next = new Set(selected);
    if (next.has(path)) {
      next.delete(path);
    } else {
      next.add(path);
    }
    selected = next;
  }

  function autoSelect(strategy: KeepStrategy) {
    selected = selectDuplicates(duplicates, strategy);
  }

  function clearSelection() {
    selected = new Set();
  }

  function openConfirm() {
    allowFullGroups = false;
    lastResults = null;
    showConfirm = true;
  }

  async function handleDelete() {
    deleting = true;
    error = '';
    try {
      const results = await deleteFiles(Array.from(selected), deleteMode);
      lastResults = results;

      const deletedPaths = new Set(results.filter(r => r.success).map(r => r.path));
      duplicates = applyDeletions(duplicates, deletedPaths);

      // Keep only failed paths selected so the user can retry or inspect them
      selected = new Set(results.filter(r => !r.success).map(r => r.path));
      showConfirm = false;
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to delete files';
    } finally {
      deleting = false;
    }
  }

  let deletedCount = $derived(lastResults?.filter(r => r.success).length ?? 0);
  let failedResults = $derived(lastResults?.filter(r => !r.success) ?? []);
  let deletedSize = $derived(
    lastResults
      ?.filter(r => r.success)
      .reduce((sum, r) => sum + (sizeByPath.get(r.path) ?? 0), 0) ?? 0
  );
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

  <!-- Last delete results -->
  {#if lastResults}
    <div class="bg-white rounded-lg shadow p-4 mb-6">
      <p class="text-sm text-gray-800">
        {#if deleteMode === 'trash'}
          ✅ Moved <strong>{deletedCount}</strong> file{deletedCount !== 1 ? 's' : ''} to the system trash
          ({formatSize(deletedSize)} — freed for good once the trash is emptied).
        {:else}
          ✅ Permanently deleted <strong>{deletedCount}</strong> file{deletedCount !== 1 ? 's' : ''} ({formatSize(deletedSize)}).
        {/if}
      </p>
      {#if failedResults.length > 0}
        <div class="mt-3 p-3 bg-red-50 border border-red-200 rounded">
          <p class="text-sm font-semibold text-red-800 mb-2">
            {failedResults.length} file{failedResults.length !== 1 ? 's' : ''} could not be deleted (still selected below):
          </p>
          <ul class="space-y-1 max-h-[20vh] overflow-y-auto">
            {#each failedResults as result}
              <li class="text-xs text-red-700">
                <span class="font-mono">{result.path}</span>
                <span class="text-red-500"> — {result.error ?? 'unknown error'}</span>
              </li>
            {/each}
          </ul>
        </div>
      {/if}
    </div>
  {/if}

  {#if duplicates.length > 0}
    <div class="grid grid-cols-1 md:grid-cols-3 gap-4 mb-6">
      <StatCard label="Duplicate Groups" value={duplicates.length} icon="📋" color="yellow" />
      <StatCard label="Wasted Space" value={formatSize(totalWasted)} icon="💸" color="red" />
      <StatCard
        label="Selected"
        value={`${selected.size} (${formatSize(selectedSize)})`}
        icon="✓"
        color="blue"
      />
    </div>

    <!-- Toolbar -->
    <div class="bg-white rounded-lg shadow p-4 mb-6 flex flex-wrap items-center gap-3">
      <span class="text-sm font-medium text-gray-700">Auto-select all but one per group, keeping:</span>
      <button
        onclick={() => autoSelect('newest')}
        class="px-3 py-1.5 text-sm border border-blue-300 text-blue-700 rounded hover:bg-blue-50"
      >
        Newest
      </button>
      <button
        onclick={() => autoSelect('oldest')}
        class="px-3 py-1.5 text-sm border border-blue-300 text-blue-700 rounded hover:bg-blue-50"
      >
        Oldest
      </button>
      <button
        onclick={() => autoSelect('shortest-path')}
        class="px-3 py-1.5 text-sm border border-blue-300 text-blue-700 rounded hover:bg-blue-50"
      >
        Shortest path
      </button>
      {#if selected.size > 0}
        <button
          onclick={clearSelection}
          class="px-3 py-1.5 text-sm border border-gray-300 text-gray-600 rounded hover:bg-gray-50"
        >
          Clear selection
        </button>
      {/if}

      <div class="flex-1"></div>

      <label for="sort-select" class="text-sm font-medium text-gray-700">Sort by:</label>
      <select
        id="sort-select"
        bind:value={sortBy}
        class="px-3 py-1.5 text-sm border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
      >
        <option value="size">Wasted Space</option>
        <option value="count">File Count</option>
        <option value="default">Default</option>
      </select>

      <button
        onclick={openConfirm}
        disabled={selected.size === 0 || deleting}
        class="px-5 py-1.5 text-sm bg-red-600 text-white rounded-lg hover:bg-red-700 disabled:bg-gray-300 disabled:cursor-not-allowed font-medium"
      >
        🗑️ Delete Selected ({selected.size})
      </button>
    </div>

    <!-- Delete confirmation panel -->
    {#if showConfirm}
      <div class="bg-white rounded-lg shadow border-2 border-red-200 p-5 mb-6">
        <h3 class="text-base font-bold text-gray-900 mb-3">
          Delete {selected.size} file{selected.size !== 1 ? 's' : ''} ({formatSize(selectedSize)})?
        </h3>

        <div class="space-y-2 mb-4">
          <label class="flex items-start gap-2 cursor-pointer">
            <input type="radio" bind:group={deleteMode} value="trash" class="mt-1" />
            <span class="text-sm">
              <span class="font-medium text-gray-800">Move to system trash</span>
              <span class="text-gray-600"> — recoverable; space is freed once the trash is emptied</span>
            </span>
          </label>
          <label class="flex items-start gap-2 cursor-pointer">
            <input type="radio" bind:group={deleteMode} value="permanent" class="mt-1" />
            <span class="text-sm">
              <span class="font-medium text-red-700">Delete permanently</span>
              <span class="text-gray-600"> — cannot be undone</span>
            </span>
          </label>
        </div>

        {#if endangeredGroups.length > 0}
          <div class="mb-4 p-3 bg-amber-50 border border-amber-300 rounded">
            <p class="text-sm font-semibold text-amber-800">
              ⚠️ In {endangeredGroups.length} group{endangeredGroups.length !== 1 ? 's' : ''} EVERY copy is selected —
              deleting would lose that content entirely.
            </p>
            <div class="mt-2 flex flex-wrap items-center gap-3">
              <button
                onclick={() => (selected = keepOnePerGroup(duplicates, selected))}
                class="px-3 py-1.5 text-xs font-medium bg-amber-600 text-white rounded hover:bg-amber-700"
              >
                Keep one file in each group
              </button>
              <label class="flex items-center gap-2 cursor-pointer text-xs text-amber-800">
                <input type="checkbox" bind:checked={allowFullGroups} />
                I understand, delete every copy anyway
              </label>
            </div>
          </div>
        {/if}

        <div class="flex items-center justify-end gap-3">
          <button
            onclick={() => (showConfirm = false)}
            disabled={deleting}
            class="px-4 py-2 text-sm bg-gray-100 text-gray-700 rounded hover:bg-gray-200 disabled:opacity-50"
          >
            Cancel
          </button>
          <button
            onclick={handleDelete}
            disabled={deleting || selected.size === 0 || (endangeredGroups.length > 0 && !allowFullGroups)}
            class="px-5 py-2 text-sm bg-red-600 text-white rounded hover:bg-red-700 disabled:bg-gray-300 disabled:cursor-not-allowed font-medium"
          >
            {deleting ? 'Deleting...' : deleteMode === 'trash' ? 'Move to Trash' : 'Delete Permanently'}
          </button>
        </div>
      </div>
    {/if}

    <div class="space-y-4">
      {#each sortedDuplicates as group (group.hash)}
        <div class="bg-white rounded-lg shadow p-6">
          <div class="flex items-center justify-between mb-4 gap-2">
            <h3 class="text-sm font-bold text-gray-900 font-mono truncate" title={group.hash}>
              #{group.hash.slice(0, 12)}
            </h3>
            <div class="text-sm text-gray-600 whitespace-nowrap">
              <span class="font-medium">{group.count} × {formatSize(group.files[0]?.size ?? 0)}</span>
              <span class="mx-2">•</span>
              <span class="text-red-600 font-medium">Wasted: {formatSize(group.wasted_space)}</span>
            </div>
          </div>

          <div class="space-y-2">
            {#each group.files as file (file.path)}
              <button
                type="button"
                class="flex items-center gap-3 p-3 rounded border hover:bg-gray-50 w-full text-left {selected.has(file.path) ? 'border-red-300 bg-red-50' : ''}"
                onclick={() => toggleFile(file.path)}
              >
                <input
                  type="checkbox"
                  checked={selected.has(file.path)}
                  class="rounded pointer-events-none"
                  tabindex="-1"
                  aria-label={`Select ${file.path} for deletion`}
                />
                <div class="flex-1 min-w-0">
                  <p class="font-mono text-sm text-gray-900 truncate" title={file.path}>{file.path}</p>
                  <p class="text-xs text-gray-500 mt-1">
                    {formatSize(file.size)} • {file.file_type} • modified {new Date(file.modified * 1000).toLocaleDateString()}
                  </p>
                </div>
              </button>
            {/each}
          </div>
        </div>
      {/each}
    </div>
  {:else if !loading && hasScanned}
    <div class="bg-white rounded-lg shadow p-12 text-center text-gray-500">
      <p class="text-xl mb-2">No duplicates found</p>
      <p class="text-sm">Every file in the scanned paths is unique 🎉</p>
    </div>
  {:else if !loading}
    <div class="bg-white rounded-lg shadow p-12 text-center text-gray-500">
      <p class="text-xl mb-2">Ready to scan</p>
      <p class="text-sm">Scan a directory to find duplicate files</p>
    </div>
  {/if}
</div>
