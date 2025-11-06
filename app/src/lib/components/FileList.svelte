<script lang="ts">
  import { formatSize, formatDate } from '$lib/utils/format';
  
  export let files: Array<{
    path: string;
    size: number;
    modified: number;
    file_type: string;
  }> = [];
  
  export let selectable = false;
  export let selectedFiles: string[] = [];
  
  function toggleSelection(path: string) {
    if (!selectable) return;
    
    if (selectedFiles.includes(path)) {
      selectedFiles = selectedFiles.filter(p => p !== path);
    } else {
      selectedFiles = [...selectedFiles, path];
    }
  }
  
  function getTypeColor(type: string): string {
    switch (type) {
      case 'Image': return 'text-blue-600';
      case 'Video': return 'text-purple-600';
      case 'Document': return 'text-green-600';
      case 'Archive': return 'text-yellow-600';
      default: return 'text-gray-600';
    }
  }
</script>

<div class="bg-white rounded-lg shadow overflow-hidden">
  <div class="overflow-x-auto">
    <table class="w-full">
      <thead class="bg-gray-50 border-b">
        <tr>
          {#if selectable}
            <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase w-12">
              <input type="checkbox" class="rounded" />
            </th>
          {/if}
          <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase">
            Path
          </th>
          <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase w-32">
            Size
          </th>
          <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase w-32">
            Type
          </th>
          <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase w-48">
            Modified
          </th>
        </tr>
      </thead>
      <tbody class="divide-y divide-gray-200">
        {#each files as file}
          <tr 
            class="hover:bg-gray-50 transition-colors {selectable ? 'cursor-pointer' : ''}"
            on:click={() => toggleSelection(file.path)}
          >
            {#if selectable}
              <td class="px-4 py-3">
                <input
                  type="checkbox"
                  class="rounded"
                  checked={selectedFiles.includes(file.path)}
                  on:click|stopPropagation
                />
              </td>
            {/if}
            <td class="px-4 py-3 text-sm text-gray-900 font-mono truncate max-w-md">
              {file.path}
            </td>
            <td class="px-4 py-3 text-sm text-gray-600">
              {formatSize(file.size)}
            </td>
            <td class="px-4 py-3 text-sm {getTypeColor(file.file_type)}">
              {file.file_type}
            </td>
            <td class="px-4 py-3 text-sm text-gray-600">
              {formatDate(file.modified)}
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
  
  {#if files.length === 0}
    <div class="p-8 text-center text-gray-500">
      No files to display
    </div>
  {/if}
</div>
