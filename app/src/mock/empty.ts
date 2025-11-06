// Mock empty files
export function mockEmptyFiles(path: string): Promise<string[]> {
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
