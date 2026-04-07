/**
 * CloudProviderPanel — Cloud Provider Integration panel.
 *
 * Scans codebase for cloud service usage, generates IAM policies,
 * produces IaC templates, and estimates costs via Tauri backend commands.
 */
import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

// ── Types ─────────────────────────────────────────────────────────────────────

interface DetectedService {
  id: string;
  provider: "AWS" | "GCP" | "Azure";
  service: string;
  usage_type: string;
  confidence: number;
  file: string;
  line: number;
}

interface CostEstimate {
  service: string;
  provider: string;
  monthly: number;
  yearly: number;
  tier: string;
  notes: string;
}

interface ScanResult {
  workspace: string;
  detected_services: DetectedService[];
  providers: string[];
  files_scanned: number;
}

interface IamResult {
  provider: string;
  policy: Record<string, unknown>;
  policy_text: string;
}

interface IacResult {
  provider: string;
  format: string;
  template: string;
}

interface CostResult {
  total_monthly_usd: number;
  total_yearly_usd: number;
  services: CostEstimate[];
}

// ── Styles ────────────────────────────────────────────────────────────────────

const panelStyle: React.CSSProperties = { padding: 16, color: "var(--text-primary)", fontFamily: "var(--font-family)", fontSize: 13, height: "100%", flex: 1, minHeight: 0, overflow: "auto", background: "var(--bg-primary)" };
const headingStyle: React.CSSProperties = { margin: "0 0 12px", fontSize: 15, fontWeight: 600, color: "var(--text-primary)" };
const cardStyle: React.CSSProperties = { background: "var(--bg-secondary)", borderRadius: 6, padding: 12, marginBottom: 10, border: "1px solid var(--border-color)" };
const labelStyle: React.CSSProperties = { fontSize: 11, color: "var(--text-secondary)", marginBottom: 4 };
const btnStyle: React.CSSProperties = { padding: "6px 14px", borderRadius: 4, border: "1px solid var(--border-color)", background: "var(--bg-tertiary)", color: "var(--text-primary)", cursor: "pointer", fontSize: 12 };
const tabBtnStyle = (active: boolean): React.CSSProperties => ({ ...btnStyle, background: active ? "var(--accent-primary)" : "var(--bg-tertiary)", color: active ? "var(--text-primary)" : "var(--text-primary)", marginRight: 4 });

const preStyle: React.CSSProperties = { background: "var(--bg-tertiary)", padding: 10, borderRadius: 4, fontSize: 11, overflow: "auto", whiteSpace: "pre-wrap", border: "1px solid var(--border-color)", maxHeight: 400 };
const providerColor: Record<string, string> = { AWS: "var(--warning-color)", GCP: "var(--info-color)", Azure: "var(--accent-primary)" };
const confidenceColor = (c: number) => c >= 0.9 ? "var(--success-color)" : c >= 0.7 ? "var(--warning-color)" : "var(--error-color)";

const thStyle: React.CSSProperties = { textAlign: "left", padding: "6px 10px", borderBottom: "1px solid var(--border-color)", fontSize: 11, color: "var(--text-secondary)" };
const tdStyle: React.CSSProperties = { padding: "6px 10px", borderBottom: "1px solid var(--border-color)", fontSize: 12 };
const errorStyle: React.CSSProperties = { ...cardStyle, color: "var(--error-color)", borderColor: "var(--error-color)" };
const spinnerStyle: React.CSSProperties = { ...cardStyle, color: "var(--text-secondary)", fontStyle: "italic" };

// ── Component ─────────────────────────────────────────────────────────────────

type Tab = "scan" | "iam" | "iac" | "cost";

interface CloudConnection {
  provider: string;
  connected: boolean;
  expired: boolean;
  email: string;
  display_name: string;
}

const CLOUD_OAUTH_MAP: Record<string, string> = {
  AWS: "google",
  GCP: "google",
  Azure: "microsoft",
};

const IAC_FORMATS = ["Terraform", "CloudFormation", "Pulumi"];

export function CloudProviderPanel() {
  const [tab, setTab] = useState<Tab>("scan");
  const [iacFormat, setIacFormat] = useState<string>("Terraform");
  const [connections, setConnections] = useState<CloudConnection[]>([]);

  // Scan state
  const [scanning, setScanning] = useState(false);
  const [scanResult, setScanResult] = useState<ScanResult | null>(null);
  const [scanError, setScanError] = useState<string | null>(null);

  // IAM state
  const [iamLoading, setIamLoading] = useState(false);
  const [iamResult, setIamResult] = useState<IamResult | null>(null);
  const [iamError, setIamError] = useState<string | null>(null);
  const [iamProvider, setIamProvider] = useState<string>("AWS");

  // IaC state
  const [iacLoading, setIacLoading] = useState(false);
  const [iacResult, setIacResult] = useState<IacResult | null>(null);
  const [iacError, setIacError] = useState<string | null>(null);

  // Cost state
  const [costLoading, setCostLoading] = useState(false);
  const [costResult, setCostResult] = useState<CostResult | null>(null);
  const [costError, setCostError] = useState<string | null>(null);

  useEffect(() => {
    invoke<CloudConnection[]>("cloud_oauth_list_connected")
      .then(setConnections)
      .catch(() => {});
  }, []);

  const isCloudConnected = (cloudProvider: string): boolean => {
    const oauthId = CLOUD_OAUTH_MAP[cloudProvider];
    return connections.some(c => c.provider === oauthId && c.connected && !c.expired);
  };

  const connectedProviders = ["AWS", "GCP", "Azure"].filter(isCloudConnected);

  // Detected services as simple JSON objects for passing to backend
  const detectedServices = scanResult?.detected_services ?? [];

  // Unique providers from scan results
  const detectedProviders = Array.from(new Set(detectedServices.map(s => s.provider)));

  const runScan = useCallback(async () => {
    setScanning(true);
    setScanError(null);
    try {
      // Use current directory as workspace
      const result = await invoke<ScanResult>("cloud_provider_scan", { workspace: "." });
      setScanResult(result);
      // Auto-set IAM provider to first detected provider
      if (result.detected_services.length > 0) {
        setIamProvider(result.detected_services[0].provider);
      }
    } catch (e) {
      setScanError(e instanceof Error ? e.message : String(e));
    } finally {
      setScanning(false);
    }
  }, []);

  // Auto-scan on mount
  useEffect(() => {
    runScan();
  }, [runScan]);

  const generateIam = useCallback(async (provider: string) => {
    setIamLoading(true);
    setIamError(null);
    try {
      const result = await invoke<IamResult>("cloud_provider_iam", {
        provider,
        services: detectedServices,
      });
      setIamResult(result);
    } catch (e) {
      setIamError(e instanceof Error ? e.message : String(e));
    } finally {
      setIamLoading(false);
    }
  }, [detectedServices]);

  const generateIac = useCallback(async (provider: string, format: string) => {
    setIacLoading(true);
    setIacError(null);
    try {
      const result = await invoke<IacResult>("cloud_provider_iac", {
        provider,
        format,
        services: detectedServices,
      });
      setIacResult(result);
    } catch (e) {
      setIacError(e instanceof Error ? e.message : String(e));
    } finally {
      setIacLoading(false);
    }
  }, [detectedServices]);

  const estimateCosts = useCallback(async () => {
    setCostLoading(true);
    setCostError(null);
    try {
      const result = await invoke<CostResult>("cloud_provider_cost", {
        services: detectedServices,
      });
      setCostResult(result);
    } catch (e) {
      setCostError(e instanceof Error ? e.message : String(e));
    } finally {
      setCostLoading(false);
    }
  }, [detectedServices]);

  // Auto-fetch data when switching tabs (if scan is done)
  useEffect(() => {
    if (!scanResult || detectedServices.length === 0) return;
    if (tab === "iam" && !iamResult && !iamLoading) {
      generateIam(iamProvider);
    } else if (tab === "iac" && !iacResult && !iacLoading) {
      generateIac(detectedProviders[0] ?? "AWS", iacFormat);
    } else if (tab === "cost" && !costResult && !costLoading) {
      estimateCosts();
    }
  }, [tab, scanResult, detectedServices, iamResult, iamLoading, iacResult, iacLoading, costResult, costLoading, iamProvider, iacFormat, detectedProviders, generateIam, generateIac, estimateCosts]);

  return (
    <div style={panelStyle}>
      <h2 style={headingStyle}>Cloud Provider Integration</h2>

      {/* Connection status banner */}
      <div style={{ ...cardStyle, display: "flex", alignItems: "center", gap: 10, fontSize: 12 }}>
        <span style={{ fontWeight: 600 }}>Connected:</span>
        {connectedProviders.length > 0 ? (
          connectedProviders.map(p => (
            <span key={p} style={{ padding: "2px 8px", borderRadius: 10, fontSize: 11, background: "var(--success-bg)", color: "var(--success-color)" }}>
              {p}
            </span>
          ))
        ) : (
          <span style={{ color: "var(--text-secondary)" }}>
            None — connect providers in Settings &rarr; OAuth
          </span>
        )}
      </div>

      <div style={{ marginBottom: 12 }}>
        <button style={tabBtnStyle(tab === "scan")} onClick={() => setTab("scan")}>Scan</button>
        <button style={tabBtnStyle(tab === "iam")} onClick={() => setTab("iam")}>IAM</button>
        <button style={tabBtnStyle(tab === "iac")} onClick={() => setTab("iac")}>IaC</button>
        <button style={tabBtnStyle(tab === "cost")} onClick={() => setTab("cost")}>Cost</button>
      </div>

      {tab === "scan" && (
        <div>
          <div style={{ marginBottom: 10 }}>
            <button style={btnStyle} onClick={runScan} disabled={scanning}>
              {scanning ? "Scanning..." : "Re-scan Workspace"}
            </button>
          </div>

          {scanning && <div style={spinnerStyle}>Scanning workspace for cloud service patterns...</div>}
          {scanError && <div style={errorStyle}>Scan error: {scanError}</div>}

          {scanResult && !scanning && (
            <>
              <div style={{ ...cardStyle, fontSize: 12 }}>
                Detected {detectedServices.length} cloud service{detectedServices.length !== 1 ? "s" : ""} across{" "}
                {detectedProviders.length} provider{detectedProviders.length !== 1 ? "s" : ""}.
                {scanResult.files_scanned > 0 && (
                  <span style={{ color: "var(--text-secondary)", marginLeft: 8 }}>
                    ({scanResult.files_scanned} files scanned)
                  </span>
                )}
              </div>
              {detectedServices.length === 0 && (
                <div style={{ ...cardStyle, color: "var(--text-secondary)" }}>
                  No cloud services detected in workspace. Make sure source files use AWS/GCP/Azure SDK patterns.
                </div>
              )}
              {detectedServices.map((s) => (
                <div key={s.id} style={{ ...cardStyle, display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                  <div>
                    <span style={{ fontWeight: 600, color: providerColor[s.provider] }}>[{s.provider}]</span>{" "}
                    <span style={{ fontWeight: 600 }}>{s.service}</span>
                    <span style={{ fontSize: 10, color: "var(--text-secondary)", marginLeft: 8 }}>({s.usage_type})</span>
                    <div style={{ fontSize: 10, color: "var(--text-secondary)", marginTop: 2 }}>{s.file}:{s.line}</div>
                  </div>
                  <div style={{ textAlign: "right" }}>
                    <div style={{ fontSize: 11, color: "var(--text-secondary)" }}>Confidence</div>
                    <div style={{ fontWeight: 600, color: confidenceColor(s.confidence) }}>{(s.confidence * 100).toFixed(0)}%</div>
                  </div>
                </div>
              ))}
            </>
          )}
        </div>
      )}

      {tab === "iam" && (
        <div>
          {detectedServices.length === 0 ? (
            <div style={{ ...cardStyle, color: "var(--text-secondary)" }}>
              Run a scan first to detect cloud services, then generate IAM policies.
            </div>
          ) : (
            <>
              <div style={{ marginBottom: 10 }}>
                {detectedProviders.map(p => (
                  <button
                    key={p}
                    style={tabBtnStyle(iamProvider === p)}
                    onClick={() => {
                      setIamProvider(p);
                      setIamResult(null);
                      generateIam(p);
                    }}
                  >
                    {p}
                  </button>
                ))}
              </div>

              {iamLoading && <div style={spinnerStyle}>Generating IAM policy for {iamProvider}...</div>}
              {iamError && <div style={errorStyle}>IAM error: {iamError}</div>}

              {iamResult && !iamLoading && (
                <div style={cardStyle}>
                  <div style={labelStyle}>Generated least-privilege IAM policy for {iamResult.provider} services</div>
                  <pre style={preStyle}>{iamResult.policy_text}</pre>
                  <div style={{ marginTop: 8 }}>
                    <button style={btnStyle} onClick={() => navigator.clipboard?.writeText(iamResult.policy_text)}>Copy Policy</button>
                  </div>
                </div>
              )}
            </>
          )}
        </div>
      )}

      {tab === "iac" && (
        <div>
          {detectedServices.length === 0 ? (
            <div style={{ ...cardStyle, color: "var(--text-secondary)" }}>
              Run a scan first to detect cloud services, then generate IaC templates.
            </div>
          ) : (
            <>
              <div style={{ marginBottom: 10 }}>
                {IAC_FORMATS.map((fmt) => (
                  <button
                    key={fmt}
                    style={tabBtnStyle(iacFormat === fmt)}
                    onClick={() => {
                      setIacFormat(fmt);
                      setIacResult(null);
                      generateIac(detectedProviders[0] ?? "AWS", fmt);
                    }}
                  >
                    {fmt}
                  </button>
                ))}
              </div>

              {iacLoading && <div style={spinnerStyle}>Generating {iacFormat} template...</div>}
              {iacError && <div style={errorStyle}>IaC error: {iacError}</div>}

              {iacResult && !iacLoading && (
                <div style={cardStyle}>
                  <div style={labelStyle}>{iacResult.format} template for detected {iacResult.provider} services</div>
                  <pre style={preStyle}>{iacResult.template}</pre>
                  <div style={{ marginTop: 8 }}>
                    <button style={btnStyle} onClick={() => navigator.clipboard?.writeText(iacResult.template)}>Copy Template</button>
                  </div>
                </div>
              )}
            </>
          )}
        </div>
      )}

      {tab === "cost" && (
        <div>
          {detectedServices.length === 0 ? (
            <div style={{ ...cardStyle, color: "var(--text-secondary)" }}>
              Run a scan first to detect cloud services, then estimate costs.
            </div>
          ) : (
            <>
              <div style={{ marginBottom: 10 }}>
                <button style={btnStyle} onClick={estimateCosts} disabled={costLoading}>
                  {costLoading ? "Estimating..." : "Refresh Estimates"}
                </button>
              </div>

              {costLoading && <div style={spinnerStyle}>Estimating costs for detected services...</div>}
              {costError && <div style={errorStyle}>Cost error: {costError}</div>}

              {costResult && !costLoading && (
                <>
                  <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 10, marginBottom: 12 }}>
                    <div style={cardStyle}>
                      <div style={labelStyle}>Estimated Monthly</div>
                      <div style={{ fontSize: 22, fontWeight: 700, fontFamily: "var(--font-mono)", color: "var(--accent-primary)" }}>${costResult.total_monthly_usd.toFixed(2)}</div>
                    </div>
                    <div style={cardStyle}>
                      <div style={labelStyle}>Estimated Yearly</div>
                      <div style={{ fontSize: 22, fontWeight: 700, fontFamily: "var(--font-mono)", color: "var(--accent-primary)" }}>${costResult.total_yearly_usd.toFixed(2)}</div>
                    </div>
                  </div>
                  <div style={cardStyle}>
                    <table style={{ width: "100%", borderCollapse: "collapse" }}>
                      <thead>
                        <tr>
                          <th style={thStyle}>Service</th>
                          <th style={thStyle}>Provider</th>
                          <th style={thStyle}>Tier</th>
                          <th style={{ ...thStyle, textAlign: "right" }}>Monthly</th>
                          <th style={{ ...thStyle, textAlign: "right" }}>Yearly</th>
                        </tr>
                      </thead>
                      <tbody>
                        {costResult.services.map((c, i) => (
                          <tr key={i}>
                            <td style={tdStyle}>{c.service}</td>
                            <td style={{ ...tdStyle, color: providerColor[c.provider] }}>{c.provider}</td>
                            <td style={{ ...tdStyle, fontSize: 11, color: "var(--text-secondary)" }}>{c.tier}</td>
                            <td style={{ ...tdStyle, textAlign: "right" }}>${c.monthly.toFixed(2)}</td>
                            <td style={{ ...tdStyle, textAlign: "right" }}>${c.yearly.toFixed(2)}</td>
                          </tr>
                        ))}
                      </tbody>
                      <tfoot>
                        <tr>
                          <td colSpan={3} style={{ ...tdStyle, fontWeight: 600 }}>Total</td>
                          <td style={{ ...tdStyle, textAlign: "right", fontWeight: 600 }}>${costResult.total_monthly_usd.toFixed(2)}</td>
                          <td style={{ ...tdStyle, textAlign: "right", fontWeight: 600 }}>${costResult.total_yearly_usd.toFixed(2)}</td>
                        </tr>
                      </tfoot>
                    </table>
                  </div>
                </>
              )}
            </>
          )}
        </div>
      )}
    </div>
  );
}
