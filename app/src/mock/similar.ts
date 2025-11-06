import type { SimilarGroup } from '$lib/types';

// Mock similar images
export function mockFindSimilar(path: string, threshold: number): Promise<SimilarGroup[]> {
  return new Promise((resolve) => {
    setTimeout(() => {
      resolve([
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
      ]);
    }, 1200);
  });
}
