/**
 * ModelWizardPanel — Step-by-step wizard for fine-tuning models and deploying inference.
 *
 * 7-step guided workflow:
 *   1. Choose Base Model
 *   2. Prepare Dataset
 *   3. Configure Fine-Tuning
 *   4. Select Environment
 *   5. Quantize Model
 *   6. Deploy Inference
 *   7. Review & Launch
 *
 * Each step collects config, validates inputs, and generates runnable
 * commands/scripts. The final step produces a complete deployment package.
 */
import { useState, useCallback } from "react";

// ── Wizard State ─────────────────────────────────────────────────────────

interface WizardConfig {
  // Step 1: Base Model
  baseModel: string;
  modelSize: string;
  modelSource: string;
  // Step 2: Dataset
  dataSource: string;
  dataPath: string;
  dataFormat: string;
  docProcessor: string;
  // Step 3: Fine-Tuning
  library: string;
  method: string;
  loraRank: number;
  epochs: number;
  batchSize: number;
  learningRate: string;
  alignment: string;
  // Step 4: Environment
  environment: string;
  gpuCount: number;
  // Step 5: Quantization
  quantMethod: string;
  skipQuantize: boolean;
  // Step 6: Deployment
  inferenceBackend: string;
  deployTarget: string;
  port: number;
  autoScale: boolean;
  maxReplicas: number;
}

const DEFAULT_CONFIG: WizardConfig = {
  baseModel: "meta-llama/Llama-3.1-8B-Instruct",
  modelSize: "8B",
  modelSource: "huggingface",
  dataSource: "codebase",
  dataPath: "./data/train.jsonl",
  dataFormat: "chatml",
  docProcessor: "builtin",
  library: "unsloth",
  method: "qlora",
  loraRank: 16,
  epochs: 3,
  batchSize: 4,
  learningRate: "2e-4",
  alignment: "sft",
  environment: "colab",
  gpuCount: 1,
  quantMethod: "gguf-q4km",
  skipQuantize: false,
  inferenceBackend: "ollama",
  deployTarget: "docker",
  port: 8080,
  autoScale: false,
  maxReplicas: 3,
};

const STEPS = [
  { id: 1, label: "Base Model", icon: "1" },
  { id: 2, label: "Dataset", icon: "2" },
  { id: 3, label: "Fine-Tune", icon: "3" },
  { id: 4, label: "Environment", icon: "4" },
  { id: 5, label: "Quantize", icon: "5" },
  { id: 6, label: "Deploy", icon: "6" },
  { id: 7, label: "Review", icon: "7" },
];

// ── Styles ───────────────────────────────────────────────────────────────

const panelStyle: React.CSSProperties = { display: "flex", flexDirection: "column", height: "100%", color: "var(--text-primary)", fontFamily: "var(--font-family)", fontSize: 13, background: "var(--bg-primary)" };
const cardStyle: React.CSSProperties = { background: "var(--bg-secondary)", borderRadius: 6, padding: 12, marginBottom: 10, border: "1px solid var(--border-color)" };
const labelStyle: React.CSSProperties = { fontSize: 11, color: "var(--text-secondary)", marginBottom: 4, display: "block" };
const inputStyle: React.CSSProperties = { width: "100%", padding: "6px 10px", borderRadius: 4, border: "1px solid var(--border-color)", background: "var(--bg-tertiary)", color: "var(--text-primary)", fontSize: 12, fontFamily: "var(--font-family)", boxSizing: "border-box" as const };
const selectStyle: React.CSSProperties = { ...inputStyle, cursor: "pointer" };
const btnPrimary: React.CSSProperties = { padding: "8px 20px", borderRadius: 4, border: "none", background: "var(--accent-primary, #7c6aef)", color: "var(--btn-primary-fg)", cursor: "pointer", fontSize: 13, fontWeight: 600 };
const btnSecondary: React.CSSProperties = { padding: "8px 20px", borderRadius: 4, border: "1px solid var(--border-color)", background: "transparent", color: "var(--text-primary)", cursor: "pointer", fontSize: 13 };
const optionBtn = (active: boolean): React.CSSProperties => ({
  padding: "8px 14px", borderRadius: 6, cursor: "pointer", fontSize: 12, textAlign: "left" as const,
  border: `1px solid ${active ? "var(--accent-primary, #7c6aef)" : "var(--border-color)"}`,
  background: active ? "rgba(124,106,239,0.12)" : "var(--bg-tertiary)",
  color: active ? "var(--accent-primary, #7c6aef)" : "var(--text-primary)",
  fontWeight: active ? 600 : 400,
});
const codeBlock: React.CSSProperties = { background: "var(--bg-tertiary)", padding: 12, borderRadius: 4, fontSize: 11, fontFamily: "var(--font-mono, monospace)", whiteSpace: "pre-wrap" as const, overflow: "auto", maxHeight: 300, border: "1px solid var(--border-color)" };

// ── Component ────────────────────────────────────────────────────────────

export function ModelWizardPanel() {
  const [step, setStep] = useState(1);
  const [config, setConfig] = useState<WizardConfig>({ ...DEFAULT_CONFIG });
  const [copied, setCopied] = useState(false);

  const set = useCallback(<K extends keyof WizardConfig>(key: K, value: WizardConfig[K]) => {
    setConfig(prev => ({ ...prev, [key]: value }));
  }, []);

  const next = () => setStep(s => Math.min(s + 1, 7));
  const prev = () => setStep(s => Math.max(s - 1, 1));
  const goTo = (s: number) => setStep(s);

  const copyScript = (text: string) => {
    navigator.clipboard.writeText(text);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  // ── Script Generation ──────────────────────────────────────────────

  const generateFullScript = (): string => {
    const lines: string[] = [];
    lines.push("#!/bin/bash");
    lines.push("# VibeCody Model Wizard — Generated Script");
    lines.push(`# Model: ${config.baseModel}`);
    lines.push(`# Library: ${config.library} | Method: ${config.method}`);
    lines.push(`# Environment: ${config.environment} | GPUs: ${config.gpuCount}`);
    lines.push("");

    // Step 1: Environment setup
    lines.push("# === Step 1: Environment Setup ===");
    if (config.environment === "colab") {
      lines.push("# Run in Google Colab (free T4 GPU)");
    }
    const pipPkgs: Record<string, string> = {
      unsloth: "unsloth", axolotl: "axolotl", "llama-factory": "llamafactory",
      deepspeed: "deepspeed", trl: "trl", peft: "peft",
    };
    lines.push(`pip install ${pipPkgs[config.library] || config.library} transformers datasets accelerate bitsandbytes`);
    lines.push("");

    // Step 2: Data preparation
    lines.push("# === Step 2: Data Preparation ===");
    if (config.dataSource === "codebase") {
      lines.push(`# Extract training data from codebase`);
      lines.push(`vibecli /train dataset from-codebase --format ${config.dataFormat} --output ${config.dataPath}`);
    } else if (config.dataSource === "git") {
      lines.push(`vibecli /train dataset from-git --max-commits 5000 --format ${config.dataFormat} --output ${config.dataPath}`);
    } else if (config.dataSource === "documents") {
      if (config.docProcessor === "mineru") {
        lines.push(`magic-pdf -p ./documents/ -o ./parsed/ -m auto`);
      }
      lines.push(`vibecli /ingest ./parsed/ --output ${config.dataPath}`);
    } else {
      lines.push(`# Using existing dataset: ${config.dataPath}`);
    }
    lines.push(`vibecli /train dataset validate --file ${config.dataPath}`);
    lines.push("");

    // Step 3: Fine-tuning
    lines.push("# === Step 3: Fine-Tuning ===");
    if (config.library === "unsloth") {
      lines.push(`python -c "`);
      lines.push(`from unsloth import FastLanguageModel`);
      lines.push(`model, tokenizer = FastLanguageModel.from_pretrained('${config.baseModel}', max_seq_length=2048, load_in_4bit=True)`);
      lines.push(`model = FastLanguageModel.get_peft_model(model, r=${config.loraRank}, lora_alpha=${config.loraRank})`);
      lines.push(`from trl import SFTTrainer`);
      lines.push(`from transformers import TrainingArguments`);
      lines.push(`from datasets import load_dataset`);
      lines.push(`dataset = load_dataset('json', data_files='${config.dataPath}')`);
      lines.push(`trainer = SFTTrainer(model=model, tokenizer=tokenizer, train_dataset=dataset['train'],`);
      lines.push(`    args=TrainingArguments(output_dir='./output', per_device_train_batch_size=${config.batchSize},`);
      lines.push(`        num_train_epochs=${config.epochs}, learning_rate=${config.learningRate}))`);
      lines.push(`trainer.train()`);
      lines.push(`model.save_pretrained('./output')`);
      lines.push(`"`);
    } else if (config.library === "axolotl") {
      lines.push(`cat > axolotl.yaml << 'YAML'`);
      lines.push(`base_model: ${config.baseModel}`);
      lines.push(`datasets:`);
      lines.push(`  - path: ${config.dataPath}`);
      lines.push(`    type: ${config.dataFormat}`);
      lines.push(`output_dir: ./output`);
      lines.push(`lora_r: ${config.loraRank}`);
      lines.push(`micro_batch_size: ${config.batchSize}`);
      lines.push(`num_epochs: ${config.epochs}`);
      lines.push(`learning_rate: ${config.learningRate}`);
      lines.push(`YAML`);
      lines.push(`axolotl train axolotl.yaml`);
    } else if (config.library === "llama-factory") {
      lines.push(`llamafactory-cli train \\`);
      lines.push(`  --model_name_or_path ${config.baseModel} \\`);
      lines.push(`  --dataset ${config.dataPath} \\`);
      lines.push(`  --output_dir ./output \\`);
      lines.push(`  --finetuning_type ${config.method === "full" ? "full" : "lora"} \\`);
      lines.push(`  --lora_rank ${config.loraRank} \\`);
      lines.push(`  --num_train_epochs ${config.epochs} \\`);
      lines.push(`  --per_device_train_batch_size ${config.batchSize} \\`);
      lines.push(`  --learning_rate ${config.learningRate}`);
    } else if (config.library === "trl") {
      lines.push(`python -c "`);
      lines.push(`from trl import ${config.alignment === "dpo" ? "DPOTrainer, DPOConfig" : "SFTTrainer, SFTConfig"}`);
      lines.push(`from transformers import AutoModelForCausalLM, AutoTokenizer`);
      lines.push(`from datasets import load_dataset`);
      lines.push(`model = AutoModelForCausalLM.from_pretrained('${config.baseModel}')`);
      lines.push(`tokenizer = AutoTokenizer.from_pretrained('${config.baseModel}')`);
      lines.push(`dataset = load_dataset('json', data_files='${config.dataPath}')`);
      lines.push(`trainer = ${config.alignment === "dpo" ? "DPOTrainer" : "SFTTrainer"}(`);
      lines.push(`    model=model, tokenizer=tokenizer, train_dataset=dataset['train'],`);
      lines.push(`    args=${config.alignment === "dpo" ? "DPOConfig" : "SFTConfig"}(output_dir='./output', num_train_epochs=${config.epochs}))`);
      lines.push(`trainer.train()`);
      lines.push(`"`);
    } else {
      lines.push(`# ${config.library} training — see docs for full setup`);
      lines.push(`vibecli /train finetune --library ${config.library} --model ${config.baseModel} --dataset ${config.dataPath}`);
    }
    lines.push("");

    // Step 4: Quantization
    if (!config.skipQuantize) {
      lines.push("# === Step 4: Quantization ===");
      if (config.quantMethod.startsWith("gguf")) {
        lines.push(`# Convert to GGUF format`);
        lines.push(`python llama.cpp/convert_hf_to_gguf.py ./output --outfile model.gguf`);
        lines.push(`./llama.cpp/llama-quantize model.gguf model-${config.quantMethod}.gguf ${config.quantMethod.replace("gguf-", "").toUpperCase()}`);
      } else {
        lines.push(`vibecli /inference quantize --method ${config.quantMethod} --model ./output --output ./quantized`);
      }
      lines.push("");
    }

    // Step 5: Deploy
    lines.push("# === Step 5: Deploy Inference ===");
    const modelPath = config.skipQuantize ? "./output" : (config.quantMethod.startsWith("gguf") ? `model-${config.quantMethod}.gguf` : "./quantized");

    if (config.inferenceBackend === "ollama") {
      lines.push(`# Create Ollama Modelfile`);
      lines.push(`cat > Modelfile << 'EOF'`);
      lines.push(`FROM ${modelPath}`);
      lines.push(`PARAMETER temperature 0.7`);
      lines.push(`PARAMETER top_p 0.9`);
      lines.push(`SYSTEM "You are a helpful AI assistant."`);
      lines.push(`EOF`);
      lines.push(`ollama create my-model -f Modelfile`);
      lines.push(`ollama run my-model "Hello, test the model"`);
    } else if (config.inferenceBackend === "vllm") {
      lines.push(`python -m vllm.entrypoints.openai.api_server \\`);
      lines.push(`  --model ${modelPath} \\`);
      lines.push(`  --port ${config.port} \\`);
      lines.push(`  --tensor-parallel-size ${config.gpuCount}`);
    } else if (config.inferenceBackend === "llamacpp") {
      lines.push(`./llama.cpp/llama-server \\`);
      lines.push(`  -m ${modelPath} \\`);
      lines.push(`  --host 0.0.0.0 --port ${config.port} \\`);
      lines.push(`  -ngl 99  # offload all layers to GPU`);
    } else if (config.inferenceBackend === "tgi") {
      lines.push(`docker run --gpus all -p ${config.port}:80 \\`);
      lines.push(`  -v $(pwd)/${modelPath}:/model \\`);
      lines.push(`  ghcr.io/huggingface/text-generation-inference:latest \\`);
      lines.push(`  --model-id /model`);
    }
    lines.push("");

    // Step 6: Docker packaging
    if (config.deployTarget === "docker" || config.deployTarget === "k8s") {
      lines.push("# === Step 6: Package as Docker Container ===");
      lines.push(`cat > Dockerfile << 'EOF'`);
      if (config.inferenceBackend === "llamacpp") {
        lines.push(`FROM ghcr.io/ggerganov/llama.cpp:server`);
        lines.push(`COPY ${modelPath} /model.gguf`);
        lines.push(`ENTRYPOINT ["/llama-server", "-m", "/model.gguf", "--host", "0.0.0.0", "--port", "${config.port}"]`);
      } else if (config.inferenceBackend === "vllm") {
        lines.push(`FROM vllm/vllm-openai:latest`);
        lines.push(`COPY ${modelPath} /model`);
        lines.push(`ENTRYPOINT ["python", "-m", "vllm.entrypoints.openai.api_server", "--model", "/model", "--port", "${config.port}"]`);
      } else {
        lines.push(`FROM python:3.11-slim`);
        lines.push(`COPY ${modelPath} /model`);
        lines.push(`CMD ["python", "-m", "your_server", "--model", "/model"]`);
      }
      lines.push(`EOF`);
      lines.push(`docker build -t my-model-server .`);
      lines.push(`docker run --gpus all -p ${config.port}:${config.port} my-model-server`);
    }

    if (config.deployTarget === "k8s") {
      lines.push("");
      lines.push("# === Kubernetes Deployment ===");
      lines.push(`cat > k8s-deploy.yaml << 'EOF'`);
      lines.push(`apiVersion: apps/v1`);
      lines.push(`kind: Deployment`);
      lines.push(`metadata:`);
      lines.push(`  name: my-model-server`);
      lines.push(`spec:`);
      lines.push(`  replicas: ${config.autoScale ? 1 : config.maxReplicas}`);
      lines.push(`  selector:`);
      lines.push(`    matchLabels:`);
      lines.push(`      app: my-model-server`);
      lines.push(`  template:`);
      lines.push(`    metadata:`);
      lines.push(`      labels:`);
      lines.push(`        app: my-model-server`);
      lines.push(`    spec:`);
      lines.push(`      containers:`);
      lines.push(`      - name: model`);
      lines.push(`        image: my-model-server:latest`);
      lines.push(`        ports:`);
      lines.push(`        - containerPort: ${config.port}`);
      lines.push(`        resources:`);
      lines.push(`          limits:`);
      lines.push(`            nvidia.com/gpu: ${config.gpuCount}`);
      lines.push(`EOF`);
      lines.push(`kubectl apply -f k8s-deploy.yaml`);
    }

    lines.push("");
    lines.push("# === Connect to VibeCody ===");
    if (config.inferenceBackend === "ollama") {
      lines.push(`vibecli --provider ollama --model my-model`);
    } else {
      lines.push(`vibecli --provider openai --model my-model --api-url http://localhost:${config.port}/v1`);
    }

    return lines.join("\n");
  };

  // ── Render ─────────────────────────────────────────────────────────

  return (
    <div style={panelStyle}>
      {/* Header */}
      <div style={{ padding: "10px 16px", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)", flexShrink: 0 }}>
        <div style={{ fontSize: 15, fontWeight: 600 }}>Model Wizard</div>
        <div style={{ fontSize: 11, color: "var(--text-secondary)" }}>Fine-tune, quantize, and deploy in 7 steps</div>
      </div>

      {/* Step indicator */}
      <div style={{ display: "flex", padding: "10px 16px", gap: 4, borderBottom: "1px solid var(--border-color)", flexShrink: 0, overflowX: "auto" }}>
        {STEPS.map(s => (
          <button key={s.id} onClick={() => goTo(s.id)} style={{
            padding: "4px 10px", borderRadius: 12, border: "none", fontSize: 11, cursor: "pointer",
            background: step === s.id ? "var(--accent-primary, #7c6aef)" : s.id < step ? "var(--accent-green)" : "var(--bg-tertiary)",
            color: step === s.id || s.id < step ? "var(--btn-primary-fg, #fff)" : "var(--text-secondary)",
            fontWeight: step === s.id ? 600 : 400, whiteSpace: "nowrap" as const,
          }}>
            {s.icon}. {s.label}
          </button>
        ))}
      </div>

      {/* Step content */}
      <div style={{ flex: 1, overflow: "auto", padding: 16 }}>

        {/* ── Step 1: Base Model ──────────────────────────────────────── */}
        {step === 1 && (
          <div>
            <h3 style={{ margin: "0 0 12px", fontSize: 16 }}>Choose Base Model</h3>
            <label style={labelStyle}>Model Source</label>
            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: 6, marginBottom: 12 }}>
              {[
                { v: "huggingface", l: "HuggingFace Hub" },
                { v: "ollama", l: "Ollama Library" },
                { v: "local", l: "Local Checkpoint" },
              ].map(o => (
                <button key={o.v} style={optionBtn(config.modelSource === o.v)} onClick={() => set("modelSource", o.v)}>{o.l}</button>
              ))}
            </div>

            <label style={labelStyle}>Base Model</label>
            <select style={selectStyle} value={config.baseModel} onChange={e => { set("baseModel", e.target.value); set("modelSize", e.target.value.includes("70") ? "70B" : e.target.value.includes("13") || e.target.value.includes("14") ? "14B" : e.target.value.includes("3") ? "3B" : "8B"); }}>
              <optgroup label="Meta Llama">
                <option value="meta-llama/Llama-3.1-8B-Instruct">Llama 3.1 8B Instruct</option>
                <option value="meta-llama/Llama-3.1-70B-Instruct">Llama 3.1 70B Instruct</option>
                <option value="meta-llama/Llama-3.2-3B-Instruct">Llama 3.2 3B Instruct</option>
              </optgroup>
              <optgroup label="Mistral">
                <option value="mistralai/Mistral-7B-Instruct-v0.3">Mistral 7B v0.3</option>
                <option value="mistralai/Mixtral-8x7B-Instruct-v0.1">Mixtral 8x7B (MoE)</option>
              </optgroup>
              <optgroup label="Google">
                <option value="google/gemma-2-9b-it">Gemma 2 9B</option>
                <option value="google/gemma-2-27b-it">Gemma 2 27B</option>
              </optgroup>
              <optgroup label="Microsoft">
                <option value="microsoft/Phi-3.5-mini-instruct">Phi 3.5 Mini (3.8B)</option>
                <option value="microsoft/Phi-3-medium-4k-instruct">Phi 3 Medium (14B)</option>
              </optgroup>
              <optgroup label="Qwen">
                <option value="Qwen/Qwen2.5-7B-Instruct">Qwen 2.5 7B</option>
                <option value="Qwen/Qwen2.5-Coder-7B-Instruct">Qwen 2.5 Coder 7B</option>
                <option value="Qwen/Qwen2.5-72B-Instruct">Qwen 2.5 72B</option>
              </optgroup>
              <optgroup label="DeepSeek">
                <option value="deepseek-ai/DeepSeek-R1-Distill-Qwen-7B">DeepSeek R1 Distill 7B</option>
              </optgroup>
            </select>
            <div style={{ marginTop: 6, fontSize: 11, color: "var(--text-secondary)" }}>
              Model size: {config.modelSize} | VRAM needed: ~{config.modelSize === "3B" ? "4" : config.modelSize === "8B" ? "8" : config.modelSize === "14B" ? "16" : "80"} GB (QLoRA)
            </div>

            {config.modelSource === "local" && (
              <div style={{ marginTop: 8 }}>
                <label style={labelStyle}>Local Path</label>
                <input style={inputStyle} placeholder="/path/to/model/checkpoint" />
              </div>
            )}
          </div>
        )}

        {/* ── Step 2: Dataset ─────────────────────────────────────────── */}
        {step === 2 && (
          <div>
            <h3 style={{ margin: "0 0 12px", fontSize: 16 }}>Prepare Dataset</h3>
            <label style={labelStyle}>Data Source</label>
            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 6, marginBottom: 12 }}>
              {[
                { v: "codebase", l: "Mine from codebase", d: "Extract function docs + implementations" },
                { v: "git", l: "Git commit history", d: "Commit messages + diffs" },
                { v: "documents", l: "Documents (PDF/MD)", d: "Process with MinerU or built-in" },
                { v: "existing", l: "Existing JSONL file", d: "Pre-prepared dataset" },
              ].map(o => (
                <button key={o.v} style={{ ...optionBtn(config.dataSource === o.v), textAlign: "left" as const }} onClick={() => set("dataSource", o.v)}>
                  <div style={{ fontWeight: 600, fontSize: 12 }}>{o.l}</div>
                  <div style={{ fontSize: 10, opacity: 0.7, marginTop: 2 }}>{o.d}</div>
                </button>
              ))}
            </div>

            {config.dataSource === "documents" && (
              <div style={{ marginBottom: 12 }}>
                <label style={labelStyle}>Document Processor</label>
                <select style={selectStyle} value={config.docProcessor} onChange={e => set("docProcessor", e.target.value)}>
                  <option value="builtin">VibeCody Built-in (9 formats)</option>
                  <option value="mineru">MinerU (PDF to Markdown, 109 languages)</option>
                  <option value="docling">Docling (IBM document understanding)</option>
                  <option value="unstructured">Unstructured (document ETL)</option>
                  <option value="llamaparse">LlamaParse (LlamaIndex)</option>
                </select>
              </div>
            )}

            <label style={labelStyle}>Output Format</label>
            <select style={selectStyle} value={config.dataFormat} onChange={e => set("dataFormat", e.target.value)}>
              <option value="chatml">ChatML (OpenAI compatible)</option>
              <option value="alpaca">Alpaca (instruction/input/output)</option>
              <option value="sharegpt">ShareGPT (conversations)</option>
              <option value="completion">Completion (prompt/completion pairs)</option>
            </select>

            <div style={{ marginTop: 8 }}>
              <label style={labelStyle}>Dataset Path</label>
              <input style={inputStyle} value={config.dataPath} onChange={e => set("dataPath", e.target.value)} />
            </div>
          </div>
        )}

        {/* ── Step 3: Fine-Tuning Config ──────────────────────────────── */}
        {step === 3 && (
          <div>
            <h3 style={{ margin: "0 0 12px", fontSize: 16 }}>Configure Fine-Tuning</h3>
            <label style={labelStyle}>Fine-Tuning Library</label>
            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: 6, marginBottom: 12 }}>
              {[
                { v: "unsloth", l: "Unsloth", d: "2x speed, 60% less VRAM" },
                { v: "axolotl", l: "Axolotl", d: "YAML config, reproducible" },
                { v: "llama-factory", l: "LLaMA Factory", d: "100+ models, RLHF" },
                { v: "trl", l: "HF TRL", d: "SFT/DPO/PPO trainers" },
                { v: "peft", l: "PEFT", d: "LoRA, AdaLoRA, IA3" },
                { v: "deepspeed", l: "DeepSpeed", d: "Multi-GPU distributed" },
              ].map(o => (
                <button key={o.v} style={{ ...optionBtn(config.library === o.v), textAlign: "left" as const }} onClick={() => set("library", o.v)}>
                  <div style={{ fontWeight: 600, fontSize: 12 }}>{o.l}</div>
                  <div style={{ fontSize: 10, opacity: 0.7, marginTop: 2 }}>{o.d}</div>
                </button>
              ))}
            </div>

            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 10, marginBottom: 12 }}>
              <div>
                <label style={labelStyle}>Method</label>
                <select style={selectStyle} value={config.method} onChange={e => set("method", e.target.value)}>
                  <option value="qlora">QLoRA (4-bit, memory efficient)</option>
                  <option value="lora">LoRA (full precision adapters)</option>
                  <option value="full">Full fine-tune (all parameters)</option>
                </select>
              </div>
              <div>
                <label style={labelStyle}>Alignment</label>
                <select style={selectStyle} value={config.alignment} onChange={e => set("alignment", e.target.value)}>
                  <option value="sft">SFT (Supervised Fine-Tuning)</option>
                  <option value="dpo">DPO (Direct Preference Optimization)</option>
                  <option value="ppo">PPO (Proximal Policy Optimization)</option>
                  <option value="kto">KTO (Kahneman-Tversky Optimization)</option>
                </select>
              </div>
            </div>

            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr 1fr", gap: 10 }}>
              <div>
                <label style={labelStyle}>LoRA Rank</label>
                <select style={selectStyle} value={config.loraRank} onChange={e => set("loraRank", Number(e.target.value))}>
                  {[8, 16, 32, 64, 128].map(r => <option key={r} value={r}>{r}</option>)}
                </select>
              </div>
              <div>
                <label style={labelStyle}>Epochs</label>
                <input type="number" style={inputStyle} value={config.epochs} min={1} max={20} onChange={e => set("epochs", Number(e.target.value))} />
              </div>
              <div>
                <label style={labelStyle}>Batch Size</label>
                <select style={selectStyle} value={config.batchSize} onChange={e => set("batchSize", Number(e.target.value))}>
                  {[1, 2, 4, 8, 16].map(b => <option key={b} value={b}>{b}</option>)}
                </select>
              </div>
              <div>
                <label style={labelStyle}>Learning Rate</label>
                <select style={selectStyle} value={config.learningRate} onChange={e => set("learningRate", e.target.value)}>
                  {["1e-5", "2e-5", "5e-5", "1e-4", "2e-4", "5e-4"].map(lr => <option key={lr} value={lr}>{lr}</option>)}
                </select>
              </div>
            </div>
          </div>
        )}

        {/* ── Step 4: Environment ──────────────────────────────────────── */}
        {step === 4 && (
          <div>
            <h3 style={{ margin: "0 0 12px", fontSize: 16 }}>Select Training Environment</h3>
            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 6, marginBottom: 12 }}>
              {[
                { v: "colab", l: "Google Colab", d: "Free T4 16GB | Pro: A100 40GB", cost: "Free" },
                { v: "kaggle", l: "Kaggle Notebook", d: "Free P100 16GB | 30hr/week", cost: "Free" },
                { v: "sagemaker", l: "AWS SageMaker", d: "ml.g5.xlarge to ml.p4d.24xlarge", cost: "$1-30/hr" },
                { v: "lightning", l: "Lightning AI", d: "T4, A10G, A100 on demand", cost: "$0.6-4/hr" },
                { v: "local", l: "Local Machine", d: "Your GPU(s) — no upload needed", cost: "Free" },
                { v: "runpod", l: "RunPod / Lambda", d: "A100 80GB, H100, on demand", cost: "$1-4/hr" },
              ].map(o => (
                <button key={o.v} style={{ ...optionBtn(config.environment === o.v), textAlign: "left" as const }} onClick={() => set("environment", o.v)}>
                  <div style={{ display: "flex", justifyContent: "space-between" }}>
                    <span style={{ fontWeight: 600, fontSize: 12 }}>{o.l}</span>
                    <span style={{ fontSize: 10, color: o.cost === "Free" ? "var(--success-color)" : "var(--text-secondary)" }}>{o.cost}</span>
                  </div>
                  <div style={{ fontSize: 10, opacity: 0.7, marginTop: 2 }}>{o.d}</div>
                </button>
              ))}
            </div>

            <div>
              <label style={labelStyle}>GPU Count</label>
              <select style={selectStyle} value={config.gpuCount} onChange={e => set("gpuCount", Number(e.target.value))}>
                {[1, 2, 4, 8].map(n => <option key={n} value={n}>{n} GPU{n > 1 ? "s" : ""}</option>)}
              </select>
            </div>
          </div>
        )}

        {/* ── Step 5: Quantization ─────────────────────────────────────── */}
        {step === 5 && (
          <div>
            <h3 style={{ margin: "0 0 12px", fontSize: 16 }}>Quantize Model</h3>
            <label style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 12, cursor: "pointer", fontSize: 13 }}>
              <input type="checkbox" checked={config.skipQuantize} onChange={e => set("skipQuantize", e.target.checked)} />
              Skip quantization (deploy full-precision model)
            </label>

            {!config.skipQuantize && (
              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 6 }}>
                {[
                  { v: "gguf-q4km", l: "GGUF Q4_K_M", d: "Best quality/size ratio. ~4 bits. Runs on CPU+GPU.", size: "~4.5 GB for 8B" },
                  { v: "gguf-q5km", l: "GGUF Q5_K_M", d: "Higher quality, slightly larger.", size: "~5.5 GB for 8B" },
                  { v: "gptq-4bit", l: "GPTQ 4-bit", d: "GPU-only. Fast inference. Needs calibration data.", size: "~4 GB for 8B" },
                  { v: "awq-4bit", l: "AWQ 4-bit", d: "GPU-only. Activation-aware. Best for vLLM.", size: "~4 GB for 8B" },
                  { v: "int8", l: "Int8", d: "8-bit quantization. Good balance. bitsandbytes.", size: "~8 GB for 8B" },
                  { v: "fp16", l: "FP16", d: "Half precision. No quality loss. GPU only.", size: "~16 GB for 8B" },
                ].map(o => (
                  <button key={o.v} style={{ ...optionBtn(config.quantMethod === o.v), textAlign: "left" as const }} onClick={() => set("quantMethod", o.v)}>
                    <div style={{ fontWeight: 600, fontSize: 12 }}>{o.l}</div>
                    <div style={{ fontSize: 10, opacity: 0.7, marginTop: 2 }}>{o.d}</div>
                    <div style={{ fontSize: 10, color: "var(--accent-primary)", marginTop: 2 }}>{o.size}</div>
                  </button>
                ))}
              </div>
            )}
          </div>
        )}

        {/* ── Step 6: Deploy ───────────────────────────────────────────── */}
        {step === 6 && (
          <div>
            <h3 style={{ margin: "0 0 12px", fontSize: 16 }}>Deploy Inference Service</h3>
            <label style={labelStyle}>Inference Backend</label>
            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: 6, marginBottom: 12 }}>
              {[
                { v: "ollama", l: "Ollama", d: "Easiest. Local or server. GGUF models." },
                { v: "vllm", l: "vLLM", d: "Fastest GPU serving. OpenAI-compatible API." },
                { v: "llamacpp", l: "llama.cpp", d: "CPU+GPU. Edge/IoT. GGUF format." },
                { v: "tgi", l: "TGI", d: "HuggingFace. Docker-ready. Production." },
                { v: "triton", l: "Triton", d: "NVIDIA. Multi-model. Enterprise." },
                { v: "onnx", l: "ONNX Runtime", d: "Cross-platform. CPU/GPU/NPU." },
              ].map(o => (
                <button key={o.v} style={{ ...optionBtn(config.inferenceBackend === o.v), textAlign: "left" as const }} onClick={() => set("inferenceBackend", o.v)}>
                  <div style={{ fontWeight: 600, fontSize: 12 }}>{o.l}</div>
                  <div style={{ fontSize: 10, opacity: 0.7, marginTop: 2 }}>{o.d}</div>
                </button>
              ))}
            </div>

            <label style={labelStyle}>Deploy Target</label>
            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 6, marginBottom: 12 }}>
              {[
                { v: "local", l: "Local Machine", d: "Run directly, no container" },
                { v: "docker", l: "Docker Container", d: "Portable, GPU-enabled" },
                { v: "k8s", l: "Kubernetes", d: "Auto-scaling, production-grade" },
                { v: "edge", l: "Edge / IoT Device", d: "Raspberry Pi, Jetson, ARM" },
              ].map(o => (
                <button key={o.v} style={{ ...optionBtn(config.deployTarget === o.v), textAlign: "left" as const }} onClick={() => set("deployTarget", o.v)}>
                  <div style={{ fontWeight: 600, fontSize: 12 }}>{o.l}</div>
                  <div style={{ fontSize: 10, opacity: 0.7, marginTop: 2 }}>{o.d}</div>
                </button>
              ))}
            </div>

            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 10 }}>
              <div>
                <label style={labelStyle}>Port</label>
                <input type="number" style={inputStyle} value={config.port} onChange={e => set("port", Number(e.target.value))} />
              </div>
              {config.deployTarget === "k8s" && (
                <div>
                  <label style={labelStyle}>Max Replicas</label>
                  <input type="number" style={inputStyle} value={config.maxReplicas} min={1} max={16} onChange={e => set("maxReplicas", Number(e.target.value))} />
                </div>
              )}
            </div>
          </div>
        )}

        {/* ── Step 7: Review & Launch ──────────────────────────────────── */}
        {step === 7 && (
          <div>
            <h3 style={{ margin: "0 0 12px", fontSize: 16 }}>Review & Launch</h3>

            {/* Config summary */}
            <div style={cardStyle}>
              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 6, fontSize: 12 }}>
                <div><span style={{ color: "var(--text-secondary)" }}>Model:</span> {config.baseModel.split("/").pop()}</div>
                <div><span style={{ color: "var(--text-secondary)" }}>Size:</span> {config.modelSize}</div>
                <div><span style={{ color: "var(--text-secondary)" }}>Library:</span> {config.library}</div>
                <div><span style={{ color: "var(--text-secondary)" }}>Method:</span> {config.method} (rank {config.loraRank})</div>
                <div><span style={{ color: "var(--text-secondary)" }}>Alignment:</span> {config.alignment.toUpperCase()}</div>
                <div><span style={{ color: "var(--text-secondary)" }}>Epochs:</span> {config.epochs} | Batch: {config.batchSize}</div>
                <div><span style={{ color: "var(--text-secondary)" }}>Environment:</span> {config.environment} ({config.gpuCount} GPU)</div>
                <div><span style={{ color: "var(--text-secondary)" }}>Quantization:</span> {config.skipQuantize ? "None" : config.quantMethod}</div>
                <div><span style={{ color: "var(--text-secondary)" }}>Backend:</span> {config.inferenceBackend}</div>
                <div><span style={{ color: "var(--text-secondary)" }}>Deploy:</span> {config.deployTarget} (:{config.port})</div>
              </div>
            </div>

            {/* Generated script */}
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 6 }}>
              <label style={{ ...labelStyle, margin: 0 }}>Generated Script</label>
              <button style={{ ...btnSecondary, fontSize: 11, padding: "3px 10px" }} onClick={() => copyScript(generateFullScript())}>
                {copied ? "Copied!" : "Copy"}
              </button>
            </div>
            <pre style={codeBlock}>{generateFullScript()}</pre>
          </div>
        )}
      </div>

      {/* Navigation footer */}
      <div style={{ padding: "10px 16px", borderTop: "1px solid var(--border-color)", display: "flex", justifyContent: "space-between", flexShrink: 0 }}>
        <button style={btnSecondary} onClick={prev} disabled={step === 1}>Back</button>
        <div style={{ fontSize: 11, color: "var(--text-secondary)", alignSelf: "center" }}>Step {step} of 7</div>
        {step < 7 ? (
          <button style={btnPrimary} onClick={next}>Next</button>
        ) : (
          <button style={{ ...btnPrimary, background: "var(--accent-green)" }} onClick={() => copyScript(generateFullScript())}>
            {copied ? "Copied!" : "Copy Script"}
          </button>
        )}
      </div>
    </div>
  );
}
