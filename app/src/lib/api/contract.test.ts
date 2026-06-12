import { describe, it, expect } from 'vitest';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

/**
 * Contract test between the Tauri backend and the frontend API layer.
 *
 * The frontend talks to the backend exclusively through src/lib/api/index.ts,
 * and web mode must be able to demo every backend capability. This test
 * parses the source files on both sides and fails when they drift:
 *
 * 1. every command defined in commands.rs is registered in lib.rs
 *    (an unregistered command compiles fine but is unreachable)
 * 2. every registered command has an invoke() wrapper in the API layer
 *    (a missing wrapper means web mode cannot cover that feature)
 * 3. every invoke() call targets a registered command
 *    (a typo here only fails at runtime in Tauri mode)
 */

const read = (relative: string) =>
  readFileSync(fileURLToPath(new URL(relative, import.meta.url)), 'utf-8');

const commandsRs = read('../../../src-tauri/src/commands.rs');
const libRs = read('../../../src-tauri/src/lib.rs');
const apiTs = read('./index.ts');

function definedCommands(source: string): Set<string> {
  return new Set(
    [...source.matchAll(/#\[tauri::command\]\s*(?:pub\s+)?(?:async\s+)?fn\s+(\w+)/g)].map(
      (m) => m[1]
    )
  );
}

function registeredCommands(source: string): Set<string> {
  const handler = source.match(/generate_handler!\[([^\]]*)\]/);
  if (!handler) return new Set();
  return new Set(
    handler[1]
      .split(',')
      .map((name) => name.trim())
      .filter(Boolean)
  );
}

function invokedCommands(source: string): Set<string> {
  return new Set(
    [...source.matchAll(/\binvoke(?:<[^(]*>)?\(\s*["'](\w+)["']/g)].map((m) => m[1])
  );
}

const defined = definedCommands(commandsRs);
const registered = registeredCommands(libRs);
const invoked = invokedCommands(apiTs);

const difference = (a: Set<string>, b: Set<string>) => [...a].filter((x) => !b.has(x)).sort();

describe('Tauri command contract', () => {
  it('parses a non-empty command list from each source file', () => {
    expect(defined.size).toBeGreaterThan(0);
    expect(registered.size).toBeGreaterThan(0);
    expect(invoked.size).toBeGreaterThan(0);
  });

  it('registers every command defined in commands.rs', () => {
    expect(difference(defined, registered)).toEqual([]);
  });

  it('only registers commands that are defined in commands.rs', () => {
    expect(difference(registered, defined)).toEqual([]);
  });

  it('wraps every registered command in the API layer (web mode must cover it)', () => {
    expect(difference(registered, invoked)).toEqual([]);
  });

  it('only invokes commands that are registered in the backend', () => {
    expect(difference(invoked, registered)).toEqual([]);
  });
});
