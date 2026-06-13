<script lang="ts">
  import { validatePath, type PathValidationResult } from '$lib/utils/path';
  import PathInput from './PathInput.svelte';
  import PathList from './PathList.svelte';
  import InvalidPathList from './InvalidPathList.svelte';

  interface Props {
    /** Current list of paths */
    paths: string[];
    /** Replace the whole list (used when adding, which may drop redundant subpaths) */
    onSetPaths: (paths: string[]) => void;
    /** Remove a single path from the list */
    onRemove: (path: string) => void;
    /** Clear the whole list */
    onClearAll: () => void;
    placeholder?: string;
  }

  let { paths, onSetPaths, onRemove, onClearAll, placeholder }: Props = $props();

  interface InvalidPath {
    path: string;
    validation: PathValidationResult;
  }
  let invalidPaths = $state<InvalidPath[]>([]);

  // Same logic as the Scan Paths selector: a valid path (or one that only makes
  // existing entries redundant) is added, pushing any now-redundant subpaths to
  // the invalid list; duplicates / already-covered paths are rejected instead.
  function handlePathAdded(path: string) {
    const validation = validatePath(path, paths);

    if (
      validation.isValid ||
      (validation.hasSubpaths.length > 0 &&
        !validation.isDuplicate &&
        validation.isSubpathOf.length === 0)
    ) {
      const newPaths = [...paths.filter((p) => !validation.hasSubpaths.includes(p)), path];

      validation.hasSubpaths.forEach((subpath) => {
        const subpathValidation = validatePath(subpath, newPaths);
        if (!invalidPaths.some((ip) => ip.path === subpath)) {
          invalidPaths = [...invalidPaths, { path: subpath, validation: subpathValidation }];
        }
      });

      onSetPaths(newPaths);
    } else if (!invalidPaths.some((ip) => ip.path === path)) {
      invalidPaths = [...invalidPaths, { path, validation }];
    }
  }

  function removeInvalidPath(path: string) {
    invalidPaths = invalidPaths.filter((ip) => ip.path !== path);
  }

  function clearInvalidPaths() {
    invalidPaths = [];
  }

  function handleClearAll() {
    onClearAll();
    invalidPaths = [];
  }
</script>

<div class="space-y-3">
  <PathInput existingPaths={paths} {placeholder} onPathAdded={handlePathAdded} />

  <PathList {paths} {onRemove} onClearAll={handleClearAll} />

  <InvalidPathList {invalidPaths} onRemove={removeInvalidPath} onClearAll={clearInvalidPaths} />
</div>
