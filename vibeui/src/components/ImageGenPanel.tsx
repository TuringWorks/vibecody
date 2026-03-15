import React, { useState } from "react";

interface GeneratedImage {
  id: string;
  prompt: string;
  model: string;
  width: number;
  height: number;
  style: string;
  generatedAt: string;
  cost: number;
}

interface BatchRequest {
  id: string;
  prompt: string;
  status: "Queued" | "Running" | "Done" | "Failed";
  cost: number;
}

const ImageGenPanel: React.FC = () => {
  const [activeTab, setActiveTab] = useState<string>("generate");
  const [prompt, setPrompt] = useState("");
  const [style, setStyle] = useState("Photorealistic");
  const [width, setWidth] = useState(1024);
  const [height, setHeight] = useState(1024);
  const [model, setModel] = useState("DALL-E 3");
  const [gallery, setGallery] = useState<GeneratedImage[]>([
    { id: "img-1", prompt: "A futuristic city at sunset", model: "DALL-E 3", width: 1024, height: 1024, style: "Photorealistic", generatedAt: "2 min ago", cost: 0.04 },
    { id: "img-2", prompt: "Abstract neural network visualization", model: "Stable Diffusion XL", width: 1024, height: 768, style: "Digital Art", generatedAt: "15 min ago", cost: 0.02 },
    { id: "img-3", prompt: "Minimalist logo for AI startup", model: "Midjourney v6", width: 512, height: 512, style: "Minimalist", generatedAt: "1 hr ago", cost: 0.05 },
  ]);
  const [batch, setBatch] = useState<BatchRequest[]>([
    { id: "b-1", prompt: "Product hero image - dark theme", status: "Done", cost: 0.04 },
    { id: "b-2", prompt: "Landing page illustration", status: "Running", cost: 0.03 },
    { id: "b-3", prompt: "Icon set - 16 app icons", status: "Queued", cost: 0.08 },
    { id: "b-4", prompt: "Team avatar placeholders", status: "Queued", cost: 0.02 },
  ]);

  const styles = ["Photorealistic", "Digital Art", "Watercolor", "Oil Painting", "Pixel Art",
    "Anime", "Minimalist", "3D Render", "Sketch", "Comic Book"];
  const models = ["DALL-E 3", "Stable Diffusion XL", "Midjourney v6", "Imagen 3", "Flux Pro"];

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

  const handleGenerate = () => {
    if (!prompt.trim()) return;
    const img: GeneratedImage = {
      id: `img-${Date.now()}`, prompt, model, width, height, style, generatedAt: "just now", cost: costEstimate,
    };
    setGallery(prev => [img, ...prev]);
    setPrompt("");
  };

  const statusColor = (s: string) =>
    s === "Done" ? "var(--success-color)" : s === "Running" ? "var(--info-color)" : s === "Failed" ? "var(--error-color)" : "var(--text-muted)";

  const renderGenerate = () => (
    <div>
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
      <button style={{ ...btnStyle, width: "100%", marginTop: "8px", padding: "10px" }} onClick={handleGenerate}>
        Generate Image
      </button>
    </div>
  );

  const renderGallery = () => (
    <div>
      <div style={{ fontSize: "12px", opacity: 0.7, marginBottom: "8px" }}>{gallery.length} images</div>
      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "8px" }}>
        {gallery.map(img => (
          <div key={img.id} style={cardStyle}>
            <div style={{ width: "100%", height: "80px", backgroundColor: "var(--bg-tertiary)",
              borderRadius: "4px", marginBottom: "8px", display: "flex", alignItems: "center", justifyContent: "center",
              fontSize: "11px", opacity: 0.5 }}>{img.width}x{img.height}</div>
            <div style={{ fontSize: "12px", fontWeight: 600, marginBottom: "4px", overflow: "hidden",
              textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{img.prompt}</div>
            <div style={{ fontSize: "11px", opacity: 0.7 }}>{img.model} &middot; {img.generatedAt}</div>
          </div>
        ))}
      </div>
    </div>
  );

  const totalBatchCost = batch.reduce((sum, b) => sum + b.cost, 0);

  const renderBatch = () => (
    <div>
      <div style={{ ...cardStyle, display: "flex", justifyContent: "space-between", marginBottom: "12px" }}>
        <span>{batch.length} requests</span>
        <strong>Total: ${totalBatchCost.toFixed(2)}</strong>
      </div>
      {batch.map((b) => (
        <div key={b.id} style={{ ...cardStyle, display: "flex", justifyContent: "space-between", alignItems: "center" }}>
          <div>
            <div style={{ fontWeight: 600, fontSize: "13px" }}>{b.prompt}</div>
            <div style={{ fontSize: "11px", opacity: 0.7 }}>${b.cost.toFixed(2)}</div>
          </div>
          <span style={badgeStyle(statusColor(b.status))}>{b.status}</span>
        </div>
      ))}
      <button style={{ ...btnStyle, width: "100%", marginTop: "8px", padding: "10px" }}
        onClick={() => setBatch(prev => prev.map(b => ({ ...b, status: "Done" as const })))}>
        Run All
      </button>
    </div>
  );

  return (
    <div style={containerStyle}>
      <h2 style={{ margin: "0 0 12px" }}>Image Generation</h2>
      <div style={tabBarStyle}>
        {[["generate", "Generate"], ["gallery", "Gallery"], ["batch", "Batch"]].map(([id, label]) => (
          <button key={id} style={tabStyle(activeTab === id)} onClick={() => setActiveTab(id)}>{label}</button>
        ))}
      </div>
      {activeTab === "generate" && renderGenerate()}
      {activeTab === "gallery" && renderGallery()}
      {activeTab === "batch" && renderBatch()}
    </div>
  );
};

export default ImageGenPanel;
