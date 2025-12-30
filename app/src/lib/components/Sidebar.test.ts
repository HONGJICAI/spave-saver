import { render, screen } from '@testing-library/svelte';
import Sidebar from './Sidebar.svelte';

// Mock $app/state
vi.mock('$app/state', () => ({
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
});
