import type { SimilarGroup, MediaKind } from '$lib/types';

// Unix seconds (the backend's FileInfo.modified is seconds, not millis)
const nowSecs = () => Math.floor(Date.now() / 1000);

// Mock similar media. Trigger words (shared mock conventions):
// - paths containing "empty-dir" -> no groups (demos the empty-state UI)
// - paths containing "locked"    -> the scan itself fails with a permission
//   error (demos the scan-error UI), worded like the backend
//
// Video similarity is NOT implemented in the backend yet (it needs ffmpeg —
// see crates/core/src/video_sim.rs). So, faithfully mirroring the backend, a
// video-only request yields nothing; the UI keeps the Videos option disabled.
//
// Image groups below the requested threshold are filtered out, like the
// backend's similarity filter, so the threshold slider is demoable (scores
// here: 0.98, 0.95, 0.91). The 0.95 group has three files at mixed
// resolutions to demo "keep the highest-resolution copy".
export function mockFindSimilarMedia(
  path: string,
  threshold: number,
  mediaTypes: MediaKind[] = ['Image']
): Promise<SimilarGroup[]> {
  if (path.includes('empty-dir')) {
    return new Promise((resolve) => setTimeout(() => resolve([]), 100));
  }
  if (path.includes('locked')) {
    return new Promise((_resolve, reject) =>
      setTimeout(() => reject(new Error('Permission denied (os error 13)')), 300)
    );
  }

  const wantImages = mediaTypes.length === 0 || mediaTypes.includes('Image');
  if (!wantImages) {
    // Video-only request: the backend has no video similarity yet -> nothing
    return new Promise((resolve) => setTimeout(() => resolve([]), 200));
  }

  const groups: SimilarGroup[] = [
    {
      media_kind: 'Image',
      similarity_score: 0.98,
      files: [
        {
          path: `${path}/photos/sunset.jpg`,
          size: 3145728,
          modified: nowSecs() - 86400,
          width: 4032,
          height: 3024,
        },
        {
          path: `${path}/photos/sunset-edit.jpg`,
          size: 2200000,
          modified: nowSecs() - 172800,
          width: 1920,
          height: 1440,
        },
      ],
    },
    {
      media_kind: 'Image',
      similarity_score: 0.95,
      files: [
        {
          path: `${path}/trip/beach.png`,
          size: 5242880,
          modified: nowSecs() - 259200,
          width: 3840,
          height: 2160,
        },
        {
          path: `${path}/trip/beach-copy.png`,
          size: 1048576,
          modified: nowSecs() - 300000,
          width: 1280,
          height: 720,
        },
        {
          path: `${path}/trip/beach-thumb.png`,
          size: 204800,
          modified: nowSecs() - 320000,
          width: 640,
          height: 360,
        },
      ],
    },
    {
      media_kind: 'Image',
      similarity_score: 0.91,
      files: [
        {
          path: `${path}/screens/screen1.png`,
          size: 1100000,
          modified: nowSecs() - 345600,
          width: 2560,
          height: 1440,
        },
        {
          path: `${path}/screens/screen2.png`,
          size: 1080000,
          modified: nowSecs() - 432000,
          width: 2560,
          height: 1440,
        },
      ],
    },
  ];

  return new Promise((resolve) => {
    setTimeout(() => resolve(groups.filter((g) => g.similarity_score >= threshold)), 1000);
  });
}

function escapeXml(s: string): string {
  return s.replace(/[<>&]/g, (c) => (c === '<' ? '&lt;' : c === '>' ? '&gt;' : '&amp;'));
}

// Deterministic hue (0-359) from a string
function hashHue(s: string): number {
  let h = 0;
  for (let i = 0; i < s.length; i++) h = (h * 31 + s.charCodeAt(i)) >>> 0;
  return h % 360;
}

// Web mode has no real files, so return a deterministic colored SVG placeholder
// instead of a real thumbnail. The hue is derived from the path with common
// variant suffixes stripped, so files in the same similar-group look alike
// (e.g. beach / beach-copy / beach-thumb share a color), which sells the demo.
export function mockImageThumbnail(path: string, _maxSize: number): Promise<string> {
  const base = path.replace(/[-_]?(copy|edit|thumb|\d+)?\.[a-z0-9]+$/i, '');
  const hue = hashHue(base);
  const name = path.split('/').pop() ?? '';
  const svg =
    `<svg xmlns="http://www.w3.org/2000/svg" width="160" height="160">` +
    `<rect width="160" height="160" fill="hsl(${hue} 60% 55%)"/>` +
    `<text x="80" y="84" font-size="11" fill="white" text-anchor="middle" font-family="sans-serif">${escapeXml(name)}</text>` +
    `</svg>`;
  return new Promise((resolve) =>
    setTimeout(() => resolve(`data:image/svg+xml;utf8,${encodeURIComponent(svg)}`), 150)
  );
}
