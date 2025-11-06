import type { StorageStats } from '$lib/types';

// Mock storage statistics
export function mockStorageStats(path: string): Promise<StorageStats> {
  return new Promise((resolve) => {
    setTimeout(() => {
      resolve({
        total_files: 1523,
        total_size: 5368709120, // 5 GB
        images: 452,
        videos: 23,
        documents: 187,
        archives: 45,
        others: 811,
        empty_files: 5
      });
    }, 700);
  });
}
