import type { DuplicateGroup } from '$lib/types';

// Unix seconds, matching the backend's FileInfo.modified
const now = Math.floor(Date.now() / 1000);
const DAY = 86400;

// Mock duplicate files
export function mockFindDuplicates(path: string): Promise<DuplicateGroup[]> {
  return new Promise((resolve) => {
    setTimeout(() => {
      resolve([
        {
          hash: "abc123def456789a",
          count: 3,
          total_size: 6291456,
          wasted_space: 4194304,
          files: [
            {
              path: `${path}/backup/image1.jpg`,
              size: 2097152,
              modified: now - DAY,
              file_type: "Image"
            },
            {
              path: `${path}/photos/image1_copy.jpg`,
              size: 2097152,
              modified: now - 2 * DAY,
              file_type: "Image"
            },
            {
              path: `${path}/old/image1_old.jpg`,
              size: 2097152,
              modified: now - 3 * DAY,
              file_type: "Image"
            }
          ]
        },
        {
          hash: "def456789abc123b",
          count: 2,
          total_size: 20971520,
          wasted_space: 10485760,
          files: [
            {
              path: `${path}/docs/manual.pdf`,
              size: 10485760,
              modified: now - 4 * DAY,
              file_type: "Document"
            },
            {
              path: `${path}/backup/manual_backup.pdf`,
              size: 10485760,
              modified: now - 5 * DAY,
              file_type: "Document"
            }
          ]
        },
        {
          // One copy lives on a "USB drive" without a trash directory:
          // trash-mode deletion fails for it, permanent deletion succeeds
          // (see the deleteFiles mock)
          hash: "9a8b7c6d5e4f3210",
          count: 2,
          total_size: 1572864000,
          wasted_space: 786432000,
          files: [
            {
              path: `${path}/videos/vacation.mp4`,
              size: 786432000,
              modified: now - 10 * DAY,
              file_type: "Video"
            },
            {
              path: `${path}/usb-drive/vacation.mp4`,
              size: 786432000,
              modified: now - 30 * DAY,
              file_type: "Video"
            }
          ]
        }
      ]);
    }, 1000);
  });
}
