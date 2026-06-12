// Mock empty files. Paths containing "empty-dir" return no results, like
// the backend scanning an empty or nonexistent directory.
export function mockEmptyFiles(path: string): Promise<string[]> {
  if (path.includes('empty-dir')) {
    return new Promise((resolve) => {
      setTimeout(() => resolve([]), 100);
    });
  }
  return new Promise((resolve) => {
    setTimeout(() => {
      resolve([
        `${path}/temp/empty1.txt`,
        `${path}/temp/empty2.log`,
        `${path}/cache/placeholder.dat`,
        `${path}/.temp/dummy.tmp`,
        `${path}/logs/empty.log`
      ]);
    }, 600);
  });
}
