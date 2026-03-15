import { useState, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

interface GeneratedFile {
  path: string;
  content: string;
  language: string;
}

const FRAMEWORKS = [
  { value: "react", label: "React (TSX)" },
  { value: "vue", label: "Vue (SFC)" },
  { value: "svelte", label: "Svelte" },
  { value: "nextjs", label: "Next.js" },
  { value: "html", label: "HTML / CSS / JS" },
];

export function ScreenshotToApp({ workspacePath }: { workspacePath: string | null }) {
  const [framework, setFramework] = useState("react");
  const [imageBase64, setImageBase64] = useState<string | null>(null);
  const [imagePreview, setImagePreview] = useState<string | null>(null);
  const [generating, setGenerating] = useState(false);
  const [files, setFiles] = useState<GeneratedFile[]>([]);
  const [expandedIdx, setExpandedIdx] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [writeStatus, setWriteStatus] = useState<Record<number, string>>({});
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
    setWriteStatus({});
    const reader = new FileReader();
    reader.onload = () => {
      const dataUrl = reader.result as string;
      setImagePreview(dataUrl);
      // Strip data URL prefix to get raw base64
      const base64 = dataUrl.replace(/^data:image\/\w+;base64,/, "");
      setImageBase64(base64);
    };
    reader.readAsDataURL(file);
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
    setGenerating(true);
    setError(null);
    setFiles([]);
    setWriteStatus({});
    setExpandedIdx(null);
    try {
      const result = await invoke<GeneratedFile[]>("generate_app_from_image", {
        imageBase64,
        framework,
      });
      setFiles(result);
      if (result.length > 0) setExpandedIdx(0);
    } catch (e: unknown) {
      setError(String(e));
    } finally {
      setGenerating(false);
    }
  };

  const handleWriteFile = async (idx: number) => {
    if (!workspacePath) {
      setError("No workspace folder open.");
      return;
    }
    const file = files[idx];
    const fullPath = workspacePath.endsWith("/")
      ? workspacePath + file.path
      : workspacePath + "/" + file.path;
    try {
      setWriteStatus(prev => ({ ...prev, [idx]: "writing" }));
      await invoke("write_file", { path: fullPath, content: file.content });
      setWriteStatus(prev => ({ ...prev, [idx]: "done" }));
    } catch (e: unknown) {
      setWriteStatus(prev => ({ ...prev, [idx]: "error" }));
      setError(`Failed to write ${file.path}: ${String(e)}`);
    }
  };

  const handleClear = () => {
    setImageBase64(null);
    setImagePreview(null);
    setFiles([]);
    setError(null);
    setWriteStatus({});
    setExpandedIdx(null);
    if (fileInputRef.current) fileInputRef.current.value = "";
  };

  const langColor = (lang: string) => {
    switch (lang) {
      case "tsx": case "jsx": return "var(--accent-color)";
      case "typescript": case "ts": return "var(--accent-color)";
      case "javascript": case "js": return "var(--warning-color)";
      case "vue": return "var(--success-color)";
      case "svelte": return "var(--error-color)";
      case "html": return "var(--error-color)";
      case "css": return "var(--accent-color)";
      default: return "var(--text-secondary)";
    }
  };

  return (
    <div style={{ padding: "12px", fontFamily: "monospace", fontSize: "13px", height: "100%", overflowY: "auto", background: "var(--bg-tertiary)" }}>
      <div style={{ fontWeight: "bold", marginBottom: "12px", color: "var(--text-primary)" }}>
        Screenshot to App
      </div>

      {/* Upload area */}
      <div
        ref={dropRef}
        onDrop={handleDrop}
        onDragOver={handleDragOver}
        onClick={() => fileInputRef.current?.click()}
        style={{
          border: "2px dashed var(--border-color)",
          borderRadius: "8px",
          padding: imagePreview ? "8px" : "32px 16px",
          textAlign: "center",
          cursor: "pointer",
          marginBottom: "12px",
          background: "var(--bg-secondary)",
          color: "var(--text-muted)",
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
              style={{ maxWidth: "100%", maxHeight: "200px", borderRadius: "4px" }}
            />
            <button
              onClick={(e) => { e.stopPropagation(); handleClear(); }}
              style={{
                position: "absolute", top: 4, right: 4,
                background: "rgba(0,0,0,0.6)", color: "var(--text-primary)",
                border: "none", borderRadius: "50%", width: 24, height: 24,
                cursor: "pointer", fontSize: "14px", lineHeight: "24px",
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
            <div style={{ fontSize: "11px", marginTop: "4px", color: "var(--text-muted)" }}>
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

      {/* Framework picker */}
      <div style={{ display: "flex", gap: "6px", marginBottom: "12px", flexWrap: "wrap" }}>
        {FRAMEWORKS.map(fw => (
          <label
            key={fw.value}
            style={{
              display: "flex", alignItems: "center", gap: "4px",
              padding: "4px 10px", borderRadius: "4px", cursor: "pointer",
              background: framework === fw.value ? "var(--accent-color)" : "var(--bg-secondary)",
              color: framework === fw.value ? "var(--text-primary)" : "var(--text-secondary)",
              border: `1px solid ${framework === fw.value ? "var(--accent-color)" : "var(--border-color)"}`,
              fontSize: "12px",
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
        disabled={!imageBase64 || generating}
        style={{
          width: "100%", padding: "10px",
          background: !imageBase64 ? "var(--bg-secondary)" : generating ? "var(--bg-tertiary)" : "var(--accent-color)",
          color: "var(--text-primary)", border: "none", borderRadius: "6px",
          cursor: !imageBase64 || generating ? "default" : "pointer",
          fontWeight: "bold", fontSize: "13px",
          marginBottom: "12px",
          opacity: !imageBase64 ? 0.5 : 1,
        }}
      >
        {generating ? "Generating..." : "Generate App"}
      </button>

      {/* Progress indicator */}
      {generating && (
        <div style={{
          background: "var(--bg-secondary)", borderRadius: "4px", padding: "12px",
          marginBottom: "12px", color: "var(--accent-color)", textAlign: "center",
          fontSize: "12px",
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
          background: "rgba(244,67,54,0.1)", color: "var(--error-color)",
          padding: "8px", borderRadius: "4px", marginBottom: "12px",
          whiteSpace: "pre-wrap", fontSize: "12px",
        }}>
          {error}
        </div>
      )}

      {/* Generated files list */}
      {files.length > 0 && (
        <div>
          <div style={{
            display: "flex", alignItems: "center", justifyContent: "space-between",
            marginBottom: "8px",
          }}>
            <span style={{ color: "var(--success-color)", fontWeight: "bold", fontSize: "12px" }}>
              {files.length} file{files.length !== 1 ? "s" : ""} generated
            </span>
            <button
              onClick={() => {
                files.forEach((_, idx) => { if (writeStatus[idx] !== "done") handleWriteFile(idx); });
              }}
              disabled={!workspacePath}
              style={{
                background: "var(--success-color)", color: "var(--text-primary)", border: "none",
                borderRadius: "4px", padding: "4px 12px", cursor: "pointer",
                fontSize: "11px", opacity: workspacePath ? 1 : 0.5,
              }}
            >
              Write All to Project
            </button>
          </div>

          {files.map((file, idx) => (
            <div
              key={idx}
              style={{
                border: "1px solid var(--border-color)", borderRadius: "6px",
                marginBottom: "6px", overflow: "hidden",
              }}
            >
              {/* File header */}
              <div
                onClick={() => setExpandedIdx(expandedIdx === idx ? null : idx)}
                style={{
                  display: "flex", alignItems: "center", gap: "8px",
                  padding: "8px 10px", cursor: "pointer",
                  background: "var(--bg-secondary)",
                }}
              >
                <span style={{
                  fontSize: "10px", transform: expandedIdx === idx ? "rotate(90deg)" : "rotate(0deg)",
                  transition: "transform 0.15s", color: "var(--text-muted)",
                }}>
                  &#9654;
                </span>
                <span style={{
                  background: langColor(file.language), color: "var(--bg-primary)",
                  padding: "1px 6px", borderRadius: "3px", fontSize: "10px",
                  fontWeight: "bold",
                }}>
                  {file.language.toUpperCase()}
                </span>
                <span style={{ color: "var(--text-primary)", fontSize: "12px", flex: 1 }}>
                  {file.path}
                </span>
                <button
                  onClick={(e) => { e.stopPropagation(); handleWriteFile(idx); }}
                  disabled={!workspacePath || writeStatus[idx] === "writing"}
                  style={{
                    background: writeStatus[idx] === "done" ? "var(--success-color)"
                      : writeStatus[idx] === "error" ? "var(--error-color)"
                      : "var(--accent-color)",
                    color: "var(--text-primary)", border: "none", borderRadius: "3px",
                    padding: "2px 8px", cursor: "pointer", fontSize: "11px",
                    opacity: workspacePath ? 1 : 0.5,
                  }}
                >
                  {writeStatus[idx] === "writing" ? "..."
                    : writeStatus[idx] === "done" ? "Written"
                    : writeStatus[idx] === "error" ? "Retry"
                    : "Write"}
                </button>
              </div>

              {/* Code preview */}
              {expandedIdx === idx && (
                <pre style={{
                  margin: 0, padding: "10px", background: "var(--bg-primary)",
                  overflow: "auto", maxHeight: "300px",
                  fontSize: "11px", lineHeight: "1.5",
                  color: "var(--text-primary)", whiteSpace: "pre",
                }}>
                  {file.content}
                </pre>
              )}
            </div>
          ))}
        </div>
      )}

      {/* Info box when idle */}
      {!generating && files.length === 0 && !error && (
        <div style={{
          background: "var(--bg-secondary)", padding: "12px", borderRadius: "6px",
          color: "var(--text-muted)", fontSize: "12px", lineHeight: "1.6",
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
  );
}
