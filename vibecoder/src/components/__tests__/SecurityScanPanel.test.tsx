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

// ── Verification ───────────────────────────────────────────────────────────
//
// The scanner matches substrings; only verification can tell a vulnerability
// from a line that looks like one. These pin the three states apart: a refuted
// candidate leaves the findings list, a confirmed one carries its evidence, and
// one nobody could check stays visible as unverified rather than becoming
// either of the other two.

interface RawVerdict {
  id: string;
  verification: 'confirmed' | 'refuted' | 'unverified';
  verificationReason: string;
}

/** Scan returns `results`; verification answers with `verdicts` per file. */
function setupVerifyMocks(
  results: RawFinding[],
  verdicts: RawVerdict[] | ((file: string) => RawVerdict[]),
) {
  mockInvoke.mockImplementation(async (cmd: string, args: Record<string, unknown>) => {
    if (cmd === 'run_security_scan') return results;
    if (cmd === 'get_security_scan_results') return [];
    if (cmd === 'get_security_scan_history') return [];
    if (cmd === 'verify_security_findings') {
      return typeof verdicts === 'function' ? verdicts(args.file as string) : verdicts;
    }
    return null;
  });
}

async function renderVerified(
  results: RawFinding[],
  verdicts: RawVerdict[] | ((file: string) => RawVerdict[]),
) {
  setupVerifyMocks(results, verdicts);
  render(<SecurityScanPanel workspacePath="/ws" provider="Ollama (devstral-2)" />);
  fireEvent.click(screen.getByRole('button', { name: 'Run Scan' }));
  await waitFor(() =>
    expect(mockInvoke.mock.calls.some(([cmd]) => cmd === 'verify_security_findings')).toBe(true)
  );
}

describe('SecurityScanPanel — verification', () => {
  it('checks each candidate with the provider selected in the toolbar', async () => {
    await renderVerified([finding()], [
      { id: 'f-1', verification: 'confirmed', verificationReason: 'req.params.id reaches the query.' },
    ]);

    const call = mockInvoke.mock.calls.find(([cmd]) => cmd === 'verify_security_findings');
    const args = call?.[1] as Record<string, unknown>;
    expect(args.provider).toBe('ollama');
    expect(args.model).toBe('devstral-2');
    expect(args.file).toBe('orbit/server/src/protocols/cql/adapter.rs');
    expect(args.candidates).toEqual([
      expect.objectContaining({ id: 'f-1', line: 1148, cwe: 'CWE-89' }),
    ]);
  });

  it('drops a refuted candidate from the findings list and keeps it under Ruled out', async () => {
    await renderVerified([finding()], [
      { id: 'f-1', verification: 'refuted', verificationReason: 'The table name is a compile-time constant.' },
    ]);

    await waitFor(() => expect(screen.getByText(/1 ruled out as false positive/)).toBeTruthy());
    expect(screen.queryByText('SQL Injection via string concatenation')).toBeNull();

    fireEvent.click(screen.getByText(/1 ruled out as false positive/));
    expect(screen.getByText(/compile-time constant/)).toBeTruthy();
  });

  it('never reports a refuted candidate to chat', async () => {
    await renderVerified(
      [finding({ id: 'f-1' }), finding({ id: 'f-2', line: 1327 })],
      [
        { id: 'f-1', verification: 'confirmed', verificationReason: 'Tainted input reaches it.' },
        { id: 'f-2', verification: 'refuted', verificationReason: 'Literal query.' },
      ]
    );

    await waitFor(() => expect(screen.getByText(/1 verified · 0 unverified · 1 ruled out/)).toBeTruthy());
    fireEvent.click(screen.getAllByRole('button', { name: 'Fix with AI' })[0]);

    expect(injected).toHaveLength(1);
    expect(injected[0]).toContain('line 1148');
    expect(injected[0]).not.toContain('line 1327');
    // A verified finding travels with the evidence that verified it.
    expect(injected[0]).toContain('Tainted input reaches it.');
  });

  it('keeps a candidate the model could not decide, marked unverified', async () => {
    await renderVerified([finding()], [
      { id: 'f-1', verification: 'unverified', verificationReason: 'The caller is not shown.' },
    ]);

    await waitFor(() => expect(screen.getByText(/0 verified · 1 unverified · 0 ruled out/)).toBeTruthy());
    expect(screen.getAllByText('SQL Injection via string concatenation').length).toBeGreaterThan(0);
    expect(screen.getAllByText('unverified').length).toBeGreaterThan(0);

    fireEvent.click(screen.getAllByRole('button', { name: 'Fix with AI' })[0]);
    expect(injected[0]).toContain('NOT verified');
    expect(injected[0]).toContain('The caller is not shown.');
  });

  it('leaves every candidate unverified when the verification call fails', async () => {
    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === 'run_security_scan') return [finding({ id: 'f-1' }), finding({ id: 'f-2', file: 'other.rs' })];
      if (cmd === 'get_security_scan_results') return [];
      if (cmd === 'get_security_scan_history') return [];
      if (cmd === 'verify_security_findings') throw new Error("Provider 'ollama' is not configured");
      return null;
    });
    render(<SecurityScanPanel workspacePath="/ws" provider="Ollama (devstral-2)" />);
    fireEvent.click(screen.getByRole('button', { name: 'Run Scan' }));

    await waitFor(() => expect(screen.getByText(/Verification stopped/)).toBeTruthy());
    // Both files' candidates stay, and neither is claimed to be checked.
    expect(screen.getByText(/0 verified · 2 unverified · 0 ruled out/)).toBeTruthy();
    // One failing file does not mean a second call: the same call would fail.
    expect(mockInvoke.mock.calls.filter(([cmd]) => cmd === 'verify_security_findings')).toHaveLength(1);
  });

  it('caps how many files one pass verifies, worst first, and says what it left', async () => {
    // 30 files, one candidate each: 29 Medium and one Critical. The cap is 25.
    const many = Array.from({ length: 30 }, (_, i) =>
      finding({ id: `f-${i}`, file: `src/f${i}.rs`, severity: i === 29 ? 'Critical' : 'Medium' })
    );
    await renderVerified(many, (file) => [
      { id: many.find((f) => f.file === file)!.id, verification: 'refuted', verificationReason: 'Literal.' },
    ]);

    await waitFor(() =>
      expect(mockInvoke.mock.calls.filter(([cmd]) => cmd === 'verify_security_findings')).toHaveLength(25)
    );
    const verified = mockInvoke.mock.calls
      .filter(([cmd]) => cmd === 'verify_security_findings')
      .map(([, args]) => (args as { file: string }).file);
    // The Critical one is checked, not dropped by the cap.
    expect(verified).toContain('src/f29.rs');
    // The cap is stated, and what it left over stays visible as unverified.
    await waitFor(() => expect(screen.getByText(/5 more file\(s\) are left unverified/)).toBeTruthy());
    expect(screen.getByText(/0 verified · 5 unverified · 25 ruled out/)).toBeTruthy();
  });

  it('verifies nothing when no model is selected, and says so', async () => {
    setupMocks([finding()]);
    render(<SecurityScanPanel workspacePath="/ws" />);
    fireEvent.click(screen.getByRole('button', { name: 'Run Scan' }));

    await waitFor(() => expect(screen.getByText(/select a provider and model/i)).toBeTruthy());
    expect(mockInvoke.mock.calls.filter(([cmd]) => cmd === 'verify_security_findings')).toHaveLength(0);
  });
});
