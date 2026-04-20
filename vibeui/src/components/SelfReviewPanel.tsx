import React, { useState, useEffect } from 'react';
import { invoke } from "@tauri-apps/api/core";

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
// Helpers
// ---------------------------------------------------------------------------

const severityColors: Record<string, string> = {
  info: 'var(--text-secondary)',
  warning: 'var(--warning-color)',
  error: 'var(--error-color)',
  critical: 'var(--error-color)',
};

const checkIcons: Record<string, string> = {
  build: 'B', lint: 'L', test: 'T', security: 'S', format: 'F', typecheck: 'TC',
};

const DEFAULT_CONFIG: ReviewConfig = {
  enabled: true, maxRetries: 3, checks: ['build', 'lint', 'test', 'security'],
  failOnWarning: false, minBlockingSeverity: 'error',
};

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

const SelfReviewPanel: React.FC = () => {
  const [config, setConfig] = useState<ReviewConfig>(DEFAULT_CONFIG);
  const [iterations, setIterations] = useState<ReviewIteration[]>([]);
  const [tab, setTab] = useState<'results' | 'config' | 'report'>('results');
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const loadData = async () => {
      setLoading(true);
      try {
        const [iterationsData, configData] = await Promise.all([
          invoke<ReviewIteration[]>("get_selfreview_iterations"),
          invoke<ReviewConfig>("get_selfreview_config"),
        ]);
        setIterations(iterationsData);
        setConfig(configData);
      } catch (err) {
        console.error("Failed to load self-review data:", err);
      } finally {
        setLoading(false);
      }
    };
    loadData();
  }, []);

  const handleSaveConfig = async (newConfig: ReviewConfig) => {
    setConfig(newConfig);
    try {
      await invoke("save_selfreview_config", { config: newConfig });
    } catch (err) {
      console.error("Failed to save self-review config:", err);
    }
  };

  const latestIteration = iterations[iterations.length - 1];
  const totalFindings = iterations.reduce((sum, it) => sum + it.checks.reduce((s, c) => s + c.findings.length, 0), 0);

  if (loading) {
    return (
      <div className="panel-container">
        <div className="panel-header"><h3>Agent Self-Review Gate</h3></div>
        <div className="panel-body"><div className="panel-loading">Loading self-review data...</div></div>
      </div>
    );
  }

  return (
    <div className="panel-container">
      <div className="panel-header"><h3>Agent Self-Review Gate</h3></div>
      <div className="panel-body">

      {/* Status banner */}
      {iterations.length === 0 ? (
        <div style={{
          padding: '12px 16px', borderRadius: "var(--radius-sm-alt)", marginBottom: 16,
          background: 'rgba(107, 114, 128, 0.1)',
          border: '1px solid var(--text-secondary)',
          display: 'flex', alignItems: 'center', gap: 12,
        }}>
          <span style={{ fontSize: 24 }}>-</span>
          <div>
            <div style={{ fontWeight: 700 }}>No review iterations yet</div>
            <div style={{ fontSize: "var(--font-size-base)", color: 'var(--text-secondary)' }}>Run an agent self-review to see results here.</div>
          </div>
        </div>
      ) : (
        <div style={{
          padding: '12px 16px', borderRadius: "var(--radius-sm-alt)", marginBottom: 16,
          background: latestIteration?.passed ? 'rgba(16, 185, 129, 0.1)' : 'rgba(239, 68, 68, 0.1)',
          border: `1px solid ${latestIteration?.passed ? 'var(--success-color)' : 'var(--error-color)'}`,
          display: 'flex', alignItems: 'center', gap: 12,
        }}>
          <span style={{ fontSize: 24 }}>{latestIteration?.passed ? '\u2713' : '\u2717'}</span>
          <div>
            <div style={{ fontWeight: 700 }}>
              {latestIteration?.passed ? 'All checks passed' : 'Checks failed — agent iterating'}
            </div>
            <div style={{ fontSize: "var(--font-size-base)", color: 'var(--text-secondary)' }}>
              {iterations.length} iteration(s) · {totalFindings} total findings
            </div>
          </div>
        </div>
      )}

      {/* Stats */}
      <div style={{ display: 'flex', gap: 12, marginBottom: 16, flexWrap: 'wrap' }}>
        {[
          { label: 'Iterations', value: iterations.length },
          { label: 'Max Retries', value: config.maxRetries },
          { label: 'Checks', value: config.checks.length },
          { label: 'Findings', value: totalFindings },
        ].map((s) => (
          <div key={s.label} style={{ background: 'var(--bg-secondary)', padding: '8px 16px', borderRadius: "var(--radius-sm)", textAlign: 'center' }}>
            <div style={{ fontSize: 18, fontWeight: 700, fontFamily: "var(--font-mono)", color: 'var(--accent-color)' }}>{s.value}</div>
            <div style={{ fontSize: "var(--font-size-sm)", color: 'var(--text-secondary)' }}>{s.label}</div>
          </div>
        ))}
      </div>

      {/* Tabs */}
      <div className="panel-tab-bar" style={{ marginBottom: 12 }}>
        {(['results', 'config', 'report'] as const).map((t) => (
          <button className={`panel-tab${tab === t ? " active" : ""}`} key={t} onClick={() => setTab(t)}>
            {t.charAt(0).toUpperCase() + t.slice(1)}
          </button>
        ))}
      </div>

      {/* Results tab */}
      {tab === 'results' && (
        <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
          {iterations.length === 0 ? (
            <div style={{ padding: 24, textAlign: 'center', color: 'var(--text-secondary)', fontSize: "var(--font-size-md)" }}>No review iterations found. Run an agent task with self-review enabled.</div>
          ) : (
            iterations.map((iter) => (
              <div key={iter.iteration} style={{
                background: 'var(--bg-secondary)', padding: 12, borderRadius: "var(--radius-sm-alt)",
                border: `1px solid ${iter.passed ? 'var(--success-color)' : 'var(--error-color)'}`,
              }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 8 }}>
                  <strong>Iteration {iter.iteration}</strong>
                  <span style={{
                    fontSize: "var(--font-size-sm)", padding: '1px 8px', borderRadius: 3,
                    background: iter.passed ? 'var(--success-color)' : 'var(--error-color)',
                    color: 'var(--btn-primary-fg)', fontWeight: 600,
                  }}>
                    {iter.passed ? 'PASS' : 'FAIL'}
                  </span>
                </div>

                {/* Check results grid */}
                <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(200px, 1fr))', gap: 8, marginBottom: iter.checks.some(c => c.findings.length > 0) ? 8 : 0 }}>
                  {iter.checks.map((check) => (
                    <div key={check.kind} style={{
                      padding: 8, borderRadius: "var(--radius-sm)",
                      background: check.passed ? 'rgba(16, 185, 129, 0.08)' : 'rgba(239, 68, 68, 0.08)',
                      border: `1px solid ${check.passed ? 'rgba(16, 185, 129, 0.3)' : 'rgba(239, 68, 68, 0.3)'}`,
                    }}>
                      <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
                        <span style={{
                          display: 'inline-block', width: 22, height: 22, lineHeight: '22px', textAlign: 'center',
                          borderRadius: "var(--radius-xs-plus)", fontSize: "var(--font-size-xs)", fontWeight: 700,
                          background: check.passed ? 'var(--success-color)' : 'var(--error-color)', color: 'var(--btn-primary-fg)',
                        }}>{checkIcons[check.kind] || check.kind[0].toUpperCase()}</span>
                        <span style={{ fontWeight: 600, fontSize: "var(--font-size-md)" }}>{check.kind}</span>
                        <span style={{ fontSize: "var(--font-size-sm)", color: 'var(--text-secondary)', marginLeft: 'auto' }}>{check.durationMs}ms</span>
                      </div>
                      {check.command && (
                        <div style={{ fontSize: "var(--font-size-xs)", fontFamily: 'var(--font-mono)', color: 'var(--text-secondary)', marginTop: 4 }}>{check.command}</div>
                      )}
                      {check.findings.length > 0 && (
                        <div style={{ fontSize: "var(--font-size-sm)", color: 'var(--text-secondary)', marginTop: 4 }}>
                          {check.findings.length} finding(s)
                        </div>
                      )}
                    </div>
                  ))}
                </div>

                {/* Findings */}
                {iter.checks.some(c => c.findings.length > 0) && (
                  <div style={{ fontSize: "var(--font-size-base)" }}>
                    {iter.checks.filter(c => c.findings.length > 0).flatMap(c => c.findings).map((f, i) => (
                      <div key={i} style={{ display: 'flex', gap: 6, padding: '3px 0', borderBottom: '1px solid var(--border-color)' }}>
                        <span style={{ color: severityColors[f.severity], fontWeight: 600, fontSize: "var(--font-size-sm)", minWidth: 55 }}>{f.severity}</span>
                        <span style={{ color: 'var(--text-primary)' }}>{f.message}</span>
                        {f.file && <span style={{ color: 'var(--text-secondary)', fontFamily: 'var(--font-mono)', marginLeft: 'auto', fontSize: "var(--font-size-sm)" }}>{f.file}{f.line ? `:${f.line}` : ''}</span>}
                      </div>
                    ))}
                  </div>
                )}
              </div>
            ))
          )}
        </div>
      )}

      {/* Config tab */}
      {tab === 'config' && (
        <div style={{ background: 'var(--bg-secondary)', padding: 16, borderRadius: "var(--radius-sm-alt)" }}>
          <h3 style={{ margin: '0 0 12px' }}>Self-Review Configuration</h3>
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 12 }}>
            <label style={{ fontSize: "var(--font-size-base)" }}>
              <input type="checkbox" checked={config.enabled} onChange={(e) => handleSaveConfig({ ...config, enabled: e.target.checked })} /> Enabled
            </label>
            <label style={{ fontSize: "var(--font-size-base)" }}>
              <input type="checkbox" checked={config.failOnWarning} onChange={(e) => handleSaveConfig({ ...config, failOnWarning: e.target.checked })} /> Fail on warnings
            </label>
            <label style={{ fontSize: "var(--font-size-base)" }}>
              Max retries
              <input type="number" value={config.maxRetries} min={1} max={10} onChange={(e) => handleSaveConfig({ ...config, maxRetries: parseInt(e.target.value) || 3 })} style={{ width: '100%', padding: 6, background: 'var(--bg-primary)', color: 'var(--text-primary)', border: '1px solid var(--border-color)', borderRadius: "var(--radius-xs-plus)" }} />
            </label>
            <label style={{ fontSize: "var(--font-size-base)" }}>
              Min blocking severity
              <select value={config.minBlockingSeverity} onChange={(e) => handleSaveConfig({ ...config, minBlockingSeverity: e.target.value })} style={{ width: '100%', padding: 6, background: 'var(--bg-primary)', color: 'var(--text-primary)', border: '1px solid var(--border-color)', borderRadius: "var(--radius-xs-plus)" }}>
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
              <label key={check} style={{ fontSize: "var(--font-size-base)", display: 'flex', alignItems: 'center', gap: 4, background: 'var(--bg-primary)', padding: '4px 12px', borderRadius: "var(--radius-xs-plus)" }}>
                <input type="checkbox" checked={config.checks.includes(check)} onChange={(e) => {
                  const newConfig = e.target.checked
                    ? { ...config, checks: [...config.checks, check] }
                    : { ...config, checks: config.checks.filter(c => c !== check) };
                  handleSaveConfig(newConfig);
                }} />
                {check}
              </label>
            ))}
          </div>
          <div style={{ marginTop: 16, padding: 12, background: 'var(--bg-primary)', borderRadius: "var(--radius-sm)", fontFamily: 'var(--font-mono)', fontSize: "var(--font-size-sm)", color: 'var(--text-secondary)' }}>
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
        <div style={{ fontFamily: 'var(--font-mono)', fontSize: "var(--font-size-base)", background: 'var(--bg-secondary)', padding: 16, borderRadius: "var(--radius-sm-alt)", whiteSpace: 'pre-wrap', color: 'var(--text-primary)', lineHeight: 1.6 }}>
{iterations.length === 0 ? 'No review data available.' : `# Self-Review Report

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
    </div>
  );
};

export default SelfReviewPanel;
