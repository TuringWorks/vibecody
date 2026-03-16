import React, { useState, useEffect, useCallback } from "react";
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

const ImageGenPanel: React.FC = () => {
  const [activeTab, setActiveTab] = useState<string>("generate");
  const [prompt, setPrompt] = useState("");
  const [style, setStyle] = useState("Photorealistic");
  const [width, setWidth] = useState(1024);
  const [height, setHeight] = useState(1024);
  const [model, setModel] = useState("DALL-E 3");
  const [gallery, setGallery] = useState<GeneratedImage[]>([]);
  const [stats, setStats] = useState<ImageGenStats | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const styles = ["Photorealistic", "Digital Art", "Watercolor", "Oil Painting", "Pixel Art",
    "Anime", "Minimalist", "3D Render", "Sketch", "Comic Book"];
  const models = ["DALL-E 3", "Stable Diffusion XL", "Midjourney v6", "Imagen 3", "Flux Pro"];

  const loadGallery = useCallback(async () => {
    try {
      const images = await invoke<GeneratedImage[]>("list_generated_images");
      setGallery(images);
    } catch (e) {
      setError(String(e));
    }
  }, []);

  const loadStats = useCallback(async () => {
    try {
      const s = await invoke<ImageGenStats>("get_image_gen_stats");
      setStats(s);
    } catch (e) {
      setError(String(e));
    }
  }, []);

  useEffect(() => {
    loadGallery();
    loadStats();
  }, [loadGallery, loadStats]);

  const containerStyle: React.CSSProperties = {
    padding: "16px", color: "var(--text-primary)",
    backgroundColor: "var(--bg-primary)",
    fontFamily: "var(--font-mono, monospace)", fontSize: "13px",
    height: "100%", overflow: "auto",
  };
  const tabBarStyle: React.CSSProperties = {
    display: "flex", gap: "4px", marginBottom: "16px",
    borderBottom: "1px solid var(--border-color)", paddingBottom: "8px",
  };
  const tabStyle = (active: boolean): React.CSSProperties => ({
    padding: "6px 14px", cursor: "pointer", border: "none",
    backgroundColor: active ? "var(--accent-color)" : "transparent",
    color: active ? "var(--text-primary)" : "var(--text-primary)",
    borderRadius: "4px", fontSize: "13px",
  });
  const inputStyle: React.CSSProperties = {
    width: "100%", padding: "6px 10px", boxSizing: "border-box",
    backgroundColor: "var(--bg-secondary)", color: "var(--text-primary)",
    border: "1px solid var(--border-color)", borderRadius: "4px",
  };
  const btnStyle: React.CSSProperties = {
    padding: "6px 14px", cursor: "pointer", border: "none", borderRadius: "4px",
    backgroundColor: "var(--accent-color)", color: "var(--text-primary)",
  };
  const cardStyle: React.CSSProperties = {
    padding: "10px", marginBottom: "8px", borderRadius: "4px",
    backgroundColor: "var(--bg-secondary)",
    border: "1px solid var(--border-color)",
  };
  const labelStyle: React.CSSProperties = { display: "block", marginBottom: "4px", fontWeight: 600, fontSize: "12px" };
  const fieldGroup: React.CSSProperties = { marginBottom: "12px" };
  const badgeStyle = (color: string): React.CSSProperties => ({
    display: "inline-block", padding: "2px 8px", borderRadius: "10px",
    fontSize: "11px", fontWeight: 600, backgroundColor: color, color: "white",
  });

  const costEstimate = model === "DALL-E 3" ? 0.04 : model === "Midjourney v6" ? 0.05 : 0.02;

  const handleGenerate = async () => {
    if (!prompt.trim()) return;
    setLoading(true);
    setError(null);
    try {
      await invoke<GeneratedImage>("generate_image", {
        prompt, model, style, width, height,
      });
      setPrompt("");
      await loadGallery();
      await loadStats();
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  const handleDelete = async (id: string) => {
    try {
      await invoke("delete_generated_image", { id });
      await loadGallery();
      await loadStats();
    } catch (e) {
      setError(String(e));
    }
  };

  const statusColor = (s: string) =>
    s === "completed" ? "var(--success-color)" : s === "pending" ? "var(--info-color)" : s === "failed" ? "var(--error-color)" : "var(--text-muted)";

  const formatTime = (epochStr: string) => {
    const epoch = parseInt(epochStr, 10);
    if (isNaN(epoch)) return epochStr;
    const diff = Math.floor(Date.now() / 1000) - epoch;
    if (diff < 60) return "just now";
    if (diff < 3600) return `${Math.floor(diff / 60)} min ago`;
    if (diff < 86400) return `${Math.floor(diff / 3600)} hr ago`;
    return `${Math.floor(diff / 86400)} days ago`;
  };

  const renderGenerate = () => (
    <div>
      {error && (
        <div style={{ ...cardStyle, borderColor: "var(--error-color)", marginBottom: "12px", color: "var(--error-color)" }}>
          {error}
        </div>
      )}
      <div style={fieldGroup}>
        <label style={labelStyle}>Prompt</label>
        <textarea style={{ ...inputStyle, minHeight: "80px", resize: "vertical" }} value={prompt}
          onChange={e => setPrompt(e.target.value)} placeholder="Describe the image you want to generate..." />
      </div>
      <div style={{ display: "flex", gap: "12px", marginBottom: "12px" }}>
        <div style={{ flex: 1 }}>
          <label style={labelStyle}>Style</label>
          <select style={inputStyle} value={style} onChange={e => setStyle(e.target.value)}>
            {styles.map(s => <option key={s} value={s}>{s}</option>)}
          </select>
        </div>
        <div style={{ flex: 1 }}>
          <label style={labelStyle}>Model</label>
          <select style={inputStyle} value={model} onChange={e => setModel(e.target.value)}>
            {models.map(m => <option key={m} value={m}>{m}</option>)}
          </select>
        </div>
      </div>
      <div style={{ display: "flex", gap: "12px", marginBottom: "12px" }}>
        <div style={{ flex: 1 }}>
          <label style={labelStyle}>Width</label>
          <input style={inputStyle} type="number" value={width} onChange={e => setWidth(Number(e.target.value))} />
        </div>
        <div style={{ flex: 1 }}>
          <label style={labelStyle}>Height</label>
          <input style={inputStyle} type="number" value={height} onChange={e => setHeight(Number(e.target.value))} />
        </div>
      </div>
      <div style={{ ...cardStyle, display: "flex", justifyContent: "space-between", alignItems: "center" }}>
        <span>Estimated Cost</span>
        <strong>${costEstimate.toFixed(2)}</strong>
      </div>
      <button style={{ ...btnStyle, width: "100%", marginTop: "8px", padding: "10px", opacity: loading ? 0.6 : 1 }}
        onClick={handleGenerate} disabled={loading}>
        {loading ? "Generating..." : "Generate Image"}
      </button>
    </div>
  );

  const renderGallery = () => (
    <div>
      <div style={{ fontSize: "12px", opacity: 0.7, marginBottom: "8px" }}>{gallery.length} images</div>
      {gallery.length === 0 && (
        <div style={{ ...cardStyle, textAlign: "center", opacity: 0.6 }}>No images yet. Generate one to get started.</div>
      )}
      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "8px" }}>
        {gallery.map(img => (
          <div key={img.id} style={cardStyle}>
            <div style={{ width: "100%", height: "80px", backgroundColor: "var(--bg-tertiary)",
              borderRadius: "4px", marginBottom: "8px", display: "flex", alignItems: "center", justifyContent: "center",
              fontSize: "11px", opacity: 0.5 }}>{img.width}x{img.height}</div>
            <div style={{ fontSize: "12px", fontWeight: 600, marginBottom: "4px", overflow: "hidden",
              textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{img.prompt}</div>
            <div style={{ fontSize: "11px", opacity: 0.7, marginBottom: "4px" }}>
              {img.model} &middot; {formatTime(img.created_at)}
              <span style={{ ...badgeStyle(statusColor(img.status)), marginLeft: "6px" }}>{img.status}</span>
            </div>
            <button style={{ ...btnStyle, fontSize: "11px", padding: "2px 8px", backgroundColor: "var(--error-color)" }}
              onClick={() => handleDelete(img.id)}>Delete</button>
          </div>
        ))}
      </div>
    </div>
  );

  const renderStats = () => (
    <div>
      {stats ? (
        <>
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "8px", marginBottom: "12px" }}>
            <div style={cardStyle}>
              <div style={{ fontSize: "11px", opacity: 0.7 }}>Total Images</div>
              <div style={{ fontSize: "20px", fontWeight: 700 }}>{stats.total_images}</div>
            </div>
            <div style={cardStyle}>
              <div style={{ fontSize: "11px", opacity: 0.7 }}>Total Cost</div>
              <div style={{ fontSize: "20px", fontWeight: 700 }}>${stats.total_cost.toFixed(2)}</div>
            </div>
            <div style={cardStyle}>
              <div style={{ fontSize: "11px", opacity: 0.7 }}>Completed</div>
              <div style={{ fontSize: "20px", fontWeight: 700, color: "var(--success-color)" }}>{stats.completed}</div>
            </div>
            <div style={cardStyle}>
              <div style={{ fontSize: "11px", opacity: 0.7 }}>Pending / Failed</div>
              <div style={{ fontSize: "20px", fontWeight: 700 }}>{stats.pending} / {stats.failed}</div>
            </div>
          </div>
          {stats.models_used.length > 0 && (
            <div style={cardStyle}>
              <div style={{ fontSize: "11px", opacity: 0.7, marginBottom: "4px" }}>Models Used</div>
              <div style={{ display: "flex", gap: "4px", flexWrap: "wrap" }}>
                {stats.models_used.map(m => (
                  <span key={m} style={badgeStyle("var(--accent-color)")}>{m}</span>
                ))}
              </div>
            </div>
          )}
          {stats.styles_used.length > 0 && (
            <div style={{ ...cardStyle, marginTop: "8px" }}>
              <div style={{ fontSize: "11px", opacity: 0.7, marginBottom: "4px" }}>Styles Used</div>
              <div style={{ display: "flex", gap: "4px", flexWrap: "wrap" }}>
                {stats.styles_used.map(s => (
                  <span key={s} style={badgeStyle("var(--info-color)")}>{s}</span>
                ))}
              </div>
            </div>
          )}
        </>
      ) : (
        <div style={{ ...cardStyle, textAlign: "center", opacity: 0.6 }}>Loading stats...</div>
      )}
    </div>
  );

  return (
    <div style={containerStyle}>
      <h2 style={{ margin: "0 0 12px" }}>Image Generation</h2>
      <div style={tabBarStyle}>
        {[["generate", "Generate"], ["gallery", "Gallery"], ["stats", "Stats"]].map(([id, label]) => (
          <button key={id} style={tabStyle(activeTab === id)} onClick={() => setActiveTab(id)}>{label}</button>
        ))}
      </div>
      {activeTab === "generate" && renderGenerate()}
      {activeTab === "gallery" && renderGallery()}
      {activeTab === "stats" && renderStats()}
    </div>
  );
};

export default ImageGenPanel;
