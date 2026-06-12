import type { ScanResult } from '$lib/types';

// Mock scan result. Paths containing "empty-dir" return no files, like the
// backend scanning an empty or nonexistent directory (demos the empty-state UI).
export function mockScanResult(path: string): Promise<ScanResult> {
  if (path.includes('empty-dir')) {
    return new Promise((resolve) => {
      setTimeout(() => resolve({ path, file_count: 0, total_size: 0, files: [] }), 100);
    });
  }
  return new Promise((resolve) => {
    setTimeout(() => {
      resolve({
        path,
        file_count: 156,
        total_size: 524288000, // 500 MB
        files: [
          {
            path: `${path}/Documents/report.pdf`,
            size: 2048000,
            modified: Date.now() - 86400000,
            file_type: "Document"
          },
          {
            path: `${path}/Pictures/photo.jpg`,
            size: 3145728,
            modified: Date.now() - 172800000,
            file_type: "Image"
          },
          {
            path: `${path}/Videos/movie.mp4`,
            size: 104857600,
            modified: Date.now() - 259200000,
            file_type: "Video"
          },
          {
            path: `${path}/Downloads/archive.zip`,
            size: 10485760,
            modified: Date.now() - 345600000,
            file_type: "Archive"
          },
          {
            path: `${path}/temp/empty.txt`,
            size: 0,
            modified: Date.now() - 432000000,
            file_type: "Other"
          }
        ]
      });
    }, 800);
  });
}
