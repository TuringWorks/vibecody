import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

interface GeneratedImage {
  id: string;
  prompt: string;
  model: string;
  width: number;
  height: number;
  style: string;
  status: string;
  created_at: string;
  file_path: string;
  cost: number;
}

interface ImageGenStats {
  total_images: number;
  total_cost: number;
  models_used: string[];
  styles_used: string[];
  completed: number;
  pending: number;
  failed: number;
}

interface ImageProvider {
  model: string;
  description: string;
  available: boolean;
  key_field: string;
}

const STYLES = ["Photorealistic", "Digital Art", "Watercolor", "Oil Painting", "Pixel Art",
  "Anime", "Minimalist", "3D Render", "Sketch", "Comic Book"];

// ── Shared styles (consistent with other panels) ──────────────────────────────

const card: React.CSSProperties = {
  padding: 10, marginBottom: 8, borderRadius: "var(--radius-sm)",
  background: "var(--bg-secondary)", border: "1px solid var(--border-color)",
};
const label: React.CSSProperties = { display: "block", marginBottom: 4, fontWeight: 600, fontSize: 11, color: "var(--text-secondary)" };
const input: React.CSSProperties = {
  width: "100%", boxSizing: "border-box", padding: "7px 10px", fontSize: 13,
  background: "var(--bg-tertiary)", border: "1px solid var(--border-color)",
  color: "var(--text-primary)", borderRadius: "var(--radius-sm)",
};
const btnPrimary: React.CSSProperties = {
  padding: "8px 16px", cursor: "pointer", border: "none", borderRadius: "var(--radius-sm)",
  background: "var(--accent-color)", color: "#fff", fontSize: 13, fontWeight: 600,
};
const badge = (bg: string): React.CSSProperties => ({
  display: "inline-block", padding: "2px 8px", borderRadius: 10,
  fontSize: 10, fontWeight: 600, background: bg, color: "#fff",
});

// ── Component ─────────────────────────────────────────────────────────────────

const ImageGenPanel: React.FC = () => {
  const [tab, setTab] = useState<"generate" | "gallery" | "stats">("generate");
  const [prompt, setPrompt] = useState("");
  const [style, setStyle] = useState("Photorealistic");
  const [width, setWidth] = useState(1024);
  const [height, setHeight] = useState(1024);
  const [sizePreset, setSizePreset] = useState("1024x1024");
  const [model, setModel] = useState("DALL-E 3");
  const [providers, setProviders] = useState<ImageProvider[]>([]);
  const [gallery, setGallery] = useState<GeneratedImage[]>([]);
  const [imageDataCache, setImageDataCache] = useState<Record<string, string>>({});
  const [stats, setStats] = useState<ImageGenStats | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const loadGallery = useCallback(async () => {
    try {
      const images = await invoke<GeneratedImage[]>("list_generated_images");
      setGallery(images);
      // Load image data for completed images that aren't cached yet
      for (const img of images) {
        if (img.status === "completed" && img.file_path && !img.file_path.startsWith("placeholder://")) {
          if (!imageDataCache[img.id]) {
            invoke<string>("get_generated_image_data", { id: img.id })
              .then(dataUrl => setImageDataCache(prev => ({ ...prev, [img.id]: dataUrl })))
              .catch(() => {});
          }
        }
      }
    } catch (e) { setError(String(e)); }
  }, [imageDataCache]);

  const loadStats = useCallback(async () => {
    try {
      const s = await invoke<ImageGenStats>("get_image_gen_stats");
      setStats(s);
    } catch (e) { setError(String(e)); }
  }, []);

  const loadProviders = useCallback(async () => {
    try {
      const p = await invoke<ImageProvider[]>("get_available_image_providers");
      setProviders(p);
      // Auto-select first available provider if current isn't available
      const available = p.filter(x => x.available);
      if (available.length > 0 && !available.some(x => x.model === model)) {
        setModel(available[0].model);
      }
    } catch { /* ignore */ }
  }, [model]);

  useEffect(() => {
    loadProviders(); loadGallery(); loadStats();
    // Refresh providers when API keys change
    const handler = () => loadProviders();
    window.addEventListener("vibeui:providers-updated", handler);
    return () => window.removeEventListener("vibeui:providers-updated", handler);
  }, []);

  const costEstimate = model === "DALL-E 3" || model === "GPT Image"
    ? ((width === 1024 && height === 1024) ? 0.04 : 0.08)
    : model === "OpenRouter Image" ? 0.03 : 0.04;

  const handleGenerate = async () => {
    if (!prompt.trim()) return;
    setLoading(true);
    setError(null);
    try {
      const img = await invoke<GeneratedImage>("generate_image", { prompt, model, style, width, height });
      setPrompt("");
      // Immediately load the image data for the new image
      if (img.status === "completed" && img.file_path) {
        invoke<string>("get_generated_image_data", { id: img.id })
          .then(dataUrl => setImageDataCache(prev => ({ ...prev, [img.id]: dataUrl })))
          .catch(() => {});
      }
      await loadGallery();
      await loadStats();
      setTab("gallery");
    } catch (e) { setError(String(e)); }
    finally { setLoading(false); }
  };

  const handleDelete = async (id: string) => {
    try {
      await invoke("delete_generated_image", { id });
      setImageDataCache(prev => { const next = { ...prev }; delete next[id]; return next; });
      await loadGallery();
      await loadStats();
    } catch (e) { setError(String(e)); }
  };

  const formatTime = (epochStr: string) => {
    const epoch = parseInt(epochStr, 10);
    if (isNaN(epoch)) return epochStr;
    const diff = Math.floor(Date.now() / 1000) - epoch;
    if (diff < 60) return "just now";
    if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
    if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
    return `${Math.floor(diff / 86400)}d ago`;
  };

  const statusBg = (s: string) =>
    s === "completed" ? "var(--success-color)" : s === "pending" ? "var(--info-color)" : "var(--error-color)";

  // ── Tab buttons ─────────────────────────────────────────────────────────────

  const tabBtn = (id: string, lbl: string) => (
    <button key={id} onClick={() => setTab(id as typeof tab)} style={{
      padding: "5px 14px", fontSize: 12, fontWeight: tab === id ? 600 : 400, cursor: "pointer",
      background: tab === id ? "var(--accent-color)" : "transparent",
      color: tab === id ? "#fff" : "var(--text-secondary)",
      border: "1px solid " + (tab === id ? "var(--accent-color)" : "var(--border-color)"),
      borderRadius: "var(--radius-sm)",
    }}>{lbl}</button>
  );

  // ── Generate tab ────────────────────────────────────────────────────────────

  const renderGenerate = () => (
    <div>
      {error && <div style={{ ...card, borderColor: "var(--error-color)", color: "var(--error-color)", marginBottom: 12 }}>{error}</div>}

      <div style={{ marginBottom: 12 }}>
        <label style={label}>Prompt</label>
        <textarea style={{ ...input, minHeight: 80, resize: "vertical", fontFamily: "inherit" }}
          value={prompt} onChange={e => setPrompt(e.target.value)}
          placeholder="Describe the image you want to generate..." />
      </div>

      <div style={{ marginBottom: 12 }}>
        <label style={label}>Model</label>
        {providers.length === 0 ? (
          <div style={{ ...card, color: "var(--text-secondary)", fontSize: 12 }}>Loading providers...</div>
        ) : (
          <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
            {providers.map(p => (
              <button key={p.model} disabled={!p.available} title={p.available ? p.description : `${p.description} — API key not configured`}
                onClick={() => p.available && setModel(p.model)}
                style={{
                  padding: "5px 12px", fontSize: 11, borderRadius: "var(--radius-sm)", cursor: p.available ? "pointer" : "not-allowed",
                  background: model === p.model ? "var(--accent-color)" : p.available ? "var(--bg-tertiary)" : "var(--bg-secondary)",
                  color: model === p.model ? "#fff" : p.available ? "var(--text-primary)" : "var(--text-secondary)",
                  border: `1px solid ${model === p.model ? "var(--accent-color)" : "var(--border-color)"}`,
                  opacity: p.available ? 1 : 0.5,
                }}>
                {p.model}{!p.available && " 🔒"}
              </button>
            ))}
          </div>
        )}
      </div>

      <div style={{ display: "flex", gap: 10, marginBottom: 12 }}>
        <div style={{ flex: 1 }}>
          <label style={label}>Style</label>
          <select style={input} value={style} onChange={e => setStyle(e.target.value)}>
            {STYLES.map(s => <option key={s} value={s}>{s}</option>)}
          </select>
        </div>
        <div style={{ flex: 1 }}>
          <label style={label}>Size</label>
          <select style={input} value={sizePreset} onChange={e => {
            const v = e.target.value;
            setSizePreset(v);
            if (v !== "custom") {
              const [w, h] = v.split("x").map(Number);
              setWidth(w); setHeight(h);
            }
          }}>
            <option value="1024x1024">1024 x 1024 — Square</option>
            <option value="1792x1024">1792 x 1024 — Landscape</option>
            <option value="1024x1792">1024 x 1792 — Portrait</option>
            <option value="512x512">512 x 512 — Small Square</option>
            <option value="1280x720">1280 x 720 — HD 16:9</option>
            <option value="720x1280">720 x 1280 — HD 9:16</option>
            <option value="1920x1080">1920 x 1080 — Full HD</option>
            <option value="1080x1920">1080 x 1920 — Full HD Portrait</option>
            <option value="2048x2048">2048 x 2048 — Large Square</option>
            <option value="custom">Custom...</option>
          </select>
        </div>
      </div>
      {sizePreset === "custom" && (
        <div style={{ display: "flex", gap: 10, marginBottom: 12 }}>
          <div style={{ flex: 1 }}>
            <label style={label}>Width (px)</label>
            <input style={{ ...input, fontFamily: "var(--font-mono, monospace)" }} type="number" min={256} max={4096} step={64}
              value={width} onChange={e => setWidth(Math.max(256, Math.min(4096, Number(e.target.value))))} />
          </div>
          <div style={{ flex: 1 }}>
            <label style={label}>Height (px)</label>
            <input style={{ ...input, fontFamily: "var(--font-mono, monospace)" }} type="number" min={256} max={4096} step={64}
              value={height} onChange={e => setHeight(Math.max(256, Math.min(4096, Number(e.target.value))))} />
          </div>
        </div>
      )}
      {sizePreset !== "custom" ? null : (
        <div style={{ fontSize: 10, color: "var(--text-secondary)", marginBottom: 10 }}>
          DALL-E 3 maps to nearest supported size: 1024x1024, 1792x1024, or 1024x1792
        </div>
      )}

      <div style={{ ...card, display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 12 }}>
        <span style={{ fontSize: 11, color: "var(--text-secondary)" }}>Estimated cost ({model})</span>
        <span style={{ fontSize: 13, fontWeight: 700, fontFamily: "var(--font-mono, monospace)" }}>${costEstimate.toFixed(2)}</span>
      </div>

      <button style={{ ...btnPrimary, width: "100%", opacity: loading ? 0.6 : 1 }}
        onClick={handleGenerate} disabled={loading}>
        {loading ? "Generating..." : "Generate Image"}
      </button>
    </div>
  );

  // ── Gallery tab ─────────────────────────────────────────────────────────────

  const renderGallery = () => (
    <div>
      <div style={{ fontSize: 11, color: "var(--text-secondary)", marginBottom: 8 }}>{gallery.length} image{gallery.length !== 1 ? "s" : ""}</div>
      {gallery.length === 0 && (
        <div style={{ ...card, textAlign: "center", color: "var(--text-secondary)", padding: 24 }}>No images yet. Generate one to get started.</div>
      )}
      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8 }}>
        {gallery.map(img => (
          <div key={img.id} style={card}>
            {imageDataCache[img.id] ? (
              <img src={imageDataCache[img.id]} alt={img.prompt}
                style={{ width: "100%", borderRadius: "var(--radius-sm)", marginBottom: 8, display: "block" }} />
            ) : (
              <div style={{
                width: "100%", aspectRatio: `${img.width}/${img.height}`, background: "var(--bg-tertiary)",
                borderRadius: "var(--radius-sm)", marginBottom: 8, display: "flex", alignItems: "center",
                justifyContent: "center", fontSize: 11, color: "var(--text-secondary)",
              }}>
                {img.status === "failed" ? "Failed" : img.status === "completed" ? "Loading..." : `${img.width} x ${img.height}`}
              </div>
            )}
            <div style={{ fontSize: 12, fontWeight: 600, marginBottom: 4, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }} title={img.prompt}>
              {img.prompt}
            </div>
            <div style={{ fontSize: 11, color: "var(--text-secondary)", marginBottom: 6, display: "flex", alignItems: "center", gap: 6, flexWrap: "wrap" }}>
              <span>{img.model}</span>
              <span>&middot;</span>
              <span>{img.style}</span>
              <span>&middot;</span>
              <span>{img.width}x{img.height}</span>
              <span>&middot;</span>
              <span>${img.cost.toFixed(2)}</span>
              <span>&middot;</span>
              <span>{formatTime(img.created_at)}</span>
              <span style={badge(statusBg(img.status))}>{img.status}</span>
            </div>
            <div style={{ display: "flex", gap: 6 }}>
              {imageDataCache[img.id] && (
                <a href={imageDataCache[img.id]} download={`${img.id}.png`}
                  style={{ ...btnPrimary, padding: "3px 10px", fontSize: 11, textDecoration: "none", background: "var(--bg-tertiary)", color: "var(--text-primary)", border: "1px solid var(--border-color)" }}>
                  Save
                </a>
              )}
              <button style={{ ...btnPrimary, padding: "3px 10px", fontSize: 11, background: "var(--error-color)" }}
                onClick={() => handleDelete(img.id)}>Delete</button>
            </div>
          </div>
        ))}
      </div>
    </div>
  );

  // ── Stats tab ───────────────────────────────────────────────────────────────

  const statCell = (lbl: string, val: string | number, color?: string) => (
    <div style={card}>
      <div style={{ fontSize: 10, color: "var(--text-secondary)", marginBottom: 2, textTransform: "uppercase", letterSpacing: "0.04em" }}>{lbl}</div>
      <div style={{ fontSize: 18, fontWeight: 700, fontFamily: "var(--font-mono, monospace)", color }}>{val}</div>
    </div>
  );

  const renderStats = () => (
    <div>
      {stats ? (
        <>
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8, marginBottom: 12 }}>
            {statCell("Total Images", stats.total_images)}
            {statCell("Total Cost", `$${stats.total_cost.toFixed(2)}`)}
            {statCell("Completed", stats.completed, "var(--success-color)")}
            {statCell("Failed", stats.failed, stats.failed > 0 ? "var(--error-color)" : undefined)}
          </div>

          {/* Per-image cost breakdown */}
          {gallery.length > 0 && (
            <div style={{ ...card, marginBottom: 8 }}>
              <div style={{ fontSize: 10, color: "var(--text-secondary)", marginBottom: 6, textTransform: "uppercase", letterSpacing: "0.04em" }}>Recent Generations</div>
              <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 11 }}>
                <thead>
                  <tr style={{ borderBottom: "1px solid var(--border-color)" }}>
                    <th style={{ textAlign: "left", padding: "4px 6px", color: "var(--text-secondary)", fontWeight: 600 }}>Prompt</th>
                    <th style={{ textAlign: "left", padding: "4px 6px", color: "var(--text-secondary)", fontWeight: 600 }}>Model</th>
                    <th style={{ textAlign: "right", padding: "4px 6px", color: "var(--text-secondary)", fontWeight: 600 }}>Size</th>
                    <th style={{ textAlign: "right", padding: "4px 6px", color: "var(--text-secondary)", fontWeight: 600 }}>Cost</th>
                  </tr>
                </thead>
                <tbody>
                  {gallery.slice(0, 20).map(img => (
                    <tr key={img.id} style={{ borderBottom: "1px solid var(--border-color)" }}>
                      <td style={{ padding: "4px 6px", maxWidth: 160, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }} title={img.prompt}>{img.prompt}</td>
                      <td style={{ padding: "4px 6px", color: "var(--text-secondary)" }}>{img.model}</td>
                      <td style={{ padding: "4px 6px", textAlign: "right", fontFamily: "var(--font-mono, monospace)", color: "var(--text-secondary)" }}>{img.width}x{img.height}</td>
                      <td style={{ padding: "4px 6px", textAlign: "right", fontFamily: "var(--font-mono, monospace)", fontWeight: 600 }}>${img.cost.toFixed(2)}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}

          {stats.models_used.length > 0 && (
            <div style={{ ...card, marginBottom: 8 }}>
              <div style={{ fontSize: 10, color: "var(--text-secondary)", marginBottom: 6, textTransform: "uppercase", letterSpacing: "0.04em" }}>Models</div>
              <div style={{ display: "flex", gap: 4, flexWrap: "wrap" }}>
                {stats.models_used.map(m => <span key={m} style={badge("var(--accent-color)")}>{m}</span>)}
              </div>
            </div>
          )}
          {stats.styles_used.length > 0 && (
            <div style={card}>
              <div style={{ fontSize: 10, color: "var(--text-secondary)", marginBottom: 6, textTransform: "uppercase", letterSpacing: "0.04em" }}>Styles</div>
              <div style={{ display: "flex", gap: 4, flexWrap: "wrap" }}>
                {stats.styles_used.map(s => <span key={s} style={badge("var(--info-color)")}>{s}</span>)}
              </div>
            </div>
          )}
        </>
      ) : (
        <div style={{ ...card, textAlign: "center", color: "var(--text-secondary)" }}>Loading stats...</div>
      )}
    </div>
  );

  // ── Layout ──────────────────────────────────────────────────────────────────

  return (
    <div style={{ padding: 16, height: "100%", overflow: "auto", color: "var(--text-primary)" }}>
      <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: 14 }}>
        <span style={{ fontSize: 14, fontWeight: 700 }}>Image Generation</span>
        <div style={{ display: "flex", gap: 4 }}>
          {tabBtn("generate", "Generate")}
          {tabBtn("gallery", "Gallery")}
          {tabBtn("stats", "Stats")}
        </div>
      </div>
      {tab === "generate" && renderGenerate()}
      {tab === "gallery" && renderGallery()}
      {tab === "stats" && renderStats()}
    </div>
  );
};

export default ImageGenPanel;
