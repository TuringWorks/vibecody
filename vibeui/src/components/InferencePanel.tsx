/**
 * InferencePanel — ML inference server management.
 *
 * Tabs: Deploy (endpoint config + CLI/Docker generation), Benchmark (results table),
 * Scaling (auto-scale + load balancer + K8s YAML).
 *
 * Pure TypeScript — no Tauri commands.
 */
import { useState } from "react";

type Tab = "deploy" | "benchmark" | "scaling";
type Backend = "vllm" | "tgi" | "triton" | "llama.cpp" | "ollama";
type Quantization = "none" | "fp16" | "bf16" | "int8" | "int4" | "gptq" | "awq" | "gguf";
type LBStrategy = "round-robin" | "least-connections" | "weighted-random";

interface DeployConfig {
  modelPath: string;
  backend: Backend;
  port: number;
  gpuCount: number;
  tensorParallel: number;
  quantization: Quantization;
  maxBatchSize: number;
  gpuMemUtil: number;
}

interface BenchmarkEntry {
  id: string;
  backend: Backend;
  model: string;
  ttft: number;
  tokensPerSec: number;
  vram: number;
}

interface ScaleConfig {
  minReplicas: number;
  maxReplicas: number;
  targetGpuUtil: number;
  targetLatencyMs: number;
  scaleUpCooldown: number;
  scaleDownCooldown: number;
}

interface LBConfig {
  strategy: LBStrategy;
  healthCheckPath: string;
  healthCheckInterval: number;
}

const BACKENDS: { value: Backend; label: string }[] = [
  { value: "vllm", label: "vLLM" },
  { value: "tgi", label: "TGI" },
  { value: "triton", label: "Triton" },
  { value: "llama.cpp", label: "llama.cpp" },
  { value: "ollama", label: "Ollama" },
];

const QUANTIZATIONS: { value: Quantization; label: string }[] = [
  { value: "none", label: "None" },
  { value: "fp16", label: "FP16" },
  { value: "bf16", label: "BF16" },
  { value: "int8", label: "INT8" },
  { value: "int4", label: "INT4" },
  { value: "gptq", label: "GPTQ" },
  { value: "awq", label: "AWQ" },
  { value: "gguf", label: "GGUF" },
];

const LB_STRATEGIES: { value: LBStrategy; label: string }[] = [
  { value: "round-robin", label: "Round Robin" },
  { value: "least-connections", label: "Least Connections" },
  { value: "weighted-random", label: "Weighted Random" },
];

/* ---- style helpers ---- */
const inputStyle: React.CSSProperties = {
  width: "100%",
  background: "var(--bg-secondary)",
  border: "1px solid var(--border-color)",
  borderRadius: 4,
  color: "var(--text-primary)",
  padding: "5px 8px",
  fontSize: 12,
  boxSizing: "border-box",
};

const selectStyle: React.CSSProperties = { ...inputStyle };

const labelStyle: React.CSSProperties = {
  display: "block",
  fontSize: 11,
  fontWeight: 600,
  marginBottom: 4,
  color: "var(--text-secondary)",
};

const btnPrimary: React.CSSProperties = {
  background: "var(--accent-color)",
  color: "var(--text-primary)",
  border: "none",
  borderRadius: 4,
  padding: "6px 14px",
  cursor: "pointer",
  fontSize: 12,
  fontWeight: 600,
};

const btnSecondary: React.CSSProperties = {
  background: "var(--bg-secondary)",
  border: "1px solid var(--border-color)",
  borderRadius: 4,
  padding: "6px 14px",
  cursor: "pointer",
  fontSize: 12,
  color: "var(--text-primary)",
};

const codeBlock: React.CSSProperties = {
  background: "var(--bg-secondary)",
  border: "1px solid var(--border-color)",
  borderRadius: 4,
  padding: 12,
  fontFamily: "var(--font-mono)",
  fontSize: 11,
  whiteSpace: "pre-wrap",
  wordBreak: "break-all",
  color: "var(--text-primary)",
  overflowX: "auto",
};

const fieldRow: React.CSSProperties = {
  display: "grid",
  gridTemplateColumns: "1fr 1fr",
  gap: 12,
  marginBottom: 12,
};

/* ---- CLI / Docker generators ---- */
function generateCliCommand(c: DeployConfig): string {
  switch (c.backend) {
    case "vllm":
      return [
        "python -m vllm.entrypoints.openai.api_server",
        `  --model ${c.modelPath || "<model>"}`,
        `  --port ${c.port}`,
        `  --tensor-parallel-size ${c.tensorParallel}`,
        c.quantization !== "none" ? `  --quantization ${c.quantization}` : null,
        `  --max-num-batched-tokens ${c.maxBatchSize}`,
        `  --gpu-memory-utilization ${(c.gpuMemUtil / 100).toFixed(2)}`,
      ].filter(Boolean).join(" \\\n");
    case "tgi":
      return [
        "text-generation-launcher",
        `  --model-id ${c.modelPath || "<model>"}`,
        `  --port ${c.port}`,
        `  --num-shard ${c.tensorParallel}`,
        c.quantization !== "none" ? `  --quantize ${c.quantization}` : null,
        `  --max-batch-total-tokens ${c.maxBatchSize}`,
      ].filter(Boolean).join(" \\\n");
    case "triton":
      return [
        "tritonserver",
        `  --model-repository=${c.modelPath || "<model-repo>"}`,
        `  --http-port=${c.port}`,
        `  --backend-config=python,shm-default-byte-size=1048576`,
      ].filter(Boolean).join(" \\\n");
    case "llama.cpp":
      return [
        "llama-server",
        `  -m ${c.modelPath || "<model.gguf>"}`,
        `  --port ${c.port}`,
        `  -ngl 999`,
        `  --parallel ${c.tensorParallel}`,
        `  -b ${c.maxBatchSize}`,
      ].filter(Boolean).join(" \\\n");
    case "ollama":
      return [
        `OLLAMA_HOST=0.0.0.0:${c.port} ollama serve`,
        `# Then: ollama run ${c.modelPath || "<model>"}`,
      ].join("\n");
  }
}

function generateDockerCompose(c: DeployConfig): string {
  const image: Record<Backend, string> = {
    vllm: "vllm/vllm-openai:latest",
    tgi: "ghcr.io/huggingface/text-generation-inference:latest",
    triton: "nvcr.io/nvidia/tritonserver:24.01-py3",
    "llama.cpp": "ghcr.io/ggerganov/llama.cpp:server",
    ollama: "ollama/ollama:latest",
  };

  const gpuSection = c.gpuCount > 0
    ? `    deploy:\n      resources:\n        reservations:\n          devices:\n            - driver: nvidia\n              count: ${c.gpuCount}\n              capabilities: [gpu]`
    : "";

  let command = "";
  switch (c.backend) {
    case "vllm":
      command = `    command: ["--model", "${c.modelPath || "<model>"}", "--port", "${c.port}", "--tensor-parallel-size", "${c.tensorParallel}", "--gpu-memory-utilization", "${(c.gpuMemUtil / 100).toFixed(2)}", "--max-num-batched-tokens", "${c.maxBatchSize}"${c.quantization !== "none" ? `, "--quantization", "${c.quantization}"` : ""}]`;
      break;
    case "tgi":
      command = `    command: ["--model-id", "${c.modelPath || "<model>"}", "--port", "${c.port}", "--num-shard", "${c.tensorParallel}"${c.quantization !== "none" ? `, "--quantize", "${c.quantization}"` : ""}]`;
      break;
    case "ollama":
      command = `    environment:\n      - OLLAMA_HOST=0.0.0.0:${c.port}`;
      break;
    default:
      command = `    # Configure command for ${c.backend}`;
  }

  return `version: "3.8"
services:
  inference:
    image: ${image[c.backend]}
    ports:
      - "${c.port}:${c.port}"
${command}
${gpuSection}
    restart: unless-stopped`;
}

function generateK8sYaml(deploy: DeployConfig, scale: ScaleConfig, lb: LBConfig): string {
  return `apiVersion: apps/v1
kind: Deployment
metadata:
  name: inference-${deploy.backend}
  labels:
    app: inference
    backend: ${deploy.backend}
spec:
  replicas: ${scale.minReplicas}
  selector:
    matchLabels:
      app: inference
  template:
    metadata:
      labels:
        app: inference
        backend: ${deploy.backend}
    spec:
      containers:
        - name: inference
          image: <image>
          ports:
            - containerPort: ${deploy.port}
          resources:
            limits:
              nvidia.com/gpu: "${deploy.gpuCount}"
          readinessProbe:
            httpGet:
              path: ${lb.healthCheckPath}
              port: ${deploy.port}
            periodSeconds: ${lb.healthCheckInterval}
---
apiVersion: v1
kind: Service
metadata:
  name: inference-svc
  annotations:
    service.beta.kubernetes.io/load-balancer-strategy: "${lb.strategy}"
spec:
  type: ClusterIP
  selector:
    app: inference
  ports:
    - port: ${deploy.port}
      targetPort: ${deploy.port}
---
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: inference-hpa
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: inference-${deploy.backend}
  minReplicas: ${scale.minReplicas}
  maxReplicas: ${scale.maxReplicas}
  behavior:
    scaleUp:
      stabilizationWindowSeconds: ${scale.scaleUpCooldown}
    scaleDown:
      stabilizationWindowSeconds: ${scale.scaleDownCooldown}
  metrics:
    - type: Pods
      pods:
        metric:
          name: gpu_utilization
        target:
          type: AverageValue
          averageValue: "${scale.targetGpuUtil}"
    - type: Pods
      pods:
        metric:
          name: request_latency_ms
        target:
          type: AverageValue
          averageValue: "${scale.targetLatencyMs}"`;
}

/* ---- Component ---- */
export function InferencePanel() {
  const [tab, setTab] = useState<Tab>("deploy");

  /* Deploy state */
  const [deploy, setDeploy] = useState<DeployConfig>({
    modelPath: "",
    backend: "vllm",
    port: 8000,
    gpuCount: 1,
    tensorParallel: 1,
    quantization: "none",
    maxBatchSize: 256,
    gpuMemUtil: 90,
  });
  const [generatedCli, setGeneratedCli] = useState<string | null>(null);
  const [generatedCompose, setGeneratedCompose] = useState<string | null>(null);

  /* Benchmark state */
  const [benchmarks, setBenchmarks] = useState<BenchmarkEntry[]>([]);
  const [benchForm, setBenchForm] = useState<Omit<BenchmarkEntry, "id">>({
    backend: "vllm",
    model: "",
    ttft: 0,
    tokensPerSec: 0,
    vram: 0,
  });
  const [compareIds, setCompareIds] = useState<Set<string>>(new Set());

  /* Scaling state */
  const [scale, setScale] = useState<ScaleConfig>({
    minReplicas: 1,
    maxReplicas: 8,
    targetGpuUtil: 80,
    targetLatencyMs: 200,
    scaleUpCooldown: 60,
    scaleDownCooldown: 300,
  });
  const [lb, setLb] = useState<LBConfig>({
    strategy: "round-robin",
    healthCheckPath: "/health",
    healthCheckInterval: 10,
  });
  const [generatedK8s, setGeneratedK8s] = useState<string | null>(null);

  /* Deploy helpers */
  const updateDeploy = <K extends keyof DeployConfig>(key: K, value: DeployConfig[K]) =>
    setDeploy((prev) => ({ ...prev, [key]: value }));

  /* Benchmark helpers */
  const addBenchmark = () => {
    if (!benchForm.model.trim()) return;
    setBenchmarks((prev) => [...prev, { ...benchForm, id: crypto.randomUUID() }]);
    setBenchForm({ backend: "vllm", model: "", ttft: 0, tokensPerSec: 0, vram: 0 });
  };

  const removeBenchmark = (id: string) => {
    setBenchmarks((prev) => prev.filter((b) => b.id !== id));
    setCompareIds((prev) => { const next = new Set(prev); next.delete(id); return next; });
  };

  const toggleCompare = (id: string) => {
    setCompareIds((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id); else next.add(id);
      return next;
    });
  };

  const comparedEntries = benchmarks.filter((b) => compareIds.has(b.id));

  const tabs: { key: Tab; label: string }[] = [
    { key: "deploy", label: "Deploy" },
    { key: "benchmark", label: "Benchmark" },
    { key: "scaling", label: "Scaling" },
  ];

  return (
    <div style={{ height: "100%", display: "flex", flexDirection: "column", background: "var(--bg-primary)", color: "var(--text-primary)" }}>
      {/* Tab bar */}
      <div style={{ display: "flex", borderBottom: "1px solid var(--border-color)", padding: "0 12px" }}>
        {tabs.map((t) => (
          <button
            key={t.key}
            onClick={() => setTab(t.key)}
            style={{
              background: "none",
              border: "none",
              borderBottom: tab === t.key ? "2px solid var(--accent-blue)" : "2px solid transparent",
              padding: "8px 16px",
              cursor: "pointer",
              fontSize: 12,
              fontWeight: tab === t.key ? 600 : 400,
              color: tab === t.key ? "var(--text-primary)" : "var(--text-secondary)",
            }}
          >
            {t.label}
          </button>
        ))}
      </div>

      {/* Tab content */}
      <div style={{ flex: 1, overflow: "auto", padding: 16 }}>
        {/* ============= DEPLOY TAB ============= */}
        {tab === "deploy" && (
          <div style={{ maxWidth: 700 }}>
            {/* Model path */}
            <div style={{ marginBottom: 12 }}>
              <label style={labelStyle}>Model Path / ID</label>
              <input
                value={deploy.modelPath}
                onChange={(e) => updateDeploy("modelPath", e.target.value)}
                placeholder="e.g. meta-llama/Llama-3.1-70B-Instruct"
                style={inputStyle}
              />
            </div>

            {/* Backend + Port */}
            <div style={fieldRow}>
              <div>
                <label style={labelStyle}>Backend</label>
                <select
                  value={deploy.backend}
                  onChange={(e) => updateDeploy("backend", e.target.value as Backend)}
                  style={selectStyle}
                >
                  {BACKENDS.map((b) => (
                    <option key={b.value} value={b.value}>{b.label}</option>
                  ))}
                </select>
              </div>
              <div>
                <label style={labelStyle}>Port</label>
                <input
                  type="number"
                  value={deploy.port}
                  onChange={(e) => updateDeploy("port", Number(e.target.value))}
                  style={inputStyle}
                />
              </div>
            </div>

            {/* GPU Count + Tensor Parallel */}
            <div style={fieldRow}>
              <div>
                <label style={labelStyle}>GPU Count</label>
                <input
                  type="number"
                  min={0}
                  value={deploy.gpuCount}
                  onChange={(e) => updateDeploy("gpuCount", Number(e.target.value))}
                  style={inputStyle}
                />
              </div>
              <div>
                <label style={labelStyle}>Tensor Parallel</label>
                <input
                  type="number"
                  min={1}
                  value={deploy.tensorParallel}
                  onChange={(e) => updateDeploy("tensorParallel", Number(e.target.value))}
                  style={inputStyle}
                />
              </div>
            </div>

            {/* Quantization + Max Batch Size */}
            <div style={fieldRow}>
              <div>
                <label style={labelStyle}>Quantization</label>
                <select
                  value={deploy.quantization}
                  onChange={(e) => updateDeploy("quantization", e.target.value as Quantization)}
                  style={selectStyle}
                >
                  {QUANTIZATIONS.map((q) => (
                    <option key={q.value} value={q.value}>{q.label}</option>
                  ))}
                </select>
              </div>
              <div>
                <label style={labelStyle}>Max Batch Size</label>
                <input
                  type="number"
                  min={1}
                  value={deploy.maxBatchSize}
                  onChange={(e) => updateDeploy("maxBatchSize", Number(e.target.value))}
                  style={inputStyle}
                />
              </div>
            </div>

            {/* GPU Memory Utilization slider */}
            <div style={{ marginBottom: 16 }}>
              <label style={labelStyle}>GPU Memory Utilization: {deploy.gpuMemUtil}%</label>
              <input
                type="range"
                min={10}
                max={100}
                value={deploy.gpuMemUtil}
                onChange={(e) => updateDeploy("gpuMemUtil", Number(e.target.value))}
                style={{ width: "100%", accentColor: "var(--accent-color)" }}
              />
              <div style={{ display: "flex", justifyContent: "space-between", fontSize: 10, opacity: 0.5 }}>
                <span>10%</span><span>100%</span>
              </div>
            </div>

            {/* Action buttons */}
            <div style={{ display: "flex", gap: 8, marginBottom: 16 }}>
              <button
                style={btnPrimary}
                onClick={() => { setGeneratedCli(generateCliCommand(deploy)); setGeneratedCompose(null); }}
              >
                Generate Command
              </button>
              <button
                style={btnSecondary}
                onClick={() => { setGeneratedCompose(generateDockerCompose(deploy)); setGeneratedCli(null); }}
              >
                Generate Docker Compose
              </button>
            </div>

            {/* Output */}
            {generatedCli && (
              <div>
                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 6 }}>
                  <span style={{ fontSize: 11, fontWeight: 600, color: "var(--text-secondary)" }}>CLI Command</span>
                  <button
                    style={{ ...btnSecondary, padding: "2px 8px", fontSize: 10 }}
                    onClick={() => navigator.clipboard.writeText(generatedCli)}
                  >
                    Copy
                  </button>
                </div>
                <pre style={codeBlock}>{generatedCli}</pre>
              </div>
            )}
            {generatedCompose && (
              <div>
                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 6 }}>
                  <span style={{ fontSize: 11, fontWeight: 600, color: "var(--text-secondary)" }}>docker-compose.yml</span>
                  <button
                    style={{ ...btnSecondary, padding: "2px 8px", fontSize: 10 }}
                    onClick={() => navigator.clipboard.writeText(generatedCompose)}
                  >
                    Copy
                  </button>
                </div>
                <pre style={codeBlock}>{generatedCompose}</pre>
              </div>
            )}
          </div>
        )}

        {/* ============= BENCHMARK TAB ============= */}
        {tab === "benchmark" && (
          <div>
            {/* Entry form */}
            <div style={{ background: "var(--bg-secondary)", borderRadius: 6, padding: 12, marginBottom: 16, border: "1px solid var(--border-color)" }}>
              <div style={{ fontSize: 11, fontWeight: 600, marginBottom: 8, color: "var(--text-secondary)" }}>Add Benchmark Entry</div>
              <div style={{ display: "grid", gridTemplateColumns: "1fr 1.5fr repeat(3, 1fr) auto", gap: 8, alignItems: "end" }}>
                <div>
                  <label style={labelStyle}>Backend</label>
                  <select
                    value={benchForm.backend}
                    onChange={(e) => setBenchForm((f) => ({ ...f, backend: e.target.value as Backend }))}
                    style={selectStyle}
                  >
                    {BACKENDS.map((b) => (
                      <option key={b.value} value={b.value}>{b.label}</option>
                    ))}
                  </select>
                </div>
                <div>
                  <label style={labelStyle}>Model</label>
                  <input
                    value={benchForm.model}
                    onChange={(e) => setBenchForm((f) => ({ ...f, model: e.target.value }))}
                    placeholder="model name"
                    style={inputStyle}
                  />
                </div>
                <div>
                  <label style={labelStyle}>TTFT (ms)</label>
                  <input
                    type="number"
                    min={0}
                    value={benchForm.ttft}
                    onChange={(e) => setBenchForm((f) => ({ ...f, ttft: Number(e.target.value) }))}
                    style={inputStyle}
                  />
                </div>
                <div>
                  <label style={labelStyle}>Tokens/sec</label>
                  <input
                    type="number"
                    min={0}
                    value={benchForm.tokensPerSec}
                    onChange={(e) => setBenchForm((f) => ({ ...f, tokensPerSec: Number(e.target.value) }))}
                    style={inputStyle}
                  />
                </div>
                <div>
                  <label style={labelStyle}>VRAM (GB)</label>
                  <input
                    type="number"
                    min={0}
                    step={0.1}
                    value={benchForm.vram}
                    onChange={(e) => setBenchForm((f) => ({ ...f, vram: Number(e.target.value) }))}
                    style={inputStyle}
                  />
                </div>
                <button onClick={addBenchmark} style={{ ...btnPrimary, padding: "5px 12px" }}>Add</button>
              </div>
            </div>

            {/* Results table */}
            {benchmarks.length === 0 ? (
              <div style={{ opacity: 0.5, fontSize: 12, textAlign: "center", padding: 24 }}>
                No benchmark entries yet. Add one above.
              </div>
            ) : (
              <div style={{ overflowX: "auto" }}>
                <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 12, fontFamily: "var(--font-mono)" }}>
                  <thead>
                    <tr style={{ background: "var(--bg-secondary)" }}>
                      <th style={{ padding: "6px 8px", textAlign: "center", borderBottom: "1px solid var(--border-color)", fontWeight: 600, width: 36 }}>Cmp</th>
                      <th style={{ padding: "6px 8px", textAlign: "left", borderBottom: "1px solid var(--border-color)", fontWeight: 600 }}>Backend</th>
                      <th style={{ padding: "6px 8px", textAlign: "left", borderBottom: "1px solid var(--border-color)", fontWeight: 600 }}>Model</th>
                      <th style={{ padding: "6px 8px", textAlign: "right", borderBottom: "1px solid var(--border-color)", fontWeight: 600 }}>TTFT (ms)</th>
                      <th style={{ padding: "6px 8px", textAlign: "right", borderBottom: "1px solid var(--border-color)", fontWeight: 600 }}>Tokens/s</th>
                      <th style={{ padding: "6px 8px", textAlign: "right", borderBottom: "1px solid var(--border-color)", fontWeight: 600 }}>VRAM (GB)</th>
                      <th style={{ padding: "6px 8px", textAlign: "center", borderBottom: "1px solid var(--border-color)", fontWeight: 600, width: 40 }}></th>
                    </tr>
                  </thead>
                  <tbody>
                    {benchmarks.map((b, i) => (
                      <tr key={b.id} style={{ background: i % 2 === 0 ? "transparent" : "var(--bg-secondary)" }}>
                        <td style={{ padding: "4px 8px", textAlign: "center", borderBottom: "1px solid var(--border-color)" }}>
                          <input
                            type="checkbox"
                            checked={compareIds.has(b.id)}
                            onChange={() => toggleCompare(b.id)}
                            style={{ accentColor: "var(--accent-color)" }}
                          />
                        </td>
                        <td style={{ padding: "4px 8px", borderBottom: "1px solid var(--border-color)" }}>
                          {BACKENDS.find((x) => x.value === b.backend)?.label ?? b.backend}
                        </td>
                        <td style={{ padding: "4px 8px", borderBottom: "1px solid var(--border-color)" }}>{b.model}</td>
                        <td style={{ padding: "4px 8px", textAlign: "right", borderBottom: "1px solid var(--border-color)" }}>{b.ttft}</td>
                        <td style={{ padding: "4px 8px", textAlign: "right", borderBottom: "1px solid var(--border-color)" }}>{b.tokensPerSec}</td>
                        <td style={{ padding: "4px 8px", textAlign: "right", borderBottom: "1px solid var(--border-color)" }}>{b.vram}</td>
                        <td style={{ padding: "4px 8px", textAlign: "center", borderBottom: "1px solid var(--border-color)" }}>
                          <button
                            onClick={() => removeBenchmark(b.id)}
                            style={{ background: "none", border: "none", color: "var(--error-color)", cursor: "pointer", fontSize: 14 }}
                            title="Remove"
                          >
                            x
                          </button>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}

            {/* Comparison view */}
            {comparedEntries.length >= 2 && (
              <div style={{ marginTop: 20 }}>
                <div style={{ fontSize: 11, fontWeight: 600, marginBottom: 8, color: "var(--text-secondary)" }}>
                  Comparison ({comparedEntries.length} selected)
                </div>
                <div style={{ display: "grid", gridTemplateColumns: `repeat(${comparedEntries.length}, 1fr)`, gap: 12 }}>
                  {comparedEntries.map((b) => {
                    const bestTtft = Math.min(...comparedEntries.map((e) => e.ttft));
                    const bestTps = Math.max(...comparedEntries.map((e) => e.tokensPerSec));
                    const bestVram = Math.min(...comparedEntries.map((e) => e.vram));
                    return (
                      <div
                        key={b.id}
                        style={{
                          background: "var(--bg-secondary)",
                          border: "1px solid var(--border-color)",
                          borderRadius: 6,
                          padding: 12,
                        }}
                      >
                        <div style={{ fontWeight: 600, fontSize: 12, marginBottom: 8 }}>
                          {BACKENDS.find((x) => x.value === b.backend)?.label} — {b.model}
                        </div>
                        <div style={{ fontSize: 11, marginBottom: 4 }}>
                          <span style={{ opacity: 0.6 }}>TTFT: </span>
                          <span style={{ color: b.ttft === bestTtft ? "var(--success-color)" : "var(--text-primary)", fontWeight: b.ttft === bestTtft ? 700 : 400 }}>
                            {b.ttft} ms
                          </span>
                        </div>
                        <div style={{ fontSize: 11, marginBottom: 4 }}>
                          <span style={{ opacity: 0.6 }}>Tokens/s: </span>
                          <span style={{ color: b.tokensPerSec === bestTps ? "var(--success-color)" : "var(--text-primary)", fontWeight: b.tokensPerSec === bestTps ? 700 : 400 }}>
                            {b.tokensPerSec}
                          </span>
                        </div>
                        <div style={{ fontSize: 11 }}>
                          <span style={{ opacity: 0.6 }}>VRAM: </span>
                          <span style={{ color: b.vram === bestVram ? "var(--success-color)" : "var(--text-primary)", fontWeight: b.vram === bestVram ? 700 : 400 }}>
                            {b.vram} GB
                          </span>
                        </div>
                      </div>
                    );
                  })}
                </div>
              </div>
            )}
          </div>
        )}

        {/* ============= SCALING TAB ============= */}
        {tab === "scaling" && (
          <div style={{ maxWidth: 700 }}>
            {/* Auto-scale config */}
            <div style={{ marginBottom: 20 }}>
              <div style={{ fontSize: 13, fontWeight: 600, marginBottom: 12 }}>Auto-Scale Configuration</div>
              <div style={fieldRow}>
                <div>
                  <label style={labelStyle}>Min Replicas</label>
                  <input
                    type="number"
                    min={0}
                    value={scale.minReplicas}
                    onChange={(e) => setScale((s) => ({ ...s, minReplicas: Number(e.target.value) }))}
                    style={inputStyle}
                  />
                </div>
                <div>
                  <label style={labelStyle}>Max Replicas</label>
                  <input
                    type="number"
                    min={1}
                    value={scale.maxReplicas}
                    onChange={(e) => setScale((s) => ({ ...s, maxReplicas: Number(e.target.value) }))}
                    style={inputStyle}
                  />
                </div>
              </div>
              <div style={fieldRow}>
                <div>
                  <label style={labelStyle}>Target GPU Utilization (%)</label>
                  <input
                    type="number"
                    min={1}
                    max={100}
                    value={scale.targetGpuUtil}
                    onChange={(e) => setScale((s) => ({ ...s, targetGpuUtil: Number(e.target.value) }))}
                    style={inputStyle}
                  />
                </div>
                <div>
                  <label style={labelStyle}>Target Latency (ms)</label>
                  <input
                    type="number"
                    min={1}
                    value={scale.targetLatencyMs}
                    onChange={(e) => setScale((s) => ({ ...s, targetLatencyMs: Number(e.target.value) }))}
                    style={inputStyle}
                  />
                </div>
              </div>
              <div style={fieldRow}>
                <div>
                  <label style={labelStyle}>Scale-Up Cooldown (s)</label>
                  <input
                    type="number"
                    min={0}
                    value={scale.scaleUpCooldown}
                    onChange={(e) => setScale((s) => ({ ...s, scaleUpCooldown: Number(e.target.value) }))}
                    style={inputStyle}
                  />
                </div>
                <div>
                  <label style={labelStyle}>Scale-Down Cooldown (s)</label>
                  <input
                    type="number"
                    min={0}
                    value={scale.scaleDownCooldown}
                    onChange={(e) => setScale((s) => ({ ...s, scaleDownCooldown: Number(e.target.value) }))}
                    style={inputStyle}
                  />
                </div>
              </div>
            </div>

            {/* Load balancer config */}
            <div style={{ marginBottom: 20 }}>
              <div style={{ fontSize: 13, fontWeight: 600, marginBottom: 12 }}>Load Balancer</div>
              <div style={fieldRow}>
                <div>
                  <label style={labelStyle}>Strategy</label>
                  <select
                    value={lb.strategy}
                    onChange={(e) => setLb((l) => ({ ...l, strategy: e.target.value as LBStrategy }))}
                    style={selectStyle}
                  >
                    {LB_STRATEGIES.map((s) => (
                      <option key={s.value} value={s.value}>{s.label}</option>
                    ))}
                  </select>
                </div>
                <div>
                  <label style={labelStyle}>Health Check Path</label>
                  <input
                    value={lb.healthCheckPath}
                    onChange={(e) => setLb((l) => ({ ...l, healthCheckPath: e.target.value }))}
                    style={inputStyle}
                  />
                </div>
              </div>
              <div style={{ ...fieldRow, gridTemplateColumns: "1fr 1fr" }}>
                <div>
                  <label style={labelStyle}>Health Check Interval (s)</label>
                  <input
                    type="number"
                    min={1}
                    value={lb.healthCheckInterval}
                    onChange={(e) => setLb((l) => ({ ...l, healthCheckInterval: Number(e.target.value) }))}
                    style={inputStyle}
                  />
                </div>
                <div></div>
              </div>
            </div>

            {/* Generate K8s YAML */}
            <div style={{ display: "flex", gap: 8, marginBottom: 16 }}>
              <button
                style={btnPrimary}
                onClick={() => setGeneratedK8s(generateK8sYaml(deploy, scale, lb))}
              >
                Generate K8s YAML
              </button>
              {generatedK8s && (
                <button
                  style={{ ...btnSecondary, padding: "6px 10px", fontSize: 10 }}
                  onClick={() => navigator.clipboard.writeText(generatedK8s)}
                >
                  Copy
                </button>
              )}
            </div>

            {generatedK8s && (
              <pre style={codeBlock}>{generatedK8s}</pre>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
