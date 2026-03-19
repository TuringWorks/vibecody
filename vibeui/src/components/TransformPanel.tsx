/**
 * TransformPanel — AI-powered code transformation and language migration.
 *
 * Supports COBOL→Java/Rust/C#, .NET upgrade, Java→Kotlin, Python2→3,
 * React Class→Hooks, and 40+ other transform paths.
 */
import { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

interface TransformPanelProps {
  provider: string;
}

interface TransformDef {
  id: string;
  label: string;
  from: string;
  to: string;
  category: string;
  description: string;
}

interface PlanItem {
  file: string;
  description: string;
  estimated_changes: number;
}

interface PlanResult {
  files: PlanItem[];
  total_files: number;
  summary: string;
}

interface ExecResult {
  files_modified: number;
  summary: string;
}

const ALL_TRANSFORMS: TransformDef[] = [
  // Legacy
  { id: "cobol_to_java",    label: "COBOL → Java",       from: "COBOL", to: "Java",       category: "legacy", description: "Convert COBOL programs to modern Java with OOP, JDBC for file I/O, BigDecimal for COMP-3" },
  { id: "cobol_to_rust",    label: "COBOL → Rust",       from: "COBOL", to: "Rust",       category: "legacy", description: "Convert COBOL to safe Rust with strong typing, pattern matching for EVALUATE" },
  { id: "cobol_to_csharp",  label: "COBOL → C#",         from: "COBOL", to: "C#",         category: "legacy", description: "Convert COBOL to C# with .NET patterns, decimal for packed fields, LINQ" },
  { id: "cobol_to_python",  label: "COBOL → Python",     from: "COBOL", to: "Python",     category: "legacy", description: "Convert COBOL to Python with dataclasses, decimal module" },
  { id: "fortran_to_rust",  label: "Fortran → Rust",     from: "Fortran", to: "Rust",     category: "legacy", description: "Convert Fortran numerical code to Rust with ndarray" },
  { id: "fortran_to_python",label: "Fortran → Python",   from: "Fortran", to: "Python",   category: "legacy", description: "Convert Fortran to Python with NumPy/SciPy" },
  { id: "vb6_to_csharp",    label: "VB6 → C#",           from: "VB6",    to: "C#",        category: "legacy", description: "Migrate Visual Basic 6 forms and modules to C# WinForms/WPF" },
  { id: "vb6_to_java",      label: "VB6 → Java",         from: "VB6",    to: "Java",      category: "legacy", description: "Migrate VB6 applications to Java Swing/JavaFX" },
  { id: "delphi_to_csharp", label: "Delphi → C#",        from: "Delphi", to: "C#",        category: "legacy", description: "Convert Delphi/Object Pascal to C# with .NET patterns" },
  { id: "delphi_to_rust",   label: "Delphi → Rust",      from: "Delphi", to: "Rust",      category: "legacy", description: "Convert Delphi to Rust with ownership-safe equivalents" },
  { id: "perl_to_python",   label: "Perl → Python",      from: "Perl",   to: "Python",    category: "legacy", description: "Convert Perl scripts to idiomatic Python" },

  // .NET
  { id: "dotnet_upgrade",   label: ".NET Upgrade",        from: ".NET Framework", to: ".NET 8+", category: "dotnet", description: "Upgrade .NET Framework to .NET 8+: SDK-style csproj, new APIs, nullable refs" },
  { id: "vbnet_to_csharp",  label: "VB.NET → C#",        from: "VB.NET", to: "C#",        category: "dotnet", description: "Convert VB.NET to equivalent C# with modern syntax" },
  { id: "csharp_to_rust",   label: "C# → Rust",          from: "C#",     to: "Rust",      category: "dotnet", description: "Convert C# to Rust with ownership model, Result types, traits" },
  { id: "packages_config_to_packageref", label: "packages.config → PackageRef", from: "NuGet", to: "SDK-style", category: "dotnet", description: "Migrate NuGet packages.config to PackageReference" },

  // Java
  { id: "java_to_kotlin",   label: "Java → Kotlin",      from: "Java",   to: "Kotlin",    category: "java", description: "Convert Java to idiomatic Kotlin with null safety, data classes" },
  { id: "java_to_rust",     label: "Java → Rust",         from: "Java",   to: "Rust",      category: "java", description: "Convert Java to Rust with ownership, enums, trait implementations" },
  { id: "java8_to_java21",  label: "Java 8 → Java 21",    from: "Java 8", to: "Java 21",   category: "java", description: "Modernize: records, sealed classes, pattern matching, virtual threads" },
  { id: "junit4_to_junit5", label: "JUnit 4 → JUnit 5",   from: "JUnit 4", to: "JUnit 5", category: "java", description: "Migrate annotations, assertions, lifecycle to JUnit 5 Jupiter" },
  { id: "kotlin_to_rust",   label: "Kotlin → Rust",       from: "Kotlin", to: "Rust",     category: "java", description: "Convert Kotlin to Rust preserving null safety via Option" },

  // JavaScript/TypeScript
  { id: "commonjs_to_esm",         label: "CommonJS → ESM",       from: "CommonJS",  to: "ESM",        category: "javascript", description: "Convert require/module.exports to import/export" },
  { id: "javascript_to_typescript", label: "JS → TypeScript",      from: "JavaScript", to: "TypeScript", category: "javascript", description: "Add TypeScript types, interfaces, strict mode" },
  { id: "react_class_to_hooks",     label: "React Class → Hooks",  from: "Class", to: "Hooks", category: "javascript", description: "Convert class components to functional with hooks" },
  { id: "express_to_fastify",       label: "Express → Fastify",    from: "Express",   to: "Fastify",    category: "javascript", description: "Migrate Express routes/middleware to Fastify with schemas" },
  { id: "vue2_to_vue3",             label: "Vue 2 → Vue 3",        from: "Vue 2",     to: "Vue 3",      category: "javascript", description: "Options API → Composition API, lifecycle hooks, v-model" },

  // Python
  { id: "python2_to_python3", label: "Python 2 → 3",       from: "Python 2", to: "Python 3", category: "python", description: "Modernize print, unicode, iterators to Python 3" },
  { id: "flask_to_fastapi",   label: "Flask → FastAPI",     from: "Flask",    to: "FastAPI",  category: "python", description: "Convert Flask routes to FastAPI with async, Pydantic, auto-docs" },
  { id: "python_to_rust",     label: "Python → Rust",       from: "Python",   to: "Rust",     category: "python", description: "Convert Python to Rust with PyO3 bindings" },
  { id: "sync_to_async_python", label: "Sync → Async Python", from: "sync",  to: "asyncio",  category: "python", description: "Convert synchronous Python to async/await" },

  // Mobile
  { id: "objc_to_swift",    label: "Obj-C → Swift",       from: "Objective-C", to: "Swift",  category: "mobile", description: "Convert Obj-C to Swift with optionals, protocols, value types" },
  { id: "swift_to_kotlin",  label: "Swift → Kotlin",      from: "Swift",   to: "Kotlin",    category: "mobile", description: "Cross-platform: convert Swift to Kotlin KMP patterns" },

  // Systems
  { id: "c_to_rust",        label: "C → Rust",            from: "C",       to: "Rust",      category: "systems", description: "Pointers → references, malloc → Vec, error codes → Result" },
  { id: "cpp_to_rust",      label: "C++ → Rust",          from: "C++",     to: "Rust",      category: "systems", description: "RAII preserved, templates → generics, exceptions → Result" },
  { id: "go_to_rust",       label: "Go → Rust",           from: "Go",      to: "Rust",      category: "systems", description: "Goroutines → tokio, interfaces → traits, error handling" },

  // Web
  { id: "ruby_to_python",   label: "Ruby → Python",       from: "Ruby",    to: "Python",    category: "web", description: "Convert Ruby/Rails to Python/Django or FastAPI" },
  { id: "ruby_to_rust",     label: "Ruby → Rust",         from: "Ruby",    to: "Rust",      category: "web", description: "Convert Ruby to Rust with Actix-web or Axum" },
  { id: "php_to_python",    label: "PHP → Python",        from: "PHP",     to: "Python",    category: "web", description: "Convert PHP to Python Django/FastAPI" },
  { id: "php_to_typescript", label: "PHP → TypeScript",   from: "PHP",     to: "TypeScript", category: "web", description: "Convert PHP to Node.js TypeScript" },

  // Universal
  { id: "sql_dialect_convert",  label: "SQL Dialect Convert",   from: "SQL",  to: "SQL",     category: "universal", description: "Convert between MySQL↔PostgreSQL↔SQLite↔MSSQL↔Oracle" },
  { id: "add_type_annotations", label: "Add Type Annotations",  from: "untyped", to: "typed", category: "universal", description: "Add type hints to Python, JS, Ruby, or PHP" },
  { id: "modernize_syntax",     label: "Modernize Syntax",      from: "legacy", to: "modern", category: "universal", description: "Update to latest language idioms" },
];

const CATEGORY_LABELS: Record<string, string> = {
  legacy: "Legacy Migration",
  dotnet: ".NET Ecosystem",
  java: "Java / JVM",
  javascript: "JavaScript / TypeScript",
  python: "Python",
  mobile: "Mobile",
  systems: "Systems",
  web: "Web",
  universal: "Universal",
};

const CATEGORY_ORDER = ["legacy", "dotnet", "java", "javascript", "python", "mobile", "systems", "web", "universal"];

export function TransformPanel({ provider }: TransformPanelProps) {
  const [detectedIds, setDetectedIds] = useState<string[]>([]);
  const [selectedTransform, setSelectedTransform] = useState<string | null>(null);
  const [plan, setPlan] = useState<PlanResult | null>(null);
  const [execResult, setExecResult] = useState<ExecResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [planning, setPlanning] = useState(false);
  const [executing, setExecuting] = useState(false);
  const [error, setError] = useState("");
  const [search, setSearch] = useState("");
  const [showAll, setShowAll] = useState(false);
  const [selectedFiles, setSelectedFiles] = useState<Set<string>>(new Set());
  const [sourceCode, setSourceCode] = useState("");
  const [transformedCode, setTransformedCode] = useState("");
  const [pasteMode, setPasteMode] = useState(false);
  const [pasteTransforming, setPasteTransforming] = useState(false);
  const [progress, setProgress] = useState<{ current: number; total: number; file: string } | null>(null);
  const mountedRef = useRef(true);

  useEffect(() => {
    mountedRef.current = true;
    return () => { mountedRef.current = false; };
  }, []);

  // Listen for per-file progress from the backend
  useEffect(() => {
    const unlisten = listen<{ current: number; total: number; file: string }>("transform:progress", (e) => {
      if (mountedRef.current) {
        setProgress(e.payload.file === "done" ? null : e.payload);
      }
    });
    return () => { unlisten.then(f => f()); };
  }, []);

  // Detect transforms for workspace
  useEffect(() => {
    (async () => {
      try {
        setLoading(true);
        const wp = localStorage.getItem("vibeui_workspace") || "";
        if (wp) {
          const detected = await invoke<string[]>("detect_transform", { workspace: wp });
          if (mountedRef.current) setDetectedIds(detected);
        }
      } catch { /* no workspace */ }
      if (mountedRef.current) setLoading(false);
    })();
  }, []);

  const handlePlan = useCallback(async (transformId: string) => {
    setSelectedTransform(transformId);
    setPlan(null);
    setExecResult(null);
    setError("");
    setPlanning(true);
    try {
      const result = await invoke<PlanResult>("plan_transform", { transformType: transformId });
      if (mountedRef.current) {
        setPlan(result);
        setSelectedFiles(new Set(result.files.map(f => f.file)));
      }
    } catch (e: any) {
      if (mountedRef.current) setError(typeof e === "string" ? e : e?.message || "Planning failed");
    } finally {
      if (mountedRef.current) setPlanning(false);
    }
  }, []);

  const handleExecute = useCallback(async () => {
    if (!selectedTransform || selectedFiles.size === 0) return;
    setExecuting(true);
    setExecResult(null);
    setError("");
    setProgress(null);
    try {
      const result = await invoke<ExecResult>("execute_transform", {
        transformType: selectedTransform,
        files: Array.from(selectedFiles),
      });
      if (mountedRef.current) setExecResult(result);
    } catch (e: any) {
      if (mountedRef.current) setError(typeof e === "string" ? e : e?.message || "Transform failed");
    } finally {
      if (mountedRef.current) setExecuting(false);
    }
  }, [selectedTransform, selectedFiles]);

  const handlePasteTransform = useCallback(async () => {
    if (!selectedTransform || !sourceCode.trim()) return;
    setPasteTransforming(true);
    setTransformedCode("");
    const def = ALL_TRANSFORMS.find(t => t.id === selectedTransform);
    try {
      const result = await invoke<string>("execute_chat", {
        provider,
        message: `You are a code transformation expert. Transform the following code using the "${def?.label || selectedTransform}" transformation (${def?.description || ""}).\n\nReturn ONLY the transformed code, no explanations.\n\nSource code:\n\`\`\`\n${sourceCode}\n\`\`\``,
      }).catch((e: unknown) => String(e));
      if (mountedRef.current) {
        let code = result.trim();
        if (code.startsWith("```")) {
          const s = code.indexOf("\n") + 1;
          const e2 = code.lastIndexOf("```");
          code = code.slice(s, e2 > s ? e2 : undefined).trim();
        }
        setTransformedCode(code);
      }
    } catch (e: any) {
      if (mountedRef.current) setTransformedCode(`Error: ${e}`);
    } finally {
      if (mountedRef.current) setPasteTransforming(false);
    }
  }, [selectedTransform, sourceCode, provider]);

  const toggleFile = (file: string) => {
    setSelectedFiles(prev => {
      const next = new Set(prev);
      if (next.has(file)) next.delete(file); else next.add(file);
      return next;
    });
  };

  // Filter transforms
  const searchLower = search.toLowerCase();
  const visibleTransforms = ALL_TRANSFORMS.filter(t => {
    if (search) return t.label.toLowerCase().includes(searchLower) || t.from.toLowerCase().includes(searchLower) || t.to.toLowerCase().includes(searchLower) || t.description.toLowerCase().includes(searchLower);
    if (!showAll && detectedIds.length > 0 && !detectedIds.includes(t.id)) return false;
    return true;
  });

  // Group by category
  const grouped = CATEGORY_ORDER.map(cat => ({
    category: cat,
    label: CATEGORY_LABELS[cat] || cat,
    transforms: visibleTransforms.filter(t => t.category === cat),
  })).filter(g => g.transforms.length > 0);

  const selectedDef = ALL_TRANSFORMS.find(t => t.id === selectedTransform);

  return (
    <div style={{ display: "flex", height: "100%", overflow: "hidden" }}>
      {/* ── Left: Transform picker ──────────────────────────────── */}
      <div style={{ width: 310, flexShrink: 0, borderRight: "1px solid var(--border-color)", display: "flex", flexDirection: "column", overflow: "hidden" }}>
        <div style={{ padding: "12px 14px", borderBottom: "1px solid var(--border-color)", flexShrink: 0 }}>
          <div style={{ fontWeight: 600, fontSize: 15, marginBottom: 8 }}>Code Transforms</div>
          <input
            value={search}
            onChange={e => setSearch(e.target.value)}
            placeholder="Search transforms... (e.g. COBOL, .NET, Rust)"
            style={{ width: "100%", background: "var(--bg-tertiary)", border: "1px solid var(--border-color)", borderRadius: 6, color: "inherit", padding: "6px 10px", fontSize: 12, boxSizing: "border-box" }}
          />
          <div style={{ display: "flex", alignItems: "center", gap: 8, marginTop: 8 }}>
            <label style={{ fontSize: 11, color: "var(--text-secondary)", cursor: "pointer", display: "flex", alignItems: "center", gap: 4 }}>
              <input type="checkbox" checked={showAll} onChange={e => setShowAll(e.target.checked)} />
              Show all ({ALL_TRANSFORMS.length})
            </label>
            {detectedIds.length > 0 && !showAll && !search && (
              <span style={{ fontSize: 11, color: "var(--text-success)" }}>{detectedIds.length} detected</span>
            )}
          </div>
          <div style={{ marginTop: 8 }}>
            <button
              onClick={() => setPasteMode(!pasteMode)}
              style={{
                width: "100%", padding: "6px 0",
                background: pasteMode ? "var(--accent-color)" : "var(--bg-tertiary)",
                border: "1px solid var(--border-color)", borderRadius: 6,
                color: "var(--text-primary)", cursor: "pointer", fontSize: 12, fontWeight: 500,
              }}
            >
              {pasteMode ? "Switch to File Mode" : "Paste & Transform"}
            </button>
          </div>
        </div>

        <div style={{ flex: 1, overflow: "auto", padding: "8px 14px" }}>
          {loading ? (
            <div style={{ color: "var(--text-secondary)", fontSize: 12, padding: 12 }}>Scanning workspace...</div>
          ) : grouped.length === 0 ? (
            <div style={{ color: "var(--text-secondary)", fontSize: 12, padding: 12 }}>
              No matching transforms found.{!showAll && " Enable \"Show all\" to see all available transforms."}
            </div>
          ) : (
            grouped.map(group => (
              <div key={group.category} style={{ marginBottom: 16 }}>
                <div style={{ fontSize: 11, fontWeight: 600, color: "var(--text-secondary)", textTransform: "uppercase", letterSpacing: "0.5px", marginBottom: 6 }}>
                  {group.label}
                </div>
                <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
                  {group.transforms.map(t => {
                    const active = selectedTransform === t.id;
                    return (
                      <div
                        key={t.id}
                        onClick={() => { setSelectedTransform(t.id); setPlan(null); setExecResult(null); setError(""); setTransformedCode(""); }}
                        style={{
                          padding: "10px 14px",
                          background: active ? "var(--accent-color)" : "var(--bg-secondary)",
                          border: `1px solid ${active ? "var(--accent-color)" : "var(--border-color)"}`,
                          borderRadius: 8, cursor: "pointer",
                          display: "flex", flexDirection: "column", gap: 4,
                        }}
                      >
                        <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
                          <span style={{ fontSize: 13, fontWeight: 600 }}>{t.label}</span>
                          {detectedIds.includes(t.id) && (
                            <span style={{ fontSize: 9, background: "var(--success-color)", color: "var(--bg-primary)", borderRadius: 4, padding: "1px 5px", fontWeight: 600 }}>DETECTED</span>
                          )}
                        </div>
                        <div style={{ fontSize: 11, color: active ? "rgba(255,255,255,0.8)" : "var(--text-secondary)", lineHeight: 1.4 }}>
                          {t.description}
                        </div>
                      </div>
                    );
                  })}
                </div>
              </div>
            ))
          )}
        </div>
      </div>

      {/* ── Right: Plan / Execute / Paste ────────────────────────── */}
      <div style={{ flex: 1, overflow: "auto", padding: 16 }}>
        {!selectedTransform ? (
          <div style={{ display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center", height: "100%", color: "var(--text-secondary)" }}>
            <div style={{ fontSize: 40, marginBottom: 12, opacity: 0.3 }}>&#8644;</div>
            <div style={{ fontSize: 15, fontWeight: 500 }}>Select a transform</div>
            <div style={{ fontSize: 12, marginTop: 4 }}>Choose a code transformation from the left panel</div>
          </div>
        ) : pasteMode ? (
          /* ── Paste & Transform ─────────────────────────────── */
          <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
            <div style={{ fontWeight: 600, fontSize: 15, marginBottom: 4 }}>{selectedDef?.label}</div>
            <div style={{ fontSize: 12, color: "var(--text-secondary)", marginBottom: 12 }}>{selectedDef?.description}</div>

            <div style={{ display: "flex", gap: 12, flex: 1, minHeight: 0 }}>
              {/* Source */}
              <div style={{ flex: 1, display: "flex", flexDirection: "column", minWidth: 0 }}>
                <div style={{ fontSize: 12, fontWeight: 600, marginBottom: 4, color: "var(--text-secondary)" }}>
                  Source ({selectedDef?.from})
                </div>
                <textarea
                  value={sourceCode}
                  onChange={e => setSourceCode(e.target.value)}
                  placeholder={`Paste your ${selectedDef?.from} code here...`}
                  spellCheck={false}
                  style={{ flex: 1, background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: 8, color: "var(--text-primary)", padding: 12, fontSize: 12, fontFamily: "var(--font-mono)", resize: "none", boxSizing: "border-box" }}
                />
              </div>

              {/* Arrow + Button */}
              <div style={{ display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center", gap: 8, flexShrink: 0 }}>
                <button
                  onClick={handlePasteTransform}
                  disabled={pasteTransforming || !sourceCode.trim()}
                  style={{ background: "var(--accent-color)", border: "none", borderRadius: 8, padding: "10px 16px", color: "var(--text-primary)", cursor: "pointer", fontWeight: 600, fontSize: 13, opacity: pasteTransforming || !sourceCode.trim() ? 0.5 : 1, whiteSpace: "nowrap" }}
                >
                  {pasteTransforming ? "..." : "Transform \u2192"}
                </button>
              </div>

              {/* Output */}
              <div style={{ flex: 1, display: "flex", flexDirection: "column", minWidth: 0 }}>
                <div style={{ fontSize: 12, fontWeight: 600, marginBottom: 4, color: "var(--text-secondary)" }}>
                  Output ({selectedDef?.to})
                </div>
                <pre style={{ flex: 1, background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: 8, color: "var(--text-success)", padding: 12, fontSize: 12, fontFamily: "var(--font-mono)", overflow: "auto", margin: 0, whiteSpace: "pre" }}>
                  {transformedCode || (pasteTransforming ? "Transforming..." : "Transformed code will appear here")}
                </pre>
              </div>
            </div>

            {transformedCode && (
              <div style={{ marginTop: 12, display: "flex", gap: 8, flexShrink: 0 }}>
                <button
                  onClick={() => navigator.clipboard.writeText(transformedCode)}
                  style={{ background: "var(--bg-tertiary)", border: "1px solid var(--border-color)", borderRadius: 6, padding: "6px 14px", color: "var(--text-primary)", cursor: "pointer", fontSize: 12 }}
                >
                  Copy Output
                </button>
                <button
                  onClick={() => { setSourceCode(transformedCode); setTransformedCode(""); }}
                  style={{ background: "var(--bg-tertiary)", border: "1px solid var(--border-color)", borderRadius: 6, padding: "6px 14px", color: "var(--text-primary)", cursor: "pointer", fontSize: 12 }}
                >
                  Use as Input (chain transforms)
                </button>
              </div>
            )}
          </div>
        ) : (
          /* ── File Mode ─────────────────────────────────────── */
          <div>
            <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: 12 }}>
              <div>
                <div style={{ fontWeight: 600, fontSize: 15 }}>{selectedDef?.label}</div>
                <div style={{ fontSize: 12, color: "var(--text-secondary)", marginTop: 2 }}>{selectedDef?.description}</div>
              </div>
              {!plan && (
                <button
                  onClick={() => handlePlan(selectedTransform)}
                  disabled={planning}
                  style={{ background: "var(--accent-color)", border: "none", borderRadius: 6, padding: "8px 20px", color: "var(--text-primary)", cursor: "pointer", fontWeight: 600, fontSize: 13, opacity: planning ? 0.5 : 1, flexShrink: 0 }}
                >
                  {planning ? "Planning..." : "Analyze & Plan"}
                </button>
              )}
            </div>

            {error && (
              <div style={{ padding: 12, background: "var(--bg-secondary)", border: "1px solid var(--error-color)", borderRadius: 8, color: "var(--error-color)", fontSize: 12, marginBottom: 12 }}>
                {error}
              </div>
            )}

            {planning && (
              <div style={{ padding: 24, textAlign: "center", color: "var(--text-secondary)" }}>
                <div style={{ fontSize: 14, marginBottom: 4 }}>Analyzing files...</div>
                <div style={{ fontSize: 12 }}>The AI is scanning your workspace and building a transformation plan</div>
              </div>
            )}

            {plan && (
              <div>
                <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: 10 }}>
                  <div style={{ fontSize: 13, fontWeight: 600 }}>
                    {plan.total_files} file{plan.total_files !== 1 ? "s" : ""} to transform
                    <span style={{ fontWeight: 400, color: "var(--text-secondary)", marginLeft: 8 }}>
                      ({selectedFiles.size} selected)
                    </span>
                  </div>
                  <div style={{ display: "flex", gap: 6 }}>
                    <button onClick={() => plan && setSelectedFiles(new Set(plan.files.map(f => f.file)))} style={{ background: "none", border: "none", color: "var(--accent-color)", cursor: "pointer", fontSize: 11, fontWeight: 600 }}>All</button>
                    <button onClick={() => setSelectedFiles(new Set())} style={{ background: "none", border: "none", color: "var(--text-secondary)", cursor: "pointer", fontSize: 11 }}>None</button>
                  </div>
                </div>

                <div style={{ maxHeight: 300, overflow: "auto", border: "1px solid var(--border-color)", borderRadius: 8, marginBottom: 12 }}>
                  {plan.files.map((item, i) => (
                    <div
                      key={item.file}
                      onClick={() => toggleFile(item.file)}
                      style={{
                        display: "flex", alignItems: "flex-start", gap: 8,
                        padding: "8px 12px",
                        borderBottom: i < plan.files.length - 1 ? "1px solid var(--border-color)" : "none",
                        cursor: "pointer",
                        background: selectedFiles.has(item.file) ? "var(--bg-secondary)" : "transparent",
                      }}
                    >
                      <input type="checkbox" checked={selectedFiles.has(item.file)} onChange={() => toggleFile(item.file)} style={{ marginTop: 2, flexShrink: 0 }} />
                      <div style={{ flex: 1, minWidth: 0 }}>
                        <div style={{ fontSize: 12, fontFamily: "var(--font-mono)", fontWeight: 500, wordBreak: "break-all" }}>{item.file}</div>
                        <div style={{ fontSize: 11, color: "var(--text-secondary)", marginTop: 2 }}>{item.description}</div>
                      </div>
                      <div style={{ fontSize: 11, color: "var(--text-secondary)", flexShrink: 0 }}>~{item.estimated_changes}</div>
                    </div>
                  ))}
                </div>

                <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
                  <button
                    onClick={handleExecute}
                    disabled={executing || selectedFiles.size === 0}
                    style={{ background: "var(--accent-color)", border: "none", borderRadius: 6, padding: "10px 24px", color: "var(--btn-primary-fg)", cursor: "pointer", fontWeight: 600, fontSize: 14, opacity: executing || selectedFiles.size === 0 ? 0.5 : 1 }}
                  >
                    {executing
                      ? (progress ? `Transforming ${progress.current}/${progress.total}...` : "Transforming...")
                      : `Transform ${selectedFiles.size} File${selectedFiles.size !== 1 ? "s" : ""}`}
                  </button>
                  <button
                    onClick={() => { setPlan(null); setExecResult(null); }}
                    style={{ background: "var(--bg-tertiary)", border: "1px solid var(--border-color)", borderRadius: 6, padding: "10px 16px", color: "var(--text-primary)", cursor: "pointer", fontSize: 13 }}
                  >
                    Re-plan
                  </button>
                </div>
                {executing && progress && (
                  <div style={{ marginTop: 8 }}>
                    <div style={{ fontSize: 11, color: "var(--text-secondary)", marginBottom: 4 }}>
                      Processing: {progress.file}
                    </div>
                    <div style={{ height: 4, background: "var(--bg-tertiary)", borderRadius: 2, overflow: "hidden" }}>
                      <div style={{
                        width: `${Math.round((progress.current / progress.total) * 100)}%`,
                        height: "100%", background: "var(--accent-color)", borderRadius: 2,
                        transition: "width 0.3s ease",
                      }} />
                    </div>
                  </div>
                )}
              </div>
            )}

            {execResult && (
              <div style={{ marginTop: 16, padding: 16, background: "var(--bg-secondary)", borderRadius: 8, border: "1px solid var(--success-color)" }}>
                <div style={{ fontWeight: 600, fontSize: 14, color: "var(--text-success)", marginBottom: 4 }}>Transform Complete</div>
                <div style={{ fontSize: 13 }}>{execResult.summary}</div>
                <div style={{ fontSize: 12, color: "var(--text-secondary)", marginTop: 4 }}>
                  {execResult.files_modified} file{execResult.files_modified !== 1 ? "s" : ""} modified. Review changes in the Git panel.
                </div>
              </div>
            )}

            {!plan && !planning && !error && (
              <div style={{ marginTop: 16, padding: 16, background: "var(--bg-secondary)", borderRadius: 8, border: "1px solid var(--border-color)" }}>
                <div style={{ fontSize: 13, fontWeight: 600, marginBottom: 8 }}>How it works</div>
                <ol style={{ fontSize: 12, color: "var(--text-secondary)", margin: 0, paddingLeft: 18, lineHeight: 1.8 }}>
                  <li>Click <strong>Analyze & Plan</strong> to scan your workspace for matching files</li>
                  <li>Review the AI-generated transformation plan and select files</li>
                  <li>Click <strong>Transform</strong> to apply changes in-place</li>
                  <li>Review diffs in the Git panel before committing</li>
                </ol>
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
