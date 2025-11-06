import type { DuplicateGroup } from '$lib/types';

// Mock duplicate files
export function mockFindDuplicates(path: string): Promise<DuplicateGroup[]> {
  return new Promise((resolve) => {
    setTimeout(() => {
      resolve([
        {
          hash: "abc123...",
          count: 3,
          total_size: 6291456,
          wasted_space: 4194304,
          files: [
            {
              path: `${path}/backup/image1.jpg`,
              size: 2097152,
              modified: Date.now() - 86400000,
              file_type: "Image"
            },
            {
              path: `${path}/photos/image1_copy.jpg`,
              size: 2097152,
              modified: Date.now() - 172800000,
              file_type: "Image"
            },
            {
              path: `${path}/old/image1_old.jpg`,
              size: 2097152,
              modified: Date.now() - 259200000,
              file_type: "Image"
            }
          ]
        },
        {
          hash: "def456...",
          count: 2,
          total_size: 20971520,
          wasted_space: 10485760,
          files: [
            {
              path: `${path}/docs/manual.pdf`,
              size: 10485760,
              modified: Date.now() - 345600000,
              file_type: "Document"
            },
            {
              path: `${path}/backup/manual_backup.pdf`,
              size: 10485760,
              modified: Date.now() - 432000000,
              file_type: "Document"
            }
          ]
        }
      ]);
    }, 1000);
  });
}
