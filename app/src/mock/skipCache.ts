// Web-mode stand-in for the backend's skip cache (the in-memory side of
// compress_skip_cache.json): compressing a file that yields no size
// reduction records an entry, the next compressible-files scan rejects
// that file with a "cached result" reason, and clearing the cache makes
// it compressible again. State lives for the page session, like the
// backend's cache lives across scans.
const entries = new Set<string>();

export const mockSkipCache = {
  record(path: string): void {
    entries.add(path);
  },
  has(path: string): boolean {
    return entries.has(path);
  },
  size(): number {
    return entries.size;
  },
  /** Returns how many entries were removed, like the backend's clear() */
  clear(): number {
    const removed = entries.size;
    entries.clear();
    return removed;
  },
};
