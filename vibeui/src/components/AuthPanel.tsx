import React, { useState, useMemo } from "react";
import { Mail } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";

type AuthProvider =
  // OAuth / Social
  | "github" | "google" | "apple" | "microsoft" | "facebook" | "twitter"
  | "discord" | "slack" | "linkedin" | "gitlab" | "bitbucket" | "spotify"
  | "twitch" | "dropbox" | "okta" | "auth0"
  // Enterprise SSO
  | "saml" | "ldap" | "oidc" | "kerberos" | "radius"
  // Token / Key
  | "jwt" | "api_key" | "oauth2_client_credentials" | "basic_auth" | "hawk" | "mtls"
  // Credential
  | "email" | "phone_otp" | "magic_link" | "passkey" | "totp_2fa"
  // Platform / BaaS
  | "supabase" | "firebase" | "clerk" | "auth0_universal" | "cognito"
  | "keycloak" | "fusionauth" | "stytch" | "workos" | "descope";

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

  const filteredFrameworks = useMemo(() => {
    let list = FRAMEWORKS;
    if (langFilter !== "All") list = list.filter(f => f.lang === langFilter);
    if (fwSearch.trim()) {
      const q = fwSearch.toLowerCase();
      list = list.filter(f => f.label.toLowerCase().includes(q) || f.lang.toLowerCase().includes(q) || f.value.toLowerCase().includes(q));
    }
    return list;
  }, [fwSearch, langFilter]);

  if (!workspacePath) {
    return <div style={{ padding: 24, color: "var(--text-secondary)", textAlign: "center" }}>Open a workspace folder to generate auth scaffolding.</div>;
  }

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

  const AUTH_CATEGORIES: { label: string; providers: { value: AuthProvider; label: string; icon: React.ReactNode }[] }[] = [
    {
      label: "OAuth / Social",
      providers: [
        { value: "github", label: "GitHub", icon: null },
        { value: "google", label: "Google", icon: null },
        { value: "apple", label: "Apple", icon: null },
        { value: "microsoft", label: "Microsoft", icon: null },
        { value: "facebook", label: "Facebook", icon: null },
        { value: "twitter", label: "Twitter / X", icon: null },
        { value: "discord", label: "Discord", icon: null },
        { value: "slack", label: "Slack", icon: null },
        { value: "linkedin", label: "LinkedIn", icon: null },
        { value: "gitlab", label: "GitLab", icon: null },
        { value: "bitbucket", label: "Bitbucket", icon: null },
        { value: "spotify", label: "Spotify", icon: null },
        { value: "twitch", label: "Twitch", icon: null },
        { value: "dropbox", label: "Dropbox", icon: null },
      ],
    },
    {
      label: "Enterprise SSO",
      providers: [
        { value: "saml", label: "SAML", icon: null },
        { value: "oidc", label: "OpenID Connect", icon: null },
        { value: "ldap", label: "LDAP / AD", icon: null },
        { value: "kerberos", label: "Kerberos", icon: null },
        { value: "radius", label: "RADIUS", icon: null },
        { value: "okta", label: "Okta", icon: null },
        { value: "auth0", label: "Auth0", icon: null },
      ],
    },
    {
      label: "Token / Key",
      providers: [
        { value: "jwt", label: "JWT Bearer", icon: null },
        { value: "api_key", label: "API Key", icon: null },
        { value: "oauth2_client_credentials", label: "OAuth2 Client Credentials", icon: null },
        { value: "basic_auth", label: "Basic Auth", icon: null },
        { value: "hawk", label: "Hawk", icon: null },
        { value: "mtls", label: "Mutual TLS (mTLS)", icon: null },
      ],
    },
    {
      label: "Credential / Passwordless",
      providers: [
        { value: "email", label: "Email + Password", icon: <Mail size={14} strokeWidth={1.5} /> },
        { value: "phone_otp", label: "Phone OTP (SMS)", icon: null },
        { value: "magic_link", label: "Magic Link", icon: null },
        { value: "passkey", label: "Passkey (WebAuthn)", icon: null },
        { value: "totp_2fa", label: "TOTP 2FA", icon: null },
      ],
    },
    {
      label: "Platform / BaaS",
      providers: [
        { value: "supabase", label: "Supabase Auth", icon: null },
        { value: "firebase", label: "Firebase Auth", icon: null },
        { value: "clerk", label: "Clerk", icon: null },
        { value: "cognito", label: "AWS Cognito", icon: null },
        { value: "keycloak", label: "Keycloak", icon: null },
        { value: "fusionauth", label: "FusionAuth", icon: null },
        { value: "auth0_universal", label: "Auth0 Universal Login", icon: null },
        { value: "stytch", label: "Stytch", icon: null },
        { value: "workos", label: "WorkOS", icon: null },
        { value: "descope", label: "Descope", icon: null },
      ],
    },
  ];

  const allProviders = AUTH_CATEGORIES.flatMap(c => c.providers);

  return (
    <div className="panel-container" style={{ display: "flex", flexDirection: "column", flex: 1, minHeight: 0, fontSize: "var(--font-size-md)" }}>
      {/* Header */}
      <div style={{ padding: "12px 12px", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)" }}>
        <span style={{ fontSize: "var(--font-size-lg)", fontWeight: 600 }}>Authorization Scaffolding</span>
        <span style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)", marginLeft: 8 }}>
          {allProviders.length} auth providers | {FRAMEWORKS.length} frameworks | {LANGUAGES.length} languages
        </span>
      </div>

      <div style={{ flex: 1, overflow: "auto", padding: 12, display: "flex", flexDirection: "column", gap: 12 }}>
        {/* Auth Provider — organized by category */}
        <div>
          <div style={labelStyle}>Auth Provider <span style={{ color: "var(--accent-color)", fontWeight: 600 }}>{allProviders.find(p => p.value === config.auth_provider)?.label}</span></div>
          <div style={{ maxHeight: 220, overflowY: "auto", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", padding: 8, background: "var(--bg-secondary)" }}>
            {AUTH_CATEGORIES.map(cat => (
              <div key={cat.label} style={{ marginBottom: 8 }}>
                <div style={{ fontSize: "var(--font-size-xs)", fontWeight: 600, color: "var(--text-secondary)", textTransform: "uppercase", marginBottom: 4, letterSpacing: 0.5 }}>{cat.label}</div>
                <div style={{ display: "flex", gap: 4, flexWrap: "wrap" }}>
                  {cat.providers.map(p => (
                    <button key={p.value} onClick={() => setConfig(c => ({ ...c, auth_provider: p.value }))}
                      style={chipStyle(config.auth_provider === p.value)}>
                      {p.icon} {p.label}
                    </button>
                  ))}
                </div>
              </div>
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
              className="panel-input" style={{ flex: 1 }}
            />
            <select value={langFilter} onChange={e => setLangFilter(e.target.value)} className="panel-select">
              <option value="All">All Languages</option>
              {LANGUAGES.map(l => <option key={l} value={l}>{l}</option>)}
            </select>
          </div>

          {/* Framework grid */}
          <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(140px, 1fr))", gap: 4, maxHeight: 200, overflowY: "auto", border: "1px solid var(--border-color)", borderRadius: "var(--radius-xs-plus)", padding: 6, background: "var(--bg-secondary)" }}>
            {filteredFrameworks.map(f => (
              <button key={f.value} onClick={() => setConfig(c => ({ ...c, framework: f.value }))}
                style={{
                  padding: "4px 8px", fontSize: "var(--font-size-sm)", border: `1px solid ${config.framework === f.value ? "var(--accent-color)" : "var(--border-color)"}`,
                  borderRadius: "var(--radius-xs-plus)", cursor: "pointer", textAlign: "left",
                  background: config.framework === f.value ? "rgba(0,122,204,0.15)" : "transparent",
                  color: config.framework === f.value ? "var(--accent-color)" : "var(--text-primary)",
                  fontWeight: config.framework === f.value ? 600 : 400,
                }}>
                {f.label}
                <span style={{ fontSize: 9, opacity: 0.6, marginLeft: 4 }}>{f.lang}</span>
              </button>
            ))}
            {filteredFrameworks.length === 0 && (
              <div style={{ gridColumn: "1 / -1", padding: 8, color: "var(--text-secondary)", textAlign: "center", fontSize: "var(--font-size-base)" }}>No frameworks match "{fwSearch}"</div>
            )}
          </div>
        </div>

        {/* Options */}
        <div style={{ display: "flex", gap: 12, alignItems: "center" }}>
          <label style={{ display: "flex", alignItems: "center", gap: 6, cursor: "pointer", fontSize: "var(--font-size-base)" }}>
            <input type="checkbox" checked={config.include_middleware} onChange={e => setConfig(c => ({ ...c, include_middleware: e.target.checked }))} />
            Auth middleware
          </label>
          <label style={{ display: "flex", alignItems: "center", gap: 6, cursor: "pointer", fontSize: "var(--font-size-base)" }}>
            <input type="checkbox" checked={config.include_tests} onChange={e => setConfig(c => ({ ...c, include_tests: e.target.checked }))} />
            Tests
          </label>
        </div>

        {/* Generate */}
        <button onClick={generate} disabled={loading}
          className="panel-btn panel-btn-primary">
          {loading ? "Generating..." : `Generate Auth for ${selectedFw?.label ?? config.framework}`}
        </button>

        {error && <div className="panel-error" style={{ fontSize: "var(--font-size-base)", padding: 8 }}>{error}</div>}

        {generatedCode && (
          <>
            <div>
              <div style={labelStyle}>Preview</div>
              <pre style={{ background: "var(--bg-secondary)", padding: 10, borderRadius: "var(--radius-xs-plus)", fontSize: "var(--font-size-sm)", fontFamily: "var(--font-mono)", whiteSpace: "pre", overflow: "auto", maxHeight: 300, border: "1px solid var(--border-color)" }}>
                {generatedCode.slice(0, 3000)}{generatedCode.length > 3000 ? "\n... (truncated)" : ""}
              </pre>
            </div>
            <div>
              <div style={labelStyle}>Save to workspace</div>
              <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
                <input className="panel-input" style={{ flex: 1 }} value={targetPath} onChange={e => setTargetPath(e.target.value)} />
                <button className="panel-btn" onClick={saveToWorkspace} disabled={loading}
                  style={{ padding: "8px 16px", background: "var(--accent-color)", color: "var(--btn-primary-fg, #fff)", border: "none", borderRadius: "var(--radius-xs-plus)", cursor: "pointer", fontSize: "var(--font-size-base)", fontWeight: 600, whiteSpace: "nowrap" }}>
                  {saved ? "Saved" : "Save Files"}
                </button>
              </div>
              {saved && <div style={{ color: "var(--success-color)", fontSize: "var(--font-size-base)", marginTop: 4 }}>Files written to {workspacePath}/{targetPath}</div>}
            </div>
          </>
        )}
      </div>
    </div>
  );
}

const labelStyle: React.CSSProperties = { display: "block", marginBottom: 4, fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" };
const chipStyle = (active: boolean): React.CSSProperties => ({
  padding: "4px 12px", border: `1px solid ${active ? "var(--accent-color)" : "var(--border-color)"}`,
  borderRadius: 12, cursor: "pointer", fontSize: "var(--font-size-base)", background: active ? "rgba(0,122,204,0.15)" : "transparent",
  color: active ? "var(--accent-color)" : "var(--text-secondary)", fontWeight: active ? 600 : 400,
});
