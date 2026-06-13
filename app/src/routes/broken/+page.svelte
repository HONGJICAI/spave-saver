<script lang="ts">
  import { appState } from '$lib/stores/app';
  import { findBrokenFiles, deleteFiles, fixFileExtensions, type BrokenFile } from '$lib/api';

  let loading = $state(false);
  let broken: BrokenFile[] = $state([]);
  let scanned = $state(false);
  let selectedPaths = $state<Set<string>>(new Set());
  let showDeleteConfirm = $state(false);
  let deleteInProgress = $state(false);
  let fixInProgress = $state(false);
  let sortBy = $state<'name' | 'category' | 'size' | 'path'>('category');
  let sortOrder = $state<'asc' | 'desc'>('asc');

  async function handleScan() {
    const paths = $appState.scanPaths;
    if (paths.length === 0) {
      appState.setError('Please enter at least one path to scan');
      return;
    }

    loading = true;
    appState.setError(null);

    try {
      broken = await findBrokenFiles(paths, $appState.filterConfig);
      selectedPaths = new Set();
      scanned = true;
    } catch (err) {
      appState.setError(err instanceof Error ? err.message : 'Failed to scan for broken files');
    } finally {
      loading = false;
    }
  }

  function toggleItem(path: string) {
    const next = new Set(selectedPaths);
    if (next.has(path)) {
      next.delete(path);
    } else {
      next.add(path);
    }
    selectedPaths = next;
  }

  function toggleAll() {
    if (selectedPaths.size === sortedItems.length) {
      selectedPaths = new Set();
    } else {
      selectedPaths = new Set(sortedItems.map((item) => item.path));
    }
  }

  function clearSelection() {
    selectedPaths = new Set();
  }

  async function handleDelete() {
    // Only corrupted files are deletable; misnamed files are renamed, never
    // deleted (their content is valid).
    const targets = selectedCorrupted.map((b) => b.path);
    if (targets.length === 0) return;

    deleteInProgress = true;
    try {
      // Defaults to the system trash; only successfully removed items leave
      // the list so any per-file failures stay visible.
      const results = await deleteFiles(targets);
      const deleted = new Set(results.filter((r) => r.success).map((r) => r.path));
      const failed = results.filter((r) => !r.success);
      broken = broken.filter((b) => !deleted.has(b.path));
      selectedPaths = new Set([...selectedPaths].filter((p) => !deleted.has(p)));
      showDeleteConfirm = false;
      if (failed.length > 0) {
        appState.setError(`Failed to delete ${failed.length} item(s): ${failed[0].error ?? 'unknown error'}`);
      }
    } catch (err) {
      appState.setError(err instanceof Error ? err.message : 'Failed to delete files');
    } finally {
      deleteInProgress = false;
    }
  }

  async function handleFix() {
    // Rename misnamed files to the extension matching their real content. A
    // fixed file is no longer broken, so it leaves the list.
    const targets = selectedMismatch.map((b) => b.path);
    if (targets.length === 0) return;

    fixInProgress = true;
    try {
      const results = await fixFileExtensions(targets);
      const fixed = new Set(results.filter((r) => r.success).map((r) => r.path));
      const failed = results.filter((r) => !r.success);
      broken = broken.filter((b) => !fixed.has(b.path));
      selectedPaths = new Set([...selectedPaths].filter((p) => !fixed.has(p)));
      if (failed.length > 0) {
        appState.setError(`Failed to rename ${failed.length} file(s): ${failed[0].error ?? 'unknown error'}`);
      }
    } catch (err) {
      appState.setError(err instanceof Error ? err.message : 'Failed to fix extensions');
    } finally {
      fixInProgress = false;
    }
  }

  function handleSort(column: 'name' | 'category' | 'size' | 'path') {
    if (sortBy === column) {
      sortOrder = sortOrder === 'asc' ? 'desc' : 'asc';
    } else {
      sortBy = column;
      sortOrder = 'asc';
    }
  }

  function getItemName(path: string): string {
    return path.split(/[/\\]/).pop() || path;
  }

  function formatSize(bytes: number): string {
    if (bytes === 0) return '0 B';
    const units = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(1024));
    return `${(bytes / Math.pow(1024, i)).toFixed(i === 0 ? 0 : 1)} ${units[i]}`;
  }

  function categoryLabel(category: BrokenFile['category']): string {
    return category === 'corrupted' ? 'Corrupted' : 'Wrong extension';
  }

  let corruptedCount = $derived(broken.filter((b) => b.category === 'corrupted').length);
  let mismatchCount = $derived(broken.filter((b) => b.category === 'extension_mismatch').length);

  // Selection split by category: corrupted files are deleted, misnamed files
  // are renamed — never deleted, since their content is valid.
  let selectedCorrupted = $derived(
    broken.filter((b) => b.category === 'corrupted' && selectedPaths.has(b.path))
  );
  let selectedMismatch = $derived(
    broken.filter((b) => b.category === 'extension_mismatch' && selectedPaths.has(b.path))
  );

  let sortedItems = $derived.by(() => {
    const items = [...broken];
    items.sort((a, b) => {
      let comparison = 0;
      switch (sortBy) {
        case 'name':
          comparison = getItemName(a.path).localeCompare(getItemName(b.path));
          break;
        case 'category':
          comparison = a.category.localeCompare(b.category) || a.path.localeCompare(b.path);
          break;
        case 'size':
          comparison = a.size - b.size;
          break;
        case 'path':
          comparison = a.path.localeCompare(b.path);
          break;
      }
      return sortOrder === 'asc' ? comparison : -comparison;
    });
    return items;
  });
</script>

<div class="p-6">
  <div class="mb-6">
    <h1 class="text-3xl font-bold text-gray-900 mb-2">Broken Files Finder</h1>
    <p class="text-gray-600">
      Find files that are invalid or corrupted — unreadable content, or content that doesn't match its extension
    </p>
  </div>

  {#if $appState.error}
    <div class="bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded mb-4">
      {$appState.error}
    </div>
  {/if}

  <div class="bg-white rounded-lg shadow p-6 mb-6">
    <button
      onclick={handleScan}
      disabled={loading || $appState.scanPaths.length === 0}
      class="px-6 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:bg-gray-300 disabled:cursor-not-allowed"
    >
      {#if loading}
        Scanning...
      {:else if $appState.scanPaths.length > 0}
        Scan {$appState.scanPaths.length} Path{$appState.scanPaths.length !== 1 ? 's' : ''}
      {:else}
        Scan for Broken Files
      {/if}
    </button>
  </div>

  {#if sortedItems.length > 0}
    <div class="bg-white rounded-lg shadow p-6 mb-6">
      <div class="flex justify-between items-center">
        <div>
          <h2 class="text-xl font-semibold">
            Found {broken.length} broken file{broken.length !== 1 ? 's' : ''}
          </h2>
          <p class="text-gray-600 mt-1">
            {corruptedCount} corrupted, {mismatchCount} with a wrong extension.
            Corrupted files can be deleted; misnamed files are renamed to the correct extension, not deleted.
          </p>
        </div>
        {#if selectedPaths.size > 0}
          <div class="flex gap-2">
            <button
              onclick={clearSelection}
              class="px-4 py-2 border border-gray-300 rounded-lg hover:bg-gray-50"
            >
              Clear Selection ({selectedPaths.size})
            </button>
            {#if selectedMismatch.length > 0}
              <button
                onclick={handleFix}
                disabled={fixInProgress}
                class="px-4 py-2 bg-amber-600 text-white rounded-lg hover:bg-amber-700 disabled:opacity-50"
              >
                {fixInProgress ? 'Renaming...' : `Fix Extension (${selectedMismatch.length})`}
              </button>
            {/if}
            {#if selectedCorrupted.length > 0}
              <button
                onclick={() => (showDeleteConfirm = true)}
                class="px-4 py-2 bg-red-600 text-white rounded-lg hover:bg-red-700"
              >
                Delete Corrupted ({selectedCorrupted.length})
              </button>
            {/if}
          </div>
        {/if}
      </div>
    </div>

    <div class="bg-white rounded-lg shadow overflow-hidden">
      <table class="min-w-full divide-y divide-gray-200">
        <thead class="bg-gray-50">
          <tr>
            <th class="px-6 py-3 text-left">
              <input
                type="checkbox"
                checked={selectedPaths.size === sortedItems.length && sortedItems.length > 0}
                onchange={toggleAll}
                class="rounded border-gray-300"
              />
            </th>
            <th
              class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider cursor-pointer hover:bg-gray-100"
              onclick={() => handleSort('category')}
            >
              Issue
              {#if sortBy === 'category'}<span class="ml-1">{sortOrder === 'asc' ? '↑' : '↓'}</span>{/if}
            </th>
            <th
              class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider cursor-pointer hover:bg-gray-100"
              onclick={() => handleSort('name')}
            >
              Name
              {#if sortBy === 'name'}<span class="ml-1">{sortOrder === 'asc' ? '↑' : '↓'}</span>{/if}
            </th>
            <th
              class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider cursor-pointer hover:bg-gray-100"
              onclick={() => handleSort('size')}
            >
              Size
              {#if sortBy === 'size'}<span class="ml-1">{sortOrder === 'asc' ? '↑' : '↓'}</span>{/if}
            </th>
            <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
              Reason
            </th>
            <th
              class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider cursor-pointer hover:bg-gray-100"
              onclick={() => handleSort('path')}
            >
              Path
              {#if sortBy === 'path'}<span class="ml-1">{sortOrder === 'asc' ? '↑' : '↓'}</span>{/if}
            </th>
          </tr>
        </thead>
        <tbody class="bg-white divide-y divide-gray-200">
          {#each sortedItems as item (item.path)}
            <tr
              class="hover:bg-gray-50 cursor-pointer {selectedPaths.has(item.path) ? 'bg-blue-50' : ''}"
              onclick={() => toggleItem(item.path)}
            >
              <td class="px-6 py-4 whitespace-nowrap">
                <input
                  type="checkbox"
                  checked={selectedPaths.has(item.path)}
                  class="rounded border-gray-300"
                  onclick={(e) => e.stopPropagation()}
                  onchange={() => toggleItem(item.path)}
                />
              </td>
              <td class="px-6 py-4 whitespace-nowrap">
                {#if item.category === 'corrupted'}
                  <span class="inline-flex items-center px-2 py-1 rounded-full text-xs font-medium bg-red-100 text-red-800">
                    {categoryLabel(item.category)}
                  </span>
                {:else}
                  <span class="inline-flex items-center px-2 py-1 rounded-full text-xs font-medium bg-amber-100 text-amber-800">
                    {categoryLabel(item.category)}
                  </span>
                {/if}
              </td>
              <td class="px-6 py-4 whitespace-nowrap">
                <span class="text-sm font-medium text-gray-900">{getItemName(item.path)}</span>
              </td>
              <td class="px-6 py-4 whitespace-nowrap">
                <span class="text-sm text-gray-600">{formatSize(item.size)}</span>
              </td>
              <td class="px-6 py-4">
                <span class="text-sm text-gray-700 truncate max-w-xs block" title={item.reason}>
                  {item.reason}
                </span>
                {#if item.category === 'extension_mismatch' && item.suggested_extension}
                  <span class="text-xs text-amber-700">→ rename to .{item.suggested_extension}</span>
                {/if}
              </td>
              <td class="px-6 py-4">
                <span class="text-sm text-gray-600 truncate max-w-xs block" title={item.path}>
                  {item.path}
                </span>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {:else if !loading}
    <div class="bg-white rounded-lg shadow p-12 text-center">
      <svg class="w-16 h-16 text-gray-400 mx-auto mb-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"></path>
      </svg>
      <p class="text-gray-500">
        {scanned
          ? 'No broken files found. Everything checks out.'
          : 'No broken files found. Scan a directory to get started.'}
      </p>
    </div>
  {/if}
</div>

<!-- Delete Confirmation Dialog -->
{#if showDeleteConfirm}
  <div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
    <div class="bg-white rounded-lg p-6 max-w-md w-full mx-4">
      <h3 class="text-lg font-semibold mb-2">Confirm Deletion</h3>
      <p class="text-gray-600 mb-4">
        Are you sure you want to delete {selectedCorrupted.length} corrupted file{selectedCorrupted.length !== 1 ? 's' : ''}?
        This action cannot be undone.
      </p>
      <div class="flex gap-2 justify-end">
        <button
          onclick={() => (showDeleteConfirm = false)}
          disabled={deleteInProgress}
          class="px-4 py-2 border border-gray-300 rounded-lg hover:bg-gray-50 disabled:opacity-50"
        >
          Cancel
        </button>
        <button
          onclick={handleDelete}
          disabled={deleteInProgress}
          class="px-4 py-2 bg-red-600 text-white rounded-lg hover:bg-red-700 disabled:opacity-50"
        >
          {deleteInProgress ? 'Deleting...' : 'Delete'}
        </button>
      </div>
    </div>
  </div>
{/if}
