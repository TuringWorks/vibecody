/**
 * ImageViewer — pan & zoom image viewer for the editor area.
 *
 * Supports PNG, JPG/JPEG, TIFF, WebP (raster via base64 data-url),
 * SVG (rendered inline via dangerouslySetInnerHTML), and
 * DrawIO (.drawio / .dio — rendered as embedded SVG from the XML).
 *
 * Features:
 *   • Mouse-wheel zoom (centered on cursor)
 *   • Click + drag panning
 *   • Toolbar: zoom in/out, reset, fit-to-view, 1:1 actual-size
 *   • Displays image dimensions and file size
 */

import { useState, useRef, useCallback, useEffect } from "react";
import "./ImageViewer.css";

// ── Helpers ──────────────────────────────────────────────────────────

const IMAGE_EXTENSIONS = new Set([
  "png", "jpg", "jpeg", "gif", "bmp", "ico", "tiff", "tif", "webp",
]);

/** Check if a filename is a supported image file */
export function isImageFile(filename: string): boolean {
  const ext = filename.split(".").pop()?.toLowerCase() || "";
  return IMAGE_EXTENSIONS.has(ext);
}

/** Returns the MIME type for the given extension */
function mimeForExt(ext: string): string {
  switch (ext) {
    case "png": return "image/png";
    case "jpg": case "jpeg": return "image/jpeg";
    case "gif": return "image/gif";
    case "bmp": return "image/bmp";
    case "ico": return "image/x-icon";
    case "tiff": case "tif": return "image/tiff";
    case "webp": return "image/webp";
    case "svg": return "image/svg+xml";
    default: return "application/octet-stream";
  }
}

// ── Props ────────────────────────────────────────────────────────────

interface ImageViewerProps {
  /** Absolute file path */
  filePath: string;
  /** Base64-encoded file content (for raster + SVG) */
  base64Data: string;
  /** Raw file content string (for SVG / DrawIO XML) */
  rawContent?: string;
}

// ── Component ────────────────────────────────────────────────────────

export function ImageViewer({ filePath, base64Data, rawContent }: ImageViewerProps) {
  const ext = filePath.split(".").pop()?.toLowerCase() || "";

  // ── zoom / pan state ───────────────────────────────────────────────
  const [scale, setScale] = useState(1);
  const [offset, setOffset] = useState({ x: 0, y: 0 });
  const [smooth, setSmooth] = useState(false);
  const [isDragging, setIsDragging] = useState(false);
  const [naturalSize, setNaturalSize] = useState<{ w: number; h: number } | null>(null);
  const [error, setError] = useState<string | null>(null);

  const canvasRef = useRef<HTMLDivElement>(null);
  const dragStart = useRef({ x: 0, y: 0, ox: 0, oy: 0 });

  // ── Data URL for raster images ─────────────────────────────────────
  const dataUrl = base64Data
    ? `data:${mimeForExt(ext)};base64,${base64Data}`
    : null;

  // ── File size from base64 ──────────────────────────────────────────
  const fileSizeBytes = base64Data ? Math.floor((base64Data.length * 3) / 4) : 0;
  const fileSizeLabel = fileSizeBytes < 1024
    ? `${fileSizeBytes} B`
    : fileSizeBytes < 1024 * 1024
      ? `${(fileSizeBytes / 1024).toFixed(1)} KB`
      : `${(fileSizeBytes / (1024 * 1024)).toFixed(1)} MB`;

  // ── Fit to view ────────────────────────────────────────────────────
  const fitToView = useCallback(() => {
    if (!canvasRef.current || !naturalSize) return;
    const rect = canvasRef.current.getBoundingClientRect();
    const scaleX = (rect.width - 40) / naturalSize.w;
    const scaleY = (rect.height - 40) / naturalSize.h;
    const newScale = Math.min(scaleX, scaleY, 1); // don't upscale beyond 1:1
    const x = (rect.width - naturalSize.w * newScale) / 2;
    const y = (rect.height - naturalSize.h * newScale) / 2;
    setSmooth(true);
    setScale(newScale);
    setOffset({ x, y });
    setTimeout(() => setSmooth(false), 250);
  }, [naturalSize]);

  // ── Auto fit on first load ─────────────────────────────────────────
  useEffect(() => {
    if (naturalSize) fitToView();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [naturalSize]);

  // ── Zoom helpers ───────────────────────────────────────────────────
  const zoomTo = useCallback((newScale: number, centerX?: number, centerY?: number) => {
    setScale((prevScale) => {
      const clampedScale = Math.max(0.05, Math.min(newScale, 32));
      if (!canvasRef.current) return clampedScale;

      const rect = canvasRef.current.getBoundingClientRect();
      const cx = centerX ?? rect.width / 2;
      const cy = centerY ?? rect.height / 2;

      setOffset((prev) => ({
        x: cx - ((cx - prev.x) / prevScale) * clampedScale,
        y: cy - ((cy - prev.y) / prevScale) * clampedScale,
      }));

      return clampedScale;
    });
  }, []);

  const zoomIn = useCallback(() => { setSmooth(true); zoomTo(scale * 1.25); setTimeout(() => setSmooth(false), 250); }, [scale, zoomTo]);
  const zoomOut = useCallback(() => { setSmooth(true); zoomTo(scale / 1.25); setTimeout(() => setSmooth(false), 250); }, [scale, zoomTo]);
  const zoomActual = useCallback(() => {
    if (!canvasRef.current || !naturalSize) return;
    const rect = canvasRef.current.getBoundingClientRect();
    setSmooth(true);
    setScale(1);
    setOffset({
      x: (rect.width - naturalSize.w) / 2,
      y: (rect.height - naturalSize.h) / 2,
    });
    setTimeout(() => setSmooth(false), 250);
  }, [naturalSize]);

  // ── Mouse wheel zoom ────────────────────────────────────────────────
  const handleWheel = useCallback((e: React.WheelEvent) => {
    e.preventDefault();
    const rect = canvasRef.current?.getBoundingClientRect();
    if (!rect) return;
    const factor = e.deltaY < 0 ? 1.1 : 1 / 1.1;
    zoomTo(scale * factor, e.clientX - rect.left, e.clientY - rect.top);
  }, [scale, zoomTo]);

  // ── Drag to pan ─────────────────────────────────────────────────────
  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    if (e.button !== 0) return;
    setIsDragging(true);
    dragStart.current = { x: e.clientX, y: e.clientY, ox: offset.x, oy: offset.y };
  }, [offset]);

  const handleMouseMove = useCallback((e: React.MouseEvent) => {
    if (!isDragging) return;
    setOffset({
      x: dragStart.current.ox + (e.clientX - dragStart.current.x),
      y: dragStart.current.oy + (e.clientY - dragStart.current.y),
    });
  }, [isDragging]);

  const handleMouseUp = useCallback(() => setIsDragging(false), []);

  // ── Image load handler ──────────────────────────────────────────────
  const handleImageLoad = useCallback((e: React.SyntheticEvent<HTMLImageElement>) => {
    const img = e.currentTarget;
    setNaturalSize({ w: img.naturalWidth, h: img.naturalHeight });
  }, []);

  // ── Handle natural size ──────────────────────────────────
  // ── Error guard ─────────────────────────────────────────────────────
  if (error) {
    return (
      <div className="image-viewer">
        <div className="image-viewer-error">
          <span className="error-icon">⚠</span>
          <span className="error-message">{error}</span>
        </div>
      </div>
    );
  }

  if (!base64Data && !rawContent) {
    return (
      <div className="image-viewer">
        <div className="image-viewer-loading">
          <div className="spinner" />
          <span>Loading image…</span>
        </div>
      </div>
    );
  }

  const zoomPercent = `${Math.round(scale * 100)}%`;
  const fileName = filePath.split("/").pop() || filePath.split("\\").pop() || filePath;

  return (
    <div className="image-viewer">
      {/* ── Toolbar ──────────────────────────────────────────────── */}
      <div className="image-viewer-toolbar">
        <div className="toolbar-group">
          <button onClick={zoomOut} title="Zoom Out (−)">−</button>
          <span className="zoom-label">{zoomPercent}</span>
          <button onClick={zoomIn} title="Zoom In (+)">+</button>
        </div>
        <div className="toolbar-separator" />
        <div className="toolbar-group">
          <button onClick={fitToView} title="Fit to View" style={{ width: "auto", padding: "0 8px", fontSize: "var(--font-size-sm)" }}>
            Fit
          </button>
          <button onClick={zoomActual} title="Actual Size (1:1)" style={{ width: "auto", padding: "0 8px", fontSize: "var(--font-size-sm)" }}>
            1:1
          </button>
        </div>

        <div className="file-info">
          {naturalSize && (
            <span className="info-badge">{naturalSize.w} × {naturalSize.h}</span>
          )}
          <span className="info-badge">{ext.toUpperCase()}</span>
          <span className="info-badge">{fileSizeLabel}</span>
        </div>
      </div>

      {/* ── Canvas ───────────────────────────────────────────────── */}
      <div
        ref={canvasRef}
        className={`image-viewer-canvas${isDragging ? " dragging" : ""}`}
        onWheel={handleWheel}
        onMouseDown={handleMouseDown}
        onMouseMove={handleMouseMove}
        onMouseUp={handleMouseUp}
        onMouseLeave={handleMouseUp}
      >
        {/* Raster image */}
        {dataUrl && (
          <div
            className={`image-wrapper${smooth ? " smooth" : ""}`}
            style={{ transform: `translate(${offset.x}px, ${offset.y}px) scale(${scale})` }}
          >
            <img
              src={dataUrl}
              alt={fileName}
              draggable={false}
              className={scale > 4 ? "pixelated" : ""}
              onLoad={handleImageLoad}
              onError={() => setError(`Failed to decode image: ${fileName}`)}
            />
          </div>
        )}
      </div>
    </div>
  );
}
