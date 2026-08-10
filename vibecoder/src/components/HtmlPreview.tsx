/**
 * HtmlPreview — renders raw HTML content in a sandboxed iframe.
 *
 * Features:
 *   • Sandboxed iframe for safe rendering (no script execution by default)
 *   • Responsive sizing to fill the editor area
 *   • Theme-aware background
 *   • Toolbar with device-size presets (Desktop, Tablet, Mobile)
 *   • Refresh button
 *
 * ## Why the preview can look "broken"
 *
 * Scripts are off by default, which is right for a file that may have arrived
 * from anywhere — but a JS-rendered page then paints its loading skeleton and
 * stops there forever, with nothing on screen saying why. The notices below
 * exist so a blank or spinning preview always explains itself: an unexplained
 * empty pane is indistinguishable from a crash.
 */

import { useState, useRef, useCallback, useEffect, useMemo } from "react";
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

  /**
   * What this document needs that the preview isn't giving it.
   *
   * Cheap string checks, not a parse: the point is to explain a stalled
   * preview, and a false positive costs one dismissible line of text while a
   * false negative costs the user ten minutes wondering what broke.
   */
  const needs = useMemo(() => {
    const head = content.slice(0, 200_000);
    return {
      scripts: /<script[\s>]/i.test(head),
      // A page pulling its own code off the network can't render from a local
      // file at all — the iframe inherits this app's CSP, which is 'self'-only.
      remoteCode:
        /<script[^>]+src=["']https?:\/\//i.test(head) ||
        /<link[^>]+rel=["']modulepreload["']/i.test(head),
    };
  }, [content]);

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

      {/* ── Why nothing is rendering ─────────────────────────────── */}
      {needs.scripts && !scriptsEnabled && (
        <div className="html-preview-notice" role="status">
          <span>
            This page builds itself with JavaScript, which is off for preview.
          </span>
          <button className="html-preview-notice__action" onClick={toggleScripts}>
            Enable scripts
          </button>
        </div>
      )}
      {needs.remoteCode && scriptsEnabled && (
        <div className="html-preview-notice html-preview-notice--warn" role="status">
          <span>
            This page loads its code from the network. Previews run offline
            under the app's content policy, so it will stay on its loading
            screen — open it in a browser instead.
          </span>
        </div>
      )}

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
              // `allow-scripts` WITHOUT `allow-same-origin`. Granting both to
              // a blob: URL minted by this app hands the framed document our
              // own origin — it could then reach `parent`, this app's
              // localStorage, and the Tauri IPC bridge. That is not a sandbox,
              // and the HTML being previewed is frequently something the user
              // just downloaded. An opaque origin still runs the page's own
              // scripts, which is all a preview needs.
              sandbox={scriptsEnabled ? "allow-scripts" : ""}
            />
          )}
        </div>
      </div>
    </div>
  );
}
