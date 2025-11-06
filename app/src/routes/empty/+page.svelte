<script lang="ts">
  import { appState } from '$lib/stores/app';
  import { findEmptyFiles, deleteFiles } from '$lib/api';

  let loading = $state(false);
  let emptyFiles: string[] = $state([]);
  let selectedFiles = $state<Set<string>>(new Set());
  let showDeleteConfirm = $state(false);
  let deleteInProgress = $state(false);
  let sortBy = $state<'name' | 'path'>('name');
  let sortOrder = $state<'asc' | 'desc'>('asc');

  async function handleScan() {
    // Use scanPaths
    const paths = $appState.scanPaths;

    if (paths.length === 0) {
      appState.setError('Please enter at least one path to scan');
      return;
    }

    loading = true;
    appState.setError(null);

    try {
      emptyFiles = await findEmptyFiles(paths, $appState.filterConfig);
    } catch (err) {
      appState.setError(err instanceof Error ? err.message : 'Failed to find empty files');
    } finally {
      loading = false;
    }
  }

  function toggleFile(path: string) {
    const newSelected = new Set(selectedFiles);
    if (newSelected.has(path)) {
      newSelected.delete(path);
    } else {
      newSelected.add(path);
    }
    selectedFiles = newSelected;
  }

  function toggleAll() {
    if (selectedFiles.size === sortedFiles.length) {
      selectedFiles = new Set();
    } else {
      selectedFiles = new Set(sortedFiles);
    }
  }

  function clearSelection() {
    selectedFiles = new Set();
  }

  async function handleDelete() {
    if (selectedFiles.size === 0) return;

    deleteInProgress = true;
    try {
      await deleteFiles(Array.from(selectedFiles));
      emptyFiles = emptyFiles.filter(path => !selectedFiles.has(path));
      selectedFiles = new Set();
      showDeleteConfirm = false;
    } catch (err) {
      $appState.error = err instanceof Error ? err.message : 'Failed to delete files';
    } finally {
      deleteInProgress = false;
    }
  }

  function handleSort(column: 'name' | 'path') {
    if (sortBy === column) {
      sortOrder = sortOrder === 'asc' ? 'desc' : 'asc';
    } else {
      sortBy = column;
      sortOrder = 'asc';
    }
  }

  function getFileName(path: string): string {
    return path.split(/[/\\]/).pop() || path;
  }

  $effect(() => {
    // Computed sorted files
    sortedFiles;
  });

  let sortedFiles = $derived.by(() => {
    const files = [...emptyFiles];
    files.sort((a, b) => {
      let comparison = 0;
      switch (sortBy) {
        case 'name':
          comparison = getFileName(a).localeCompare(getFileName(b));
          break;
        case 'path':
          comparison = a.localeCompare(b);
          break;
      }
      return sortOrder === 'asc' ? comparison : -comparison;
    });
    return files;
  });
</script>

<div class="p-6">
  <div class="mb-6">
    <h1 class="text-3xl font-bold text-gray-900 mb-2">Empty Files Finder</h1>
    <p class="text-gray-600">Find and remove empty files (0 bytes)</p>
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
        Scan for Empty Files
      {/if}
    </button>
  </div>

  {#if emptyFiles.length > 0}
    <div class="bg-white rounded-lg shadow p-6 mb-6">
      <div class="flex justify-between items-center">
        <div>
          <h2 class="text-xl font-semibold">
            Found {emptyFiles.length} empty file{emptyFiles.length !== 1 ? 's' : ''}
          </h2>
          <p class="text-gray-600 mt-1">These files contain no data and can be safely removed</p>
        </div>
        {#if selectedFiles.size > 0}
          <div class="flex gap-2">
            <button
              onclick={clearSelection}
              class="px-4 py-2 border border-gray-300 rounded-lg hover:bg-gray-50"
            >
              Clear Selection ({selectedFiles.size})
            </button>
            <button
              onclick={() => showDeleteConfirm = true}
              class="px-4 py-2 bg-red-600 text-white rounded-lg hover:bg-red-700"
            >
              Delete Selected
            </button>
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
                checked={selectedFiles.size === sortedFiles.length && sortedFiles.length > 0}
                onchange={toggleAll}
                class="rounded border-gray-300"
              />
            </th>
            <th 
              class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider cursor-pointer hover:bg-gray-100"
              onclick={() => handleSort('name')}
            >
              Name
              {#if sortBy === 'name'}
                <span class="ml-1">{sortOrder === 'asc' ? '↑' : '↓'}</span>
              {/if}
            </th>
            <th 
              class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider cursor-pointer hover:bg-gray-100"
              onclick={() => handleSort('path')}
            >
              Path
              {#if sortBy === 'path'}
                <span class="ml-1">{sortOrder === 'asc' ? '↑' : '↓'}</span>
              {/if}
            </th>
          </tr>
        </thead>
        <tbody class="bg-white divide-y divide-gray-200">
          {#each sortedFiles as filePath}
            <tr 
              class="hover:bg-gray-50 cursor-pointer {selectedFiles.has(filePath) ? 'bg-blue-50' : ''}"
              onclick={() => toggleFile(filePath)}
            >
              <td class="px-6 py-4 whitespace-nowrap">
                <input
                  type="checkbox"
                  checked={selectedFiles.has(filePath)}
                  class="rounded border-gray-300"
                  onclick={(e) => e.stopPropagation()}
                  onchange={() => toggleFile(filePath)}
                />
              </td>
              <td class="px-6 py-4 whitespace-nowrap">
                <div class="flex items-center">
                  <svg class="w-5 h-5 text-gray-400 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"></path>
                  </svg>
                  <span class="text-sm font-medium text-gray-900">{getFileName(filePath)}</span>
                </div>
              </td>
              <td class="px-6 py-4">
                <span class="text-sm text-gray-600 truncate max-w-md block" title={filePath}>
                  {filePath}
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
      <p class="text-gray-500">No empty files found. Scan a directory to get started.</p>
    </div>
  {/if}
</div>

<!-- Delete Confirmation Dialog -->
{#if showDeleteConfirm}
  <div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
    <div class="bg-white rounded-lg p-6 max-w-md w-full mx-4">
      <h3 class="text-lg font-semibold mb-2">Confirm Deletion</h3>
      <p class="text-gray-600 mb-4">
        Are you sure you want to delete {selectedFiles.size} empty file{selectedFiles.size !== 1 ? 's' : ''}?
        This action cannot be undone.
      </p>
      <div class="flex gap-2 justify-end">
        <button
          onclick={() => showDeleteConfirm = false}
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
