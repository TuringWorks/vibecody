/**
 * DiagramPreview — a whole file that is one diagram.
 *
 * `.mmd` and `.puml` files are text, so they open in the editor like any other
 * file; this is the other half of that toggle, the same way `.md` has a preview
 * and `.html` has one. The source stays editable and the picture follows what
 * is in the buffer, not what is on disk — the point of a preview is to see the
 * change you are making.
 */
import { useEffect, useState } from "react";
import { Info } from "lucide-react";

import { DiagramView } from "./DiagramView";
import {
  DIAGRAM_LABELS,
  diagramKindForFile,
  plantUmlRenderer,
  type DiagramKind,
} from "../lib/diagrams";
import "./HtmlPreview.css"; // Same toolbar shell as the other previews.
import "./DiagramView.css";

interface DiagramPreviewProps {
  /** The buffer, which is what is drawn — not the file on disk. */
  content: string;
  filePath?: string;
  /** For a file whose extension does not say, or in tests. */
  kind?: DiagramKind;
}

export function DiagramPreview({ content, filePath, kind }: DiagramPreviewProps) {
  const resolved: DiagramKind = kind ?? diagramKindForFile(filePath ?? "") ?? "mermaid";
  const [scale, setScale] = useState(1);
  const fileName = filePath?.split(/[/\\]/).pop() ?? `diagram.${resolved}`;

  return (
    <div className="html-preview diagram-preview">
      <div className="html-preview-toolbar">
        <div className="toolbar-group">
          <button onClick={() => setScale((s) => Math.max(s / 1.25, 0.25))} title="Zoom out">
            −
          </button>
          <span className="zoom-label">{Math.round(scale * 100)}%</span>
          <button onClick={() => setScale((s) => Math.min(s * 1.25, 4))} title="Zoom in">
            +
          </button>
          <button onClick={() => setScale(1)} title="Reset zoom" className="toolbar-btn-wide">
            Reset
          </button>
        </div>
        <div className="file-info">
          <span className="info-badge">{DIAGRAM_LABELS[resolved]}</span>
          <span className="info-badge">{fileName}</span>
        </div>
      </div>

      {resolved === "plantuml" && <PlantUmlSource />}

      <div className="diagram-preview-canvas">
        <DiagramView kind={resolved} source={content} className="diagram-scaled" />
      </div>
      {/* Zoom is applied here rather than inside DiagramView, which is also used
          inline in markdown where there is nothing to zoom. */}
      <style>{`.diagram-preview .diagram-scaled { transform: scale(${scale}); }`}</style>
    </div>
  );
}

/**
 * Which PlantUML is drawing, named.
 *
 * PlantUML is the one renderer that is not in the app, so "where did this
 * picture come from" is a fair question — and when the answer is "nowhere",
 * the diagram's own error says how to fix it.
 */
function PlantUmlSource() {
  const [renderer, setRenderer] = useState<string | null | undefined>(undefined);

  useEffect(() => {
    let cancelled = false;
    plantUmlRenderer()
      .then((found) => {
        if (!cancelled) setRenderer(found);
      })
      .catch(() => {
        if (!cancelled) setRenderer(null);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  if (!renderer) return null;
  return (
    <div className="diagram-renderer-note">
      <Info size={12} /> drawn locally by {renderer}
    </div>
  );
}
