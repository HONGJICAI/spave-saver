<script lang="ts">
  import { onMount } from 'svelte';
  import { appState } from '$lib/stores/app';
  import {
    findSimilarMedia,
    deleteFiles,
    getConfig,
    type SimilarGroup,
    type MediaKind,
    type DeleteMode,
    type DeleteResult,
  } from '$lib/api';
  import { formatSize } from '$lib/utils/format';
  import StatCard from '$lib/components/StatCard.svelte';
  import Thumbnail from './Thumbnail.svelte';
  import {
    bestFile,
    selectAllButBest,
    fullySelectedGroups,
    keepBestPerGroup,
    applyDeletions,
    reclaimableSize,
    potentialSavings,
    type KeepStrategy,
  } from '$lib/utils/similar';
  import { loadFromSession, saveToSession, sessionKeys } from '$lib/utils/storage';

  // Session-cached results so leaving and returning keeps the scan intact.
  interface SimilarCache {
    groups: SimilarGroup[];
    hasScanned: boolean;
    selected: string[];
  }
  const cached = loadFromSession<SimilarCache | null>(sessionKeys.SIMILAR_RESULT, null);

  let loading = $state(false);
  let error = $state('');
  let hasScanned = $state(cached?.hasScanned ?? false);
  let groups = $state<SimilarGroup[]>(cached?.groups ?? []);
  let threshold = $state(0.9);
  let selected = $state<Set<string>>(new Set(cached?.selected ?? []));
  let sortBy = $state<'similarity' | 'savings'>('similarity');
  let keepStrategy = $state<KeepStrategy>('resolution');

  // Media-type selector. Images is the only implemented kind today; video
  // similarity needs ffmpeg (see crates/core/src/video_sim.rs), so its toggle
  // is disabled until that lands.
  let scanImages = $state(true);
  let mediaTypes = $derived<MediaKind[]>(scanImages ? ['Image'] : []);

  // Delete flow
  let showConfirm = $state(false);
  let deleteMode = $state<DeleteMode>('trash');
  let allowFullGroups = $state(false);
  let deleting = $state(false);
  let lastResults = $state<DeleteResult[] | null>(null);

  $effect(() => {
    saveToSession<SimilarCache>(sessionKeys.SIMILAR_RESULT, {
      groups,
      hasScanned,
      selected: Array.from(selected),
    });
  });

  let sizeByPath = $derived.by(() => {
    const map = new Map<string, number>();
    for (const group of groups) {
      for (const file of group.files) map.set(file.path, file.size);
    }
    return map;
  });

  let selectedSize = $derived(reclaimableSize(groups, selected));
  let potential = $derived(potentialSavings(groups));
  let endangeredGroups = $derived(fullySelectedGroups(groups, selected));

  let sortedGroups = $derived.by(() => {
    const sorted = [...groups];
    switch (sortBy) {
      case 'savings':
        return sorted.sort(
          (a, b) =>
            b.files.reduce((s, f) => s + f.size, 0) - b.files.reduce((m, f) => Math.max(m, f.size), 0) -
            (a.files.reduce((s, f) => s + f.size, 0) - a.files.reduce((m, f) => Math.max(m, f.size), 0))
        );
      case 'similarity':
      default:
        return sorted.sort((a, b) => b.similarity_score - a.similarity_score);
    }
  });

  // Seed the threshold and delete mode from the saved configuration so the
  // Settings page actually drives this page's defaults.
  onMount(async () => {
    try {
      const cfg = await getConfig();
      threshold = cfg.image_similarity_threshold;
      deleteMode = cfg.default_delete_mode;
    } catch {
      // Fall back to the in-component defaults if config can't be read
    }
  });

  async function handleScan() {
    if ($appState.scanPaths.length === 0) {
      error = 'Please add at least one directory to scan first';
      return;
    }
    if (mediaTypes.length === 0) {
      error = 'Select at least one media type to scan';
      return;
    }

    loading = true;
    appState.setBusy(true);
    error = '';
    groups = [];
    selected = new Set();
    lastResults = null;
    showConfirm = false;

    try {
      groups = await findSimilarMedia($appState.scanPaths, threshold, mediaTypes, $appState.filterConfig);
      hasScanned = true;
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to find similar media';
    } finally {
      loading = false;
      appState.setBusy(false);
    }
  }

  function toggleFile(path: string) {
    const next = new Set(selected);
    if (next.has(path)) next.delete(path);
    else next.add(path);
    selected = next;
  }

  function autoSelect() {
    selected = selectAllButBest(groups, keepStrategy);
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
    appState.setBusy(true);
    error = '';
    try {
      const results = await deleteFiles(Array.from(selected), deleteMode);
      lastResults = results;

      const deletedPaths = new Set(results.filter((r) => r.success).map((r) => r.path));
      groups = applyDeletions(groups, deletedPaths);

      // Keep only failed paths selected so the user can retry or inspect them
      selected = new Set(results.filter((r) => !r.success).map((r) => r.path));
      showConfirm = false;
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to delete files';
    } finally {
      deleting = false;
      appState.setBusy(false);
    }
  }

  function resolutionLabel(width?: number | null, height?: number | null): string {
    if (width == null || height == null) return 'unknown size';
    const mp = (width * height) / 1_000_000;
    return `${width}×${height} (${mp.toFixed(1)} MP)`;
  }

  let deletedCount = $derived(lastResults?.filter((r) => r.success).length ?? 0);
  let failedResults = $derived(lastResults?.filter((r) => !r.success) ?? []);
  let deletedSize = $derived(
    lastResults?.filter((r) => r.success).reduce((sum, r) => sum + (sizeByPath.get(r.path) ?? 0), 0) ?? 0
  );
</script>

<svelte:head>
  <title>Similar Media - Space-Saver</title>
</svelte:head>

<div>
  <h1 class="text-3xl font-bold text-gray-900 mb-2">🖼️ Find Similar Media</h1>
  <p class="text-gray-600 mb-6">Find visually similar images so you can keep the best copy and delete the rest.</p>

  <!-- Scan configuration -->
  <div class="bg-white rounded-lg shadow p-6 mb-6">
    <div class="flex flex-col gap-4">
      <div>
        <span class="block text-sm font-medium text-gray-700 mb-2">Scan for:</span>
        <div class="flex flex-wrap gap-4">
          <label class="flex items-center gap-2 cursor-pointer">
            <input type="checkbox" bind:checked={scanImages} class="rounded" />
            <span class="text-sm text-gray-800">Images</span>
          </label>
          <label class="flex items-center gap-2 cursor-not-allowed opacity-60" title="Video similarity is coming soon">
            <input type="checkbox" checked={false} disabled class="rounded" />
            <span class="text-sm text-gray-500">Videos <span class="text-xs">(coming soon)</span></span>
          </label>
        </div>
      </div>

      <div>
        <label for="threshold" class="block text-sm font-medium text-gray-700 mb-2">
          Similarity threshold: {(threshold * 100).toFixed(0)}%
        </label>
        <input
          id="threshold"
          type="range"
          min="0.5"
          max="1.0"
          step="0.05"
          bind:value={threshold}
          class="w-full h-2 bg-gray-200 rounded-lg appearance-none cursor-pointer"
        />
        <div class="flex justify-between text-xs text-gray-500 mt-1">
          <span>More matches (50%)</span>
          <span>Exact only (100%)</span>
        </div>
      </div>

      <button
        onclick={handleScan}
        disabled={loading || $appState.scanPaths.length === 0 || mediaTypes.length === 0}
        class="px-6 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:bg-gray-300 disabled:cursor-not-allowed w-full"
      >
        {loading ? '⏳ Scanning…' : '🔍 Scan for Similar Media'}
      </button>

      {#if error}
        <div class="p-4 bg-red-50 text-red-700 rounded-lg">⚠️ {error}</div>
      {/if}
    </div>
  </div>

  <!-- Last delete results -->
  {#if lastResults}
    <div class="bg-white rounded-lg shadow p-4 mb-6">
      <p class="text-sm text-gray-800">
        {#if deleteMode === 'trash'}
          ✅ Moved <strong>{deletedCount}</strong> file{deletedCount !== 1 ? 's' : ''} to the system trash
          ({formatSize(deletedSize)} — freed once the trash is emptied).
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

  {#if groups.length > 0}
    <div class="grid grid-cols-1 md:grid-cols-3 gap-4 mb-6">
      <StatCard label="Similar Groups" value={groups.length} icon="🖼️" color="yellow" />
      <StatCard label="Potential Savings" value={formatSize(potential)} icon="💾" color="green" />
      <StatCard label="Selected" value={`${selected.size} (${formatSize(selectedSize)})`} icon="✓" color="blue" />
    </div>

    <!-- Toolbar -->
    <div class="bg-white rounded-lg shadow p-4 mb-6 flex flex-wrap items-center gap-3">
      <span class="text-sm font-medium text-gray-700">Auto-select all but the best, keeping the</span>
      <select
        bind:value={keepStrategy}
        aria-label="Which copy to keep"
        class="px-3 py-1.5 text-sm border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
      >
        <option value="resolution">highest resolution</option>
        <option value="size">largest file</option>
        <option value="newest">newest file</option>
      </select>
      <button
        onclick={autoSelect}
        class="px-3 py-1.5 text-sm border border-blue-300 text-blue-700 rounded hover:bg-blue-50"
      >
        Select duplicates
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
        <option value="similarity">Similarity</option>
        <option value="savings">Potential savings</option>
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
              deleting would lose that image entirely.
            </p>
            <div class="mt-2 flex flex-wrap items-center gap-3">
              <button
                onclick={() => (selected = keepBestPerGroup(groups, selected, keepStrategy))}
                class="px-3 py-1.5 text-xs font-medium bg-amber-600 text-white rounded hover:bg-amber-700"
              >
                Keep the best copy in each group
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
            {deleting ? 'Deleting…' : deleteMode === 'trash' ? 'Move to Trash' : 'Delete Permanently'}
          </button>
        </div>
      </div>
    {/if}

    <!-- Groups -->
    <div class="space-y-6">
      {#each sortedGroups as group, idx (idx)}
        {@const keep = bestFile(group.files, keepStrategy)}
        <div class="bg-white rounded-lg shadow p-6">
          <div class="flex items-center justify-between mb-4 gap-2">
            <h3 class="text-lg font-semibold text-gray-900">
              {group.files.length} similar images
              <span class="text-sm font-normal text-gray-500">({(group.similarity_score * 100).toFixed(1)}% similar)</span>
            </h3>
            <button
              onclick={() => {
                const next = new Set(selected);
                for (const f of group.files) if (f.path !== keep.path) next.add(f.path);
                selected = next;
              }}
              class="text-sm text-blue-600 hover:text-blue-800 whitespace-nowrap"
            >
              Select duplicates (keep best)
            </button>
          </div>

          <div class="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-4">
            {#each group.files as file (file.path)}
              {@const isKeep = file.path === keep.path}
              <button
                type="button"
                onclick={() => toggleFile(file.path)}
                class="text-left border-2 rounded-lg p-3 transition-all {selected.has(file.path)
                  ? 'border-red-400 bg-red-50'
                  : 'border-gray-200 hover:border-gray-300'}"
              >
                <Thumbnail path={file.path} alt={file.path} />
                <div class="text-xs">
                  <p class="font-medium text-gray-900 truncate" title={file.path}>
                    {file.path.split('/').pop()}
                  </p>
                  <p class="text-gray-500 mt-1">{resolutionLabel(file.width, file.height)}</p>
                  <p class="text-gray-500">{formatSize(file.size)} • {new Date(file.modified * 1000).toLocaleDateString()}</p>
                  <div class="mt-2 flex items-center gap-1">
                    {#if isKeep}
                      <span class="inline-block px-2 py-0.5 bg-green-100 text-green-800 rounded">Best — keep</span>
                    {/if}
                    {#if selected.has(file.path)}
                      <span class="inline-block px-2 py-0.5 bg-red-100 text-red-800 rounded">Selected</span>
                    {/if}
                  </div>
                </div>
              </button>
            {/each}
          </div>
        </div>
      {/each}
    </div>
  {:else if !loading && hasScanned}
    <div class="bg-white rounded-lg shadow p-12 text-center text-gray-500">
      <p class="text-xl mb-2">No similar media found</p>
      <p class="text-sm">Nothing in the scanned paths looks alike at this threshold 🎉</p>
    </div>
  {:else if !loading}
    <div class="bg-white rounded-lg shadow p-12 text-center text-gray-500">
      <p class="text-xl mb-2">Ready to scan</p>
      <p class="text-sm">Scan a directory to find visually similar images</p>
    </div>
  {/if}
</div>
