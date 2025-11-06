import { describe, it, expect } from 'vitest';
import { formatSize, formatDate, formatDuration, percentage } from './format';

describe('formatSize', () => {
  it('formats bytes correctly', () => {
    expect(formatSize(0)).toBe('0 B');
    expect(formatSize(500)).toBe('500 B');
    expect(formatSize(1023)).toBe('1023 B');
  });

  it('formats kilobytes correctly', () => {
    expect(formatSize(1024)).toBe('1.0 KB');
    expect(formatSize(1536)).toBe('1.5 KB');
    expect(formatSize(10240)).toBe('10.0 KB');
  });

  it('formats megabytes correctly', () => {
    expect(formatSize(1048576)).toBe('1.0 MB');
    expect(formatSize(5242880)).toBe('5.0 MB');
  });

  it('formats gigabytes correctly', () => {
    expect(formatSize(1073741824)).toBe('1.0 GB');
    expect(formatSize(5368709120)).toBe('5.0 GB');
  });

  it('formats terabytes correctly', () => {
    expect(formatSize(1099511627776)).toBe('1.0 TB');
  });
});

describe('formatDuration', () => {
  it('formats milliseconds correctly', () => {
    expect(formatDuration(0)).toBe('0ms');
    expect(formatDuration(500)).toBe('500ms');
    expect(formatDuration(999)).toBe('999ms');
  });

  it('formats seconds correctly', () => {
    expect(formatDuration(1000)).toBe('1.0s');
    expect(formatDuration(5500)).toBe('5.5s');
    expect(formatDuration(59999)).toBe('60.0s');
  });

  it('formats minutes correctly', () => {
    expect(formatDuration(60000)).toBe('1m 0s');
    expect(formatDuration(125000)).toBe('2m 5s');
  });

  it('formats hours correctly', () => {
    expect(formatDuration(3600000)).toBe('1h 0m');
    expect(formatDuration(7380000)).toBe('2h 3m');
  });
});

describe('percentage', () => {
  it('calculates percentage correctly', () => {
    expect(percentage(50, 100)).toBe(50);
    expect(percentage(25, 100)).toBe(25);
    expect(percentage(1, 3)).toBeCloseTo(33.33, 1);
  });

  it('handles zero total', () => {
    expect(percentage(10, 0)).toBe(0);
  });

  it('handles zero part', () => {
    expect(percentage(0, 100)).toBe(0);
  });
});

describe('formatDate', () => {
  it('formats dates correctly', () => {
    const dateNumber = 1705325800000;
    const formatted = formatDate(dateNumber);
    expect(formatted).toMatch(/\d{4}-\d{2}-\d{2}/);
  });
});
