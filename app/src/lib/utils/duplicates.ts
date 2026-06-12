/**
 * Selection helpers for the duplicates page
 */

import type { DuplicateGroup } from '../types';

/** Which copy to keep when auto-selecting duplicates for deletion */
export type KeepStrategy = 'newest' | 'oldest' | 'shortest-path';

/**
 * Auto-select files for deletion: in every group, keep exactly one file
 * (per the strategy) and select all the others.
 */
export function selectDuplicates(groups: DuplicateGroup[], strategy: KeepStrategy): Set<string> {
  const toDelete = new Set<string>();

  for (const group of groups) {
    if (group.files.length < 2) continue;

    let keepIndex = 0;
    for (let i = 1; i < group.files.length; i++) {
      const candidate = group.files[i];
      const current = group.files[keepIndex];
      const better =
        strategy === 'newest'
          ? candidate.modified > current.modified
          : strategy === 'oldest'
            ? candidate.modified < current.modified
            : candidate.path.length < current.path.length;
      if (better) keepIndex = i;
    }

    group.files.forEach((file, i) => {
      if (i !== keepIndex) toDelete.add(file.path);
    });
  }

  return toDelete;
}

/**
 * Groups in which EVERY file is selected — deleting the selection would
 * destroy all copies of that content.
 */
export function fullySelectedGroups(
  groups: DuplicateGroup[],
  selected: Set<string>
): DuplicateGroup[] {
  return groups.filter(
    (group) => group.files.length > 0 && group.files.every((f) => selected.has(f.path))
  );
}

/**
 * Deselect one file per fully-selected group so at least one copy survives.
 * Returns a new Set; the kept file is the first in each group.
 */
export function keepOnePerGroup(groups: DuplicateGroup[], selected: Set<string>): Set<string> {
  const result = new Set(selected);
  for (const group of fullySelectedGroups(groups, selected)) {
    result.delete(group.files[0].path);
  }
  return result;
}

/**
 * Remove deleted paths from the groups without rescanning: recompute the
 * per-group stats and drop groups that no longer have duplicates.
 */
export function applyDeletions(
  groups: DuplicateGroup[],
  deletedPaths: Set<string>
): DuplicateGroup[] {
  const updated: DuplicateGroup[] = [];

  for (const group of groups) {
    const files = group.files.filter((f) => !deletedPaths.has(f.path));
    if (files.length < 2) continue;

    const total_size = files.reduce((sum, f) => sum + f.size, 0);
    updated.push({
      ...group,
      files,
      count: files.length,
      total_size,
      wasted_space: total_size - files[0].size,
    });
  }

  return updated;
}
