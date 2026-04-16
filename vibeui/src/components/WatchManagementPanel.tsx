/* eslint-disable @typescript-eslint/no-explicit-any */
import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ChevronRight, Watch, Plus, Trash2, RefreshCw, QrCode, Shield, Wifi } from "lucide-react";

/** Detect device platform from the model string recorded at registration. */
function detectPlatform(model: string): "apple" | "wear" | "unknown" {
  const m = model.toLowerCase();
  if (m.includes("apple watch") || m.includes("watch series") || m.includes("watch ultra") || m.includes("watch se")) return "apple";
  if (m.includes("pixel watch") || m.includes("galaxy watch") || m.includes("fossil") || m.includes("wear os")) return "wear";
  // Wear OS devices register with manufacturer + model (e.g. "Google Pixel Watch 3")
  if (m.startsWith("samsung") || m.startsWith("google") || m.startsWith("fossil") || m.startsWith("mobvoi")) return "wear";
  return "unknown";
}

function PlatformBadge({ model }: { model: string }) {
  const platform = detectPlatform(model);
  const label = platform === "apple" ? "watchOS" : platform === "wear" ? "Wear OS" : "Watch";
  const color = platform === "apple" ? "var(--accent-color)" : platform === "wear" ? "#4CAF50" : "var(--text-secondary)";
  return (
    <span style={{
      fontSize: 10, fontWeight: 600, color, background: `${color}20`,
      borderRadius: 4, padding: "1px 5px", letterSpacing: "0.03em",
    }}>
      {label}
    </span>
  );
}

interface WatchDevice {
  device_id: string;
  name: string;
  model: string;
  os_version: string;
  registered_at: number;
  last_seen: number;
  revoked: boolean;
  wrist_suspended: boolean;
}

interface PairingInfo {
  endpoint: string;
  nonce: string;
  machine_id: string;
  expires_at: number;
  version: string;
}

export function WatchManagementPanel() {
  const [devices, setDevices] = useState<WatchDevice[]>([]);
  const [pairing, setPairing] = useState<PairingInfo | null>(null);
  const [loading, setLoading] = useState(false);
  const [pairingLoading, setPairingLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [showQR, setShowQR] = useState(false);
  const [revoking, setRevoking] = useState<string | null>(null);

  const loadDevices = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const result = await invoke<{ devices: WatchDevice[] }>("list_watch_devices");
      setDevices(result.devices ?? []);
    } catch (e: any) {
      setError(e?.toString() ?? "Failed to load watch devices");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { loadDevices(); }, [loadDevices]);

  const startPairing = async () => {
    setPairingLoading(true);
    setError(null);
    try {
      const info = await invoke<PairingInfo>("get_watch_pairing_info");
      setPairing(info);
      setShowQR(true);
    } catch (e: any) {
      setError(e?.toString() ?? "Failed to generate pairing info. Is the daemon running?");
    } finally {
      setPairingLoading(false);
    }
  };

  const revokeDevice = async (deviceId: string) => {
    setRevoking(deviceId);
    try {
      await invoke("revoke_watch_device", { deviceId });
      setDevices(prev => prev.filter(d => d.device_id !== deviceId));
    } catch (e: any) {
      setError(e?.toString() ?? "Failed to revoke device");
    } finally {
      setRevoking(null);
    }
  };

  const formatDate = (unix: number) => {
    if (!unix) return "—";
    return new Date(unix * 1000).toLocaleDateString(undefined, {
      month: "short", day: "numeric", hour: "2-digit", minute: "2-digit",
    });
  };

  const activeDevices = devices.filter(d => !d.revoked);
  const revokedDevices = devices.filter(d => d.revoked);

  return (
    <div style={{ padding: "var(--spacing-md)", maxWidth: 640, margin: "0 auto" }}>
      {/* Header */}
      <div style={{ display: "flex", alignItems: "center", gap: "var(--spacing-sm)", marginBottom: "var(--spacing-lg)" }}>
        <Watch size={20} style={{ color: "var(--accent-color)" }} />
        <h2 style={{ margin: 0, fontSize: "var(--font-size-lg)", fontWeight: 600 }}>
          Watches
        </h2>
        <div style={{ flex: 1 }} />
        <button
          onClick={loadDevices}
          disabled={loading}
          style={{
            background: "none", border: "none", cursor: "pointer",
            color: "var(--text-secondary)", padding: 4,
          }}
          title="Refresh"
        >
          <RefreshCw size={14} style={{ opacity: loading ? 0.4 : 1 }} />
        </button>
      </div>

      {error && (
        <div style={{
          background: "var(--error-bg, rgba(239,68,68,0.1))",
          border: "1px solid var(--error-color)",
          borderRadius: "var(--radius-sm)",
          padding: "var(--spacing-sm)",
          marginBottom: "var(--spacing-md)",
          color: "var(--error-color)",
          fontSize: "var(--font-size-sm)",
        }}>
          {error}
        </div>
      )}

      {/* Security info */}
      <div style={{
        background: "var(--bg-secondary)",
        borderRadius: "var(--radius-md)",
        padding: "var(--spacing-md)",
        marginBottom: "var(--spacing-lg)",
        display: "flex",
        gap: "var(--spacing-sm)",
        alignItems: "flex-start",
      }}>
        <Shield size={16} style={{ color: "var(--accent-color)", flexShrink: 0, marginTop: 2 }} />
        <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", lineHeight: 1.5 }}>
          <strong style={{ color: "var(--text-primary)" }}>End-to-end secure.</strong>{" "}
          Private keys live in the device's secure hardware (Secure Enclave on Apple Watch,
          StrongBox / TEE on Wear OS) and never leave the device.
          Sessions auto-lock when the watch is removed from your wrist.
          Tokens expire after 15 minutes and are renewed with cryptographic proof.
        </div>
      </div>

      {/* Transport info */}
      <div style={{
        background: "var(--bg-secondary)",
        borderRadius: "var(--radius-md)",
        padding: "var(--spacing-sm) var(--spacing-md)",
        marginBottom: "var(--spacing-lg)",
        display: "flex",
        gap: "var(--spacing-sm)",
        alignItems: "center",
        fontSize: "var(--font-size-xs)",
        color: "var(--text-secondary)",
      }}>
        <Wifi size={13} />
        <span>
          Transport: <strong>LAN</strong> when on same network →{" "}
          <strong>Tailscale</strong> when remote →{" "}
          <strong>Phone relay</strong> when offline (WatchConnectivity on iOS, Data Layer on Android)
        </span>
      </div>

      {/* Add watch button */}
      <button
        onClick={startPairing}
        disabled={pairingLoading}
        style={{
          display: "flex", alignItems: "center", gap: "var(--spacing-sm)",
          width: "100%", padding: "var(--spacing-sm) var(--spacing-md)",
          background: "var(--accent-color)", color: "var(--btn-primary-fg)",
          border: "none", borderRadius: "var(--radius-sm)",
          cursor: pairingLoading ? "wait" : "pointer",
          fontSize: "var(--font-size-sm)", fontWeight: 500,
          marginBottom: "var(--spacing-lg)",
          opacity: pairingLoading ? 0.7 : 1,
        }}
      >
        {pairingLoading ? (
          <RefreshCw size={14} style={{ animation: "spin 1s linear infinite" }} />
        ) : (
          <Plus size={14} />
        )}
        {pairingLoading ? "Generating pairing QR…" : "Pair New Watch"}
        <div style={{ flex: 1 }} />
        <QrCode size={14} />
      </button>

      {/* QR Code modal */}
      {showQR && pairing && (
        <QRModal pairing={pairing} onClose={() => { setShowQR(false); loadDevices(); }} />
      )}

      {/* Active devices */}
      {activeDevices.length > 0 && (
        <div style={{ marginBottom: "var(--spacing-lg)" }}>
          <h3 style={{ fontSize: "var(--font-size-sm)", fontWeight: 600, marginBottom: "var(--spacing-sm)", color: "var(--text-secondary)" }}>
            PAIRED DEVICES ({activeDevices.length})
          </h3>
          <div style={{ display: "flex", flexDirection: "column", gap: "var(--spacing-sm)" }}>
            {activeDevices.map(device => (
              <DeviceCard
                key={device.device_id}
                device={device}
                onRevoke={() => revokeDevice(device.device_id)}
                revoking={revoking === device.device_id}
                formatDate={formatDate}
              />
            ))}
          </div>
        </div>
      )}

      {activeDevices.length === 0 && !loading && (
        <div style={{
          textAlign: "center", padding: "var(--spacing-xl)",
          color: "var(--text-secondary)", fontSize: "var(--font-size-sm)",
        }}>
          <Watch size={32} style={{ opacity: 0.3, marginBottom: "var(--spacing-sm)", display: "block", margin: "0 auto var(--spacing-sm)" }} />
          No watches paired yet.
          <br />
          Click <strong>Pair New Watch</strong> above to pair an Apple Watch or Wear OS device.
        </div>
      )}

      {/* Revoked devices (collapsible) */}
      {revokedDevices.length > 0 && (
        <RevokedSection devices={revokedDevices} />
      )}
    </div>
  );
}

// ── Device card ───────────────────────────────────────────────────────────────

function DeviceCard({
  device, onRevoke, revoking, formatDate,
}: {
  device: WatchDevice;
  onRevoke: () => void;
  revoking: boolean;
  formatDate: (n: number) => string;
}) {
  const [showConfirm, setShowConfirm] = useState(false);

  return (
    <div style={{
      background: "var(--bg-secondary)",
      borderRadius: "var(--radius-md)",
      padding: "var(--spacing-md)",
      border: "1px solid var(--border-color)",
    }}>
      <div style={{ display: "flex", alignItems: "flex-start", gap: "var(--spacing-sm)" }}>
        <Watch size={18} style={{ color: "var(--accent-color)", flexShrink: 0, marginTop: 1 }} />
        <div style={{ flex: 1, minWidth: 0 }}>
          <div style={{ fontWeight: 600, fontSize: "var(--font-size-sm)" }}>
            {device.name}
          </div>
          <div style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)", marginTop: 2, display: "flex", alignItems: "center", gap: 6 }}>
            <PlatformBadge model={device.model} />
            <span>{device.model} · {device.os_version}</span>
          </div>
          <div style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)", marginTop: 4, display: "flex", gap: "var(--spacing-md)", flexWrap: "wrap" }}>
            <span>Paired {formatDate(device.registered_at)}</span>
            <span>Last seen {formatDate(device.last_seen)}</span>
          </div>
          {device.wrist_suspended && (
            <div style={{
              marginTop: 4,
              fontSize: "var(--font-size-xs)",
              color: "var(--warning-color)",
              display: "flex", alignItems: "center", gap: 4,
            }}>
              ⚠ Session locked (watch off wrist)
            </div>
          )}
        </div>
        <button
          onClick={() => setShowConfirm(true)}
          disabled={revoking}
          style={{
            background: "none", border: "none", cursor: "pointer",
            color: "var(--error-color)", padding: 4, opacity: revoking ? 0.4 : 1,
          }}
          title="Revoke access"
        >
          {revoking ? <RefreshCw size={14} /> : <Trash2 size={14} />}
        </button>
      </div>
      {showConfirm && (
        <div style={{
          marginTop: "var(--spacing-sm)",
          padding: "var(--spacing-sm)",
          background: "var(--error-bg, rgba(239,68,68,0.1))",
          borderRadius: "var(--radius-sm)",
          fontSize: "var(--font-size-xs)",
        }}>
          <p style={{ margin: "0 0 var(--spacing-xs)", color: "var(--text-primary)" }}>
            Revoke access for <strong>{device.name}</strong>? This immediately invalidates all its tokens.
          </p>
          <div style={{ display: "flex", gap: "var(--spacing-sm)" }}>
            <button
              onClick={() => { onRevoke(); setShowConfirm(false); }}
              style={{
                background: "var(--error-color)", color: "#fff",
                border: "none", borderRadius: "var(--radius-xs-plus)",
                padding: "2px 10px", cursor: "pointer", fontSize: "var(--font-size-xs)",
              }}
            >Revoke</button>
            <button
              onClick={() => setShowConfirm(false)}
              style={{
                background: "var(--bg-primary)", color: "var(--text-secondary)",
                border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)",
                padding: "2px 10px", cursor: "pointer", fontSize: "var(--font-size-xs)",
              }}
            >Cancel</button>
          </div>
        </div>
      )}
    </div>
  );
}

// ── Revoked section ───────────────────────────────────────────────────────────

function RevokedSection({ devices }: { devices: WatchDevice[] }) {
  const [open, setOpen] = useState(false);
  return (
    <div>
      <button
        onClick={() => setOpen(o => !o)}
        style={{
          background: "none", border: "none", cursor: "pointer",
          display: "flex", alignItems: "center", gap: "var(--spacing-xs)",
          color: "var(--text-secondary)", fontSize: "var(--font-size-xs)", padding: 0,
          marginBottom: open ? "var(--spacing-sm)" : 0,
        }}
      >
        <ChevronRight size={12} style={{ transform: open ? "rotate(90deg)" : "none", transition: "transform 0.15s" }} />
        {devices.length} revoked device{devices.length !== 1 ? "s" : ""}
      </button>
      {open && (
        <div style={{ display: "flex", flexDirection: "column", gap: "var(--spacing-xs)" }}>
          {devices.map(d => (
            <div key={d.device_id} style={{
              padding: "var(--spacing-sm)",
              background: "var(--bg-secondary)",
              borderRadius: "var(--radius-sm)",
              opacity: 0.5,
              fontSize: "var(--font-size-xs)",
              display: "flex", alignItems: "center", gap: "var(--spacing-sm)",
            }}>
              <Watch size={12} />
              <span>{d.name}</span>
              <span style={{ color: "var(--text-secondary)" }}>· {d.model}</span>
              <span style={{ flex: 1 }} />
              <span style={{ color: "var(--error-color)" }}>Revoked</span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

// ── QR Code modal ─────────────────────────────────────────────────────────────

function QRModal({ pairing, onClose }: { pairing: PairingInfo; onClose: () => void }) {
  const payload = JSON.stringify(pairing);
  const expiresIn = Math.max(0, pairing.expires_at - Math.floor(Date.now() / 1000));
  const [activeTab, setActiveTab] = useState<"qr" | "manual">("qr");
  const [copied, setCopied] = useState(false);

  // Generate a simple QR code URL using a public QR API for display
  // (payload is non-sensitive — nonce + endpoint only, no keys)
  const qrUrl = `https://api.qrserver.com/v1/create-qr-code/?size=180x180&data=${encodeURIComponent(payload)}`;

  const copyJson = async () => {
    try {
      await navigator.clipboard.writeText(payload);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      // fallback: select textarea
    }
  };

  const tabStyle = (tab: "qr" | "manual") => ({
    flex: 1,
    padding: "6px 0",
    background: activeTab === tab ? "var(--accent-color)" : "var(--bg-secondary)",
    color: activeTab === tab ? "var(--btn-primary-fg)" : "var(--text-secondary)",
    border: "1px solid var(--border-color)",
    borderRadius: tab === "qr" ? "var(--radius-sm) 0 0 var(--radius-sm)" : "0 var(--radius-sm) var(--radius-sm) 0",
    cursor: "pointer",
    fontSize: "var(--font-size-xs)",
    fontWeight: activeTab === tab ? 600 : 400,
  });

  return (
    <div style={{
      position: "fixed", inset: 0, background: "rgba(0,0,0,0.6)",
      display: "flex", alignItems: "center", justifyContent: "center",
      zIndex: 1000,
    }}
      onClick={(e) => { if (e.target === e.currentTarget) onClose(); }}
    >
      <div style={{
        background: "var(--bg-primary)",
        borderRadius: "var(--radius-lg)",
        padding: "var(--spacing-xl)",
        width: 300, maxWidth: "90vw",
        textAlign: "center",
        boxShadow: "0 24px 64px rgba(0,0,0,0.4)",
      }}>
        <h3 style={{ margin: "0 0 var(--spacing-sm)", fontSize: "var(--font-size-md)" }}>
          Pair Watch
        </h3>

        {/* Tab switcher */}
        <div style={{ display: "flex", marginBottom: "var(--spacing-md)" }}>
          <button style={tabStyle("qr")} onClick={() => setActiveTab("qr")}>QR Code</button>
          <button style={tabStyle("manual")} onClick={() => setActiveTab("manual")}>Manual</button>
        </div>

        {activeTab === "qr" && (
          <>
            <p style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)", margin: "0 0 var(--spacing-md)" }}>
              Open VibeCody on your Apple Watch or Wear OS watch and scan this QR code.
              Valid for {Math.floor(expiresIn / 60)}:{String(expiresIn % 60).padStart(2, "0")}.
            </p>
            <div style={{
              background: "#fff", padding: 8,
              borderRadius: "var(--radius-sm)",
              display: "inline-block",
              marginBottom: "var(--spacing-md)",
            }}>
              <img src={qrUrl} alt="Pairing QR" width={180} height={180} />
            </div>
            <p style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)", margin: "0 0 var(--spacing-md)" }}>
              The watch uses its <strong>Secure Enclave</strong> (Apple) or <strong>StrongBox / TEE</strong> (Wear OS)
              to generate a key pair during pairing. Your private key never leaves the device.
            </p>
          </>
        )}

        {activeTab === "manual" && (
          <>
            <p style={{ fontSize: "var(--font-size-xs)", color: "var(--text-secondary)", margin: "0 0 var(--spacing-sm)", textAlign: "left" }}>
              Copy this JSON and paste it into the watch app's <strong>Manual Pairing</strong> field.
            </p>
            <textarea
              readOnly
              value={payload}
              rows={6}
              style={{
                width: "100%",
                fontFamily: "monospace",
                fontSize: "var(--font-size-xs)",
                background: "var(--bg-secondary)",
                color: "var(--text-primary)",
                border: "1px solid var(--border-color)",
                borderRadius: "var(--radius-sm)",
                padding: "var(--spacing-xs)",
                resize: "none",
                boxSizing: "border-box",
                marginBottom: "var(--spacing-sm)",
              }}
            />
            <button
              onClick={copyJson}
              style={{
                width: "100%", padding: "var(--spacing-sm)",
                background: copied ? "var(--success-color, #22c55e)" : "var(--accent-color)",
                color: "var(--btn-primary-fg)",
                border: "none",
                borderRadius: "var(--radius-sm)",
                cursor: "pointer", fontSize: "var(--font-size-sm)",
                fontWeight: 500,
                marginBottom: "var(--spacing-sm)",
              }}
            >
              {copied ? "Copied!" : "Copy JSON"}
            </button>
          </>
        )}

        <button
          onClick={onClose}
          style={{
            width: "100%", padding: "var(--spacing-sm)",
            background: "var(--bg-secondary)",
            border: "1px solid var(--border-color)",
            borderRadius: "var(--radius-sm)",
            cursor: "pointer", fontSize: "var(--font-size-sm)",
            color: "var(--text-primary)",
          }}
        >
          Done
        </button>
      </div>
    </div>
  );
}
