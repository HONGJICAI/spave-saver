import type { EmptyScanResult } from "../lib/types";

// Mock empty files and folders. Trigger words (shared mock conventions):
// - paths containing "empty-dir" return no results, like the backend
//   scanning a directory with nothing to clean (demos the empty-state UI)
// - the file and folder under "locked/" let web mode demo deletion failing
//   with a permission error
// - the folder under "usb-drive/" lets web mode demo trash-mode deletion
//   failing (no trash directory on that volume) and permanent succeeding
export function mockEmptyItems(path: string): Promise<EmptyScanResult> {
  if (path.includes("empty-dir")) {
    return new Promise((resolve) => {
      setTimeout(() => resolve({ empty_files: [], empty_folders: [] }), 100);
    });
  }
  return new Promise((resolve) => {
    setTimeout(() => {
      resolve({
        empty_files: [
          `${path}/temp/empty1.txt`,
          `${path}/temp/empty2.log`,
          `${path}/cache/placeholder.dat`,
          `${path}/.temp/dummy.tmp`,
          `${path}/logs/empty.log`,
          `${path}/locked/readonly.cfg`
        ],
        empty_folders: [
          `${path}/old-projects/abandoned`,
          `${path}/downloads/unzipped`,
          `${path}/locked/system-cache`,
          `${path}/usb-drive/DCIM`
        ]
      });
    }, 600);
  });
}
