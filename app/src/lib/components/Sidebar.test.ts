import { render, screen } from '@testing-library/svelte';
import Sidebar from './Sidebar.svelte';
import { appState } from '$lib/stores/app';

// Mock $app/stores
vi.mock('$app/stores', () => ({
  page: {
    subscribe: (fn: (value: any) => void) => {
      fn({ url: { pathname: '/' } });
      return () => {};
    }
  }
}));

describe('Sidebar', () => {
  beforeEach(() => {
    // Mock window.__TAURI_INTERNALS__ for testing
    vi.stubGlobal('__TAURI_INTERNALS__', undefined);
    appState.setBusy(false);
  });

  it('renders all navigation items', () => {
    render(Sidebar);
    
    expect(screen.getByText('Home')).toBeInTheDocument();
    expect(screen.getByText('Duplicates')).toBeInTheDocument();
    expect(screen.getByText('Similar')).toBeInTheDocument();
    expect(screen.getByText('Empty')).toBeInTheDocument();
    expect(screen.getByText('Compress')).toBeInTheDocument();
    expect(screen.getByText('Statistics')).toBeInTheDocument();
  });

  it('displays web mode by default', () => {
    render(Sidebar);
    expect(screen.getByText('Web Mode')).toBeInTheDocument();
  });

  it('displays tauri mode when __TAURI_INTERNALS__ is present', () => {
    vi.stubGlobal('__TAURI_INTERNALS__', {});
    render(Sidebar);
    expect(screen.getByText('Desktop Mode')).toBeInTheDocument();
  });

  it('leaves navigation links enabled when not busy', () => {
    render(Sidebar);
    const link = screen.getByText('Duplicates').closest('a');
    expect(link).not.toBeNull();
    expect(link?.getAttribute('aria-disabled')).toBe('false');
  });

  it('disables navigation links while an operation is busy', () => {
    appState.setBusy(true);
    render(Sidebar);
    const link = screen.getByText('Duplicates').closest('a');
    expect(link?.getAttribute('aria-disabled')).toBe('true');
    expect(link?.className).toContain('pointer-events-none');
    expect(screen.getByText(/Navigation paused/)).toBeInTheDocument();
  });
});
