import type { BrokenFile, FixExtensionResult } from "../lib/types";

// Mock broken (invalid/corrupted) files. Trigger words (shared mock
// conventions):
// - paths containing "empty-dir" return no results, like the backend scanning
//   a directory with nothing wrong (demos the empty-state UI)
// - the "locked/" file lets web mode demo deletion failing with a permission
//   error (handled by the deleteFiles mock)
//
// Reason wording mirrors the backend (crates/core/src/broken.rs) so the UI's
// error/detail rendering is exercised against realistic strings. Both
// categories the backend can return are represented:
// - "corrupted": truncated image, invalid ZIP, invalid gzip
// - "extension_mismatch": PDF bytes wearing an image extension
export function mockFindBroken(path: string): Promise<BrokenFile[]> {
  if (path.includes("empty-dir")) {
    return new Promise((resolve) => {
      setTimeout(() => resolve([]), 100);
    });
  }
  return new Promise((resolve) => {
    setTimeout(() => {
      resolve([
        {
          path: `${path}/photos/truncated.jpg`,
          size: 18432,
          category: "corrupted",
          reason: "Image cannot be decoded: format error decoding Jpeg: unexpected EOF"
        },
        {
          path: `${path}/icons/garbage.png`,
          size: 27,
          category: "corrupted",
          reason: "Missing or invalid png file signature"
        },
        {
          path: `${path}/archives/backup.zip`,
          size: 1048576,
          category: "corrupted",
          reason: "Invalid ZIP archive: invalid Zip archive: Could not find central directory end"
        },
        {
          path: `${path}/archives/logs.gz`,
          size: 4096,
          category: "corrupted",
          reason: "Invalid gzip stream: invalid gzip header"
        },
        {
          path: `${path}/photos/scan.jpg`,
          size: 524288,
          category: "extension_mismatch",
          reason: "Content looks like pdf but the extension is .jpg",
          suggested_extension: "pdf"
        },
        {
          path: `${path}/locked/report.png`,
          size: 65536,
          category: "extension_mismatch",
          reason: "Content looks like pdf but the extension is .png",
          suggested_extension: "pdf"
        }
      ]);
    }, 800);
  });
}

// Mock the "fix extension" rename. Mirrors the backend: the misnamed files in
// the mock above are all PDF content, so they rename to .pdf. The "locked/"
// path demos a permission failure (it stays in the broken list), matching the
// backend reporting a per-file error instead of aborting.
export function mockFixExtensions(paths: string[]): Promise<FixExtensionResult[]> {
  return new Promise((resolve) => {
    setTimeout(() => {
      resolve(
        paths.map((path) => {
          if (path.includes("locked")) {
            return { path, success: false, error: "Permission denied (os error 13)" };
          }
          // Swap the current extension for the detected content's extension
          const new_path = path.replace(/\.[^./\\]+$/, ".pdf");
          return { path, success: true, new_path };
        })
      );
    }, 400);
  });
}
