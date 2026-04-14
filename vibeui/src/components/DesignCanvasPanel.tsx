import React, { useState, useCallback, useRef } from "react";

/**
 * DesignCanvasPanel — Visual component builder for VibeCody.
 *
 * Provides a drag-and-drop canvas for assembling UI components visually,
 * then generates React/Vue/Svelte code. Inspired by Bolt.new and v0.
 *
 * Features:
 * - Component palette with common UI primitives
 * - Drag-and-drop onto canvas with snap-to-grid
 * - Property editor for selected components
 * - Live code preview (React + Tailwind CSS output)
 * - AI-powered "describe what you want" → component generation
 * - Export to file with one click
 */

// ── Types ────────────────────────────────────────────────────────────────

interface CanvasComponent {
  id: string;
  type: ComponentType;
  x: number;
  y: number;
  width: number;
  height: number;
  props: Record<string, string>;
  children: string[]; // IDs of child components
}

type ComponentType =
  | "container"
  | "text"
  | "heading"
  | "button"
  | "input"
  | "image"
  | "card"
  | "list"
  | "nav"
  | "form"
  | "table"
  | "modal"
  | "sidebar"
  | "hero"
  | "footer";

type Tab = "canvas" | "code" | "ai" | "export";

// ── Component Palette ────────────────────────────────────────────────────

const PALETTE: { type: ComponentType; label: string; icon: string; defaultSize: [number, number] }[] = [
  { type: "container", label: "Container", icon: "[ ]", defaultSize: [300, 200] },
  { type: "heading",   label: "Heading",   icon: "H",   defaultSize: [200, 40] },
  { type: "text",      label: "Text",      icon: "T",   defaultSize: [200, 30] },
  { type: "button",    label: "Button",    icon: "Btn", defaultSize: [120, 40] },
  { type: "input",     label: "Input",     icon: "[ ]", defaultSize: [200, 36] },
  { type: "image",     label: "Image",     icon: "Img", defaultSize: [200, 150] },
  { type: "card",      label: "Card",      icon: "C",   defaultSize: [280, 180] },
  { type: "list",      label: "List",      icon: "L",   defaultSize: [200, 120] },
  { type: "nav",       label: "Navbar",    icon: "N",   defaultSize: [400, 50] },
  { type: "form",      label: "Form",      icon: "F",   defaultSize: [300, 250] },
  { type: "table",     label: "Table",     icon: "T",   defaultSize: [350, 200] },
  { type: "hero",      label: "Hero",      icon: "H",   defaultSize: [400, 250] },
  { type: "sidebar",   label: "Sidebar",   icon: "S",   defaultSize: [200, 400] },
  { type: "footer",    label: "Footer",    icon: "F",   defaultSize: [400, 60] },
];

// ── Code Generator ───────────────────────────────────────────────────────

function generateReactCode(components: CanvasComponent[]): string {
  if (components.length === 0) return "// Add components to the canvas to generate code";

  const lines: string[] = [
    'import React from "react";',
    "",
    "export default function GeneratedPage() {",
    "  return (",
    '    <div className="min-h-screen bg-white p-8">',
  ];

  for (const comp of components) {
    const indent = "      ";
    switch (comp.type) {
      case "heading":
        lines.push(`${indent}<h1 className="text-3xl font-bold mb-4">${comp.props.text || "Heading"}</h1>`);
        break;
      case "text":
        lines.push(`${indent}<p className="text-gray-600 mb-2">${comp.props.text || "Text content"}</p>`);
        break;
      case "button":
        lines.push(`${indent}<button className="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700">`);
        lines.push(`${indent}  ${comp.props.text || "Click me"}`);
        lines.push(`${indent}</button>`);
        break;
      case "input":
        lines.push(`${indent}<input`);
        lines.push(`${indent}  type="${comp.props.inputType || "text"}"`);
        lines.push(`${indent}  placeholder="${comp.props.placeholder || "Enter text..."}"`);
        lines.push(`${indent}  className="border rounded px-3 py-2 w-full focus:outline-none focus:ring-2 focus:ring-blue-500"`);
        lines.push(`${indent}/>`);
        break;
      case "image":
        lines.push(`${indent}<img src="${comp.props.src || "/placeholder.jpg"}" alt="${comp.props.alt || "Image"}" className="rounded-lg object-cover" style={{ width: ${comp.width}, height: ${comp.height} }} />`);
        break;
      case "card":
        lines.push(`${indent}<div className="bg-white rounded-lg shadow-md p-6 border">`);
        lines.push(`${indent}  <h3 className="text-lg font-semibold mb-2">${comp.props.title || "Card Title"}</h3>`);
        lines.push(`${indent}  <p className="text-gray-500">${comp.props.text || "Card content goes here."}</p>`);
        lines.push(`${indent}</div>`);
        break;
      case "nav":
        lines.push(`${indent}<nav className="flex items-center justify-between p-4 bg-gray-900 text-white rounded">`);
        lines.push(`${indent}  <span className="font-bold text-xl">${comp.props.brand || "Brand"}</span>`);
        lines.push(`${indent}  <div className="space-x-4">`);
        lines.push(`${indent}    <a href="#" className="hover:text-gray-300">Home</a>`);
        lines.push(`${indent}    <a href="#" className="hover:text-gray-300">About</a>`);
        lines.push(`${indent}    <a href="#" className="hover:text-gray-300">Contact</a>`);
        lines.push(`${indent}  </div>`);
        lines.push(`${indent}</nav>`);
        break;
      case "form":
        lines.push(`${indent}<form className="space-y-4 p-6 bg-gray-50 rounded-lg">`);
        lines.push(`${indent}  <div>`);
        lines.push(`${indent}    <label className="block text-sm font-medium mb-1">Name</label>`);
        lines.push(`${indent}    <input type="text" className="border rounded px-3 py-2 w-full" />`);
        lines.push(`${indent}  </div>`);
        lines.push(`${indent}  <div>`);
        lines.push(`${indent}    <label className="block text-sm font-medium mb-1">Email</label>`);
        lines.push(`${indent}    <input type="email" className="border rounded px-3 py-2 w-full" />`);
        lines.push(`${indent}  </div>`);
        lines.push(`${indent}  <button type="submit" className="px-4 py-2 bg-blue-600 text-white rounded">Submit</button>`);
        lines.push(`${indent}</form>`);
        break;
      case "hero":
        lines.push(`${indent}<section className="text-center py-16 bg-gradient-to-r from-blue-600 to-purple-600 text-white rounded-lg">`);
        lines.push(`${indent}  <h1 className="text-5xl font-bold mb-4">${comp.props.title || "Welcome"}</h1>`);
        lines.push(`${indent}  <p className="text-xl mb-8">${comp.props.subtitle || "Build something amazing"}</p>`);
        lines.push(`${indent}  <button className="px-8 py-3 bg-white text-blue-600 rounded-lg font-semibold">Get Started</button>`);
        lines.push(`${indent}</section>`);
        break;
      case "container":
        lines.push(`${indent}<div className="p-4 border-2 border-dashed border-gray-300 rounded-lg">`);
        lines.push(`${indent}  {/* Container content */}`);
        lines.push(`${indent}</div>`);
        break;
      case "table":
        lines.push(`${indent}<table className="w-full border-collapse border">`);
        lines.push(`${indent}  <thead><tr className="bg-gray-100">`);
        lines.push(`${indent}    <th className="border p-2">Column 1</th>`);
        lines.push(`${indent}    <th className="border p-2">Column 2</th>`);
        lines.push(`${indent}    <th className="border p-2">Column 3</th>`);
        lines.push(`${indent}  </tr></thead>`);
        lines.push(`${indent}  <tbody><tr><td className="border p-2">Data</td><td className="border p-2">Data</td><td className="border p-2">Data</td></tr></tbody>`);
        lines.push(`${indent}</table>`);
        break;
      case "list":
        lines.push(`${indent}<ul className="list-disc pl-5 space-y-1">`);
        lines.push(`${indent}  <li>Item 1</li>`);
        lines.push(`${indent}  <li>Item 2</li>`);
        lines.push(`${indent}  <li>Item 3</li>`);
        lines.push(`${indent}</ul>`);
        break;
      case "sidebar":
        lines.push(`${indent}<aside className="w-64 bg-gray-800 text-white p-4 rounded-lg">`);
        lines.push(`${indent}  <nav className="space-y-2">`);
        lines.push(`${indent}    <a href="#" className="block px-3 py-2 rounded hover:bg-gray-700">Dashboard</a>`);
        lines.push(`${indent}    <a href="#" className="block px-3 py-2 rounded hover:bg-gray-700">Settings</a>`);
        lines.push(`${indent}    <a href="#" className="block px-3 py-2 rounded hover:bg-gray-700">Profile</a>`);
        lines.push(`${indent}  </nav>`);
        lines.push(`${indent}</aside>`);
        break;
      case "footer":
        lines.push(`${indent}<footer className="text-center py-4 text-gray-500 border-t">`);
        lines.push(`${indent}  <p>${comp.props.text || "Built with VibeCody"}</p>`);
        lines.push(`${indent}</footer>`);
        break;
      default:
        lines.push(`${indent}<div>Unknown component: ${comp.type}</div>`);
    }
  }

  lines.push("    </div>");
  lines.push("  );");
  lines.push("}");
  return lines.join("\n");
}

// ── Main Panel ───────────────────────────────────────────────────────────

export function DesignCanvasPanel() {
  const [tab, setTab] = useState<Tab>("canvas");
  const [components, setComponents] = useState<CanvasComponent[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [aiPrompt, setAiPrompt] = useState("");
  const [dragType, setDragType] = useState<ComponentType | null>(null);
  const canvasRef = useRef<HTMLDivElement>(null);
  const nextId = useRef(1);

  const addComponent = useCallback((type: ComponentType, x: number, y: number) => {
    const palette = PALETTE.find((p) => p.type === type);
    const [w, h] = palette?.defaultSize || [200, 100];
    const id = `comp-${nextId.current++}`;
    const comp: CanvasComponent = {
      id,
      type,
      x: Math.round(x / 10) * 10, // snap to 10px grid
      y: Math.round(y / 10) * 10,
      width: w,
      height: h,
      props: {},
      children: [],
    };
    setComponents((prev) => [...prev, comp]);
    setSelectedId(id);
  }, []);

  const removeComponent = useCallback((id: string) => {
    setComponents((prev) => prev.filter((c) => c.id !== id));
    if (selectedId === id) setSelectedId(null);
  }, [selectedId]);

  const updateProp = useCallback((id: string, key: string, value: string) => {
    setComponents((prev) =>
      prev.map((c) => (c.id === id ? { ...c, props: { ...c.props, [key]: value } } : c))
    );
  }, []);

  const selected = components.find((c) => c.id === selectedId);
  const generatedCode = generateReactCode(components);

  // ── Styles ──────────────────────────────────────────────────────────

  const paletteItemStyle: React.CSSProperties = {
    padding: "6px 10px",
    cursor: "grab",
    borderRadius: "var(--radius-xs-plus)",
    border: "1px solid var(--border-color)",
    fontSize: "var(--font-size-base)",
    textAlign: "center" as const,
    userSelect: "none" as const,
  };

  return (
    <div className="panel-container">
      {/* Tab bar */}
      <div className="panel-tab-bar">
        <button className={`panel-tab ${tab === "canvas" ? "active" : ""}`} onClick={() => setTab("canvas")}>Canvas</button>
        <button className={`panel-tab ${tab === "code" ? "active" : ""}`} onClick={() => setTab("code")}>Code</button>
        <button className={`panel-tab ${tab === "ai" ? "active" : ""}`} onClick={() => setTab("ai")}>AI Generate</button>
        <button className={`panel-tab ${tab === "export" ? "active" : ""}`} onClick={() => setTab("export")}>Export</button>
        <span style={{ marginLeft: "auto", padding: "6px 12px", fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>
          {components.length} components
        </span>
      </div>

      {/* Tab content */}
      <div className="panel-body" style={{ padding: 0, display: "flex" }}>
        {tab === "canvas" && (
          <div style={{ display: "flex", flex: 1 }}>
            {/* Palette sidebar */}
            <div style={{
              width: "140px",
              borderRight: "1px solid var(--border-color)",
              padding: "8px",
              overflowY: "auto",
              display: "flex",
              flexDirection: "column",
              gap: "4px",
            }}>
              <div style={{ fontSize: "var(--font-size-sm)", fontWeight: 600, marginBottom: "4px", color: "var(--text-secondary)" }}>
                COMPONENTS
              </div>
              {PALETTE.map((item) => (
                <div
                  key={item.type}
                  style={paletteItemStyle}
                  draggable
                  onDragStart={() => setDragType(item.type)}
                  onDragEnd={() => setDragType(null)}
                >
                  <span style={{ marginRight: "6px", fontFamily: "var(--font-mono)" }}>{item.icon}</span>
                  {item.label}
                </div>
              ))}
            </div>

            {/* Canvas area */}
            <div
              ref={canvasRef}
              style={{
                flex: 1,
                position: "relative",
                background: "var(--editor-bg, #1e1e1e)",
                backgroundImage: "radial-gradient(circle, var(--border-color) 1px, transparent 1px)",
                backgroundSize: "20px 20px",
                overflow: "auto",
              }}
              onDragOver={(e) => e.preventDefault()}
              onDrop={(e) => {
                e.preventDefault();
                if (dragType && canvasRef.current) {
                  const rect = canvasRef.current.getBoundingClientRect();
                  addComponent(dragType, e.clientX - rect.left, e.clientY - rect.top);
                }
              }}
              onClick={() => setSelectedId(null)}
            >
              {components.map((comp) => (
                <div
                  key={comp.id}
                  style={{
                    position: "absolute",
                    left: comp.x,
                    top: comp.y,
                    width: comp.width,
                    height: comp.height,
                    border: selectedId === comp.id
                      ? "2px solid var(--accent-color, #007acc)"
                      : "1px solid var(--border-color)",
                    borderRadius: "var(--radius-xs-plus)",
                    background: "var(--panel-bg, #252526)",
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "center",
                    cursor: "pointer",
                    fontSize: "var(--font-size-base)",
                    color: "var(--text-secondary)",
                  }}
                  onClick={(e) => { e.stopPropagation(); setSelectedId(comp.id); }}
                >
                  <span>{comp.type}</span>
                  {comp.props.text && <span style={{ marginLeft: 4 }}>: {comp.props.text}</span>}
                </div>
              ))}

              {components.length === 0 && (
                <div style={{
                  position: "absolute",
                  top: "50%",
                  left: "50%",
                  transform: "translate(-50%, -50%)",
                  textAlign: "center",
                  color: "var(--text-secondary)",
                }}>
                  <div style={{ fontSize: "16px", marginBottom: "8px" }}>Drag components here</div>
                  <div style={{ fontSize: "var(--font-size-base)" }}>Or use AI Generate tab to describe your UI</div>
                </div>
              )}
            </div>

            {/* Properties panel */}
            {selected && (
              <div style={{
                width: "200px",
                borderLeft: "1px solid var(--border-color)",
                padding: "12px",
                overflowY: "auto",
              }}>
                <div style={{ fontSize: "var(--font-size-md)", fontWeight: 600, marginBottom: "8px" }}>
                  {selected.type}
                </div>
                <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginBottom: "12px" }}>
                  {selected.id}
                </div>

                <label style={{ fontSize: "var(--font-size-sm)", display: "block", marginBottom: "4px" }}>Text</label>
                <input
                  type="text"
                  value={selected.props.text || ""}
                  onChange={(e) => updateProp(selected.id, "text", e.target.value)}
                  style={{
                    width: "100%",
                    padding: "4px 8px",
                    fontSize: "var(--font-size-base)",
                    background: "var(--bg-secondary)",
                    color: "var(--text-primary)",
                    border: "1px solid var(--border-color)",
                    borderRadius: "3px",
                    marginBottom: "8px",
                    boxSizing: "border-box",
                  }}
                />

                {selected.type === "button" && (
                  <>
                    <label style={{ fontSize: "var(--font-size-sm)", display: "block", marginBottom: "4px" }}>Variant</label>
                    <select
                      value={selected.props.variant || "primary"}
                      onChange={(e) => updateProp(selected.id, "variant", e.target.value)}
                      style={{
                        width: "100%",
                        padding: "4px",
                        fontSize: "var(--font-size-base)",
                        background: "var(--bg-secondary)",
                        color: "var(--text-primary)",
                        border: "1px solid var(--border-color)",
                        borderRadius: "3px",
                        marginBottom: "8px",
                        boxSizing: "border-box",
                      }}
                    >
                      <option value="primary">Primary</option>
                      <option value="secondary">Secondary</option>
                      <option value="outline">Outline</option>
                      <option value="ghost">Ghost</option>
                    </select>
                  </>
                )}

                <button
                  onClick={() => removeComponent(selected.id)}
                  className="panel-btn panel-btn-danger"
                  style={{ marginTop: "16px", width: "100%" }}
                >
                  Delete
                </button>
              </div>
            )}
          </div>
        )}

        {tab === "code" && (
          <div style={{ flex: 1, padding: "12px", overflow: "auto" }}>
            <div style={{ display: "flex", justifyContent: "space-between", marginBottom: "8px" }}>
              <span style={{ fontSize: "var(--font-size-md)", fontWeight: 600 }}>Generated React + Tailwind CSS</span>
              <button
                onClick={() => navigator.clipboard.writeText(generatedCode)}
                className="panel-btn panel-btn-primary"
              >
                Copy
              </button>
            </div>
            <pre style={{
              background: "var(--terminal-bg, #1a1a1a)",
              padding: "12px",
              borderRadius: "var(--radius-xs-plus)",
              fontSize: "var(--font-size-base)",
              lineHeight: "1.5",
              whiteSpace: "pre-wrap",
              overflow: "auto",
              flex: 1,
            }}>
              {generatedCode}
            </pre>
          </div>
        )}

        {tab === "ai" && (
          <div style={{ flex: 1, padding: "16px", display: "flex", flexDirection: "column" }}>
            <div style={{ fontSize: "var(--font-size-md)", fontWeight: 600, marginBottom: "8px" }}>
              Describe Your UI
            </div>
            <p style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", marginBottom: "12px" }}>
              Describe the page or component you want to build. The AI will generate
              the component layout for you.
            </p>
            <textarea
              value={aiPrompt}
              onChange={(e) => setAiPrompt(e.target.value)}
              placeholder="e.g. A landing page with a hero section, feature cards, pricing table, and footer..."
              style={{
                flex: 1,
                maxHeight: "200px",
                padding: "10px",
                fontSize: "var(--font-size-md)",
                background: "var(--bg-secondary)",
                color: "var(--text-primary)",
                border: "1px solid var(--border-color)",
                borderRadius: "var(--radius-xs-plus)",
                resize: "vertical",
                fontFamily: "inherit",
              }}
            />
            <button
              className="panel-btn panel-btn-primary"
              style={{ marginTop: "12px", opacity: aiPrompt.trim() ? 1 : 0.5 }}
              onClick={() => {
                // Generate components from AI prompt (heuristic for now)
                const prompt = aiPrompt.toLowerCase();
                const newComponents: CanvasComponent[] = [];
                let y = 20;
                const makeComp = (type: ComponentType, props: Record<string, string> = {}): CanvasComponent => {
                  const palette = PALETTE.find((p) => p.type === type);
                  const [w, h] = palette?.defaultSize || [300, 100];
                  const comp: CanvasComponent = {
                    id: `comp-${nextId.current++}`,
                    type,
                    x: 20,
                    y,
                    width: w,
                    height: h,
                    props,
                    children: [],
                  };
                  y += h + 16;
                  return comp;
                };

                if (prompt.includes("nav") || prompt.includes("header")) {
                  newComponents.push(makeComp("nav", { brand: "MyApp" }));
                }
                if (prompt.includes("hero") || prompt.includes("landing")) {
                  newComponents.push(makeComp("hero", { title: "Welcome", subtitle: "Build amazing things" }));
                }
                if (prompt.includes("card") || prompt.includes("feature")) {
                  newComponents.push(makeComp("card", { title: "Feature 1", text: "Description here" }));
                  newComponents.push(makeComp("card", { title: "Feature 2", text: "Another feature" }));
                }
                if (prompt.includes("form") || prompt.includes("contact") || prompt.includes("signup")) {
                  newComponents.push(makeComp("form"));
                }
                if (prompt.includes("table") || prompt.includes("data") || prompt.includes("pricing")) {
                  newComponents.push(makeComp("table"));
                }
                if (prompt.includes("list")) {
                  newComponents.push(makeComp("list"));
                }
                if (prompt.includes("footer")) {
                  newComponents.push(makeComp("footer", { text: "Built with VibeCody" }));
                }
                if (newComponents.length === 0) {
                  // Default: heading + text + button
                  newComponents.push(makeComp("heading", { text: "Generated Page" }));
                  newComponents.push(makeComp("text", { text: aiPrompt.slice(0, 100) }));
                  newComponents.push(makeComp("button", { text: "Get Started" }));
                }

                setComponents(newComponents);
                setTab("canvas");
              }}
              disabled={!aiPrompt.trim()}
            >
              Generate Components
            </button>
          </div>
        )}

        {tab === "export" && (
          <div style={{ flex: 1, padding: "16px" }}>
            <div style={{ fontSize: "var(--font-size-md)", fontWeight: 600, marginBottom: "12px" }}>
              Export Generated Code
            </div>
            <div style={{ display: "flex", flexDirection: "column", gap: "8px" }}>
              <button
                onClick={() => navigator.clipboard.writeText(generatedCode)}
                className="panel-btn panel-btn-primary"
                style={{ textAlign: "left" as const }}
              >
                Copy to Clipboard
              </button>
              <button
                className="panel-btn panel-btn-secondary"
                style={{ textAlign: "left" as const }}
              >
                Save as src/components/GeneratedPage.tsx
              </button>
              <button
                onClick={() => {
                  setComponents([]);
                  setSelectedId(null);
                }}
                className="panel-btn panel-btn-secondary"
                style={{ textAlign: "left" as const }}
              >
                Clear Canvas
              </button>
            </div>
            <div style={{ marginTop: "20px", fontSize: "var(--font-size-base)", color: "var(--text-secondary)" }}>
              <p>Components: {components.length}</p>
              <p>Code lines: {generatedCode.split("\n").length}</p>
              <p>Framework: React + Tailwind CSS</p>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
