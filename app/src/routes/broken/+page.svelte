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
  let sortBy = $state<'name' | 'size' | 'path'>('name');
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

  // Select-all that operates only on one section's rows, leaving the other
  // section's selection untouched.
  function toggleAllIn(items: BrokenFile[]) {
    const next = new Set(selectedPaths);
    const allSelected = items.length > 0 && items.every((i) => next.has(i.path));
    for (const item of items) {
      if (allSelected) {
        next.delete(item.path);
      } else {
        next.add(item.path);
      }
    }
    selectedPaths = next;
  }

  function clearSelectionIn(items: BrokenFile[]) {
    const next = new Set(selectedPaths);
    for (const item of items) next.delete(item.path);
    selectedPaths = next;
  }

  async function handleDelete() {
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

  function handleSort(column: 'name' | 'size' | 'path') {
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

  function sortList(items: BrokenFile[]): BrokenFile[] {
    const sorted = [...items];
    sorted.sort((a, b) => {
      let comparison = 0;
      switch (sortBy) {
        case 'name':
          comparison = getItemName(a.path).localeCompare(getItemName(b.path));
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
    return sorted;
  }

  // Corrupted files are deleted; misnamed files are renamed — never deleted,
  // since their content is valid.
  let corruptedItems = $derived(sortList(broken.filter((b) => b.category === 'corrupted')));
  let mismatchItems = $derived(sortList(broken.filter((b) => b.category === 'extension_mismatch')));
  let selectedCorrupted = $derived(corruptedItems.filter((b) => selectedPaths.has(b.path)));
  let selectedMismatch = $derived(mismatchItems.filter((b) => selectedPaths.has(b.path)));
</script>

<div>
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

  {#snippet headerCell(label: string, column: 'name' | 'size' | 'path')}
    <th
      class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider cursor-pointer hover:bg-gray-100"
      onclick={() => handleSort(column)}
    >
      {label}
      {#if sortBy === column}<span class="ml-1">{sortOrder === 'asc' ? '↑' : '↓'}</span>{/if}
    </th>
  {/snippet}

  {#snippet fileTable(items: BrokenFile[], showRenameHint: boolean)}
    <div class="bg-white rounded-lg shadow overflow-hidden">
      <table class="min-w-full divide-y divide-gray-200">
        <thead class="bg-gray-50">
          <tr>
            <th class="px-6 py-3 text-left">
              <input
                type="checkbox"
                checked={items.length > 0 && items.every((i) => selectedPaths.has(i.path))}
                onchange={() => toggleAllIn(items)}
                class="rounded border-gray-300"
              />
            </th>
            {@render headerCell('Name', 'name')}
            {@render headerCell('Size', 'size')}
            <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
              Reason
            </th>
            {@render headerCell('Path', 'path')}
          </tr>
        </thead>
        <tbody class="bg-white divide-y divide-gray-200">
          {#each items as item (item.path)}
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
                <span class="text-sm font-medium text-gray-900">{getItemName(item.path)}</span>
              </td>
              <td class="px-6 py-4 whitespace-nowrap">
                <span class="text-sm text-gray-600">{formatSize(item.size)}</span>
              </td>
              <td class="px-6 py-4">
                <span class="text-sm text-gray-700 truncate max-w-xs block" title={item.reason}>
                  {item.reason}
                </span>
                {#if showRenameHint && item.suggested_extension}
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
  {/snippet}

  {#if corruptedItems.length > 0}
    <div class="mb-8">
      <div class="bg-white rounded-lg shadow p-6 mb-4">
        <div class="flex justify-between items-center">
          <div>
            <h2 class="text-xl font-semibold">
              Corrupted files ({corruptedItems.length})
            </h2>
            <p class="text-gray-600 mt-1">
              Content can't be read as its format (truncated or damaged). These can be deleted.
            </p>
          </div>
          {#if selectedCorrupted.length > 0}
            <div class="flex gap-2">
              <button
                onclick={() => clearSelectionIn(corruptedItems)}
                class="px-4 py-2 border border-gray-300 rounded-lg hover:bg-gray-50"
              >
                Clear ({selectedCorrupted.length})
              </button>
              <button
                onclick={() => (showDeleteConfirm = true)}
                class="px-4 py-2 bg-red-600 text-white rounded-lg hover:bg-red-700"
              >
                Delete ({selectedCorrupted.length})
              </button>
            </div>
          {/if}
        </div>
      </div>
      {@render fileTable(corruptedItems, false)}
    </div>
  {/if}

  {#if mismatchItems.length > 0}
    <div class="mb-8">
      <div class="bg-white rounded-lg shadow p-6 mb-4">
        <div class="flex justify-between items-center">
          <div>
            <h2 class="text-xl font-semibold">
              Wrong extension ({mismatchItems.length})
            </h2>
            <p class="text-gray-600 mt-1">
              The content is valid but the file is misnamed. Rename to the matching extension — don't delete.
            </p>
          </div>
          {#if selectedMismatch.length > 0}
            <div class="flex gap-2">
              <button
                onclick={() => clearSelectionIn(mismatchItems)}
                class="px-4 py-2 border border-gray-300 rounded-lg hover:bg-gray-50"
              >
                Clear ({selectedMismatch.length})
              </button>
              <button
                onclick={handleFix}
                disabled={fixInProgress}
                class="px-4 py-2 bg-amber-600 text-white rounded-lg hover:bg-amber-700 disabled:opacity-50"
              >
                {fixInProgress ? 'Renaming...' : `Fix Extension (${selectedMismatch.length})`}
              </button>
            </div>
          {/if}
        </div>
      </div>
      {@render fileTable(mismatchItems, true)}
    </div>
  {/if}

  {#if broken.length === 0 && !loading}
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
