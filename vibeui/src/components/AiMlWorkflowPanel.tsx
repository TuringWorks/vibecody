/**
 * AiMlWorkflowPanel — End-to-end AI/ML pipeline builder.
 *
 * Unified workflow panel that chains VibeCody's AI/ML modules into
 * configurable pipelines: data preparation → training → quantization →
 * inference → deployment → monitoring.
 *
 * Supports building agents from foundation models through to deployment
 * on cloud, edge, and IoT environments.
 */
import { useState, useMemo, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

// ── Types ────────────────────────────────────────────────────────────────

interface WorkflowStage {
  id: string;
  label: string;
  description: string;
  status: "pending" | "configuring" | "ready" | "running" | "complete" | "error";
  config: Record<string, string>;
}

type Tab = "pipeline" | "examples" | "deploy" | "monitor";

// ── Stage Definitions ────────────────────────────────────────────────────

const STAGE_TEMPLATES: Omit<WorkflowStage, "status" | "config">[] = [
  { id: "data", label: "1. Data Preparation", description: "Ingest documents, mine code, prepare datasets (ChatML/Alpaca/ShareGPT)" },
  { id: "rag", label: "2. RAG Pipeline", description: "Chunk, embed, and index documents into vector database (Qdrant/Pinecone/pgvector)" },
  { id: "finetune", label: "3. Fine-Tuning", description: "Fine-tune foundation models with LoRA/QLoRA on your data (OpenAI/Together/Fireworks/Local)" },
  { id: "training", label: "4. Distributed Training", description: "Full training runs with DeepSpeed/FSDP/Megatron across GPU clusters" },
  { id: "quantize", label: "5. Quantization", description: "Compress models: GPTQ, AWQ, GGUF, Int4/Int8 for efficient deployment" },
  { id: "evaluate", label: "6. Evaluation", description: "SWE-bench, custom benchmarks, A/B testing via Model Arena" },
  { id: "inference", label: "7. Inference Server", description: "Deploy with vLLM/TGI/Triton/llama.cpp — Docker, K8s, auto-scaling" },
  { id: "agent", label: "8. Agent Assembly", description: "Wire model into agent with tools (MCP/ACP), reasoning, and memory" },
  { id: "deploy", label: "9. Deployment", description: "Ship to cloud (AWS/GCP/Azure), edge (ONNX/TFLite), or IoT (llama.cpp)" },
  { id: "monitor", label: "10. Monitoring", description: "Cost tracking, latency metrics, drift detection, usage metering" },
];

// ── Example Workflows ────────────────────────────────────────────────────

interface ExampleWorkflow {
  title: string;
  description: string;
  difficulty: "beginner" | "intermediate" | "advanced";
  timeEstimate: string;
  stages: string[];
  steps: string[];
}

const EXAMPLE_WORKFLOWS: ExampleWorkflow[] = [
  {
    title: "RAG-Powered Code Assistant",
    description: "Build a codebase-aware assistant that answers questions about your project using retrieval-augmented generation.",
    difficulty: "beginner",
    timeEstimate: "30 min",
    stages: ["data", "rag", "agent"],
    steps: [
      "1. Run /init to detect your project structure",
      "2. Use /ingest to index your codebase into chunks",
      "3. Run /rag to build embeddings and store in the in-memory vector DB",
      "4. Configure the agent with @codebase: context for semantic search",
      "5. Chat with the agent — it retrieves relevant code before answering",
      "6. Export the RAG config for deployment with /bundle export",
    ],
  },
  {
    title: "Fine-Tune a Code Review Model",
    description: "Create a specialized code review model by fine-tuning on your team's PR review history.",
    difficulty: "intermediate",
    timeEstimate: "2 hours",
    stages: ["data", "finetune", "evaluate", "inference"],
    steps: [
      "1. Extract training data: /train dataset from-git --max-commits 5000",
      "2. Validate and split: /train dataset validate && /train dataset split 0.9",
      "3. Export in ChatML format: /train dataset export --format chatml",
      "4. Fine-tune via Together AI: /train finetune --provider together --model llama-3.1-8b",
      "5. Monitor training: /train status",
      "6. Evaluate on held-out set: /benchmark run --model fine-tuned-id",
      "7. Deploy: /inference deploy --backend vllm --model fine-tuned-id --gpu 1",
      "8. Wire into VibeCody: vibecli --provider openrouter --model your-model",
    ],
  },
  {
    title: "Edge-Deployed Reasoning Agent",
    description: "Quantize a model and deploy it on edge hardware with tool-calling via MCP.",
    difficulty: "advanced",
    timeEstimate: "4 hours",
    stages: ["finetune", "quantize", "inference", "agent", "deploy"],
    steps: [
      "1. Start with a fine-tuned model or use Llama 3.1 8B as base",
      "2. Quantize to GGUF Q4_K_M: /inference quantize --method gguf-q4km --model ./model",
      "3. Test locally: /inference deploy --backend llama.cpp --model model-q4.gguf",
      "4. Benchmark: /inference benchmark --model model-q4.gguf --prompts 100",
      "5. Create MCP server config for your tools (file access, API calls, DB queries)",
      "6. Wire agent: vibecli --provider ollama --model model-q4 --mcp-server ./tools.json",
      "7. Test agent end-to-end with real tasks",
      "8. Package as Docker container: docker build -t my-agent .",
      "9. Deploy to edge: docker run --gpus all -p 8080:8080 my-agent",
      "10. For IoT: cross-compile llama.cpp for ARM, deploy model + binary",
    ],
  },
  {
    title: "Multi-Agent Customer Support System",
    description: "Build a team of specialized agents (FAQ, escalation, technical) that collaborate on support tickets.",
    difficulty: "advanced",
    timeEstimate: "6 hours",
    stages: ["data", "rag", "finetune", "agent", "deploy", "monitor"],
    steps: [
      "1. Ingest knowledge base: /ingest ./docs --format markdown",
      "2. Build RAG index: /rag build --provider pgvector --collection support-kb",
      "3. Fine-tune FAQ model on historical tickets: /train finetune --dataset tickets.jsonl",
      "4. Create 3 agent configs in .vibecli/agents/:",
      "   - faq-agent.toml (trigger: common questions, model: fine-tuned FAQ)",
      "   - escalation-agent.toml (trigger: angry/urgent, model: claude-sonnet)",
      "   - technical-agent.toml (trigger: code/error, model: fine-tuned + RAG)",
      "5. Wire with /team create support-team --agents faq,escalation,technical",
      "6. Deploy as channel daemon: vibecli --channel-daemon slack",
      "7. Monitor: /metering status (track cost per agent, per ticket)",
      "8. Iterate: review traces, update training data, re-fine-tune monthly",
    ],
  },
  {
    title: "Quantized Model for IoT Deployment",
    description: "Take a 7B parameter model, quantize it to 2GB, and run it on a Raspberry Pi or Jetson Nano.",
    difficulty: "intermediate",
    timeEstimate: "1 hour",
    stages: ["quantize", "inference", "deploy"],
    steps: [
      "1. Download base model: ollama pull llama3.2:3b",
      "2. Export to GGUF: /inference export --model llama3.2:3b --format gguf",
      "3. Quantize aggressively: /inference quantize --method gguf-q4km --model llama3.2.gguf",
      "4. Verify size: ls -lh llama3.2-q4.gguf (should be ~1.5GB)",
      "5. Test locally: /inference deploy --backend llama.cpp --model llama3.2-q4.gguf",
      "6. Cross-compile llama.cpp for ARM: make LLAMA_CROSS=arm64",
      "7. Copy binary + model to device: scp llama-server model.gguf pi@device:",
      "8. Run on device: ./llama-server -m model.gguf --host 0.0.0.0 --port 8080",
      "9. Connect VibeCody: vibecli --provider ollama --model custom --api-url http://device:8080",
    ],
  },
];

// ── Deploy Targets ───────────────────────────────────────────────────────

interface DeployTarget {
  name: string;
  category: "cloud" | "edge" | "iot" | "hybrid";
  description: string;
  command: string;
}

const DEPLOY_TARGETS: DeployTarget[] = [
  { name: "AWS SageMaker", category: "cloud", description: "Managed inference with auto-scaling", command: "/cloud deploy --provider aws --service sagemaker" },
  { name: "GCP Vertex AI", category: "cloud", description: "Google Cloud managed ML", command: "/cloud deploy --provider gcp --service vertex" },
  { name: "Azure ML", category: "cloud", description: "Azure managed inference endpoints", command: "/cloud deploy --provider azure --service azureml" },
  { name: "Docker (self-hosted)", category: "cloud", description: "Any server with Docker and GPU", command: "/inference deploy --backend vllm --docker" },
  { name: "Kubernetes", category: "cloud", description: "K8s with GPU node pool", command: "/inference deploy --backend vllm --k8s" },
  { name: "NVIDIA Jetson", category: "edge", description: "Edge GPU (Nano/Orin) with llama.cpp", command: "/inference deploy --backend llama.cpp --target jetson" },
  { name: "Raspberry Pi", category: "iot", description: "ARM64 with quantized GGUF model", command: "/inference deploy --backend llama.cpp --target arm64" },
  { name: "ONNX Runtime", category: "edge", description: "Cross-platform inference (CPU/GPU/NPU)", command: "/inference deploy --backend onnxruntime --model model.onnx" },
  { name: "TensorFlow Lite", category: "iot", description: "Mobile and microcontroller deployment", command: "/inference export --format tflite --model model" },
  { name: "WebAssembly", category: "edge", description: "In-browser inference via wasm-llm", command: "/inference export --format wasm --model model-q4.gguf" },
  { name: "Lambda/Serverless", category: "cloud", description: "AWS Lambda with ONNX Runtime", command: "/cloud deploy --provider aws --service lambda --runtime onnx" },
  { name: "Fly.io", category: "cloud", description: "Edge-distributed GPU containers", command: "/inference deploy --backend vllm --platform fly" },
];

// ── Styles ───────────────────────────────────────────────────────────────

const headingStyle: React.CSSProperties = { margin: "0 0 4px", fontSize: "var(--font-size-xl)", fontWeight: 600 };
const badgeStyle = (color: string): React.CSSProperties => ({ display: "inline-block", padding: "2px 8px", borderRadius: "var(--radius-md)", fontSize: "var(--font-size-xs)", fontWeight: 600, color: "var(--btn-primary-fg)", background: color });

// ── Component ────────────────────────────────────────────────────────────

export function AiMlWorkflowPanel() {
  const [tab, setTab] = useState<Tab>("pipeline");
  const [stages, setStages] = useState<WorkflowStage[]>(
    STAGE_TEMPLATES.map(s => ({ ...s, status: "pending", config: {} }))
  );
  const [selectedStage, setSelectedStage] = useState<string | null>(null);

  // Load persisted pipeline config on mount
  useEffect(() => {
    invoke<WorkflowStage[]>("get_aiml_pipeline_config").then(saved => {
      if (Array.isArray(saved) && saved.length > 0) setStages(saved);
    }).catch(() => {});
  }, []);
  const [expandedExample, setExpandedExample] = useState<number | null>(null);
  const [deployFilter, setDeployFilter] = useState<string>("all");

  const activeStages = stages.filter(s => s.status !== "pending");
  const completedCount = stages.filter(s => s.status === "complete").length;

  const filteredTargets = useMemo(() => {
    if (deployFilter === "all") return DEPLOY_TARGETS;
    return DEPLOY_TARGETS.filter(t => t.category === deployFilter);
  }, [deployFilter]);

  const toggleStage = (id: string) => {
    setStages(prev => {
      const updated = prev.map(s =>
        s.id === id ? { ...s, status: s.status === "pending" ? "configuring" as const : "pending" as const } : s
      );
      invoke("save_aiml_pipeline_config", { config: updated }).catch(() => {});
      return updated;
    });
  };

  const difficultyColor = (d: string) => {
    if (d === "beginner") return "var(--accent-green)";
    if (d === "intermediate") return "var(--warning-color, #ff9800)";
    return "var(--accent-rose)";
  };

  const categoryColor = (c: string) => {
    if (c === "cloud") return "var(--accent-primary, #7c6aef)";
    if (c === "edge") return "var(--warning-color, #ff9800)";
    if (c === "iot") return "var(--accent-green)";
    return "var(--text-secondary)";
  };

  return (
    <div className="panel-container">
      <h2 style={headingStyle}>AI/ML Workflow Builder</h2>
      <p className="panel-label" style={{ marginBottom: 12 }}>
        End-to-end pipeline: data prep, training, quantization, inference, agent assembly, deployment
      </p>

      {/* Tabs */}
      <div className="panel-tab-bar" style={{ marginBottom: 12 }}>
        {(["pipeline", "examples", "deploy", "monitor"] as Tab[]).map(t => (
          <button key={t} className={`panel-tab ${tab === t ? "active" : ""}`} onClick={() => setTab(t)}>
            {t === "pipeline" ? `Pipeline (${activeStages.length}/${stages.length})` :
             t === "examples" ? `Examples (${EXAMPLE_WORKFLOWS.length})` :
             t === "deploy" ? `Deploy (${DEPLOY_TARGETS.length} targets)` : "Monitor"}
          </button>
        ))}
      </div>

      {/* ── PIPELINE TAB ──────────────────────────────────────────────── */}
      {tab === "pipeline" && (
        <div>
          <div className="panel-card" style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
            <span>{completedCount}/{stages.length} stages complete</span>
            <div style={{ display: "flex", gap: 4 }}>
              {stages.map(s => (
                <div key={s.id} style={{
                  width: 20, height: 6, borderRadius: 3,
                  background: s.status === "complete" ? "var(--success-color)" :
                    s.status === "running" ? "var(--accent-primary)" :
                    s.status === "configuring" || s.status === "ready" ? "var(--warning-color)" :
                    s.status === "error" ? "var(--error-color)" : "var(--border-color)",
                }} />
              ))}
            </div>
          </div>

          {stages.map(stage => (
            <div key={stage.id} className="panel-card" style={{
              borderLeft: `3px solid ${
                stage.status === "complete" ? "var(--success-color)" :
                stage.status === "configuring" ? "var(--warning-color)" :
                stage.status === "pending" ? "var(--border-color)" : "var(--accent-primary)"
              }`,
              opacity: stage.status === "pending" ? 0.6 : 1,
            }}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start" }}>
                <div style={{ flex: 1, cursor: "pointer" }} onClick={() => setSelectedStage(selectedStage === stage.id ? null : stage.id)}>
                  <div style={{ fontWeight: 600 }}>{stage.label}</div>
                  <div className="panel-label">{stage.description}</div>
                </div>
                <div style={{ display: "flex", gap: 4, alignItems: "center" }}>
                  <button className="panel-btn panel-btn-secondary" onClick={() => toggleStage(stage.id)}>
                    {stage.status === "pending" ? "Enable" : "Disable"}
                  </button>
                </div>
              </div>

              {/* Expanded config for selected stage */}
              {selectedStage === stage.id && stage.status !== "pending" && (
                <div style={{ marginTop: 10, padding: 10, background: "var(--bg-tertiary)", borderRadius: "var(--radius-xs-plus)", fontSize: "var(--font-size-base)" }}>
                  {stage.id === "data" && (
                    <div>
                      <div className="panel-label">Data Sources</div>
                      <div style={{ display: "flex", gap: 6, flexWrap: "wrap", marginBottom: 8 }}>
                        {["Codebase files", "Git history", "Agent traces", "Documents", "CSV/JSON", "PDF (scientific)"].map(s => (
                          <span key={s} className="panel-btn panel-btn-secondary" style={{ fontSize: "var(--font-size-sm)" }}>{s}</span>
                        ))}
                      </div>
                      <div className="panel-label">Document Processors</div>
                      <div style={{ display: "flex", gap: 6, flexWrap: "wrap", marginBottom: 8 }}>
                        {["MinerU (PDF to MD)", "Docling (IBM)", "Unstructured", "LlamaParse", "VibeCody Built-in"].map(p => (
                          <span key={p} className="panel-btn panel-btn-secondary" style={{ fontSize: "var(--font-size-sm)" }}>{p}</span>
                        ))}
                      </div>
                      <div style={{ marginTop: 8, color: "var(--text-secondary)", fontSize: "var(--font-size-sm)" }}>
                        <div>CLI: <code>/ingest ./docs</code> | <code>/train dataset from-codebase</code> | <code>/train dataset from-git</code></div>
                        <div style={{ marginTop: 4 }}>MinerU: <code>magic-pdf -p input.pdf -o output/ -m auto</code></div>
                      </div>
                    </div>
                  )}
                  {stage.id === "rag" && (
                    <div>
                      <div className="panel-label">Vector DB Provider</div>
                      <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
                        {["In-Memory", "Qdrant", "Pinecone", "pgvector", "Milvus", "Weaviate", "Chroma"].map(p => (
                          <span key={p} className="panel-btn panel-btn-secondary" style={{ fontSize: "var(--font-size-sm)" }}>{p}</span>
                        ))}
                      </div>
                      <div style={{ marginTop: 8, color: "var(--text-secondary)" }}>
                        CLI: <code>/rag build</code> | <code>/rag search "query"</code>
                      </div>
                    </div>
                  )}
                  {stage.id === "finetune" && (
                    <div>
                      <div className="panel-label">Fine-Tuning Libraries</div>
                      <div style={{ display: "flex", gap: 6, flexWrap: "wrap", marginBottom: 8 }}>
                        {["Unsloth", "Axolotl", "LLaMA Factory", "DeepSpeed", "HuggingFace TRL", "PEFT"].map(p => (
                          <span key={p} className="panel-btn panel-btn-secondary" style={{ fontSize: "var(--font-size-sm)" }}>{p}</span>
                        ))}
                      </div>
                      <div className="panel-label">Cloud Providers</div>
                      <div style={{ display: "flex", gap: 6, flexWrap: "wrap", marginBottom: 8 }}>
                        {["OpenAI", "Together AI", "Fireworks", "Local (LoRA/QLoRA)"].map(p => (
                          <span key={p} className="panel-btn panel-btn-secondary" style={{ fontSize: "var(--font-size-sm)" }}>{p}</span>
                        ))}
                      </div>
                      <div className="panel-label">Notebook Environments</div>
                      <div style={{ display: "flex", gap: 6, flexWrap: "wrap", marginBottom: 8 }}>
                        {["Google Colab (free T4)", "Kaggle (free P100)", "SageMaker Studio", "Lightning AI", "Local Jupyter"].map(e => (
                          <span key={e} className="panel-btn panel-btn-secondary" style={{ fontSize: "var(--font-size-sm)" }}>{e}</span>
                        ))}
                      </div>
                      <div className="panel-label">Alignment Methods: SFT, DPO, PPO, RLHF, KTO</div>
                      <div className="panel-label">Formats: ChatML, Alpaca, ShareGPT, Completion</div>
                      <div style={{ marginTop: 8, color: "var(--text-secondary)" }}>
                        CLI: <code>/train finetune --library unsloth --model llama-3.1-8b</code>
                      </div>
                    </div>
                  )}
                  {stage.id === "training" && (
                    <div>
                      <div className="panel-label">Framework</div>
                      <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
                        {["DeepSpeed", "FSDP", "Megatron", "Horovod", "Ray Train", "Colossal AI"].map(f => (
                          <span key={f} className="panel-btn panel-btn-secondary" style={{ fontSize: "var(--font-size-sm)" }}>{f}</span>
                        ))}
                      </div>
                      <div style={{ marginTop: 6 }}>
                        <div className="panel-label">Parallelism: Data | Tensor | Pipeline | Expert | Sequence</div>
                        <div className="panel-label">Precision: FP32, FP16, BF16, FP8</div>
                      </div>
                      <div style={{ marginTop: 8, color: "var(--text-secondary)" }}>
                        CLI: <code>/train run --framework deepspeed --gpus 4</code>
                      </div>
                    </div>
                  )}
                  {stage.id === "quantize" && (
                    <div>
                      <div className="panel-label">Quantization Method</div>
                      <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
                        {["FP16", "BF16", "Int8", "Int4", "GPTQ", "AWQ", "SqueezeLLM", "GGUF-Q4", "GGUF-Q5"].map(q => (
                          <span key={q} className="panel-btn panel-btn-secondary" style={{ fontSize: "var(--font-size-sm)" }}>{q}</span>
                        ))}
                      </div>
                      <div style={{ marginTop: 8, color: "var(--text-secondary)" }}>
                        CLI: <code>/inference quantize --method gguf-q4km --model ./model</code>
                      </div>
                    </div>
                  )}
                  {stage.id === "inference" && (
                    <div>
                      <div className="panel-label">Inference Backend</div>
                      <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
                        {["vLLM", "TGI", "Triton", "llama.cpp", "Ollama", "TorchServe", "ONNX Runtime", "TRT-LLM"].map(b => (
                          <span key={b} className="panel-btn panel-btn-secondary" style={{ fontSize: "var(--font-size-sm)" }}>{b}</span>
                        ))}
                      </div>
                      <div style={{ marginTop: 8, color: "var(--text-secondary)" }}>
                        CLI: <code>/inference deploy --backend vllm --model ./model --gpu 1</code>
                      </div>
                    </div>
                  )}
                  {stage.id === "agent" && (
                    <div>
                      <div className="panel-label">Agent Capabilities</div>
                      <div style={{ display: "flex", gap: 6, flexWrap: "wrap", marginBottom: 8 }}>
                        {["Tool Calling (MCP)", "Agent Protocol (ACP)", "RAG Context", "Memory", "Multi-Agent Teams", "Reasoning (think tool)"].map(c => (
                          <span key={c} className="panel-btn panel-btn-secondary" style={{ fontSize: "var(--font-size-sm)" }}>{c}</span>
                        ))}
                      </div>
                      <div className="panel-label">RL Training Environments</div>
                      <div style={{ display: "flex", gap: 6, flexWrap: "wrap", marginBottom: 8 }}>
                        {["NeMo Gym (60+ tasks)", "OpenAI Gymnasium", "Reasoning Gym", "SWE-Bench", "TRL PPO (RLHF)", "Aviary (tool-use)", "LMSYS Arena"].map(e => (
                          <span key={e} className="panel-btn panel-btn-secondary" style={{ fontSize: "var(--font-size-sm)" }}>{e}</span>
                        ))}
                      </div>
                      <div style={{ marginTop: 8, color: "var(--text-secondary)", fontSize: "var(--font-size-sm)" }}>
                        <div>CLI: <code>/agent "task"</code> | <code>/team create my-team</code></div>
                        <div style={{ marginTop: 4 }}>RL: <code>/benchmark run --env nemo-gym --tasks gpqa,coding</code></div>
                      </div>
                    </div>
                  )}
                  {stage.id === "deploy" && (
                    <div>
                      <div className="panel-label">See Deploy tab for {DEPLOY_TARGETS.length} deployment targets</div>
                      <button className="panel-btn panel-btn-secondary" style={{ marginTop: 4 }} onClick={() => setTab("deploy")}>View Deploy Targets</button>
                    </div>
                  )}
                  {(stage.id === "evaluate" || stage.id === "monitor") && (
                    <div>
                      <div style={{ color: "var(--text-secondary)" }}>
                        CLI: <code>/benchmark run</code> | <code>/metering status</code> | <code>/cost</code>
                      </div>
                    </div>
                  )}
                </div>
              )}
            </div>
          ))}
        </div>
      )}

      {/* ── EXAMPLES TAB ──────────────────────────────────────────────── */}
      {tab === "examples" && (
        <div>
          <div className="panel-card" style={{ fontSize: "var(--font-size-base)" }}>
            {EXAMPLE_WORKFLOWS.length} end-to-end examples you can try today. Each uses existing VibeCody commands.
          </div>

          {EXAMPLE_WORKFLOWS.map((ex, idx) => (
            <div key={idx} className="panel-card" style={{ cursor: "pointer" }} onClick={() => setExpandedExample(expandedExample === idx ? null : idx)}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start" }}>
                <div style={{ flex: 1 }}>
                  <div style={{ fontWeight: 600, fontSize: "var(--font-size-lg)" }}>{ex.title}</div>
                  <div className="panel-label">{ex.description}</div>
                  <div style={{ display: "flex", gap: 8, marginTop: 6 }}>
                    <span style={badgeStyle(difficultyColor(ex.difficulty))}>{ex.difficulty}</span>
                    <span style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>{ex.timeEstimate}</span>
                    <span style={{ fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>{ex.stages.length} stages</span>
                  </div>
                </div>
                <span style={{ fontSize: 16, color: "var(--text-secondary)" }}>{expandedExample === idx ? "v" : ">"}</span>
              </div>

              {expandedExample === idx && (
                <div style={{ marginTop: 12, padding: 10, background: "var(--bg-tertiary)", borderRadius: "var(--radius-xs-plus)" }}>
                  <div style={{ fontSize: "var(--font-size-sm)", fontWeight: 600, marginBottom: 6, color: "var(--text-secondary)", textTransform: "uppercase" }}>
                    Stages: {ex.stages.join(" > ")}
                  </div>
                  <div style={{ fontSize: "var(--font-size-base)" }}>
                    {ex.steps.map((step, si) => (
                      <div key={si} style={{
                        padding: "4px 0",
                        borderBottom: si < ex.steps.length - 1 ? "1px solid var(--border-color)" : "none",
                        fontFamily: step.trimStart().startsWith("/") || step.includes("vibecli") || step.includes("docker") || step.includes("ollama") || step.includes("make") || step.includes("scp") ? "var(--font-mono, monospace)" : "inherit",
                        color: step.trimStart().startsWith("/") || step.includes("--") ? "var(--accent-primary, #7c6aef)" : "var(--text-primary)",
                      }}>
                        {step}
                      </div>
                    ))}
                  </div>
                </div>
              )}
            </div>
          ))}
        </div>
      )}

      {/* ── DEPLOY TAB ────────────────────────────────────────────────── */}
      {tab === "deploy" && (
        <div>
          <div style={{ display: "flex", gap: 4, marginBottom: 10, flexWrap: "wrap" }}>
            {["all", "cloud", "edge", "iot"].map(f => (
              <button key={f} className={`panel-btn ${deployFilter === f ? "panel-btn-primary" : "panel-btn-secondary"}`} onClick={() => setDeployFilter(f)}>
                {f === "all" ? `All (${DEPLOY_TARGETS.length})` : `${f.charAt(0).toUpperCase() + f.slice(1)} (${DEPLOY_TARGETS.filter(t => t.category === f).length})`}
              </button>
            ))}
          </div>

          {filteredTargets.map((target, idx) => (
            <div key={idx} className="panel-card" style={{ borderLeft: `3px solid ${categoryColor(target.category)}` }}>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start" }}>
                <div style={{ flex: 1 }}>
                  <div style={{ fontWeight: 600 }}>
                    {target.name}
                    <span style={{ ...badgeStyle(categoryColor(target.category)), marginLeft: 8 }}>{target.category}</span>
                  </div>
                  <div className="panel-label">{target.description}</div>
                  <code style={{ fontSize: "var(--font-size-sm)", color: "var(--accent-primary, #7c6aef)" }}>{target.command}</code>
                </div>
              </div>
            </div>
          ))}
        </div>
      )}

      {/* ── MONITOR TAB ───────────────────────────────────────────────── */}
      {tab === "monitor" && (
        <div>
          <div className="panel-card">
            <div className="panel-label">Monitoring Commands</div>
            <div style={{ display: "flex", flexDirection: "column", gap: 6, fontSize: "var(--font-size-base)" }}>
              <div><code>/metering status</code> — Token usage, costs, budgets per provider</div>
              <div><code>/cost</code> — Current session cost breakdown</div>
              <div><code>/benchmark run</code> — SWE-bench performance evaluation</div>
              <div><code>/benchmark compare</code> — Compare model runs side-by-side</div>
              <div><code>/arena compare provider1 provider2</code> — Blind A/B model comparison</div>
              <div><code>/trace</code> — Agent session traces with timing data</div>
              <div><code>/vulnscan scan</code> — Security scan deployed agents</div>
            </div>
          </div>

          <div className="panel-card">
            <div className="panel-label">VibeUI Panels for Monitoring</div>
            <div style={{ display: "flex", flexDirection: "column", gap: 4, fontSize: "var(--font-size-base)" }}>
              <div><strong>Cost Observatory</strong> — Per-provider spend, budget alerts, historical trends</div>
              <div><strong>Model Arena</strong> — Blind A/B testing with ELO rankings</div>
              <div><strong>Usage Metering</strong> — Credit budgets, team allocation, chargeback reports</div>
              <div><strong>SWE-bench</strong> — Benchmark runs with pass@1 tracking</div>
              <div><strong>Traces</strong> — Full agent session replay with tool call timeline</div>
              <div><strong>Health Monitor</strong> — Service health checks and uptime tracking</div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
