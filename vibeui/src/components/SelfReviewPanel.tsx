import React, { useState } from 'react';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface Finding {
  check: string;
  severity: 'info' | 'warning' | 'error' | 'critical';
  message: string;
  file?: string;
  line?: number;
  suggestion?: string;
}

interface CheckResult {
  kind: string;
  passed: boolean;
  findings: Finding[];
  durationMs: number;
  command?: string;
}

interface ReviewIteration {
  iteration: number;
  checks: CheckResult[];
  passed: boolean;
  feedback?: string;
}

interface ReviewConfig {
  enabled: boolean;
  maxRetries: number;
  checks: string[];
  failOnWarning: boolean;
  minBlockingSeverity: string;
}

// ---------------------------------------------------------------------------
// Demo data
// ---------------------------------------------------------------------------

const DEMO_CONFIG: ReviewConfig = {
  enabled: true, maxRetries: 3, checks: ['build', 'lint', 'test', 'security'],
  failOnWarning: false, minBlockingSeverity: 'error',
};

const DEMO_ITERATIONS: ReviewIteration[] = [
  {
    iteration: 1, passed: false,
    feedback: 'Self-review found issues:\n## lint check FAILED\n- [lint] error: unused import `std::io` (main.rs:3)\n- [lint] warning: variable `x` unused (lib.rs:42)\n\n## security check FAILED\n- [security] critical: Potential AWS Access Key (config.rs:15)',
    checks: [
      { kind: 'build', passed: true, findings: [], durationMs: 2100, command: 'cargo check --quiet' },
      { kind: 'lint', passed: false, durationMs: 3400, command: 'cargo clippy --quiet', findings: [
        { check: 'lint', severity: 'error', message: 'unused import `std::io`', file: 'main.rs', line: 3, suggestion: 'Remove the import' },
        { check: 'lint', severity: 'warning', message: 'variable `x` is unused', file: 'lib.rs', line: 42 },
      ]},
      { kind: 'test', passed: true, findings: [], durationMs: 8200, command: 'cargo test --quiet' },
      { kind: 'security', passed: false, durationMs: 450, command: 'secret-scan', findings: [
        { check: 'security', severity: 'critical', message: 'Potential AWS Access Key', file: 'config.rs', line: 15, suggestion: 'Move to environment variable' },
      ]},
    ],
  },
  {
    iteration: 2, passed: true,
    checks: [
      { kind: 'build', passed: true, findings: [], durationMs: 1800, command: 'cargo check --quiet' },
      { kind: 'lint', passed: true, findings: [], durationMs: 3100, command: 'cargo clippy --quiet' },
      { kind: 'test', passed: true, findings: [], durationMs: 8500, command: 'cargo test --quiet' },
      { kind: 'security', passed: true, findings: [], durationMs: 380, command: 'secret-scan' },
    ],
  },
];

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const severityColors: Record<string, string> = {
  info: 'var(--vp-c-text-2)',
  warning: '#f59e0b',
  error: 'var(--vp-c-red-1, #ef4444)',
  critical: '#dc2626',
};

const checkIcons: Record<string, string> = {
  build: 'B', lint: 'L', test: 'T', security: 'S', format: 'F', typecheck: 'TC',
};

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

const SelfReviewPanel: React.FC = () => {
  const [config, setConfig] = useState<ReviewConfig>(DEMO_CONFIG);
  const [iterations] = useState<ReviewIteration[]>(DEMO_ITERATIONS);
  const [tab, setTab] = useState<'results' | 'config' | 'report'>('results');

  const latestIteration = iterations[iterations.length - 1];
  const totalFindings = iterations.reduce((sum, it) => sum + it.checks.reduce((s, c) => s + c.findings.length, 0), 0);

  return (
    <div style={{ padding: 16, color: 'var(--vp-c-text-1)', background: 'var(--vp-c-bg)', minHeight: '100%' }}>
      <h2 style={{ margin: '0 0 12px' }}>Agent Self-Review Gate</h2>

      {/* Status banner */}
      <div style={{
        padding: '10px 16px', borderRadius: 8, marginBottom: 16,
        background: latestIteration?.passed ? 'rgba(16, 185, 129, 0.1)' : 'rgba(239, 68, 68, 0.1)',
        border: `1px solid ${latestIteration?.passed ? 'var(--vp-c-green-1, #10b981)' : 'var(--vp-c-red-1, #ef4444)'}`,
        display: 'flex', alignItems: 'center', gap: 12,
      }}>
        <span style={{ fontSize: 24 }}>{latestIteration?.passed ? '\u2713' : '\u2717'}</span>
        <div>
          <div style={{ fontWeight: 700 }}>
            {latestIteration?.passed ? 'All checks passed' : 'Checks failed — agent iterating'}
          </div>
          <div style={{ fontSize: 12, color: 'var(--vp-c-text-2)' }}>
            {iterations.length} iteration(s) · {totalFindings} total findings
          </div>
        </div>
      </div>

      {/* Stats */}
      <div style={{ display: 'flex', gap: 12, marginBottom: 16, flexWrap: 'wrap' }}>
        {[
          { label: 'Iterations', value: iterations.length },
          { label: 'Max Retries', value: config.maxRetries },
          { label: 'Checks', value: config.checks.length },
          { label: 'Findings', value: totalFindings },
        ].map((s) => (
          <div key={s.label} style={{ background: 'var(--vp-c-bg-soft)', padding: '6px 14px', borderRadius: 6, textAlign: 'center' }}>
            <div style={{ fontSize: 18, fontWeight: 700, color: 'var(--vp-c-brand)' }}>{s.value}</div>
            <div style={{ fontSize: 11, color: 'var(--vp-c-text-2)' }}>{s.label}</div>
          </div>
        ))}
      </div>

      {/* Tabs */}
      <div style={{ display: 'flex', gap: 4, marginBottom: 12, borderBottom: '1px solid var(--vp-c-divider)' }}>
        {(['results', 'config', 'report'] as const).map((t) => (
          <button key={t} onClick={() => setTab(t)} style={{
            padding: '6px 16px', border: 'none', cursor: 'pointer', fontSize: 13,
            background: tab === t ? 'var(--vp-c-brand)' : 'transparent',
            color: tab === t ? '#fff' : 'var(--vp-c-text-2)', borderRadius: '6px 6px 0 0',
          }}>
            {t.charAt(0).toUpperCase() + t.slice(1)}
          </button>
        ))}
      </div>

      {/* Results tab */}
      {tab === 'results' && (
        <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
          {iterations.map((iter) => (
            <div key={iter.iteration} style={{
              background: 'var(--vp-c-bg-soft)', padding: 12, borderRadius: 8,
              border: `1px solid ${iter.passed ? 'var(--vp-c-green-1, #10b981)' : 'var(--vp-c-red-1, #ef4444)'}`,
            }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 8 }}>
                <strong>Iteration {iter.iteration}</strong>
                <span style={{
                  fontSize: 11, padding: '1px 8px', borderRadius: 3,
                  background: iter.passed ? 'var(--vp-c-green-1, #10b981)' : 'var(--vp-c-red-1, #ef4444)',
                  color: '#fff', fontWeight: 600,
                }}>
                  {iter.passed ? 'PASS' : 'FAIL'}
                </span>
              </div>

              {/* Check results grid */}
              <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(200px, 1fr))', gap: 8, marginBottom: iter.checks.some(c => c.findings.length > 0) ? 8 : 0 }}>
                {iter.checks.map((check) => (
                  <div key={check.kind} style={{
                    padding: 8, borderRadius: 6,
                    background: check.passed ? 'rgba(16, 185, 129, 0.08)' : 'rgba(239, 68, 68, 0.08)',
                    border: `1px solid ${check.passed ? 'rgba(16, 185, 129, 0.3)' : 'rgba(239, 68, 68, 0.3)'}`,
                  }}>
                    <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
                      <span style={{
                        display: 'inline-block', width: 22, height: 22, lineHeight: '22px', textAlign: 'center',
                        borderRadius: 4, fontSize: 10, fontWeight: 700,
                        background: check.passed ? 'var(--vp-c-green-1, #10b981)' : 'var(--vp-c-red-1, #ef4444)', color: '#fff',
                      }}>{checkIcons[check.kind] || check.kind[0].toUpperCase()}</span>
                      <span style={{ fontWeight: 600, fontSize: 13 }}>{check.kind}</span>
                      <span style={{ fontSize: 11, color: 'var(--vp-c-text-3)', marginLeft: 'auto' }}>{check.durationMs}ms</span>
                    </div>
                    {check.command && (
                      <div style={{ fontSize: 10, fontFamily: 'monospace', color: 'var(--vp-c-text-3)', marginTop: 4 }}>{check.command}</div>
                    )}
                    {check.findings.length > 0 && (
                      <div style={{ fontSize: 11, color: 'var(--vp-c-text-2)', marginTop: 4 }}>
                        {check.findings.length} finding(s)
                      </div>
                    )}
                  </div>
                ))}
              </div>

              {/* Findings */}
              {iter.checks.some(c => c.findings.length > 0) && (
                <div style={{ fontSize: 12 }}>
                  {iter.checks.filter(c => c.findings.length > 0).flatMap(c => c.findings).map((f, i) => (
                    <div key={i} style={{ display: 'flex', gap: 6, padding: '3px 0', borderBottom: '1px solid var(--vp-c-divider)' }}>
                      <span style={{ color: severityColors[f.severity], fontWeight: 600, fontSize: 11, minWidth: 55 }}>{f.severity}</span>
                      <span style={{ color: 'var(--vp-c-text-1)' }}>{f.message}</span>
                      {f.file && <span style={{ color: 'var(--vp-c-text-3)', fontFamily: 'monospace', marginLeft: 'auto', fontSize: 11 }}>{f.file}{f.line ? `:${f.line}` : ''}</span>}
                    </div>
                  ))}
                </div>
              )}
            </div>
          ))}
        </div>
      )}

      {/* Config tab */}
      {tab === 'config' && (
        <div style={{ background: 'var(--vp-c-bg-soft)', padding: 16, borderRadius: 8 }}>
          <h3 style={{ margin: '0 0 12px' }}>Self-Review Configuration</h3>
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 12 }}>
            <label style={{ fontSize: 12 }}>
              <input type="checkbox" checked={config.enabled} onChange={(e) => setConfig({ ...config, enabled: e.target.checked })} /> Enabled
            </label>
            <label style={{ fontSize: 12 }}>
              <input type="checkbox" checked={config.failOnWarning} onChange={(e) => setConfig({ ...config, failOnWarning: e.target.checked })} /> Fail on warnings
            </label>
            <label style={{ fontSize: 12 }}>
              Max retries
              <input type="number" value={config.maxRetries} min={1} max={10} onChange={(e) => setConfig({ ...config, maxRetries: parseInt(e.target.value) || 3 })} style={{ width: '100%', padding: 6, background: 'var(--vp-c-bg)', color: 'var(--vp-c-text-1)', border: '1px solid var(--vp-c-divider)', borderRadius: 4 }} />
            </label>
            <label style={{ fontSize: 12 }}>
              Min blocking severity
              <select value={config.minBlockingSeverity} onChange={(e) => setConfig({ ...config, minBlockingSeverity: e.target.value })} style={{ width: '100%', padding: 6, background: 'var(--vp-c-bg)', color: 'var(--vp-c-text-1)', border: '1px solid var(--vp-c-divider)', borderRadius: 4 }}>
                <option value="info">Info</option>
                <option value="warning">Warning</option>
                <option value="error">Error</option>
                <option value="critical">Critical</option>
              </select>
            </label>
          </div>
          <h4 style={{ margin: '16px 0 8px' }}>Active Checks</h4>
          <div style={{ display: 'flex', gap: 8, flexWrap: 'wrap' }}>
            {['build', 'lint', 'test', 'security', 'format', 'typecheck', 'diff_review'].map((check) => (
              <label key={check} style={{ fontSize: 12, display: 'flex', alignItems: 'center', gap: 4, background: 'var(--vp-c-bg)', padding: '4px 10px', borderRadius: 4 }}>
                <input type="checkbox" checked={config.checks.includes(check)} onChange={(e) => {
                  if (e.target.checked) {
                    setConfig({ ...config, checks: [...config.checks, check] });
                  } else {
                    setConfig({ ...config, checks: config.checks.filter(c => c !== check) });
                  }
                }} />
                {check}
              </label>
            ))}
          </div>
          <div style={{ marginTop: 16, padding: 12, background: 'var(--vp-c-bg)', borderRadius: 6, fontFamily: 'monospace', fontSize: 11, color: 'var(--vp-c-text-2)' }}>
            <div># config.toml</div>
            <div>[agent]</div>
            <div>self_review = {config.enabled.toString()}</div>
            <div>self_review_max_retries = {config.maxRetries}</div>
            <div>self_review_checks = [{config.checks.map(c => `"${c}"`).join(', ')}]</div>
            <div>self_review_fail_on_warning = {config.failOnWarning.toString()}</div>
            <div>self_review_min_blocking_severity = "{config.minBlockingSeverity}"</div>
          </div>
        </div>
      )}

      {/* Report tab */}
      {tab === 'report' && (
        <div style={{ fontFamily: 'monospace', fontSize: 12, background: 'var(--vp-c-bg-soft)', padding: 16, borderRadius: 8, whiteSpace: 'pre-wrap', color: 'var(--vp-c-text-1)', lineHeight: 1.6 }}>
{`# Self-Review Report

**Status**: ${latestIteration?.passed ? 'PASSED' : 'FAILED'}
**Iterations**: ${iterations.length}
**Total findings**: ${totalFindings}
**Checks**: ${config.checks.join(', ')}

${iterations.map(iter => `## Iteration ${iter.iteration}: ${iter.passed ? 'PASS' : 'FAIL'}
- Errors: ${iter.checks.reduce((s, c) => s + c.findings.filter(f => f.severity === 'error' || f.severity === 'critical').length, 0)}, Warnings: ${iter.checks.reduce((s, c) => s + c.findings.filter(f => f.severity === 'warning').length, 0)}
${iter.checks.map(c => `  - ${c.kind}: ${c.passed ? 'pass' : 'fail'} (${c.findings.length} findings, ${c.durationMs}ms)`).join('\n')}`).join('\n\n')}`}
        </div>
      )}
    </div>
  );
};

export default SelfReviewPanel;
