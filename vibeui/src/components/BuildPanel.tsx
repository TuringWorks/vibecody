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
import { X } from "lucide-react";

interface BuildPanelProps {
  workspacePath: string | null;
  currentFile?: string | null;
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
  idle: "var(--text-secondary)",
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

export function BuildPanel({ workspacePath, currentFile, onOpenFile }: BuildPanelProps) {
  const [systems, setSystems] = useState<BuildSystem[]>([]);
  const [selectedIdx, setSelectedIdx] = useState(0);
  const [status, setStatus] = useState<Status>("idle");
  const [result, setResult] = useState<BuildResult | null>(null);
  const [log, setLog] = useState<string[]>([]);
  const [customBuildCmd, setCustomBuildCmd] = useState("");
  const [customRunCmd, setCustomRunCmd] = useState("");
  const [showErrors, setShowErrors] = useState(true);
  const [showCustom, setShowCustom] = useState(false);
  const [buildDir, setBuildDir] = useState<string>(""); // empty = workspace root
  const [subdirs, setSubdirs] = useState<string[]>([]);
  const logEndRef = useRef<HTMLDivElement>(null);

  // Derive the current file's directory as default build dir
  useEffect(() => {
    if (currentFile && workspacePath) {
      const dir = currentFile.substring(0, currentFile.lastIndexOf("/"));
      if (dir && dir !== workspacePath) {
        setBuildDir(dir);
      }
    }
  }, [currentFile, workspacePath]);

  // Load subdirectories for the directory picker
  useEffect(() => {
    if (!workspacePath) return;
    invoke<string[]>("list_workspace_subdirs", { workspace: workspacePath })
      .then(dirs => setSubdirs(dirs || []))
      .catch(() => {});
  }, [workspacePath]);

  // The effective directory where build/run commands execute
  const effectiveDir = buildDir || workspacePath || "";

  // Auto-detect build systems in the effective directory
  useEffect(() => {
    if (!effectiveDir) return;
    invoke<BuildSystem[]>("detect_build_system", { workspace: effectiveDir })
      .then(s => { setSystems(s || []); setSelectedIdx(0); })
      .catch(() => {});
  }, [effectiveDir]);

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
    if (!effectiveDir) return;
    setStatus("building");
    setLog([`[pwd] ${effectiveDir}`]);
    setResult(null);
    try {
      const cmd = customBuildCmd.trim() || undefined;
      const r = await invoke<BuildResult>("run_build", { workspace: effectiveDir, command: cmd });
      setResult(r);
      setStatus(r.success ? "success" : "error");
    } catch (e) {
      setStatus("error");
      setLog(prev => [...prev, `Build failed: ${e}`]);
    }
  }, [effectiveDir, customBuildCmd]);

  const handleRun = useCallback(async () => {
    if (!effectiveDir) return;
    setStatus("running");
    setLog([`[pwd] ${effectiveDir}`]);
    setResult(null);
    try {
      const cmd = customRunCmd.trim() || undefined;
      const r = await invoke<BuildResult>("run_app", { workspace: effectiveDir, command: cmd });
      setResult(r);
      setStatus(r.success ? "success" : "error");
    } catch (e) {
      setStatus("error");
      setLog(prev => [...prev, `Run failed: ${e}`]);
    }
  }, [effectiveDir, customRunCmd]);

  const handleBuildAndRun = useCallback(async () => {
    if (!effectiveDir) return;
    setStatus("building");
    setLog([`[pwd] ${effectiveDir}`]);
    setResult(null);
    try {
      const buildCmd = customBuildCmd.trim() || undefined;
      const buildResult = await invoke<BuildResult>("run_build", { workspace: effectiveDir, command: buildCmd });
      if (!buildResult.success) {
        setResult(buildResult);
        setStatus("error");
        return;
      }
      setStatus("running");
      const runCmd = customRunCmd.trim() || undefined;
      const runResult = await invoke<BuildResult>("run_app", { workspace: effectiveDir, command: runCmd });
      setResult(runResult);
      setStatus(runResult.success ? "success" : "error");
    } catch (e) {
      setStatus("error");
      setLog(prev => [...prev, `Failed: ${e}`]);
    }
  }, [effectiveDir, customBuildCmd, customRunCmd]);

  const selected = systems[selectedIdx];
  const errorCount = result?.errors.filter(e => e.severity === "error").length ?? 0;
  const warningCount = result?.errors.filter(e => e.severity === "warning").length ?? 0;
  const busy = status === "building" || status === "running";

  // Short display path for the working directory
  const shortDir = effectiveDir && workspacePath
    ? (effectiveDir === workspacePath ? "/" : effectiveDir.replace(workspacePath, "").replace(/^\//, "") || "/")
    : "/";

  return (
    <div className="panel-container">
      {/* Working directory selector */}
      <div style={{ padding: "4px 12px", borderBottom: "1px solid var(--border-color)", display: "flex", alignItems: "center", gap: 6, fontSize: "var(--font-size-sm)", background: "var(--bg-secondary)" }}>
        <span style={{ color: "var(--text-secondary)", flexShrink: 0 }}>Working dir:</span>
        <select
          style={{ flex: 1, minWidth: 0, padding: "2px 6px", fontSize: "var(--font-size-sm)", borderRadius: "var(--radius-xs-plus)", border: "1px solid var(--border-color)", background: "var(--bg-primary)", color: "var(--text-primary)", fontFamily: "var(--font-mono, monospace)" }}
          value={buildDir}
          onChange={e => setBuildDir(e.target.value)}
        >
          <option value="">{workspacePath ? `/ (workspace root)` : "No workspace"}</option>
          {subdirs.map(d => (
            <option key={d} value={`${workspacePath}/${d}`}>{d}/</option>
          ))}
        </select>
        {buildDir && buildDir !== workspacePath && (
          <button
            onClick={() => setBuildDir("")}
            aria-label="Reset to workspace root"
            style={{ background: "none", border: "none", color: "var(--text-secondary)", cursor: "pointer", padding: "0 4px", display: "flex", alignItems: "center" }}
            title="Reset to workspace root"
          ><X size={12} /></button>
        )}
      </div>

      {/* Header */}
      <div className="panel-header" style={{ flexWrap: "wrap" }}>
        <span style={{ fontSize: "var(--font-size-lg)", fontWeight: 700 }}>Build</span>
        <span style={{ fontSize: "var(--font-size-xs)", fontFamily: "var(--font-mono, monospace)", color: "var(--text-secondary)", background: "var(--bg-tertiary)", padding: "1px 6px", borderRadius: 3 }}>{shortDir}</span>

        {/* Status badge */}
        <span style={{
          fontSize: "var(--font-size-xs)", padding: "2px 8px", borderRadius: "var(--radius-md)", fontWeight: 600,
          background: `${STATUS_COLORS[status]}22`, color: STATUS_COLORS[status],
        }}>
          {status === "idle" ? "Ready" : status === "building" ? "Building..." : status === "running" ? "Running..." : status === "success" ? "Success" : "Failed"}
        </span>

        {/* Build system selector */}
        {systems.length > 1 && (
          <select
            style={{ fontSize: "var(--font-size-sm)", padding: "2px 6px", borderRadius: "var(--radius-xs-plus)", border: "1px solid var(--border-color)", background: "var(--bg-secondary)", color: "var(--text-primary)" }}
            value={selectedIdx}
            onChange={e => setSelectedIdx(Number(e.target.value))}
          >
            {systems.map((s, i) => <option key={i} value={i}>{SYSTEM_ICONS[s.name] || s.name}{s.project_path ? ` — ${s.project_path}/` : ""} ({s.config_file})</option>)}
          </select>
        )}
        {systems.length === 1 && selected && (
          <span style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>
            {SYSTEM_ICONS[selected.name] || selected.name}
            {selected.project_path && <span style={{ color: "var(--text-secondary)" }}> — {selected.project_path}/</span>}
          </span>
        )}

        <div style={{ flex: 1 }} />

        {/* Action buttons */}
        <button onClick={handleBuild} disabled={busy || !effectiveDir} className="panel-btn panel-btn-primary" style={{ opacity: busy ? 0.5 : 1 }}>
          Build
        </button>
        <button onClick={handleRun} disabled={busy || !effectiveDir} className="panel-btn panel-btn-secondary" style={{ opacity: busy ? 0.5 : 1 }}>
          Run
        </button>
        <button onClick={handleBuildAndRun} disabled={busy || !effectiveDir} className="panel-btn panel-btn-primary" style={{ opacity: busy ? 0.5 : 1 }}>
          Build & Run
        </button>
        <button onClick={() => setShowCustom(prev => !prev)} className="panel-btn panel-btn-secondary" style={{ fontSize: "var(--font-size-xs)", padding: "2px 6px" }}>
          {showCustom ? "Hide" : "Custom"}
        </button>
      </div>

      {/* Custom command inputs */}
      {showCustom && (
        <div style={{ padding: "6px 12px", borderBottom: "1px solid var(--border-color)", display: "flex", gap: 6, alignItems: "center", fontSize: "var(--font-size-sm)" }}>
          <span style={{ color: "var(--text-secondary)", flexShrink: 0 }}>Build:</span>
          <input
            style={{ flex: 1, minWidth: 0, padding: "3px 6px", fontSize: "var(--font-size-sm)", borderRadius: "var(--radius-xs-plus)", border: "1px solid var(--border-color)", background: "var(--bg-primary)", color: "var(--text-primary)" }}
            value={customBuildCmd}
            onChange={e => setCustomBuildCmd(e.target.value)}
            placeholder={selected?.build_command || "auto-detect"}
          />
          <span style={{ color: "var(--text-secondary)", flexShrink: 0 }}>Run:</span>
          <input
            style={{ flex: 1, minWidth: 0, padding: "3px 6px", fontSize: "var(--font-size-sm)", borderRadius: "var(--radius-xs-plus)", border: "1px solid var(--border-color)", background: "var(--bg-primary)", color: "var(--text-primary)" }}
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
          background: "var(--warning-bg)", fontSize: "var(--font-size-base)", display: "flex", alignItems: "flex-start", gap: 8, flexWrap: "wrap",
        }}>
          <span style={{ color: "var(--warning-color)", fontWeight: 600, flexShrink: 0 }}>Tool not found:</span>
          <span style={{ color: "var(--text-primary)" }}>`{selected.name}` is not installed.</span>
          {selected.install_hint && (
            <span style={{ color: "var(--text-secondary)", fontSize: "var(--font-size-sm)", fontFamily: "var(--font-mono)" }}>{selected.install_hint}</span>
          )}
        </div>
      )}

      {/* Build result summary */}
      {result && (
        <div style={{
          padding: "6px 12px", borderBottom: "1px solid var(--border-color)",
          display: "flex", gap: 12, alignItems: "center", fontSize: "var(--font-size-sm)",
          background: result.success ? "var(--success-bg)" : "var(--error-bg)",
        }}>
          <span style={{ fontWeight: 600, color: result.success ? "var(--success-color)" : "var(--error-color)" }}>
            {result.success ? "Build succeeded" : "Build failed"} — exit {result.exit_code}
          </span>
          <span style={{ color: "var(--text-secondary)" }}>{(result.duration_ms / 1000).toFixed(1)}s</span>
          {errorCount > 0 && (
            <button onClick={() => setShowErrors(true)} className="panel-btn panel-btn-secondary" style={{ fontSize: "var(--font-size-xs)", padding: "1px 6px", color: "var(--error-color)" }}>
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
            <span style={{ fontSize: "var(--font-size-xs)", fontWeight: 700, textTransform: "uppercase", color: "var(--text-secondary)", letterSpacing: "0.05em" }}>
              Diagnostics ({result.errors.length})
            </span>
            <div style={{ flex: 1 }} />
            <button onClick={() => setShowErrors(false)} className="panel-btn panel-btn-secondary" style={{ fontSize: 9, padding: "1px 4px" }}>Hide</button>
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
                padding: "3px 12px", fontSize: "var(--font-size-sm)", cursor: err.file ? "pointer" : "default",
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
                <span style={{ fontFamily: "var(--font-mono)", fontSize: "var(--font-size-xs)", color: "var(--accent-color)", flexShrink: 0 }}>
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
      <div className="panel-body" style={{ padding: "8px 12px" }}>
        {log.length === 0 && !busy && (
          <div style={{ textAlign: "center", color: "var(--text-secondary)", padding: 24, fontSize: "var(--font-size-base)" }}>
            {systems.length > 0 ? (
              `${SYSTEM_ICONS[systems[0]?.name] || systems[0]?.name} project detected. Click Build to compile.`
            ) : workspacePath ? (
              <div>
                <div style={{ marginBottom: 12 }}>No build system detected. Select a language or use Custom.</div>
                <select
                  style={{
                    padding: "6px 10px", fontSize: "var(--font-size-base)", borderRadius: "var(--radius-sm)",
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
                <div style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>
                  This will pre-fill the Custom build and run commands.
                </div>
              </div>
            ) : "Open a folder to start building."}
          </div>
        )}
        <pre style={{ margin: 0, fontFamily: "var(--font-mono)", fontSize: "var(--font-size-sm)", lineHeight: 1.5, whiteSpace: "pre-wrap", wordBreak: "break-all", overflow: "auto", color: "var(--text-primary)" }}>
          {log.join("\n")}
        </pre>
        <div ref={logEndRef} />
      </div>
    </div>
  );
}

