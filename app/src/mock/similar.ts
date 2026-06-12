import type { SimilarGroup } from '$lib/types';

// Mock similar images. Groups below the requested threshold are excluded,
// like the backend's similarity filter, so the threshold slider is testable
// in web mode (scores here: 0.95 and 0.92). Paths containing "empty-dir"
// return no groups, like scanning an empty or nonexistent directory.
export function mockFindSimilar(path: string, threshold: number): Promise<SimilarGroup[]> {
  if (path.includes('empty-dir')) {
    return new Promise((resolve) => {
      setTimeout(() => resolve([]), 100);
    });
  }
  const groups: SimilarGroup[] = [
    {
      similarity_score: 0.95,
      files: [
        {
          path: `${path}/photos/sunset1.jpg`,
          size: 3145728,
          modified: Date.now() - 86400000,
          file_type: "Image"
        },
        {
          path: `${path}/photos/sunset2.jpg`,
          size: 3200000,
          modified: Date.now() - 172800000,
          file_type: "Image"
        }
      ]
    },
    {
      similarity_score: 0.92,
      files: [
        {
          path: `${path}/screenshots/screen1.png`,
          size: 1048576,
          modified: Date.now() - 259200000,
          file_type: "Image"
        },
        {
          path: `${path}/screenshots/screen2.png`,
          size: 1100000,
          modified: Date.now() - 345600000,
          file_type: "Image"
        }
      ]
    }
  ];
  return new Promise((resolve) => {
    setTimeout(() => {
      resolve(groups.filter((g) => g.similarity_score >= threshold));
    }, 1200);
  });
}
