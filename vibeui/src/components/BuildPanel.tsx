/**
 * BuildPanel — Build, compile, and run projects with auto-detected build systems.
 *
 * Features:
 * - Auto-detects build system (Cargo, npm, Maven, Gradle, CMake, Make, Go, Python, .NET, etc.)
 * - Streams build output in real-time via Tauri events
 * - Parses compiler errors with clickable file:line links
 * - Build, Run, and Build & Run buttons
 * - Custom command override
 */
import { useState, useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

interface BuildPanelProps {
  workspacePath: string | null;
  onOpenFile?: (path: string, line?: number) => void;
}

interface BuildSystem {
  name: string;
  build_command: string;
  run_command: string;
  config_file: string;
  tool_available: boolean;
  install_hint: string;
  project_path: string;
}

interface BuildError {
  file: string | null;
  line: number | null;
  column: number | null;
  message: string;
  severity: string;
}

interface BuildResult {
  build_system: string;
  success: boolean;
  exit_code: number;
  duration_ms: number;
  errors: BuildError[];
  output: string;
}

type Status = "idle" | "building" | "running" | "success" | "error";

const STATUS_COLORS: Record<Status, string> = {
  idle: "var(--text-muted)",
  building: "var(--accent-color)",
  running: "var(--accent-color)",
  success: "var(--success-color)",
  error: "var(--error-color)",
};

const SYSTEM_ICONS: Record<string, string> = {
  cargo: "Rust", rustc: "Rust", npm: "Node", yarn: "Yarn", bun: "Bun", pnpm: "pnpm",
  maven: "Maven", gradle: "Gradle", cmake: "CMake", make: "Make",
  go: "Go", python: "Python", elixir: "Elixir", dotnet: ".NET", ruby: "Ruby",
  javac: "Java", "g++": "C++", "c++": "C++", gcc: "C", cc: "C",
  typescript: "TypeScript", kotlin: "Kotlin", swift: "Swift",
  zig: "Zig", nim: "Nim", crystal: "Crystal", gnat: "Ada",
  dmd: "D", dart: "Dart", ghc: "Haskell", scalac: "Scala", ocaml: "OCaml",
  erlc: "Erlang", perl: "Perl", php: "PHP", lua: "Lua", r: "R",
  julia: "Julia", gfortran: "Fortran", vlang: "V", racket: "Racket", fpc: "Pascal",
};

// Manual build system presets when auto-detection fails
const MANUAL_PRESETS: { label: string; build: string; run: string }[] = [
  { label: "Rust (Cargo)", build: "cargo build --release", run: "cargo run" },
  { label: "Node.js (npm)", build: "npm run build", run: "npm start" },
  { label: "Node.js (yarn)", build: "yarn build", run: "yarn start" },
  { label: "Node.js (pnpm)", build: "pnpm build", run: "pnpm start" },
  { label: "Node.js (bun)", build: "bun run build", run: "bun run start" },
  { label: "TypeScript (tsc)", build: "npx tsc", run: "node dist/index.js" },
  { label: "Go", build: "go build ./...", run: "go run ." },
  { label: "Python", build: "pip install -r requirements.txt", run: "python main.py" },
  { label: "Python (Poetry)", build: "poetry install", run: "poetry run python main.py" },
  { label: "Python (uv)", build: "uv sync", run: "uv run python main.py" },
  { label: "Java (Maven)", build: "mvn package", run: "java -jar target/*.jar" },
  { label: "Java (Gradle)", build: "./gradlew build", run: "./gradlew run" },
  { label: "C (gcc)", build: "gcc -o main main.c", run: "./main" },
  { label: "C++ (g++)", build: "g++ -o main main.cpp", run: "./main" },
  { label: "C++ (CMake)", build: "cmake -B build && cmake --build build", run: "./build/main" },
  { label: "C/C++ (Make)", build: "make", run: "./a.out" },
  { label: ".NET (C#)", build: "dotnet build", run: "dotnet run" },
  { label: "Swift", build: "swift build", run: "swift run" },
  { label: "Kotlin (Gradle)", build: "./gradlew build", run: "./gradlew run" },
  { label: "Ruby", build: "bundle install", run: "ruby main.rb" },
  { label: "Ruby on Rails", build: "bundle install", run: "rails server" },
  { label: "PHP", build: "composer install", run: "php -S localhost:8000" },
  { label: "PHP (Laravel)", build: "composer install", run: "php artisan serve" },
  { label: "Elixir (Mix)", build: "mix compile", run: "mix run" },
  { label: "Elixir (Phoenix)", build: "mix deps.get && mix compile", run: "mix phx.server" },
  { label: "Dart", build: "dart compile exe bin/main.dart", run: "dart run" },
  { label: "Flutter", build: "flutter build", run: "flutter run" },
  { label: "Zig", build: "zig build", run: "zig-out/bin/main" },
  { label: "Haskell (Cabal)", build: "cabal build", run: "cabal run" },
  { label: "Haskell (Stack)", build: "stack build", run: "stack run" },
  { label: "Scala (sbt)", build: "sbt compile", run: "sbt run" },
  { label: "OCaml (dune)", build: "dune build", run: "dune exec main" },
  { label: "Nim", build: "nim c main.nim", run: "./main" },
  { label: "Crystal", build: "crystal build main.cr", run: "./main" },
  { label: "Lua", build: "", run: "lua main.lua" },
  { label: "Julia", build: "", run: "julia main.jl" },
  { label: "R", build: "", run: "Rscript main.R" },
  { label: "Perl", build: "", run: "perl main.pl" },
  { label: "Fortran", build: "gfortran -o main main.f90", run: "./main" },
  { label: "V", build: "v .", run: "./main" },
  { label: "Docker", build: "docker build -t app .", run: "docker run -it app" },
  { label: "Docker Compose", build: "docker compose build", run: "docker compose up" },
];

export function BuildPanel({ workspacePath, onOpenFile }: BuildPanelProps) {
  const [systems, setSystems] = useState<BuildSystem[]>([]);
  const [selectedIdx, setSelectedIdx] = useState(0);
  const [status, setStatus] = useState<Status>("idle");
  const [result, setResult] = useState<BuildResult | null>(null);
  const [log, setLog] = useState<string[]>([]);
  const [customBuildCmd, setCustomBuildCmd] = useState("");
  const [customRunCmd, setCustomRunCmd] = useState("");
  const [showErrors, setShowErrors] = useState(true);
  const [showCustom, setShowCustom] = useState(false);
  const logEndRef = useRef<HTMLDivElement>(null);

  // Auto-detect build systems
  useEffect(() => {
    if (!workspacePath) return;
    invoke<BuildSystem[]>("detect_build_system", { workspace: workspacePath })
      .then(s => { setSystems(s || []); setSelectedIdx(0); })
      .catch(() => {});
  }, [workspacePath]);

  // Listen for build/run log events
  useEffect(() => {
    const unlisteners: Array<() => void> = [];
    let cancelled = false;
    (async () => {
      const u1 = await listen<string>("build:log", (e) => {
        if (!cancelled) setLog(prev => [...prev.slice(-499), e.payload]);
      });
      if (cancelled) { u1(); return; }
      unlisteners.push(u1);

      const u2 = await listen<string>("run:log", (e) => {
        if (!cancelled) setLog(prev => [...prev.slice(-499), e.payload]);
      });
      if (cancelled) { u2(); return; }
      unlisteners.push(u2);
    })();
    return () => { cancelled = true; unlisteners.forEach(f => f()); };
  }, []);

  // Auto-scroll log
  useEffect(() => {
    logEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [log]);

  const handleBuild = useCallback(async () => {
    if (!workspacePath) return;
    setStatus("building");
    setLog([]);
    setResult(null);
    try {
      const cmd = customBuildCmd.trim() || undefined;
      const r = await invoke<BuildResult>("run_build", { workspace: workspacePath, command: cmd });
      setResult(r);
      setStatus(r.success ? "success" : "error");
    } catch (e) {
      setStatus("error");
      setLog(prev => [...prev, `Build failed: ${e}`]);
    }
  }, [workspacePath, customBuildCmd]);

  const handleRun = useCallback(async () => {
    if (!workspacePath) return;
    setStatus("running");
    setLog([]);
    setResult(null);
    try {
      const cmd = customRunCmd.trim() || undefined;
      const r = await invoke<BuildResult>("run_app", { workspace: workspacePath, command: cmd });
      setResult(r);
      setStatus(r.success ? "success" : "error");
    } catch (e) {
      setStatus("error");
      setLog(prev => [...prev, `Run failed: ${e}`]);
    }
  }, [workspacePath, customRunCmd]);

  const handleBuildAndRun = useCallback(async () => {
    if (!workspacePath) return;
    setStatus("building");
    setLog([]);
    setResult(null);
    try {
      const buildCmd = customBuildCmd.trim() || undefined;
      const buildResult = await invoke<BuildResult>("run_build", { workspace: workspacePath, command: buildCmd });
      if (!buildResult.success) {
        setResult(buildResult);
        setStatus("error");
        return;
      }
      setStatus("running");
      const runCmd = customRunCmd.trim() || undefined;
      const runResult = await invoke<BuildResult>("run_app", { workspace: workspacePath, command: runCmd });
      setResult(runResult);
      setStatus(runResult.success ? "success" : "error");
    } catch (e) {
      setStatus("error");
      setLog(prev => [...prev, `Failed: ${e}`]);
    }
  }, [workspacePath, customBuildCmd, customRunCmd]);

  const selected = systems[selectedIdx];
  const errorCount = result?.errors.filter(e => e.severity === "error").length ?? 0;
  const warningCount = result?.errors.filter(e => e.severity === "warning").length ?? 0;
  const busy = status === "building" || status === "running";

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", color: "var(--text-primary)" }}>
      {/* Header */}
      <div style={{ padding: "8px 12px", borderBottom: "1px solid var(--border-color)", display: "flex", alignItems: "center", gap: 8, flexWrap: "wrap" }}>
        <span style={{ fontSize: 14, fontWeight: 700 }}>Build</span>

        {/* Status badge */}
        <span style={{
          fontSize: 10, padding: "2px 8px", borderRadius: 10, fontWeight: 600,
          background: `${STATUS_COLORS[status]}22`, color: STATUS_COLORS[status],
        }}>
          {status === "idle" ? "Ready" : status === "building" ? "Building..." : status === "running" ? "Running..." : status === "success" ? "Success" : "Failed"}
        </span>

        {/* Build system selector */}
        {systems.length > 1 && (
          <select
            style={{ fontSize: 11, padding: "2px 6px", borderRadius: 4, border: "1px solid var(--border-color)", background: "var(--bg-secondary)", color: "var(--text-primary)" }}
            value={selectedIdx}
            onChange={e => setSelectedIdx(Number(e.target.value))}
          >
            {systems.map((s, i) => <option key={i} value={i}>{SYSTEM_ICONS[s.name] || s.name}{s.project_path ? ` — ${s.project_path}/` : ""} ({s.config_file})</option>)}
          </select>
        )}
        {systems.length === 1 && selected && (
          <span style={{ fontSize: 11, color: "var(--text-secondary)" }}>
            {SYSTEM_ICONS[selected.name] || selected.name}
            {selected.project_path && <span style={{ color: "var(--text-muted)" }}> — {selected.project_path}/</span>}
          </span>
        )}

        <div style={{ flex: 1 }} />

        {/* Action buttons */}
        <button onClick={handleBuild} disabled={busy || !workspacePath} style={{ ...btnS, background: "var(--accent-color)", color: "var(--btn-primary-fg)", borderColor: "var(--accent-color)", opacity: busy ? 0.5 : 1 }}>
          Build
        </button>
        <button onClick={handleRun} disabled={busy || !workspacePath} style={{ ...btnS, opacity: busy ? 0.5 : 1 }}>
          Run
        </button>
        <button onClick={handleBuildAndRun} disabled={busy || !workspacePath} style={{ ...btnS, background: "var(--accent-color)", color: "var(--btn-primary-fg)", borderColor: "var(--accent-color)", opacity: busy ? 0.5 : 1 }}>
          Build & Run
        </button>
        <button onClick={() => setShowCustom(prev => !prev)} style={{ ...btnS, fontSize: 10, padding: "2px 6px" }}>
          {showCustom ? "Hide" : "Custom"}
        </button>
      </div>

      {/* Custom command inputs */}
      {showCustom && (
        <div style={{ padding: "6px 12px", borderBottom: "1px solid var(--border-color)", display: "flex", gap: 6, alignItems: "center", fontSize: 11 }}>
          <span style={{ color: "var(--text-muted)", flexShrink: 0 }}>Build:</span>
          <input
            style={inputS}
            value={customBuildCmd}
            onChange={e => setCustomBuildCmd(e.target.value)}
            placeholder={selected?.build_command || "auto-detect"}
          />
          <span style={{ color: "var(--text-muted)", flexShrink: 0 }}>Run:</span>
          <input
            style={inputS}
            value={customRunCmd}
            onChange={e => setCustomRunCmd(e.target.value)}
            placeholder={selected?.run_command || "auto-detect"}
          />
        </div>
      )}

      {/* Tool availability warning */}
      {selected && !selected.tool_available && (
        <div style={{
          padding: "8px 12px", borderBottom: "1px solid var(--border-color)",
          background: "var(--warning-bg)", fontSize: 12, display: "flex", alignItems: "flex-start", gap: 8, flexWrap: "wrap",
        }}>
          <span style={{ color: "var(--warning-color)", fontWeight: 600, flexShrink: 0 }}>Tool not found:</span>
          <span style={{ color: "var(--text-primary)" }}>`{selected.name}` is not installed.</span>
          {selected.install_hint && (
            <span style={{ color: "var(--text-secondary)", fontSize: 11, fontFamily: "var(--font-mono)" }}>{selected.install_hint}</span>
          )}
        </div>
      )}

      {/* Build result summary */}
      {result && (
        <div style={{
          padding: "6px 12px", borderBottom: "1px solid var(--border-color)",
          display: "flex", gap: 12, alignItems: "center", fontSize: 11,
          background: result.success ? "var(--success-bg)" : "var(--error-bg)",
        }}>
          <span style={{ fontWeight: 600, color: result.success ? "var(--success-color)" : "var(--error-color)" }}>
            {result.success ? "Build succeeded" : "Build failed"} — exit {result.exit_code}
          </span>
          <span style={{ color: "var(--text-secondary)" }}>{(result.duration_ms / 1000).toFixed(1)}s</span>
          {errorCount > 0 && (
            <button onClick={() => setShowErrors(true)} style={{ ...btnS, fontSize: 10, padding: "1px 6px", color: "var(--error-color)" }}>
              {errorCount} error{errorCount !== 1 ? "s" : ""}
            </button>
          )}
          {warningCount > 0 && (
            <span style={{ color: "var(--warning-color)" }}>{warningCount} warning{warningCount !== 1 ? "s" : ""}</span>
          )}
        </div>
      )}

      {/* Error list */}
      {showErrors && result && result.errors.length > 0 && (
        <div style={{ maxHeight: 150, overflowY: "auto", borderBottom: "1px solid var(--border-color)" }}>
          <div style={{ display: "flex", alignItems: "center", padding: "4px 12px", background: "var(--bg-tertiary)" }}>
            <span style={{ fontSize: 10, fontWeight: 700, textTransform: "uppercase", color: "var(--text-muted)", letterSpacing: "0.05em" }}>
              Diagnostics ({result.errors.length})
            </span>
            <div style={{ flex: 1 }} />
            <button onClick={() => setShowErrors(false)} style={{ ...btnS, fontSize: 9, padding: "1px 4px" }}>Hide</button>
          </div>
          {result.errors.map((err, i) => (
            <div
              key={i}
              onClick={() => {
                if (err.file && onOpenFile) {
                  const fullPath = err.file.startsWith("/") ? err.file : `${workspacePath}/${err.file}`;
                  onOpenFile(fullPath, err.line ?? undefined);
                }
              }}
              style={{
                padding: "3px 12px", fontSize: 11, cursor: err.file ? "pointer" : "default",
                borderBottom: "1px solid var(--border-color)",
                display: "flex", gap: 8, alignItems: "baseline",
              }}
            >
              <span style={{
                fontSize: 9, fontWeight: 700, flexShrink: 0,
                color: err.severity === "error" ? "var(--error-color)" : "var(--warning-color)",
              }}>
                {err.severity === "error" ? "ERR" : "WARN"}
              </span>
              {err.file && (
                <span style={{ fontFamily: "var(--font-mono)", fontSize: 10, color: "var(--accent-color)", flexShrink: 0 }}>
                  {err.file.split("/").pop()}{err.line ? `:${err.line}` : ""}
                </span>
              )}
              <span style={{ color: "var(--text-secondary)", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                {err.message}
              </span>
            </div>
          ))}
        </div>
      )}

      {/* Output log */}
      <div style={{ flex: 1, overflowY: "auto", background: "var(--bg-primary)", padding: "8px 12px" }}>
        {log.length === 0 && !busy && (
          <div style={{ textAlign: "center", color: "var(--text-muted)", padding: 24, fontSize: 12 }}>
            {systems.length > 0 ? (
              `${SYSTEM_ICONS[systems[0]?.name] || systems[0]?.name} project detected. Click Build to compile.`
            ) : workspacePath ? (
              <div>
                <div style={{ marginBottom: 12 }}>No build system detected. Select a language or use Custom.</div>
                <select
                  style={{
                    padding: "6px 10px", fontSize: 12, borderRadius: "var(--radius-sm)",
                    border: "1px solid var(--border-color)", background: "var(--bg-secondary)",
                    color: "var(--text-primary)", width: 280, marginBottom: 8,
                  }}
                  defaultValue=""
                  onChange={e => {
                    const preset = MANUAL_PRESETS.find(p => p.label === e.target.value);
                    if (preset) {
                      setCustomBuildCmd(preset.build);
                      setCustomRunCmd(preset.run);
                      setShowCustom(true);
                    }
                  }}
                >
                  <option value="" disabled>Select language / build system...</option>
                  {MANUAL_PRESETS.map(p => (
                    <option key={p.label} value={p.label}>{p.label}</option>
                  ))}
                </select>
                <div style={{ fontSize: 11, color: "var(--text-secondary)" }}>
                  This will pre-fill the Custom build and run commands.
                </div>
              </div>
            ) : "Open a folder to start building."}
          </div>
        )}
        <pre style={{ margin: 0, fontFamily: "var(--font-mono)", fontSize: 11, lineHeight: 1.5, whiteSpace: "pre-wrap", wordBreak: "break-all", color: "var(--text-primary)" }}>
          {log.join("\n")}
        </pre>
        <div ref={logEndRef} />
      </div>
    </div>
  );
}

const btnS: React.CSSProperties = {
  padding: "4px 10px", fontSize: 11, fontWeight: 600, borderRadius: 4,
  border: "1px solid var(--border-color)", background: "var(--bg-elevated)",
  color: "var(--text-primary)", cursor: "pointer",
};

const inputS: React.CSSProperties = {
  flex: 1, minWidth: 0, padding: "3px 6px", fontSize: 11, borderRadius: 4,
  border: "1px solid var(--border-color)", background: "var(--bg-primary)",
  color: "var(--text-primary)",
};
