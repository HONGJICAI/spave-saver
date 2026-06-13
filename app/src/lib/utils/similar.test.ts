import { describe, it, expect } from 'vitest';
import type { SimilarGroup } from '$lib/types';
import {
  bestFile,
  selectAllButBest,
  fullySelectedGroups,
  keepBestPerGroup,
  applyDeletions,
  reclaimableSize,
  potentialSavings,
} from './similar';

const file = (path: string, size: number, width: number, height: number, modified = 0) => ({
  path,
  size,
  modified,
  width,
  height,
});

// A 0.95 group of three at mixed resolutions, plus a 0.9 pair.
const groups = (): SimilarGroup[] => [
  {
    media_kind: 'Image',
    similarity_score: 0.95,
    files: [
      file('/a/big.png', 1000, 3840, 2160, 100), // largest size & resolution
      file('/a/mid.png', 500, 1280, 720, 300), // newest
      file('/a/small.png', 200, 640, 360, 200),
    ],
  },
  {
    media_kind: 'Image',
    similarity_score: 0.9,
    files: [file('/b/x.png', 800, 1920, 1080, 10), file('/b/y.png', 400, 1920, 1080, 20)],
  },
];

describe('similar selection helpers', () => {
  it('bestFile picks by the chosen strategy', () => {
    const g = groups()[0];
    expect(bestFile(g.files, 'resolution').path).toBe('/a/big.png');
    expect(bestFile(g.files, 'size').path).toBe('/a/big.png');
    expect(bestFile(g.files, 'newest').path).toBe('/a/mid.png');
  });

  it('bestFile treats missing dimensions as zero area', () => {
    const files = [file('/a/known.png', 1, 100, 100), { path: '/a/unknown.png', size: 5, modified: 0 }];
    expect(bestFile(files, 'resolution').path).toBe('/a/known.png');
  });

  it('selectAllButBest keeps one per group and selects the rest', () => {
    const sel = selectAllButBest(groups(), 'resolution');
    // keeps /a/big.png and /b/x.png, selects the other three
    expect(sel).toEqual(new Set(['/a/mid.png', '/a/small.png', '/b/y.png']));
  });

  it('selectAllButBest skips groups with fewer than two files', () => {
    const lonely: SimilarGroup[] = [
      { media_kind: 'Image', similarity_score: 1, files: [file('/solo.png', 1, 10, 10)] },
    ];
    expect(selectAllButBest(lonely, 'size').size).toBe(0);
  });

  it('fullySelectedGroups flags only groups with every file selected', () => {
    const gs = groups();
    const selected = new Set(['/b/x.png', '/b/y.png']);
    const flagged = fullySelectedGroups(gs, selected);
    expect(flagged).toHaveLength(1);
    expect(flagged[0].similarity_score).toBe(0.9);
  });

  it('keepBestPerGroup deselects the best file of fully-selected groups', () => {
    const gs = groups();
    const selected = new Set(['/b/x.png', '/b/y.png']);
    const next = keepBestPerGroup(gs, selected, 'size');
    // /b/x.png is larger -> kept (deselected); /b/y.png stays selected
    expect(next.has('/b/x.png')).toBe(false);
    expect(next.has('/b/y.png')).toBe(true);
  });

  it('reclaimableSize sums selected file sizes', () => {
    const selected = new Set(['/a/mid.png', '/a/small.png']);
    expect(reclaimableSize(groups(), selected)).toBe(700);
  });

  it('potentialSavings sums everything but the largest file per group', () => {
    // group A: 1700 total - 1000 largest = 700; group B: 1200 - 800 = 400
    expect(potentialSavings(groups())).toBe(1100);
  });

  it('applyDeletions drops removed files and groups falling below two', () => {
    const remaining = applyDeletions(groups(), new Set(['/b/y.png', '/a/small.png']));
    // group B now has one file -> dropped; group A keeps two
    expect(remaining).toHaveLength(1);
    expect(remaining[0].files.map((f) => f.path)).toEqual(['/a/big.png', '/a/mid.png']);
  });
});
