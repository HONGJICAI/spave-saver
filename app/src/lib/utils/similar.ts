import type { SimilarGroup, SimilarFile } from '$lib/types';

/**
 * Which copy to KEEP when auto-selecting the rest of a similar-group for
 * deletion. "resolution" keeps the largest pixel area (the best original),
 * "size" the largest byte size, "newest" the most recently modified.
 */
export type KeepStrategy = 'resolution' | 'size' | 'newest';

/** Higher rank = better candidate to keep. */
function fileRank(file: SimilarFile, strategy: KeepStrategy): number {
  switch (strategy) {
    case 'resolution':
      return (file.width ?? 0) * (file.height ?? 0);
    case 'size':
      return file.size;
    case 'newest':
      return file.modified;
  }
}

/** The single file to keep in a group under `strategy` (ties: the first). */
export function bestFile(files: SimilarFile[], strategy: KeepStrategy): SimilarFile {
  return files.reduce((best, f) => (fileRank(f, strategy) > fileRank(best, strategy) ? f : best));
}

/**
 * Select every file EXCEPT the best one in each group — i.e. the copies to
 * delete, keeping the best per group. Groups with fewer than 2 files are
 * skipped (nothing to deduplicate).
 */
export function selectAllButBest(groups: SimilarGroup[], strategy: KeepStrategy): Set<string> {
  const selected = new Set<string>();
  for (const group of groups) {
    if (group.files.length < 2) continue;
    const keep = bestFile(group.files, strategy);
    for (const f of group.files) {
      if (f.path !== keep.path) selected.add(f.path);
    }
  }
  return selected;
}

/**
 * Groups where EVERY file is selected — deleting would lose that content
 * entirely (no copy kept). Used to warn before a destructive delete.
 */
export function fullySelectedGroups(groups: SimilarGroup[], selected: Set<string>): SimilarGroup[] {
  return groups.filter((g) => g.files.length > 0 && g.files.every((f) => selected.has(f.path)));
}

/**
 * Deselect the best file in each fully-selected group, so at least one copy
 * survives. Other selections are left untouched.
 */
export function keepBestPerGroup(
  groups: SimilarGroup[],
  selected: Set<string>,
  strategy: KeepStrategy
): Set<string> {
  const next = new Set(selected);
  for (const g of fullySelectedGroups(groups, selected)) {
    next.delete(bestFile(g.files, strategy).path);
  }
  return next;
}

/**
 * Remove deleted paths from each group, then drop groups that no longer have
 * at least two files (a lone survivor is not "similar to" anything).
 */
export function applyDeletions(groups: SimilarGroup[], deleted: Set<string>): SimilarGroup[] {
  return groups
    .map((g) => ({ ...g, files: g.files.filter((f) => !deleted.has(f.path)) }))
    .filter((g) => g.files.length > 1);
}

/**
 * Bytes reclaimed by the current selection: the sum of selected files' sizes
 * (each selected file is a copy being removed).
 */
export function reclaimableSize(groups: SimilarGroup[], selected: Set<string>): number {
  let total = 0;
  for (const g of groups) {
    for (const f of g.files) {
      if (selected.has(f.path)) total += f.size;
    }
  }
  return total;
}

/**
 * Headline "potential savings": for each group, everything but its largest
 * file — the most you could reclaim while keeping one copy per group.
 */
export function potentialSavings(groups: SimilarGroup[]): number {
  let total = 0;
  for (const g of groups) {
    if (g.files.length < 2) continue;
    const groupTotal = g.files.reduce((s, f) => s + f.size, 0);
    const largest = g.files.reduce((m, f) => Math.max(m, f.size), 0);
    total += groupTotal - largest;
  }
  return total;
}
