<script lang="ts">
  import { appState } from '$lib/stores/app';
  import { findSimilarImages, deleteFiles } from '$lib/api';
  import type { SimilarGroup } from '$lib/api';
  import { formatSize } from '$lib/utils/format';

  let loading = $state(false);
  let groups: SimilarGroup[] = $state([]);
  let threshold = $state(0.9);
  let selectedFiles = $state<Set<string>>(new Set());
  let showDeleteConfirm = $state(false);
  let deleteInProgress = $state(false);

  async function handleScan() {
    // Use scanPaths
    const paths = $appState.scanPaths;

    if (paths.length === 0) {
      appState.setError('Please select at least one directory first');
      return;
    }

    loading = true;
    appState.setError(null);

    try {
      groups = await findSimilarImages(paths, threshold, $appState.filterConfig);
    } catch (err) {
      appState.setError(err instanceof Error ? err.message : 'Failed to find similar images');
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

  function selectAllInGroup(group: SimilarGroup, exceptFirst: boolean = true) {
    const newSelected = new Set(selectedFiles);
    const filesToSelect = exceptFirst ? group.files.slice(1) : group.files;
    filesToSelect.forEach(file => newSelected.add(file.path));
    selectedFiles = newSelected;
  }

  function clearSelection() {
    selectedFiles = new Set();
  }

  async function handleDelete() {
    if (selectedFiles.size === 0) return;

    deleteInProgress = true;
    try {
      await deleteFiles(Array.from(selectedFiles));
      // Remove deleted files from groups
      groups = groups.map(group => ({
        ...group,
        files: group.files.filter(f => !selectedFiles.has(f.path))
      })).filter(group => group.files.length > 1);
      
      selectedFiles = new Set();
      showDeleteConfirm = false;
    } catch (err) {
      $appState.error = err instanceof Error ? err.message : 'Failed to delete files';
    } finally {
      deleteInProgress = false;
    }
  }

  function getTotalWaste(): number {
    return groups.reduce((total, group) => {
      // Each group wastes space equal to (n-1) copies of the file
      return total + (group.files[0].size * (group.files.length - 1));
    }, 0);
  }
</script>

<div class="p-6">
  <div class="mb-6">
    <h1 class="text-3xl font-bold text-gray-900 mb-2">Similar Images Finder</h1>
    <p class="text-gray-600">Find visually similar images that may be duplicates</p>
  </div>

  {#if $appState.error}
    <div class="bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded mb-4">
      {$appState.error}
    </div>
  {/if}

  <div class="bg-white rounded-lg shadow p-6 mb-6">
    <div class="flex items-center gap-4">
      <div class="flex-1">
        <label class="block text-sm font-medium text-gray-700 mb-2">
          Similarity Threshold: {(threshold * 100).toFixed(0)}%
        </label>
        <input 
          type="range" 
          min="0.7" 
          max="1.0" 
          step="0.05" 
          bind:value={threshold}
          class="w-full h-2 bg-gray-200 rounded-lg appearance-none cursor-pointer"
        />
        <div class="flex justify-between text-xs text-gray-500 mt-1">
          <span>More matches (70%)</span>
          <span>Exact only (100%)</span>
        </div>
      </div>
      <button
        onclick={handleScan}
        disabled={loading || $appState.scanPaths.length === 0}
        class="px-6 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:bg-gray-300 disabled:cursor-not-allowed whitespace-nowrap"
      >
        {loading ? 'Scanning...' : 'Scan for Similar Images'}
      </button>
    </div>
  </div>

  {#if groups.length > 0}
    <div class="bg-white rounded-lg shadow p-6 mb-6">
      <div class="flex justify-between items-center">
        <div>
          <h2 class="text-xl font-semibold">
            Found {groups.length} groups of similar images
          </h2>
          <p class="text-gray-600 mt-1">
            Potential space savings: <span class="font-semibold text-green-600">{formatSize(getTotalWaste())}</span>
          </p>
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

    <div class="space-y-6">
      {#each groups as group, idx}
        <div class="bg-white rounded-lg shadow p-6">
          <div class="flex justify-between items-center mb-4">
            <h3 class="text-lg font-semibold text-gray-900">
              Group {idx + 1} - {group.files.length} similar images ({(group.similarity * 100).toFixed(1)}% similar)
            </h3>
            <button
              onclick={() => selectAllInGroup(group, true)}
              class="text-sm text-blue-600 hover:text-blue-800"
            >
              Select duplicates (keep first)
            </button>
          </div>

          <div class="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-4">
            {#each group.files as file, fileIdx}
              <div 
                class="border-2 rounded-lg p-3 cursor-pointer transition-all {selectedFiles.has(file.path) ? 'border-blue-500 bg-blue-50' : 'border-gray-200 hover:border-gray-300'}"
                onclick={() => toggleFile(file.path)}
              >
                <div class="aspect-square bg-gray-200 rounded mb-2 flex items-center justify-center">
                  <svg class="w-16 h-16 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z"></path>
                  </svg>
                </div>
                <div class="text-xs">
                  <p class="font-medium text-gray-900 truncate" title={file.name}>{file.name}</p>
                  <p class="text-gray-500 mt-1">{formatSize(file.size)}</p>
                  {#if fileIdx === 0}
                    <span class="inline-block mt-2 px-2 py-1 text-xs bg-green-100 text-green-800 rounded">Original</span>
                  {/if}
                </div>
              </div>
            {/each}
          </div>
        </div>
      {/each}
    </div>
  {:else if !loading}
    <div class="bg-white rounded-lg shadow p-12 text-center">
      <svg class="w-16 h-16 text-gray-400 mx-auto mb-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z"></path>
      </svg>
      <p class="text-gray-500">No similar images found. Scan a directory to get started.</p>
    </div>
  {/if}
</div>

<!-- Delete Confirmation Dialog -->
{#if showDeleteConfirm}
  <div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
    <div class="bg-white rounded-lg p-6 max-w-md w-full mx-4">
      <h3 class="text-lg font-semibold mb-2">Confirm Deletion</h3>
      <p class="text-gray-600 mb-4">
        Are you sure you want to delete {selectedFiles.size} file{selectedFiles.size !== 1 ? 's' : ''}?
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
