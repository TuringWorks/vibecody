import { useState, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { GeneratedFileList, type GeneratedFile } from "./design/GeneratedFileList";

/** Extract a human-readable message from raw API error strings. */
function parseApiError(raw: string): string {
  // Strip leading exception wrapper (e.g. "Error: ...")
  const body = raw.replace(/^Error:\s*/i, "").trim();
  // Try to pull the "error" field from embedded JSON
  try {
    const m = body.match(/\{[\s\S]*\}/);
    if (m) {
      const obj = JSON.parse(m[0]);
      if (typeof obj.error === "string") return obj.error;
      if (typeof obj.message === "string") return obj.message;
    }
  } catch { /* not JSON, use as-is */ }
  return body;
}

/*
 * There is deliberately no client-side "is this model vision-capable?" list
 * here. There used to be — a keyword match on the provider's display name —
 * and it refused every vision model whose name did not contain "claude",
 * "gpt" or "gemini", local ones included. `generate_app_from_image` asks the
 * active provider itself (`supports_vision()`) and returns a real error, so
 * the answer comes from the provider rather than from a guess about its name.
 */

const FRAMEWORKS = [
  { value: "react", label: "React (TSX)" },
  { value: "vue", label: "Vue (SFC)" },
  { value: "svelte", label: "Svelte" },
  { value: "nextjs", label: "Next.js" },
  { value: "html", label: "HTML / CSS / JS" },
];

export function ScreenshotToApp({ workspacePath, provider: propProvider }: { workspacePath: string | null; provider?: string }) {
  const [framework, setFramework] = useState("react");
  // Provider follows the toolbar selection (no local override — CLAUDE.md rule:
  // panels must use the toolbar's selected provider, and never silently default
  // to Anthropic when the toolbar selection is empty).
  const selectedProvider = propProvider ?? "";
  const [imageBase64, setImageBase64] = useState<string | null>(null);
  const [imageMime, setImageMime] = useState<string>("image/png");
  const [imagePreview, setImagePreview] = useState<string | null>(null);
  const [generating, setGenerating] = useState(false);
  const [files, setFiles] = useState<GeneratedFile[]>([]);
  const [error, setError] = useState<string | null>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);
  const dropRef = useRef<HTMLDivElement>(null);

  const ACCEPTED = ["image/png", "image/jpeg", "image/jpg", "image/webp"];

  const loadImage = useCallback((file: File) => {
    if (!ACCEPTED.includes(file.type)) {
      setError("Unsupported format. Please use PNG, JPG, or WEBP.");
      return;
    }
    setError(null);
    setFiles([]);
    // Normalize jpg → jpeg; backend mime list mirrors the provider trait's accepted types.
    setImageMime(file.type === "image/jpg" ? "image/jpeg" : file.type);
    const reader = new FileReader();
    reader.onload = () => {
      const dataUrl = reader.result as string;
      setImagePreview(dataUrl);
      // Strip data URL prefix to get raw base64
      const base64 = dataUrl.replace(/^data:image\/\w+;base64,/, "");
      setImageBase64(base64);
    };
    reader.readAsDataURL(file);
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const handleDrop = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    const file = e.dataTransfer.files?.[0];
    if (file) loadImage(file);
  }, [loadImage]);

  const handleDragOver = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
  }, []);

  const handleFileInput = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (file) loadImage(file);
  }, [loadImage]);

  const handleGenerate = async () => {
    if (!imageBase64) return;
    if (!selectedProvider) {
      setError("No provider selected. Pick one in the toolbar dropdown.");
      return;
    }
    setGenerating(true);
    setError(null);
    setFiles([]);
    try {
      const result = await invoke<GeneratedFile[]>("generate_app_from_image", {
        imageBase64,
        mediaType: imageMime,
        framework,
        provider: selectedProvider,
      });
      setFiles(result);
    } catch (e: unknown) {
      setError(parseApiError(String(e)));
    } finally {
      setGenerating(false);
    }
  };

  const handleClear = () => {
    setImageBase64(null);
    setImagePreview(null);
    setFiles([]);
    setError(null);
    if (fileInputRef.current) fileInputRef.current.value = "";
  };

  return (
    <div className="panel-container">
      <div className="panel-header"><h3>Screenshot to App</h3></div>
      <div className="panel-body">

      {/* Upload area */}
      <div
        ref={dropRef}
        onDrop={handleDrop}
        onDragOver={handleDragOver}
        onClick={() => fileInputRef.current?.click()}
        style={{
          border: "2px dashed var(--border-color)",
          borderRadius: "var(--radius-sm-alt)",
          padding: imagePreview ? "8px" : "32px 16px",
          textAlign: "center",
          cursor: "pointer",
          marginBottom: "12px",
          background: "var(--bg-secondary)",
          color: "var(--text-secondary)",
          transition: "border-color 0.2s",
        }}
        onDragEnter={(e) => { e.currentTarget.style.borderColor = "var(--accent-color)"; }}
        onDragLeave={(e) => { e.currentTarget.style.borderColor = "var(--border-color)"; }}
      >
        {imagePreview ? (
          <div style={{ position: "relative" }}>
            <img
              src={imagePreview}
              alt="Uploaded screenshot"
              style={{ maxWidth: "100%", maxHeight: "200px", borderRadius: "var(--radius-xs-plus)" }}
            />
            <button
              onClick={(e) => { e.stopPropagation(); handleClear(); }}
              style={{
                position: "absolute", top: 4, right: 4,
                background: "rgba(0,0,0,0.6)", color: "var(--text-primary)",
                border: "none", borderRadius: "50%", width: 24, height: 24,
                cursor: "pointer", fontSize: "var(--font-size-lg)", lineHeight: "24px",
              }}
              title="Remove image"
            >
              x
            </button>
          </div>
        ) : (
          <>
            <div style={{ fontSize: "28px", marginBottom: "8px" }}>+</div>
            <div>Drag & drop an image here, or click to browse</div>
            <div style={{ fontSize: "var(--font-size-sm)", marginTop: "4px", color: "var(--text-secondary)" }}>
              PNG, JPG, WEBP
            </div>
          </>
        )}
        <input
          ref={fileInputRef}
          type="file"
          accept=".png,.jpg,.jpeg,.webp"
          onChange={handleFileInput}
          style={{ display: "none" }}
        />
      </div>

      {/* Provider row — follows the toolbar selection (read-only here) */}
      <div style={{ display: "flex", gap: "8px", marginBottom: "10px", alignItems: "center", flexWrap: "wrap" }}>
        <span style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>Provider:</span>
        <span style={{
          fontSize: "var(--font-size-sm)", color: "var(--text-primary)",
          padding: "2px 8px", borderRadius: "var(--radius-xs-plus)",
          background: "var(--bg-secondary)", border: "1px solid var(--border-color)",
        }}>
          {selectedProvider || "(none selected)"}
        </span>
        {!selectedProvider && (
          <span style={{ fontSize: "var(--font-size-xs)", color: "var(--warning-color)" }}>
            ⚠ Pick a provider in the toolbar dropdown.
          </span>
        )}
      </div>

      {/* Framework picker */}
      <div style={{ display: "flex", gap: "6px", marginBottom: "12px", flexWrap: "wrap" }}>
        {FRAMEWORKS.map(fw => (
          <label
            key={fw.value}
            style={{
              display: "flex", alignItems: "center", gap: "4px",
              padding: "4px 10px", borderRadius: "var(--radius-xs-plus)", cursor: "pointer",
              background: framework === fw.value ? "var(--accent-color)" : "var(--bg-secondary)",
              color: framework === fw.value ? "var(--text-primary)" : "var(--text-secondary)",
              border: `1px solid ${framework === fw.value ? "var(--accent-color)" : "var(--border-color)"}`,
              fontSize: "var(--font-size-base)",
              transition: "background 0.15s",
            }}
          >
            <input
              type="radio"
              name="framework"
              value={fw.value}
              checked={framework === fw.value}
              onChange={() => setFramework(fw.value)}
              style={{ display: "none" }}
            />
            {fw.label}
          </label>
        ))}
      </div>

      {/* Generate button */}
      <button
        onClick={handleGenerate}
        disabled={!imageBase64 || generating || !selectedProvider}
        style={{
          width: "100%", padding: "10px",
          background: !imageBase64 ? "var(--bg-secondary)" : generating ? "var(--bg-tertiary)" : "var(--accent-color)",
          color: "var(--text-primary)", border: "none", borderRadius: "var(--radius-sm)",
          cursor: !imageBase64 || generating ? "default" : "pointer",
          fontWeight: "bold", fontSize: "var(--font-size-md)",
          marginBottom: "12px",
          opacity: !imageBase64 ? 0.5 : 1,
        }}
      >
        {generating ? "Generating..." : "Generate App"}
      </button>

      {/* Progress indicator */}
      {generating && (
        <div style={{
          background: "var(--bg-secondary)", borderRadius: "var(--radius-xs-plus)", padding: "12px",
          marginBottom: "12px", color: "var(--accent-color)", textAlign: "center",
          fontSize: "var(--font-size-base)",
        }}>
          <div style={{ marginBottom: "8px" }}>Analyzing screenshot and generating code...</div>
          <div style={{
            width: "100%", height: "4px", background: "var(--bg-secondary)", borderRadius: "2px",
            overflow: "hidden",
          }}>
            <div style={{
              width: "60%", height: "100%", background: "var(--accent-color)",
              borderRadius: "2px",
              animation: "pulse 1.5s ease-in-out infinite",
            }} />
          </div>
          <style>{`@keyframes pulse { 0%,100% { opacity: 0.4; } 50% { opacity: 1; } }`}</style>
        </div>
      )}

      {/* Error display */}
      {error && (
        <div style={{
          background: "color-mix(in srgb, var(--accent-rose) 10%, transparent)", color: "var(--error-color)",
          padding: "8px", borderRadius: "var(--radius-xs-plus)", marginBottom: "12px",
          whiteSpace: "pre-wrap", fontSize: "var(--font-size-base)",
        }}>
          {error}
        </div>
      )}

      {/* Generated files — the same review-and-write list the Import tab and
          the Figma import use, so all three behave identically. */}
      <GeneratedFileList
        files={files}
        workspacePath={workspacePath}
        onError={setError}
      />


      {/* Info box when idle */}
      {!generating && files.length === 0 && !error && (
        <div style={{
          background: "var(--bg-secondary)", padding: "12px", borderRadius: "var(--radius-sm)",
          color: "var(--text-secondary)", fontSize: "var(--font-size-base)", lineHeight: "1.6",
        }}>
          <div style={{ marginBottom: "4px", fontWeight: "bold", color: "var(--text-secondary)" }}>
            How it works:
          </div>
          <ol style={{ margin: 0, paddingLeft: "18px" }}>
            <li>Upload a screenshot or design mockup</li>
            <li>Pick a target framework</li>
            <li>AI analyzes the layout, colors, and structure</li>
            <li>Complete app code is generated with components, styles, and routing</li>
            <li>Write files directly into your project</li>
          </ol>
        </div>
      )}
      </div>
    </div>
  );
}
