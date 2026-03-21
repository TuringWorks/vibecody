import React, { useState, useMemo } from "react";
import { Mail } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";

type AuthProvider = "github" | "google" | "email" | "jwt" | "saml" | "ldap";

interface AuthConfig {
  auth_provider: AuthProvider;
  framework: string;
  include_middleware: boolean;
  include_tests: boolean;
}

interface FrameworkInfo {
  value: string;
  label: string;
  lang: string;
}

// ── All frameworks from TuringWorks/FrameworkBenchmarks ──────────────────────

const FRAMEWORKS: FrameworkInfo[] = [
  // Go
  { value: "gin", label: "Gin", lang: "Go" },
  { value: "fiber", label: "Fiber", lang: "Go" },
  { value: "echo", label: "Echo", lang: "Go" },
  { value: "chi", label: "Chi", lang: "Go" },
  { value: "go-std", label: "Go net/http", lang: "Go" },
  { value: "fasthttp", label: "FastHTTP", lang: "Go" },
  { value: "hertz", label: "Hertz", lang: "Go" },
  { value: "goframe", label: "GoFrame", lang: "Go" },
  { value: "goravel", label: "Goravel", lang: "Go" },
  // Java
  { value: "spring", label: "Spring Boot", lang: "Java" },
  { value: "spring-webflux", label: "Spring WebFlux", lang: "Java" },
  { value: "quarkus", label: "Quarkus", lang: "Java" },
  { value: "micronaut", label: "Micronaut", lang: "Java" },
  { value: "vertx", label: "Vert.x", lang: "Java" },
  { value: "javalin", label: "Javalin", lang: "Java" },
  { value: "helidon", label: "Helidon", lang: "Java" },
  { value: "dropwizard", label: "Dropwizard", lang: "Java" },
  { value: "jetty", label: "Jetty", lang: "Java" },
  { value: "undertow", label: "Undertow", lang: "Java" },
  { value: "netty", label: "Netty", lang: "Java" },
  { value: "play2-java", label: "Play (Java)", lang: "Java" },
  // Kotlin
  { value: "ktor", label: "Ktor", lang: "Kotlin" },
  { value: "http4k", label: "http4k", lang: "Kotlin" },
  { value: "hexagon", label: "Hexagon", lang: "Kotlin" },
  // C# / .NET
  { value: "aspnet", label: "ASP.NET Core", lang: "C#" },
  { value: "fastendpoints", label: "FastEndpoints", lang: "C#" },
  { value: "carter", label: "Carter", lang: "C#" },
  { value: "servicestack", label: "ServiceStack", lang: "C#" },
  // TypeScript / JavaScript
  { value: "nextjs", label: "Next.js", lang: "TypeScript" },
  { value: "express", label: "Express.js", lang: "JavaScript" },
  { value: "fastify", label: "Fastify", lang: "TypeScript" },
  { value: "nest", label: "NestJS", lang: "TypeScript" },
  { value: "hono", label: "Hono", lang: "TypeScript" },
  { value: "elysia", label: "Elysia (Bun)", lang: "TypeScript" },
  { value: "koa", label: "Koa", lang: "JavaScript" },
  { value: "hapi", label: "Hapi", lang: "JavaScript" },
  { value: "oak", label: "Oak (Deno)", lang: "TypeScript" },
  { value: "supabase", label: "Supabase Auth", lang: "TypeScript" },
  // Python
  { value: "fastapi", label: "FastAPI", lang: "Python" },
  { value: "django", label: "Django", lang: "Python" },
  { value: "flask", label: "Flask", lang: "Python" },
  { value: "starlette", label: "Starlette", lang: "Python" },
  { value: "litestar", label: "Litestar", lang: "Python" },
  { value: "sanic", label: "Sanic", lang: "Python" },
  { value: "tornado", label: "Tornado", lang: "Python" },
  { value: "falcon", label: "Falcon", lang: "Python" },
  { value: "robyn", label: "Robyn", lang: "Python" },
  // Rust
  { value: "axum", label: "Axum", lang: "Rust" },
  { value: "actix", label: "Actix Web", lang: "Rust" },
  { value: "rocket", label: "Rocket", lang: "Rust" },
  { value: "warp-rust", label: "Warp", lang: "Rust" },
  { value: "salvo", label: "Salvo", lang: "Rust" },
  { value: "tide", label: "Tide", lang: "Rust" },
  // Ruby
  { value: "rails", label: "Rails", lang: "Ruby" },
  { value: "sinatra", label: "Sinatra", lang: "Ruby" },
  { value: "hanami", label: "Hanami", lang: "Ruby" },
  { value: "grape", label: "Grape", lang: "Ruby" },
  { value: "rage", label: "Rage", lang: "Ruby" },
  // PHP
  { value: "laravel", label: "Laravel", lang: "PHP" },
  { value: "symfony", label: "Symfony", lang: "PHP" },
  { value: "slim", label: "Slim", lang: "PHP" },
  { value: "cakephp", label: "CakePHP", lang: "PHP" },
  { value: "codeigniter", label: "CodeIgniter", lang: "PHP" },
  { value: "yii2", label: "Yii2", lang: "PHP" },
  { value: "hyperf", label: "Hyperf", lang: "PHP" },
  // Elixir
  { value: "phoenix", label: "Phoenix", lang: "Elixir" },
  { value: "plug", label: "Plug", lang: "Elixir" },
  // Scala
  { value: "play2-scala", label: "Play (Scala)", lang: "Scala" },
  { value: "akka-http", label: "Akka HTTP", lang: "Scala" },
  { value: "http4s", label: "http4s", lang: "Scala" },
  { value: "zio-http", label: "ZIO HTTP", lang: "Scala" },
  // Swift
  { value: "vapor", label: "Vapor", lang: "Swift" },
  { value: "hummingbird", label: "Hummingbird", lang: "Swift" },
  // Dart
  { value: "dart_frog", label: "Dart Frog", lang: "Dart" },
  { value: "shelf", label: "Shelf", lang: "Dart" },
  // Clojure
  { value: "ring", label: "Ring", lang: "Clojure" },
  // Haskell
  { value: "servant", label: "Servant", lang: "Haskell" },
  // Crystal
  { value: "kemal", label: "Kemal", lang: "Crystal" },
  // Nim
  { value: "jester", label: "Jester", lang: "Nim" },
  // Zig
  { value: "zap", label: "Zap", lang: "Zig" },
];

// Unique languages sorted by popularity
const LANGUAGES = [...new Set(FRAMEWORKS.map(f => f.lang))];

export function AuthPanel({ workspacePath, provider }: { workspacePath: string | null; provider: string }) {
  const [config, setConfig] = useState<AuthConfig>({
    auth_provider: "github",
    framework: "nextjs",
    include_middleware: true,
    include_tests: true,
  });
  const [generatedCode, setGeneratedCode] = useState("");
  const [targetPath, setTargetPath] = useState("src/auth");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);
  const [fwSearch, setFwSearch] = useState("");
  const [langFilter, setLangFilter] = useState("All");

  if (!workspacePath) {
    return <div style={{ padding: 24, color: "var(--text-secondary)", textAlign: "center" }}>Open a workspace folder to generate auth scaffolding.</div>;
  }

  const filteredFrameworks = useMemo(() => {
    let list = FRAMEWORKS;
    if (langFilter !== "All") list = list.filter(f => f.lang === langFilter);
    if (fwSearch.trim()) {
      const q = fwSearch.toLowerCase();
      list = list.filter(f => f.label.toLowerCase().includes(q) || f.lang.toLowerCase().includes(q) || f.value.toLowerCase().includes(q));
    }
    return list;
  }, [fwSearch, langFilter]);

  const selectedFw = FRAMEWORKS.find(f => f.value === config.framework);

  const generate = async () => {
    setLoading(true); setError(null); setSaved(false);
    try {
      const code = await invoke<string>("generate_auth_scaffold", {
        workspacePath, provider,
        authProvider: config.auth_provider,
        framework: config.framework,
        includeMiddleware: config.include_middleware,
        includeTests: config.include_tests,
      });
      setGeneratedCode(code);
    } catch (e) { setError(String(e)); }
    finally { setLoading(false); }
  };

  const saveToWorkspace = async () => {
    if (!generatedCode) return;
    setLoading(true);
    try {
      await invoke("write_auth_scaffold", { workspacePath, targetPath, code: generatedCode, framework: config.framework });
      setSaved(true);
    } catch (e) { setError(String(e)); }
    finally { setLoading(false); }
  };

  const AUTH_PROVIDERS: { value: AuthProvider; label: string; icon: React.ReactNode }[] = [
    { value: "github", label: "GitHub OAuth", icon: null },
    { value: "google", label: "Google OAuth", icon: null },
    { value: "email", label: "Email + Password", icon: <Mail size={14} strokeWidth={1.5} /> },
    { value: "jwt", label: "JWT / Bearer", icon: null },
    { value: "saml", label: "SAML SSO", icon: null },
    { value: "ldap", label: "LDAP / AD", icon: null },
  ];

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", background: "var(--bg-primary)", color: "var(--text-primary)", fontSize: 13 }}>
      {/* Header */}
      <div style={{ padding: "10px 12px", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)" }}>
        <span style={{ fontSize: 14, fontWeight: 600 }}>Auth Scaffolding</span>
        <span style={{ fontSize: 11, color: "var(--text-secondary)", marginLeft: 8 }}>
          {FRAMEWORKS.length} frameworks across {LANGUAGES.length} languages
        </span>
      </div>

      <div style={{ flex: 1, overflow: "auto", padding: 12, display: "flex", flexDirection: "column", gap: 12 }}>
        {/* Auth Provider */}
        <div>
          <div style={labelStyle}>Auth Provider</div>
          <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
            {AUTH_PROVIDERS.map(p => (
              <button key={p.value} onClick={() => setConfig(c => ({ ...c, auth_provider: p.value }))}
                style={chipStyle(config.auth_provider === p.value)}>
                {p.icon} {p.label}
              </button>
            ))}
          </div>
        </div>

        {/* Framework Selection */}
        <div>
          <div style={labelStyle}>
            Framework
            {selectedFw && <span style={{ marginLeft: 8, color: "var(--accent-color)", fontWeight: 600 }}>{selectedFw.label} ({selectedFw.lang})</span>}
          </div>

          {/* Search + Language filter row */}
          <div style={{ display: "flex", gap: 6, marginBottom: 8 }}>
            <input
              type="text" placeholder="Search frameworks..."
              value={fwSearch} onChange={e => setFwSearch(e.target.value)}
              style={{ ...inputStyle, flex: 1 }}
            />
            <select value={langFilter} onChange={e => setLangFilter(e.target.value)} style={{ ...inputStyle, width: "auto", cursor: "pointer" }}>
              <option value="All">All Languages</option>
              {LANGUAGES.map(l => <option key={l} value={l}>{l}</option>)}
            </select>
          </div>

          {/* Framework grid */}
          <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(140px, 1fr))", gap: 4, maxHeight: 200, overflowY: "auto", border: "1px solid var(--border-color)", borderRadius: 4, padding: 6, background: "var(--bg-secondary)" }}>
            {filteredFrameworks.map(f => (
              <button key={f.value} onClick={() => setConfig(c => ({ ...c, framework: f.value }))}
                style={{
                  padding: "5px 8px", fontSize: 11, border: `1px solid ${config.framework === f.value ? "var(--accent-color)" : "var(--border-color)"}`,
                  borderRadius: 4, cursor: "pointer", textAlign: "left",
                  background: config.framework === f.value ? "rgba(0,122,204,0.15)" : "transparent",
                  color: config.framework === f.value ? "var(--accent-color)" : "var(--text-primary)",
                  fontWeight: config.framework === f.value ? 600 : 400,
                }}>
                {f.label}
                <span style={{ fontSize: 9, opacity: 0.6, marginLeft: 4 }}>{f.lang}</span>
              </button>
            ))}
            {filteredFrameworks.length === 0 && (
              <div style={{ gridColumn: "1 / -1", padding: 8, color: "var(--text-secondary)", textAlign: "center", fontSize: 12 }}>No frameworks match "{fwSearch}"</div>
            )}
          </div>
        </div>

        {/* Options */}
        <div style={{ display: "flex", gap: 12, alignItems: "center" }}>
          <label style={{ display: "flex", alignItems: "center", gap: 6, cursor: "pointer", fontSize: 12 }}>
            <input type="checkbox" checked={config.include_middleware} onChange={e => setConfig(c => ({ ...c, include_middleware: e.target.checked }))} />
            Auth middleware
          </label>
          <label style={{ display: "flex", alignItems: "center", gap: 6, cursor: "pointer", fontSize: 12 }}>
            <input type="checkbox" checked={config.include_tests} onChange={e => setConfig(c => ({ ...c, include_tests: e.target.checked }))} />
            Tests
          </label>
        </div>

        {/* Generate */}
        <button onClick={generate} disabled={loading}
          style={{ padding: "8px 16px", background: "var(--accent-color)", color: "var(--btn-primary-fg, #fff)", border: "none", borderRadius: 4, cursor: "pointer", fontSize: 13, fontWeight: 600 }}>
          {loading ? "Generating..." : `Generate Auth for ${selectedFw?.label ?? config.framework}`}
        </button>

        {error && <div style={{ color: "var(--error-color)", fontSize: 12, background: "rgba(244,67,54,0.1)", padding: 8, borderRadius: 4 }}>{error}</div>}

        {generatedCode && (
          <>
            <div>
              <div style={labelStyle}>Preview</div>
              <pre style={{ background: "var(--bg-secondary)", padding: 10, borderRadius: 4, fontSize: 11, fontFamily: "var(--font-mono)", whiteSpace: "pre", overflow: "auto", maxHeight: 300, border: "1px solid var(--border-color)" }}>
                {generatedCode.slice(0, 3000)}{generatedCode.length > 3000 ? "\n... (truncated)" : ""}
              </pre>
            </div>
            <div>
              <div style={labelStyle}>Save to workspace</div>
              <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
                <input style={inputStyle} value={targetPath} onChange={e => setTargetPath(e.target.value)} />
                <button onClick={saveToWorkspace} disabled={loading}
                  style={{ padding: "8px 16px", background: "var(--accent-color)", color: "var(--btn-primary-fg, #fff)", border: "none", borderRadius: 4, cursor: "pointer", fontSize: 12, fontWeight: 600, whiteSpace: "nowrap" }}>
                  {saved ? "Saved" : "Save Files"}
                </button>
              </div>
              {saved && <div style={{ color: "var(--success-color)", fontSize: 12, marginTop: 4 }}>Files written to {workspacePath}/{targetPath}</div>}
            </div>
          </>
        )}
      </div>
    </div>
  );
}

const labelStyle: React.CSSProperties = { display: "block", marginBottom: 4, fontSize: 11, color: "var(--text-secondary)" };
const inputStyle: React.CSSProperties = { background: "var(--bg-tertiary)", border: "1px solid var(--border-color)", color: "var(--text-primary)", padding: "6px 8px", borderRadius: 4, fontSize: 12, boxSizing: "border-box", outline: "none" };
const chipStyle = (active: boolean): React.CSSProperties => ({
  padding: "4px 12px", border: `1px solid ${active ? "var(--accent-color)" : "var(--border-color)"}`,
  borderRadius: 12, cursor: "pointer", fontSize: 12, background: active ? "rgba(0,122,204,0.15)" : "transparent",
  color: active ? "var(--accent-color)" : "var(--text-secondary)", fontWeight: active ? 600 : 400,
});
