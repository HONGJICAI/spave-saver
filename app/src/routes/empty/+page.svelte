<script lang="ts">
  import { appState } from '$lib/stores/app';
  import { findEmptyItems, deleteFiles } from '$lib/api';

  type ItemKind = 'file' | 'folder';
  interface EmptyItem {
    path: string;
    kind: ItemKind;
  }

  let loading = $state(false);
  let emptyFiles: string[] = $state([]);
  let emptyFolders: string[] = $state([]);
  let scanned = $state(false);
  let selectedPaths = $state<Set<string>>(new Set());
  let showDeleteConfirm = $state(false);
  let deleteInProgress = $state(false);
  let sortBy = $state<'name' | 'path' | 'type'>('name');
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
      const result = await findEmptyItems(paths, $appState.filterConfig);
      emptyFiles = result.empty_files;
      emptyFolders = result.empty_folders;
      selectedPaths = new Set();
      scanned = true;
    } catch (err) {
      appState.setError(err instanceof Error ? err.message : 'Failed to find empty files and folders');
    } finally {
      loading = false;
    }
  }

  function toggleItem(path: string) {
    const newSelected = new Set(selectedPaths);
    if (newSelected.has(path)) {
      newSelected.delete(path);
    } else {
      newSelected.add(path);
    }
    selectedPaths = newSelected;
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
    if (selectedPaths.size === 0) return;

    deleteInProgress = true;
    try {
      // Defaults to the system trash; only successfully removed items leave
      // the list. The backend refuses folders that gained files after the scan.
      const results = await deleteFiles(Array.from(selectedPaths));
      const deleted = new Set(results.filter(r => r.success).map(r => r.path));
      const failed = results.filter(r => !r.success);
      emptyFiles = emptyFiles.filter(path => !deleted.has(path));
      emptyFolders = emptyFolders.filter(path => !deleted.has(path));
      selectedPaths = new Set();
      showDeleteConfirm = false;
      if (failed.length > 0) {
        $appState.error = `Failed to delete ${failed.length} item(s): ${failed[0].error ?? 'unknown error'}`;
      }
    } catch (err) {
      $appState.error = err instanceof Error ? err.message : 'Failed to delete items';
    } finally {
      deleteInProgress = false;
    }
  }

  function handleSort(column: 'name' | 'path' | 'type') {
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

  let selectedFileCount = $derived(emptyFiles.filter((path) => selectedPaths.has(path)).length);
  let selectedFolderCount = $derived(emptyFolders.filter((path) => selectedPaths.has(path)).length);

  let sortedItems = $derived.by(() => {
    const items: EmptyItem[] = [
      ...emptyFiles.map((path): EmptyItem => ({ path, kind: 'file' })),
      ...emptyFolders.map((path): EmptyItem => ({ path, kind: 'folder' }))
    ];
    items.sort((a, b) => {
      let comparison = 0;
      switch (sortBy) {
        case 'name':
          comparison = getItemName(a.path).localeCompare(getItemName(b.path));
          break;
        case 'path':
          comparison = a.path.localeCompare(b.path);
          break;
        case 'type':
          comparison = a.kind.localeCompare(b.kind) || a.path.localeCompare(b.path);
          break;
      }
      return sortOrder === 'asc' ? comparison : -comparison;
    });
    return items;
  });
</script>

<div>
  <div class="mb-6">
    <h1 class="text-3xl font-bold text-gray-900 mb-2">Empty Files & Folders Finder</h1>
    <p class="text-gray-600">Find and remove empty files (0 bytes) and folders that contain no files</p>
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
        Scan for Empty Files & Folders
      {/if}
    </button>
  </div>

  {#if sortedItems.length > 0}
    <div class="bg-white rounded-lg shadow p-6 mb-6">
      <div class="flex justify-between items-center">
        <div>
          <h2 class="text-xl font-semibold">
            Found {emptyFiles.length} empty file{emptyFiles.length !== 1 ? 's' : ''}
            and {emptyFolders.length} empty folder{emptyFolders.length !== 1 ? 's' : ''}
          </h2>
          <p class="text-gray-600 mt-1">These items contain no data and can be safely removed</p>
        </div>
        {#if selectedPaths.size > 0}
          <div class="flex gap-2">
            <button
              onclick={clearSelection}
              class="px-4 py-2 border border-gray-300 rounded-lg hover:bg-gray-50"
            >
              Clear Selection ({selectedPaths.size})
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
                checked={selectedPaths.size === sortedItems.length && sortedItems.length > 0}
                onchange={toggleAll}
                class="rounded border-gray-300"
              />
            </th>
            <th
              class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider cursor-pointer hover:bg-gray-100"
              onclick={() => handleSort('type')}
            >
              Type
              {#if sortBy === 'type'}
                <span class="ml-1">{sortOrder === 'asc' ? '↑' : '↓'}</span>
              {/if}
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
                {#if item.kind === 'folder'}
                  <span class="inline-flex items-center px-2 py-1 rounded-full text-xs font-medium bg-amber-100 text-amber-800">
                    Folder
                  </span>
                {:else}
                  <span class="inline-flex items-center px-2 py-1 rounded-full text-xs font-medium bg-blue-100 text-blue-800">
                    File
                  </span>
                {/if}
              </td>
              <td class="px-6 py-4 whitespace-nowrap">
                <div class="flex items-center">
                  {#if item.kind === 'folder'}
                    <svg class="w-5 h-5 text-amber-400 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"></path>
                    </svg>
                  {:else}
                    <svg class="w-5 h-5 text-gray-400 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"></path>
                    </svg>
                  {/if}
                  <span class="text-sm font-medium text-gray-900">{getItemName(item.path)}</span>
                </div>
              </td>
              <td class="px-6 py-4">
                <span class="text-sm text-gray-600 truncate max-w-md block" title={item.path}>
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
          ? 'No empty files or folders found.'
          : 'No empty files or folders found. Scan a directory to get started.'}
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
        Are you sure you want to delete
        {#if selectedFileCount > 0}{selectedFileCount} empty file{selectedFileCount !== 1 ? 's' : ''}{/if}{#if selectedFileCount > 0 && selectedFolderCount > 0}{' and '}{/if}{#if selectedFolderCount > 0}{selectedFolderCount} empty folder{selectedFolderCount !== 1 ? 's' : ''}{/if}?
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
