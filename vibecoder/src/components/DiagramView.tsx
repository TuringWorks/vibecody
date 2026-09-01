/**
 * DiagramView — one Mermaid or PlantUML diagram, drawn.
 *
 * Used in two places, which is why it is its own component: fenced blocks in a
 * markdown preview, and a whole file in the diagram preview.
 *
 * What it will not do is fail quietly. A diagram that does not render shows the
 * renderer's own message *and the source that produced it*, because the source
 * is the thing you have to change and it is usually four lines long. A blank
 * space where a picture should be is the one outcome worth ruling out.
 */
import { useEffect, useRef, useState } from "react";
import { AlertTriangle } from "lucide-react";

import { renderDiagram, DIAGRAM_LABELS, type DiagramKind } from "../lib/diagrams";
import "./DiagramView.css";

interface DiagramViewProps {
  kind: DiagramKind;
  source: string;
  /** Extra classes for the frame — the file preview scales it, markdown does not. */
  className?: string;
}

type Drawing =
  | { status: "drawing" }
  | { status: "drawn"; svg: string }
  | { status: "failed"; message: string };

/**
 * How long a keystroke waits before the diagram is redrawn.
 *
 * The preview is fed by the editor, so every character would otherwise be a
 * render — and a PlantUML render is a JVM starting up.
 */
const REDRAW_DELAY_MS = 300;

export function DiagramView({ kind, source, className }: DiagramViewProps) {
  const [drawing, setDrawing] = useState<Drawing>({ status: "drawing" });
  // Keeps the last good picture on screen while the next one is drawn, so
  // editing a diagram does not flash empty between keystrokes.
  const lastDrawn = useRef<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    if (!source.trim()) {
      setDrawing({ status: "failed", message: "This diagram is empty." });
      return;
    }
    setDrawing({ status: "drawing" });

    const timer = setTimeout(() => {
      renderDiagram(kind, source)
        .then((svg) => {
          if (cancelled) return;
          lastDrawn.current = svg;
          setDrawing({ status: "drawn", svg });
        })
        .catch((error) => {
          if (cancelled) return;
          setDrawing({ status: "failed", message: messageFor(error) });
        });
    }, REDRAW_DELAY_MS);

    return () => {
      cancelled = true;
      clearTimeout(timer);
    };
  }, [kind, source]);

  if (drawing.status === "failed") {
    return (
      <div className={`diagram-view diagram-failed${className ? ` ${className}` : ""}`}>
        <div className="diagram-error">
          <AlertTriangle size={14} className="error-icon" />
          <div>
            <div className="diagram-error-message">{drawing.message}</div>
            <div className="diagram-error-kind">{DIAGRAM_LABELS[kind]}</div>
          </div>
        </div>
        {/* The source is what has to change, so it stays on screen. */}
        <pre className="diagram-source">{source}</pre>
      </div>
    );
  }

  if (drawing.status === "drawing") {
    // A redraw keeps the previous picture; a first draw has nothing to show.
    return lastDrawn.current ? (
      <div
        className={`diagram-view diagram-stale${className ? ` ${className}` : ""}`}
        dangerouslySetInnerHTML={{ __html: lastDrawn.current }}
      />
    ) : (
      <div className={`diagram-view diagram-drawing${className ? ` ${className}` : ""}`}>
        <div className="doc-spinner" />
        <span>Drawing {DIAGRAM_LABELS[kind]} diagram…</span>
      </div>
    );
  }

  return (
    <div
      className={`diagram-view${className ? ` ${className}` : ""}`}
      /* Sanitised in `renderDiagram`: SVG is a document format with scripts in
         it, and the source came from a file. */
      dangerouslySetInnerHTML={{ __html: drawing.svg }}
    />
  );
}

/** Whatever the renderer threw, as something worth reading. */
function messageFor(error: unknown): string {
  if (typeof error === "string") return error;
  if (error instanceof Error) return error.message;
  return String(error);
}
