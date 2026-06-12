import { describe, it, expect } from 'vitest';
import {
  selectDuplicates,
  fullySelectedGroups,
  keepOnePerGroup,
  applyDeletions,
} from './duplicates';
import type { DuplicateGroup } from '../types';

function group(hash: string, files: Array<[path: string, size: number, modified: number]>): DuplicateGroup {
  const fileInfos = files.map(([path, size, modified]) => ({
    path,
    size,
    modified,
    file_type: 'Document',
  }));
  const total_size = fileInfos.reduce((s, f) => s + f.size, 0);
  return {
    hash,
    files: fileInfos,
    count: fileInfos.length,
    total_size,
    wasted_space: total_size - (fileInfos[0]?.size ?? 0),
  };
}

describe('duplicates selection utils', () => {
  const groups: DuplicateGroup[] = [
    group('aaa', [
      ['/docs/old.txt', 100, 1000],
      ['/docs/new.txt', 100, 2000],
      ['/backup/deeply/nested/copy.txt', 100, 1500],
    ]),
    group('bbb', [
      ['/pics/a.jpg', 500, 50],
      ['/pics/b.jpg', 500, 60],
    ]),
  ];

  describe('selectDuplicates', () => {
    it('keeps the newest file per group', () => {
      const toDelete = selectDuplicates(groups, 'newest');
      expect(toDelete.has('/docs/new.txt')).toBe(false);
      expect(toDelete.has('/docs/old.txt')).toBe(true);
      expect(toDelete.has('/backup/deeply/nested/copy.txt')).toBe(true);
      expect(toDelete.has('/pics/b.jpg')).toBe(false);
      expect(toDelete.has('/pics/a.jpg')).toBe(true);
      expect(toDelete.size).toBe(3);
    });

    it('keeps the oldest file per group', () => {
      const toDelete = selectDuplicates(groups, 'oldest');
      expect(toDelete.has('/docs/old.txt')).toBe(false);
      expect(toDelete.has('/pics/a.jpg')).toBe(false);
      expect(toDelete.size).toBe(3);
    });

    it('keeps the shortest path per group', () => {
      const toDelete = selectDuplicates(groups, 'shortest-path');
      expect(toDelete.has('/docs/old.txt')).toBe(false);
      expect(toDelete.has('/backup/deeply/nested/copy.txt')).toBe(true);
    });

    it('always leaves at least one file unselected per group', () => {
      for (const strategy of ['newest', 'oldest', 'shortest-path'] as const) {
        const toDelete = selectDuplicates(groups, strategy);
        for (const g of groups) {
          const remaining = g.files.filter((f) => !toDelete.has(f.path));
          expect(remaining.length).toBeGreaterThanOrEqual(1);
        }
      }
    });
  });

  describe('fullySelectedGroups / keepOnePerGroup', () => {
    it('detects groups where every copy is selected', () => {
      const selected = new Set(['/pics/a.jpg', '/pics/b.jpg', '/docs/old.txt']);
      const endangered = fullySelectedGroups(groups, selected);
      expect(endangered.map((g) => g.hash)).toEqual(['bbb']);
    });

    it('keepOnePerGroup deselects one file per endangered group', () => {
      const selected = new Set(['/pics/a.jpg', '/pics/b.jpg']);
      const fixed = keepOnePerGroup(groups, selected);
      expect(fixed.size).toBe(1);
      expect(fullySelectedGroups(groups, fixed)).toHaveLength(0);
    });
  });

  describe('applyDeletions', () => {
    it('removes deleted files and recomputes group stats', () => {
      const updated = applyDeletions(groups, new Set(['/docs/old.txt']));
      const docs = updated.find((g) => g.hash === 'aaa')!;
      expect(docs.count).toBe(2);
      expect(docs.total_size).toBe(200);
      expect(docs.wasted_space).toBe(100);
    });

    it('dissolves groups left without duplicates', () => {
      const updated = applyDeletions(groups, new Set(['/pics/a.jpg']));
      expect(updated.find((g) => g.hash === 'bbb')).toBeUndefined();
      expect(updated.find((g) => g.hash === 'aaa')).toBeDefined();
    });
  });
});
