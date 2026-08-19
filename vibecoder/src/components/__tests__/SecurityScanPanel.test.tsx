import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

// ── Mock Tauri invoke ──────────────────────────────────────────────────────

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

// ── Import after mocks ────────────────────────────────────────────────────

import SecurityScanPanel from '../SecurityScanPanel';

// ── Test data ──────────────────────────────────────────────────────────────

interface RawFinding {
  id: string;
  title: string;
  severity: string;
  file: string;
  line: number;
  description: string;
  cwe: string;
  remediation: string;
  suppressed: boolean;
}

function finding(over: Partial<RawFinding> = {}): RawFinding {
  return {
    id: 'f-1',
    title: 'SQL Injection via string concatenation',
    severity: 'Critical',
    file: 'orbit/server/src/protocols/cql/adapter.rs',
    line: 1148,
    description: 'Query is built by concatenating an untrusted value.',
    cwe: 'CWE-89',
    remediation: 'Use a parameterised query.',
    suppressed: false,
    ...over,
  };
}

/** `run_security_scan` returns `results`; everything else is empty. */
function setupMocks(results: RawFinding[]) {
  mockInvoke.mockImplementation(async (cmd: string) => {
    if (cmd === 'run_security_scan') return results;
    if (cmd === 'get_security_scan_results') return [];
    if (cmd === 'get_security_scan_history') return [];
    return null;
  });
}

const injected: string[] = [];
const listener = (e: Event) => injected.push((e as CustomEvent<string>).detail);

beforeEach(() => {
  mockInvoke.mockReset();
  injected.length = 0;
  window.addEventListener('vibecoder:inject-context', listener);
});

afterEach(() => {
  window.removeEventListener('vibecoder:inject-context', listener);
});

async function renderWithFindings(results: RawFinding[]) {
  setupMocks(results);
  render(<SecurityScanPanel workspacePath="/ws" />);
  fireEvent.click(screen.getByRole('button', { name: 'Run Scan' }));
  await waitFor(() => expect(screen.getAllByText(results[0].title).length).toBeGreaterThan(0));
}

describe('SecurityScanPanel — Fix with AI', () => {
  it('hands one finding to chat instead of editing anything', async () => {
    await renderWithFindings([finding()]);

    fireEvent.click(screen.getAllByRole('button', { name: 'Fix with AI' })[0]);

    expect(injected).toHaveLength(1);
    const request = injected[0];
    // The path and line must be in the request, or the model writes a second
    // copy of the file instead of fixing the one that has the bug.
    expect(request).toContain('orbit/server/src/protocols/cql/adapter.rs');
    expect(request).toContain('line 1148');
    expect(request).toContain('CWE-89');
    expect(request).toContain('Use a parameterised query.');
    expect(request).toMatch(/do not create a new file/i);
    // A pattern scanner reports candidates; the request has to say so.
    expect(request).toMatch(/false positive/i);

    // Nothing was written and nothing was suppressed.
    const mutations = mockInvoke.mock.calls.filter(([cmd]) =>
      String(cmd).includes('write') || String(cmd).includes('suppress')
    );
    expect(mutations).toEqual([]);
  });

  it('confirms the request reached chat', async () => {
    await renderWithFindings([finding()]);
    const button = screen.getAllByRole('button', { name: 'Fix with AI' })[0];
    fireEvent.click(button);
    await waitFor(() => expect(screen.getAllByRole('button', { name: /Sent to chat/ }).length).toBeGreaterThan(0));
  });

  it('sends a whole CWE group in one request', async () => {
    await renderWithFindings([
      finding({ id: 'f-1', line: 1148 }),
      finding({ id: 'f-2', line: 1327, file: 'orbit/server/src/protocols/mcp/sql_generator.rs' }),
      finding({ id: 'f-3', cwe: 'CWE-22', title: 'Path traversal', severity: 'High' }),
    ]);

    // Grouping is by CWE by default: the SQL group carries two findings.
    fireEvent.click(screen.getByRole('button', { name: 'Fix all 2 with AI' }));

    expect(injected).toHaveLength(1);
    expect(injected[0]).toContain('adapter.rs');
    expect(injected[0]).toContain('sql_generator.rs');
    expect(injected[0]).not.toContain('Path traversal');
  });

  it('caps a batch and says in the label and the request what it dropped', async () => {
    const many = Array.from({ length: 40 }, (_, i) =>
      finding({ id: `f-${i}`, line: 100 + i })
    );
    await renderWithFindings(many);

    // A capped hand-off must never read as the whole set — the header CTA and
    // the CWE group both carry all 40, so both labels name the cap.
    const capped = screen.getAllByRole('button', { name: 'Fix first 25 of 40 with AI' });
    expect(capped).toHaveLength(2);
    fireEvent.click(capped[0]);

    expect(injected).toHaveLength(1);
    expect(injected[0]).toContain('first 25 of 40');
    expect(injected[0]).toContain('line 124');   // the 25th
    expect(injected[0]).not.toContain('line 125'); // the 26th, dropped
  });

  it('sends only what the severity filter leaves visible', async () => {
    await renderWithFindings([
      finding({ id: 'f-1', title: 'Critical one' }),
      finding({ id: 'f-2', title: 'Medium one', severity: 'Medium', cwe: 'CWE-22' }),
    ]);

    fireEvent.click(screen.getByRole('button', { name: '1 Medium' }));
    // The header CTA is the only "Fix all 1" once the filter narrows to one.
    fireEvent.click(screen.getAllByRole('button', { name: 'Fix with AI' })[0]);

    expect(injected).toHaveLength(1);
    expect(injected[0]).toContain('Medium one');
    expect(injected[0]).not.toContain('Critical one');
  });
});
