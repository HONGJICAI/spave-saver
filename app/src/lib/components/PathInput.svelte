<script lang="ts">
  import { appState } from '$lib/stores/app';
  import type { PathValidationResult } from '$lib/utils/path';

  interface Props {
    onPathAdded?: (path: string, hasSubpaths: string[]) => void;
  }
  
  let { onPathAdded }: Props = $props();

  let inputPath = $state('');
  let pathValidation = $state<PathValidationResult | null>(null);

  // Validate path as user types
  $effect(() => {
    if (inputPath.trim()) {
      pathValidation = appState.validatePath(inputPath.trim());
    } else {
      pathValidation = null;
    }
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
          const pathsToProcess = Array.isArray(selected) ? selected : [selected];
          pathsToProcess.forEach(path => handlePathSelection(path));
          inputPath = '';
        }
      } catch (e) {
        console.error('Failed to open dialog:', e);
      }
    }
  }

  function handleAddPath() {
    if (inputPath.trim()) {
      handlePathSelection(inputPath.trim());
      inputPath = '';
      pathValidation = null;
    }
  }

  function handlePathSelection(path: string) {
    const validation = appState.validatePath(path);
    
    if (validation.isValid || (validation.hasSubpaths.length > 0 && !validation.isDuplicate && validation.isSubpathOf.length === 0)) {
      onPathAdded?.(path, validation.hasSubpaths);
    } else {
      // Let parent handle invalid paths
      onPathAdded?.(path, []);
    }
  }
</script>

<div class="flex gap-2">
  <div class="flex-1">
    <input
      type="text"
      bind:value={inputPath}
      onkeydown={(e) => e.key === 'Enter' && handleAddPath()}
      placeholder="Add directory path..."
      class="w-full px-3 py-2 text-sm border-2 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent transition-colors"
      class:border-gray-300={!pathValidation || (pathValidation.isValid && pathValidation.warnings.length === 0)}
      class:border-yellow-400={pathValidation && pathValidation.warnings.length > 0 && !pathValidation.isDuplicate && pathValidation.isSubpathOf.length === 0}
      class:border-red-500={pathValidation && (pathValidation.isDuplicate || pathValidation.isSubpathOf.length > 0)}
    />
    {#if pathValidation && pathValidation.warnings.length > 0}
      <div class="mt-2 space-y-1">
        {#each pathValidation.warnings as warning}
          <div class="flex items-start gap-2 p-2 rounded-md"
               class:bg-yellow-50={!pathValidation.isDuplicate && pathValidation.isSubpathOf.length === 0}
               class:bg-red-50={pathValidation.isDuplicate || pathValidation.isSubpathOf.length > 0}>
            <svg class="w-4 h-4 mt-0.5 flex-shrink-0" 
                 class:text-yellow-600={!pathValidation.isDuplicate && pathValidation.isSubpathOf.length === 0}
                 class:text-red-600={pathValidation.isDuplicate || pathValidation.isSubpathOf.length > 0}
                 fill="currentColor" viewBox="0 0 20 20">
              <path fill-rule="evenodd" d="M8.257 3.099c.765-1.36 2.722-1.36 3.486 0l5.58 9.92c.75 1.334-.213 2.98-1.742 2.98H4.42c-1.53 0-2.493-1.646-1.743-2.98l5.58-9.92zM11 13a1 1 0 11-2 0 1 1 0 012 0zm-1-8a1 1 0 00-1 1v3a1 1 0 002 0V6a1 1 0 00-1-1z" clip-rule="evenodd"></path>
            </svg>
            <span class="text-xs font-medium"
                  class:text-yellow-800={!pathValidation.isDuplicate && pathValidation.isSubpathOf.length === 0}
                  class:text-red-800={pathValidation.isDuplicate || pathValidation.isSubpathOf.length > 0}>
              {warning}
            </span>
          </div>
        {/each}
      </div>
    {/if}
  </div>
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
    disabled={!inputPath.trim() || (pathValidation !== null && (pathValidation.isDuplicate || pathValidation.isSubpathOf.length > 0))}
    class="px-3 py-2 bg-blue-600 text-white text-sm rounded-lg hover:bg-blue-700 disabled:bg-gray-300 disabled:cursor-not-allowed transition-colors"
    title={pathValidation?.isDuplicate ? "Path already exists" : pathValidation?.isSubpathOf.length ? "Path is redundant" : "Add path"}
  >
    Add
  </button>
</div>
