import type { StorageStats } from '$lib/types';

// Mock storage statistics. Paths containing "empty-dir" return zeroed stats,
// like the backend scanning an empty or nonexistent directory.
export function mockStorageStats(path: string): Promise<StorageStats> {
  if (path.includes('empty-dir')) {
    return new Promise((resolve) => {
      setTimeout(
        () =>
          resolve({
            total_files: 0,
            total_size: 0,
            images: 0,
            videos: 0,
            documents: 0,
            archives: 0,
            others: 0,
            empty_files: 0
          }),
        100
      );
    });
  }
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
