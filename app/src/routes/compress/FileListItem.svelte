<script lang="ts">
  import type { CompressibleFile } from "$lib/api";
  import { formatSize } from "$lib/utils/format";

  type Props = {
    file: CompressibleFile;
    isSelected: boolean;
    onToggle: (path: string) => void;
  };

  let { file, isSelected, onToggle }: Props = $props();
</script>

<div class="p-3 hover:bg-gray-50">
  <button onclick={() => onToggle(file.path)} class="flex items-start cursor-pointer w-full text-left">
    <input 
      type="checkbox" 
      checked={isSelected} 
      class="mt-1 mr-3 flex-shrink-0 pointer-events-none" 
      aria-label={`Select ${file.path}`} 
    />
    <div class="flex-1 min-w-0">
      <p class="text-sm font-medium text-gray-900 truncate">{file.path}</p>
      <div class="mt-1 flex flex-wrap gap-x-4 gap-y-1 text-xs text-gray-600">
        <span>Original: {formatSize(file.original_size)}</span>
        <span class="text-blue-600">→ {formatSize(file.estimated_compressed_size)}</span>
        <span class="text-green-600 font-medium">
          Savings: {formatSize(file.estimated_savings)} 
          ({(file.estimated_savings / file.original_size * 100).toFixed(1)}%)
        </span>
        <span class="text-purple-600">Plugin: {file.plugin_name}</span>
      </div>
    </div>
  </button>
</div>
