/**
 * HtmlPreview — renders raw HTML content in a sandboxed iframe.
 *
 * Features:
 *   • Sandboxed iframe for safe rendering (no script execution by default)
 *   • Responsive sizing to fill the editor area
 *   • Theme-aware background
 *   • Toolbar with device-size presets (Desktop, Tablet, Mobile)
 *   • Open-in-browser button (external)
 *   • Refresh button
 */

import { useState, useRef, useCallback, useEffect } from "react";
import "./HtmlPreview.css";

interface HtmlPreviewProps {
  /** Raw HTML content string */
  content: string;
  /** File path (used for display and blob URL) */
  filePath?: string;
}

type DevicePreset = "responsive" | "desktop" | "tablet" | "mobile";

const DEVICE_WIDTHS: Record<DevicePreset, string> = {
  responsive: "100%",
  desktop: "1440px",
  tablet: "768px",
  mobile: "375px",
};

export function HtmlPreview({ content, filePath }: HtmlPreviewProps) {
  const [device, setDevice] = useState<DevicePreset>("responsive");
  const [scriptsEnabled, setScriptsEnabled] = useState(false);
  const iframeRef = useRef<HTMLIFrameElement>(null);
  const [key, setKey] = useState(0); // for force-refreshing the iframe

  const fileName = filePath?.split("/").pop() || filePath?.split("\\").pop() || "preview";

  // Build blob URL from the HTML content
  const [blobUrl, setBlobUrl] = useState<string | null>(null);

  useEffect(() => {
    const blob = new Blob([content], { type: "text/html;charset=utf-8" });
    const url = URL.createObjectURL(blob);
    setBlobUrl(url);
    return () => URL.revokeObjectURL(url);
  }, [content, key]);

  const refresh = useCallback(() => setKey(k => k + 1), []);

  const toggleScripts = useCallback(() => {
    setScriptsEnabled(s => !s);
    setKey(k => k + 1); // force iframe reload with new sandbox policy
  }, []);

  return (
    <div className="html-preview">
      {/* ── Toolbar ──────────────────────────────────────────────── */}
      <div className="html-preview-toolbar">
        <div className="toolbar-group">
          {(["responsive", "desktop", "tablet", "mobile"] as DevicePreset[]).map(d => (
            <button
              key={d}
              className={`device-btn${device === d ? " active" : ""}`}
              onClick={() => setDevice(d)}
              title={d.charAt(0).toUpperCase() + d.slice(1)}
            >
              {d === "responsive" ? "↔" : d === "desktop" ? "🖥" : d === "tablet" ? "📱" : "📲"}
            </button>
          ))}
        </div>

        <div className="toolbar-separator" />

        <div className="toolbar-group">
          <button onClick={refresh} title="Refresh Preview" className="preview-action-btn">
            ↻
          </button>
          <button
            onClick={toggleScripts}
            title={scriptsEnabled ? "Disable Scripts" : "Enable Scripts"}
            className={`preview-action-btn${scriptsEnabled ? " active" : ""}`}
          >
            JS
          </button>
        </div>

        <div className="file-info">
          <span className="info-badge">HTML Preview</span>
          <span className="info-badge">{fileName}</span>
          {device !== "responsive" && (
            <span className="info-badge">{DEVICE_WIDTHS[device]}</span>
          )}
        </div>
      </div>

      {/* ── Preview area ─────────────────────────────────────────── */}
      <div className="html-preview-canvas">
        <div
          className={`html-preview-frame-wrapper device-${device}`}
        >
          {blobUrl && (
            <iframe
              ref={iframeRef}
              key={key}
              src={blobUrl}
              title={`HTML Preview: ${fileName}`}
              className="html-preview-iframe"
              sandbox={
                scriptsEnabled
                  ? "allow-scripts allow-same-origin"
                  : "allow-same-origin"
              }
            />
          )}
        </div>
      </div>
    </div>
  );
}
