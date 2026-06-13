import { describe, it, expect, beforeEach } from 'vitest';
import { get } from 'svelte/store';
import { appState } from './app';

describe('appState store', () => {
  beforeEach(() => {
    localStorage.clear();
    appState.reset();
  });

  it('starts not busy', () => {
    expect(get(appState).busy).toBe(false);
  });

  it('toggles the busy flag via setBusy', () => {
    appState.setBusy(true);
    expect(get(appState).busy).toBe(true);
    appState.setBusy(false);
    expect(get(appState).busy).toBe(false);
  });

  it('clears the busy flag on reset', () => {
    appState.setBusy(true);
    appState.reset();
    expect(get(appState).busy).toBe(false);
  });

  it('tracks scan paths and persists them', () => {
    appState.addScanPath('/tmp/a');
    expect(get(appState).scanPaths).toEqual(['/tmp/a']);
    appState.addScanPath('/tmp/a'); // duplicate is ignored
    expect(get(appState).scanPaths).toEqual(['/tmp/a']);
    appState.removeScanPath('/tmp/a');
    expect(get(appState).scanPaths).toEqual([]);
  });

  it('records errors via setError', () => {
    appState.setError('boom');
    expect(get(appState).error).toBe('boom');
    appState.setError(null);
    expect(get(appState).error).toBeNull();
  });
});
