/**
 * DrawioPreview — renders Draw.io XML diagrams via the official viewer iframe.
 *
 * Uses viewer.diagrams.net with a data URI to render the diagram offline
 * (assuming the browser caches the viewer or has internet access).
 */

import { useState, useRef, useCallback, useEffect } from "react";
import { Maximize2, Monitor, Tablet, Smartphone } from "lucide-react";
import "./HtmlPreview.css"; // Reuse the same toolbar and container styling

interface DrawioPreviewProps {
  /** Raw XML content of the drawio file */
  content: string;
  /** File path */
  filePath?: string;
}

export function DrawioPreview({ content, filePath }: DrawioPreviewProps) {
  const [device, setDevice] = useState<"responsive" | "desktop" | "tablet" | "mobile">("responsive");
  const iframeRef = useRef<HTMLIFrameElement>(null);
  const [key, setKey] = useState(0);

  const fileName = filePath?.split("/").pop() || filePath?.split("\\").pop() || "diagram.drawio";

  const viewerUrl = `https://viewer.diagrams.net/?nav=1&highlight=0000ff&border=20&title=${encodeURIComponent(fileName)}&spin=1`;

  // Listen for the 'init' event from the iframe, and then send the raw XML
  useEffect(() => {
    const handleMessage = (e: MessageEvent) => {
      // Filter out messages that don't belong to our iframe
      if (!iframeRef.current || e.source !== iframeRef.current.contentWindow) return;
      
      try {
        const msg = typeof e.data === 'string' ? JSON.parse(e.data) : e.data;
        if (msg.event === 'init') {
          // Drawio iframe is ready to receive diagram data!
          iframeRef.current.contentWindow?.postMessage(
            JSON.stringify({
              action: 'load',
              xml: content
            }),
            '*'
          );
        }
      } catch (_err) {
        // Ignore unrelated postMessage parsing errors
      }
    };

    window.addEventListener('message', handleMessage);
    return () => window.removeEventListener('message', handleMessage);
  }, [content, key]);

  const refresh = useCallback(() => setKey(k => k + 1), []);

  return (
    <div className="html-preview">
      {/* ── Toolbar ──────────────────────────────────────────────── */}
      <div className="html-preview-toolbar">
        <div className="toolbar-group">
          {(["responsive", "desktop", "tablet", "mobile"] as const).map(d => (
            <button
              key={d}
              className={`device-btn${device === d ? " active" : ""}`}
              onClick={() => setDevice(d)}
              title={d.charAt(0).toUpperCase() + d.slice(1)}
            >
              {d === "responsive" ? <Maximize2 size={13} strokeWidth={1.5} /> : d === "desktop" ? <Monitor size={13} strokeWidth={1.5} /> : d === "tablet" ? <Tablet size={13} strokeWidth={1.5} /> : <Smartphone size={13} strokeWidth={1.5} />}
            </button>
          ))}
        </div>

        <div className="toolbar-separator" />

        <div className="toolbar-group">
          <button onClick={refresh} title="Refresh Preview" className="preview-action-btn">
            ↻
          </button>
        </div>

        <div className="file-info">
          <span className="info-badge">Draw.io</span>
          <span className="info-badge">{fileName}</span>
        </div>
      </div>

      {/* ── Preview area ─────────────────────────────────────────── */}
      <div className="html-preview-canvas">
        <div className={`html-preview-frame-wrapper device-${device}`}>
          {viewerUrl && (
            <iframe
              ref={iframeRef}
              key={key}
              src={viewerUrl}
              title={`Draw.io: ${fileName}`}
              className="html-preview-iframe"
              sandbox="allow-scripts allow-same-origin allow-popups"
            />
          )}
        </div>
      </div>
    </div>
  );
}
