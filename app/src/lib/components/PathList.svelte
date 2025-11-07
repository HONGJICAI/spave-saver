<script lang="ts">
  interface Props {
    paths: string[];
    onRemove?: (path: string) => void;
    onClearAll?: () => void;
  }
  
  let { paths, onRemove, onClearAll }: Props = $props();
</script>

{#if paths.length > 0}
  <div class="space-y-2">
    <div class="flex items-center justify-between">
      <span class="text-xs font-medium text-gray-600">
        {paths.length} path{paths.length !== 1 ? 's' : ''} selected
      </span>
      <button
        onclick={onClearAll}
        class="text-xs text-red-600 hover:text-red-700"
      >
        Clear all
      </button>
    </div>
    <div class="flex flex-wrap gap-2">
      {#each paths as path}
        <div class="flex items-center gap-1 px-2 py-1 bg-blue-100 text-blue-800 rounded-md text-xs">
          <span class="max-w-xs truncate" title={path}>{path}</span>
          <button
            onclick={() => onRemove?.(path)}
            class="hover:bg-blue-200 rounded p-0.5 ml-1"
            aria-label="Remove path"
            title="Remove"
          >
            <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
            </svg>
          </button>
        </div>
      {/each}
    </div>
  </div>
{/if}
