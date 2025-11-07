<script lang="ts">
  import type { PathValidationResult } from '$lib/utils/path';

  interface InvalidPath {
    path: string;
    validation: PathValidationResult;
  }

  interface Props {
    invalidPaths: InvalidPath[];
    onRemove?: (path: string) => void;
    onClearAll?: () => void;
  }
  
  let { invalidPaths, onRemove, onClearAll }: Props = $props();
</script>

{#if invalidPaths.length > 0}
  <div class="space-y-2">
    <div class="flex items-center justify-between">
      <span class="text-xs font-medium text-red-600">
        {invalidPaths.length} invalid path{invalidPaths.length !== 1 ? 's' : ''} (not added)
      </span>
      <button
        onclick={onClearAll}
        class="text-xs text-red-600 hover:text-red-700"
      >
        Dismiss all
      </button>
    </div>
    <div class="space-y-2">
      {#each invalidPaths as { path, validation }}
        <div class="bg-red-50 border border-red-200 rounded-lg p-2">
          <div class="flex items-start gap-2">
            <svg class="w-4 h-4 text-red-600 flex-shrink-0 mt-0.5" fill="currentColor" viewBox="0 0 20 20">
              <path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 1 0 00-1.414-1.414L10 8.586 8.707 7.293z" clip-rule="evenodd"></path>
            </svg>
            <div class="flex-1 min-w-0">
              <p class="text-xs font-mono text-red-900 break-all">{path}</p>
              <div class="mt-1 space-y-0.5">
                {#each validation.warnings as warning}
                  <p class="text-xs text-red-700">{warning}</p>
                {/each}
              </div>
            </div>
            <button
              onclick={() => onRemove?.(path)}
              class="hover:bg-red-100 rounded p-0.5 flex-shrink-0"
              aria-label="Dismiss warning"
              title="Dismiss"
            >
              <svg class="w-3 h-3 text-red-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
              </svg>
            </button>
          </div>
        </div>
      {/each}
    </div>
  </div>
{/if}
