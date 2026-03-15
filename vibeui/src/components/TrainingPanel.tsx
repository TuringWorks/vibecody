/**
 * TrainingPanel — distributed ML training management panel.
 *
 * Tabs: Config (distributed training), LoRA (fine-tuning), Cluster (SLURM/hostfile/estimator)
 * Pure TypeScript — no Tauri commands.
 */
import { useState } from "react";

type TabId = "config" | "lora" | "cluster";
type Framework = "DeepSpeed" | "FSDP" | "Megatron" | "Horovod" | "Ray Train" | "Colossal-AI";
type MixedPrecision = "FP32" | "FP16" | "BF16" | "FP8";
type DeepSpeedStage = "0" | "1" | "2" | "3" | "Infinity";
type BiasOption = "none" | "all" | "lora_only";
type TaskType = "CAUSAL_LM" | "SEQ_2_SEQ_LM" | "SEQ_CLS" | "TOKEN_CLS" | "QUESTION_ANS" | "FEATURE_EXTRACTION";

const TARGET_MODULES = ["q_proj", "k_proj", "v_proj", "o_proj", "gate_proj", "up_proj", "down_proj"] as const;

interface HostEntry {
  hostname: string;
  slots: number;
}

interface ParallelismSuggestion {
  dp: number;
  tp: number;
  pp: number;
  note: string;
}

// ---------------------------------------------------------------------------
// Shared styles
// ---------------------------------------------------------------------------
const labelStyle: React.CSSProperties = { fontSize: 11, fontWeight: 600, marginBottom: 2, color: "var(--text-secondary)" };
const inputStyle: React.CSSProperties = {
  width: "100%", background: "var(--bg-secondary)", border: "1px solid var(--border-color)",
  borderRadius: 4, color: "var(--text-primary)", padding: "5px 8px", fontSize: 12, boxSizing: "border-box",
};
const selectStyle: React.CSSProperties = { ...inputStyle, appearance: "auto" as never };
const btnPrimary: React.CSSProperties = {
  background: "var(--accent-color)", color: "var(--text-primary)", border: "none",
  borderRadius: 4, padding: "6px 14px", cursor: "pointer", fontSize: 12, fontWeight: 600,
};
const btnSecondary: React.CSSProperties = {
  background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: 4,
  padding: "6px 14px", cursor: "pointer", fontSize: 12, color: "var(--text-primary)",
};
const codeBlockStyle: React.CSSProperties = {
  background: "var(--bg-secondary)", border: "1px solid var(--border-color)", borderRadius: 4,
  padding: 12, fontFamily: "monospace", fontSize: 11, whiteSpace: "pre-wrap", overflowX: "auto",
  color: "var(--text-primary)", maxHeight: 360, overflowY: "auto",
};
const fieldRow: React.CSSProperties = { display: "grid", gridTemplateColumns: "1fr 1fr", gap: 12, marginBottom: 10 };
const singleField: React.CSSProperties = { marginBottom: 10 };

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------
function generateDsConfig(
  framework: Framework, batchSize: number, gradAccum: number, lr: number,
  precision: MixedPrecision, gradCkpt: boolean, flashAttn: boolean, dsStage: DeepSpeedStage,
): string {
  if (framework === "DeepSpeed") {
    const stageNum = dsStage === "Infinity" ? 3 : Number(dsStage);
    const offload = dsStage === "Infinity" || dsStage === "3";
    return JSON.stringify({
      train_batch_size: batchSize,
      gradient_accumulation_steps: gradAccum,
      gradient_clipping: 1.0,
      fp16: { enabled: precision === "FP16" },
      bf16: { enabled: precision === "BF16" },
      zero_optimization: {
        stage: stageNum,
        offload_optimizer: offload ? { device: "cpu", pin_memory: true } : undefined,
        offload_param: dsStage === "Infinity" ? { device: "nvme", nvme_path: "/local_nvme" } : undefined,
        overlap_comm: true,
        contiguous_gradients: true,
        reduce_bucket_size: 5e8,
      },
      optimizer: { type: "AdamW", params: { lr, betas: [0.9, 0.999], eps: 1e-8, weight_decay: 0.01 } },
      scheduler: { type: "WarmupDecayLR", params: { warmup_min_lr: 0, warmup_max_lr: lr, warmup_num_steps: 100, total_num_steps: 1000 } },
      activation_checkpointing: gradCkpt ? { partition_activations: true, contiguous_memory_optimization: true } : undefined,
      flops_profiler: { enabled: false },
      ...(flashAttn ? { _comment: "Enable flash attention in model config, not in ds_config" } : {}),
    }, null, 2);
  }
  // For non-DeepSpeed frameworks, return a representative config
  return JSON.stringify({
    framework,
    training: { batch_size: batchSize, gradient_accumulation_steps: gradAccum, learning_rate: lr },
    precision: precision.toLowerCase(),
    gradient_checkpointing: gradCkpt,
    flash_attention: flashAttn,
  }, null, 2);
}

function generateLaunchCmd(
  framework: Framework, modelPath: string, datasetPath: string, outputDir: string,
  numNodes: number, gpusPerNode: number, precision: MixedPrecision, gradCkpt: boolean, flashAttn: boolean,
): string {
  const totalGpus = numNodes * gpusPerNode;
  const precisionFlag = precision === "BF16" ? "--bf16" : precision === "FP16" ? "--fp16" : "";
  const ckptFlag = gradCkpt ? "--gradient_checkpointing true" : "";
  const flashFlag = flashAttn ? "--attn_implementation flash_attention_2" : "";

  switch (framework) {
    case "DeepSpeed":
      return [
        `deepspeed --num_nodes ${numNodes} --num_gpus ${gpusPerNode} \\`,
        `  --hostfile hostfile \\`,
        `  train.py \\`,
        `  --model_name_or_path ${modelPath || "<MODEL_PATH>"} \\`,
        `  --dataset_path ${datasetPath || "<DATASET_PATH>"} \\`,
        `  --output_dir ${outputDir || "<OUTPUT_DIR>"} \\`,
        `  --deepspeed ds_config.json \\`,
        `  ${precisionFlag} ${ckptFlag} ${flashFlag}`.trim(),
      ].join("\n");
    case "FSDP":
      return [
        `torchrun --nproc_per_node ${gpusPerNode} --nnodes ${numNodes} \\`,
        `  --rdzv_backend c10d --rdzv_endpoint $MASTER_ADDR:$MASTER_PORT \\`,
        `  train.py \\`,
        `  --model_name_or_path ${modelPath || "<MODEL_PATH>"} \\`,
        `  --dataset_path ${datasetPath || "<DATASET_PATH>"} \\`,
        `  --output_dir ${outputDir || "<OUTPUT_DIR>"} \\`,
        `  --fsdp "full_shard auto_wrap" \\`,
        `  ${precisionFlag} ${ckptFlag} ${flashFlag}`.trim(),
      ].join("\n");
    case "Megatron":
      return [
        `python -m torch.distributed.launch \\`,
        `  --nproc_per_node ${gpusPerNode} --nnodes ${numNodes} \\`,
        `  pretrain_gpt.py \\`,
        `  --tensor-model-parallel-size ${Math.min(gpusPerNode, 8)} \\`,
        `  --pipeline-model-parallel-size ${numNodes} \\`,
        `  --num-layers 32 --hidden-size 4096 --num-attention-heads 32 \\`,
        `  --data-path ${datasetPath || "<DATASET_PATH>"} \\`,
        `  --save ${outputDir || "<OUTPUT_DIR>"} \\`,
        `  ${precisionFlag === "--bf16" ? "--bf16" : "--fp16"}`,
      ].join("\n");
    case "Horovod":
      return [
        `horovodrun -np ${totalGpus} -H ${Array(numNodes).fill(`localhost:${gpusPerNode}`).join(",")} \\`,
        `  python train.py \\`,
        `  --model_name_or_path ${modelPath || "<MODEL_PATH>"} \\`,
        `  --dataset_path ${datasetPath || "<DATASET_PATH>"} \\`,
        `  --output_dir ${outputDir || "<OUTPUT_DIR>"} \\`,
        `  ${precisionFlag} ${ckptFlag}`.trim(),
      ].join("\n");
    case "Ray Train":
      return [
        `ray job submit --runtime-env-json='{"pip": ["ray[train]", "transformers", "accelerate"]}' -- \\`,
        `  python train_ray.py \\`,
        `  --model_name_or_path ${modelPath || "<MODEL_PATH>"} \\`,
        `  --dataset_path ${datasetPath || "<DATASET_PATH>"} \\`,
        `  --output_dir ${outputDir || "<OUTPUT_DIR>"} \\`,
        `  --num_workers ${totalGpus} --use_gpu \\`,
        `  ${precisionFlag} ${ckptFlag}`.trim(),
      ].join("\n");
    case "Colossal-AI":
      return [
        `colossalai run --nproc_per_node ${gpusPerNode} --host $MASTER_ADDR \\`,
        `  train.py \\`,
        `  --model_name_or_path ${modelPath || "<MODEL_PATH>"} \\`,
        `  --dataset_path ${datasetPath || "<DATASET_PATH>"} \\`,
        `  --output_dir ${outputDir || "<OUTPUT_DIR>"} \\`,
        `  --plugin gemini \\`,
        `  ${precisionFlag} ${ckptFlag} ${flashFlag}`.trim(),
      ].join("\n");
  }
}

function estimateVram(paramsB: number, precision: MixedPrecision): { model: number; optimizer: number; gradients: number; activations: number; total: number } {
  const bytesPerParam = precision === "FP32" ? 4 : precision === "FP8" ? 1 : 2;
  const modelGB = (paramsB * 1e9 * bytesPerParam) / 1e9;
  const optimizerGB = (paramsB * 1e9 * 8) / 1e9; // AdamW: 2 states x fp32
  const gradientsGB = (paramsB * 1e9 * bytesPerParam) / 1e9;
  const activationsGB = paramsB * 0.5; // rough estimate
  return { model: modelGB, optimizer: optimizerGB, gradients: gradientsGB, activations: activationsGB, total: modelGB + optimizerGB + gradientsGB + activationsGB };
}

function suggestParallelism(paramsB: number, gpuCount: number, vramPerGpu: number): ParallelismSuggestion {
  const totalVram = gpuCount * vramPerGpu;
  const needed = estimateVram(paramsB, "BF16").total;

  // Simple heuristic-based suggestions
  if (paramsB <= 7) {
    return { dp: gpuCount, tp: 1, pp: 1, note: `Model fits in single GPU with ZeRO-2. Data parallel across ${gpuCount} GPUs.` };
  }
  if (paramsB <= 13) {
    const tp = Math.min(gpuCount, 2);
    const dp = Math.floor(gpuCount / tp);
    return { dp, tp, pp: 1, note: `13B model: TP=${tp} across NVLink pairs, DP=${dp}. ZeRO-3 recommended.` };
  }
  if (paramsB <= 70) {
    const tp = Math.min(gpuCount, 8);
    const dp = Math.max(1, Math.floor(gpuCount / tp));
    const pp = needed > totalVram ? Math.ceil(needed / totalVram) : 1;
    return { dp, tp, pp, note: `70B model: TP=${tp} within node, PP=${pp} across nodes, DP=${dp}. Use ZeRO-3 + offload if VRAM tight.` };
  }
  // >70B
  const tp = 8;
  const pp = Math.max(1, Math.ceil(paramsB / 70));
  const dp = Math.max(1, Math.floor(gpuCount / (tp * pp)));
  return { dp, tp, pp, note: `${paramsB}B model: Full 3D parallelism. TP=${tp} intra-node, PP=${pp} inter-node, DP=${dp}. Megatron-LM or DeepSpeed ZeRO-Infinity recommended.` };
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------
export function TrainingPanel() {
  const [tab, setTab] = useState<TabId>("config");

  // Config tab state
  const [modelPath, setModelPath] = useState("");
  const [datasetPath, setDatasetPath] = useState("");
  const [outputDir, setOutputDir] = useState("./output");
  const [framework, setFramework] = useState<Framework>("DeepSpeed");
  const [numNodes, setNumNodes] = useState(1);
  const [gpusPerNode, setGpusPerNode] = useState(8);
  const [batchSize, setBatchSize] = useState(32);
  const [gradAccum, setGradAccum] = useState(4);
  const [lr, setLr] = useState(2e-5);
  const [precision, setPrecision] = useState<MixedPrecision>("BF16");
  const [gradCkpt, setGradCkpt] = useState(true);
  const [flashAttn, setFlashAttn] = useState(true);
  const [dsStage, setDsStage] = useState<DeepSpeedStage>("2");
  const [configOutput, setConfigOutput] = useState("");
  const [launchOutput, setLaunchOutput] = useState("");

  // LoRA tab state
  const [loraRank, setLoraRank] = useState(16);
  const [loraAlpha, setLoraAlpha] = useState(32);
  const [loraDropout, setLoraDropout] = useState(0.05);
  const [loraTargets, setLoraTargets] = useState<Set<string>>(new Set(["q_proj", "v_proj"]));
  const [loraBias, setLoraBias] = useState<BiasOption>("none");
  const [loraTaskType, setLoraTaskType] = useState<TaskType>("CAUSAL_LM");
  const [loraOutput, setLoraOutput] = useState("");

  // Cluster tab state
  const [slurmPartition, setSlurmPartition] = useState("gpu");
  const [slurmNodes, setSlurmNodes] = useState(2);
  const [slurmGpus, setSlurmGpus] = useState(8);
  const [slurmOutput, setSlurmOutput] = useState("");
  const [hosts, setHosts] = useState<HostEntry[]>([{ hostname: "node-0", slots: 8 }]);
  const [hostfileOutput, setHostfileOutput] = useState("");
  const [estimatorParams, setEstimatorParams] = useState(7);
  const [estimatorPrecision, setEstimatorPrecision] = useState<MixedPrecision>("BF16");
  const [estimatorGpuCount, setEstimatorGpuCount] = useState(8);
  const [estimatorVram, setEstimatorVram] = useState(80);

  // ---------------------------------------------------------------------------
  // Tab renderers
  // ---------------------------------------------------------------------------
  const renderConfig = () => (
    <div style={{ padding: 16, overflowY: "auto", flex: 1 }}>
      <div style={singleField}>
        <div style={labelStyle}>Model Path</div>
        <input style={inputStyle} value={modelPath} onChange={(e) => setModelPath(e.target.value)} placeholder="meta-llama/Llama-3-70b-hf" />
      </div>
      <div style={fieldRow}>
        <div>
          <div style={labelStyle}>Dataset Path</div>
          <input style={inputStyle} value={datasetPath} onChange={(e) => setDatasetPath(e.target.value)} placeholder="/data/train.jsonl" />
        </div>
        <div>
          <div style={labelStyle}>Output Directory</div>
          <input style={inputStyle} value={outputDir} onChange={(e) => setOutputDir(e.target.value)} placeholder="./output" />
        </div>
      </div>
      <div style={fieldRow}>
        <div>
          <div style={labelStyle}>Framework</div>
          <select style={selectStyle} value={framework} onChange={(e) => setFramework(e.target.value as Framework)}>
            {(["DeepSpeed", "FSDP", "Megatron", "Horovod", "Ray Train", "Colossal-AI"] as Framework[]).map((f) => (
              <option key={f} value={f}>{f}</option>
            ))}
          </select>
        </div>
        <div>
          <div style={labelStyle}>Mixed Precision</div>
          <select style={selectStyle} value={precision} onChange={(e) => setPrecision(e.target.value as MixedPrecision)}>
            {(["FP32", "FP16", "BF16", "FP8"] as MixedPrecision[]).map((p) => (
              <option key={p} value={p}>{p}</option>
            ))}
          </select>
        </div>
      </div>
      <div style={fieldRow}>
        <div>
          <div style={labelStyle}>Num Nodes</div>
          <input style={inputStyle} type="number" min={1} value={numNodes} onChange={(e) => setNumNodes(Math.max(1, Number(e.target.value)))} />
        </div>
        <div>
          <div style={labelStyle}>GPUs per Node</div>
          <input style={inputStyle} type="number" min={1} value={gpusPerNode} onChange={(e) => setGpusPerNode(Math.max(1, Number(e.target.value)))} />
        </div>
      </div>
      <div style={fieldRow}>
        <div>
          <div style={labelStyle}>Batch Size</div>
          <input style={inputStyle} type="number" min={1} value={batchSize} onChange={(e) => setBatchSize(Math.max(1, Number(e.target.value)))} />
        </div>
        <div>
          <div style={labelStyle}>Gradient Accumulation Steps</div>
          <input style={inputStyle} type="number" min={1} value={gradAccum} onChange={(e) => setGradAccum(Math.max(1, Number(e.target.value)))} />
        </div>
      </div>
      <div style={fieldRow}>
        <div>
          <div style={labelStyle}>Learning Rate</div>
          <input style={inputStyle} type="number" step="0.00001" min={0} value={lr} onChange={(e) => setLr(Number(e.target.value))} />
        </div>
        {framework === "DeepSpeed" && (
          <div>
            <div style={labelStyle}>DeepSpeed Stage</div>
            <select style={selectStyle} value={dsStage} onChange={(e) => setDsStage(e.target.value as DeepSpeedStage)}>
              {(["0", "1", "2", "3", "Infinity"] as DeepSpeedStage[]).map((s) => (
                <option key={s} value={s}>Stage {s}</option>
              ))}
            </select>
          </div>
        )}
      </div>
      <div style={{ display: "flex", gap: 20, marginBottom: 14 }}>
        <label style={{ display: "flex", alignItems: "center", gap: 6, fontSize: 12, cursor: "pointer", color: "var(--text-primary)" }}>
          <input type="checkbox" checked={gradCkpt} onChange={(e) => setGradCkpt(e.target.checked)} />
          Gradient Checkpointing
        </label>
        <label style={{ display: "flex", alignItems: "center", gap: 6, fontSize: 12, cursor: "pointer", color: "var(--text-primary)" }}>
          <input type="checkbox" checked={flashAttn} onChange={(e) => setFlashAttn(e.target.checked)} />
          Flash Attention
        </label>
      </div>
      <div style={{ display: "flex", gap: 8, marginBottom: 14 }}>
        <button style={btnPrimary} onClick={() => {
          setConfigOutput(generateDsConfig(framework, batchSize, gradAccum, lr, precision, gradCkpt, flashAttn, dsStage));
          setLaunchOutput("");
        }}>Generate Config</button>
        <button style={btnSecondary} onClick={() => {
          setLaunchOutput(generateLaunchCmd(framework, modelPath, datasetPath, outputDir, numNodes, gpusPerNode, precision, gradCkpt, flashAttn));
          setConfigOutput("");
        }}>Generate Launch Command</button>
      </div>
      {configOutput && (
        <div>
          <div style={{ ...labelStyle, marginBottom: 6 }}>{framework === "DeepSpeed" ? "ds_config.json" : `${framework.toLowerCase().replace(/\s/g, "_")}_config.json`}</div>
          <pre style={codeBlockStyle}>{configOutput}</pre>
        </div>
      )}
      {launchOutput && (
        <div>
          <div style={{ ...labelStyle, marginBottom: 6 }}>Launch Command</div>
          <pre style={codeBlockStyle}>{launchOutput}</pre>
        </div>
      )}
    </div>
  );

  const renderLora = () => (
    <div style={{ padding: 16, overflowY: "auto", flex: 1 }}>
      <div style={fieldRow}>
        <div>
          <div style={labelStyle}>Rank (r)</div>
          <input style={inputStyle} type="number" min={1} value={loraRank} onChange={(e) => setLoraRank(Math.max(1, Number(e.target.value)))} />
        </div>
        <div>
          <div style={labelStyle}>Alpha</div>
          <input style={inputStyle} type="number" min={1} value={loraAlpha} onChange={(e) => setLoraAlpha(Math.max(1, Number(e.target.value)))} />
        </div>
      </div>
      <div style={fieldRow}>
        <div>
          <div style={labelStyle}>Dropout</div>
          <input style={inputStyle} type="number" step="0.01" min={0} max={1} value={loraDropout} onChange={(e) => setLoraDropout(Number(e.target.value))} />
        </div>
        <div>
          <div style={labelStyle}>Bias</div>
          <select style={selectStyle} value={loraBias} onChange={(e) => setLoraBias(e.target.value as BiasOption)}>
            <option value="none">none</option>
            <option value="all">all</option>
            <option value="lora_only">lora_only</option>
          </select>
        </div>
      </div>
      <div style={singleField}>
        <div style={labelStyle}>Task Type</div>
        <select style={selectStyle} value={loraTaskType} onChange={(e) => setLoraTaskType(e.target.value as TaskType)}>
          {(["CAUSAL_LM", "SEQ_2_SEQ_LM", "SEQ_CLS", "TOKEN_CLS", "QUESTION_ANS", "FEATURE_EXTRACTION"] as TaskType[]).map((t) => (
            <option key={t} value={t}>{t}</option>
          ))}
        </select>
      </div>
      <div style={singleField}>
        <div style={labelStyle}>Target Modules</div>
        <div style={{ display: "flex", flexWrap: "wrap", gap: 8, marginTop: 4 }}>
          {TARGET_MODULES.map((mod) => (
            <label key={mod} style={{ display: "flex", alignItems: "center", gap: 4, fontSize: 12, cursor: "pointer", color: "var(--text-primary)" }}>
              <input
                type="checkbox"
                checked={loraTargets.has(mod)}
                onChange={(e) => {
                  const next = new Set(loraTargets);
                  e.target.checked ? next.add(mod) : next.delete(mod);
                  setLoraTargets(next);
                }}
              />
              <code style={{ fontFamily: "monospace", fontSize: 11 }}>{mod}</code>
            </label>
          ))}
        </div>
      </div>
      <button style={{ ...btnPrimary, marginTop: 8, marginBottom: 14 }} onClick={() => {
        const config = {
          r: loraRank,
          lora_alpha: loraAlpha,
          lora_dropout: loraDropout,
          target_modules: Array.from(loraTargets),
          bias: loraBias,
          task_type: loraTaskType,
          inference_mode: false,
        };
        setLoraOutput(JSON.stringify(config, null, 2));
      }}>Generate LoRA Config</button>
      {loraOutput && (
        <div>
          <div style={{ ...labelStyle, marginBottom: 6 }}>lora_config.json</div>
          <pre style={codeBlockStyle}>{loraOutput}</pre>
        </div>
      )}
    </div>
  );

  const renderCluster = () => (
    <div style={{ padding: 16, overflowY: "auto", flex: 1 }}>
      {/* SLURM script generator */}
      <div style={{ marginBottom: 20, padding: 12, border: "1px solid var(--border-color)", borderRadius: 6, background: "var(--bg-primary)" }}>
        <div style={{ fontSize: 13, fontWeight: 600, marginBottom: 10, color: "var(--text-primary)" }}>SLURM Script Generator</div>
        <div style={fieldRow}>
          <div>
            <div style={labelStyle}>Partition Name</div>
            <input style={inputStyle} value={slurmPartition} onChange={(e) => setSlurmPartition(e.target.value)} />
          </div>
          <div>
            <div style={labelStyle}>Nodes</div>
            <input style={inputStyle} type="number" min={1} value={slurmNodes} onChange={(e) => setSlurmNodes(Math.max(1, Number(e.target.value)))} />
          </div>
        </div>
        <div style={{ ...singleField, maxWidth: "calc(50% - 6px)" }}>
          <div style={labelStyle}>GPUs per Node</div>
          <input style={inputStyle} type="number" min={1} value={slurmGpus} onChange={(e) => setSlurmGpus(Math.max(1, Number(e.target.value)))} />
        </div>
        <button style={btnPrimary} onClick={() => {
          const script = [
            "#!/bin/bash",
            `#SBATCH --job-name=train`,
            `#SBATCH --partition=${slurmPartition}`,
            `#SBATCH --nodes=${slurmNodes}`,
            `#SBATCH --ntasks-per-node=1`,
            `#SBATCH --gres=gpu:${slurmGpus}`,
            `#SBATCH --cpus-per-task=${slurmGpus * 4}`,
            `#SBATCH --mem=0`,
            `#SBATCH --time=48:00:00`,
            `#SBATCH --output=slurm-%j.out`,
            `#SBATCH --error=slurm-%j.err`,
            "",
            "export MASTER_ADDR=$(scontrol show hostnames $SLURM_JOB_NODELIST | head -n 1)",
            "export MASTER_PORT=29500",
            "export WORLD_SIZE=$((SLURM_NNODES * ${slurmGpus}))",
            "",
            "srun --jobid $SLURM_JOB_ID bash -c '\\",
            `  torchrun --nproc_per_node ${slurmGpus} \\\\`,
            "    --nnodes $SLURM_NNODES \\\\",
            "    --node_rank $SLURM_NODEID \\\\",
            "    --rdzv_endpoint $MASTER_ADDR:$MASTER_PORT \\\\",
            "    train.py'",
          ].join("\n");
          setSlurmOutput(script);
        }}>Generate SLURM Script</button>
        {slurmOutput && <pre style={{ ...codeBlockStyle, marginTop: 10 }}>{slurmOutput}</pre>}
      </div>

      {/* Hostfile generator */}
      <div style={{ marginBottom: 20, padding: 12, border: "1px solid var(--border-color)", borderRadius: 6, background: "var(--bg-primary)" }}>
        <div style={{ fontSize: 13, fontWeight: 600, marginBottom: 10, color: "var(--text-primary)" }}>Hostfile Generator</div>
        {hosts.map((h, i) => (
          <div key={i} style={{ display: "flex", gap: 8, marginBottom: 6, alignItems: "center" }}>
            <input
              style={{ ...inputStyle, flex: 1 }}
              value={h.hostname}
              onChange={(e) => { const n = [...hosts]; n[i] = { ...h, hostname: e.target.value }; setHosts(n); }}
              placeholder="hostname"
            />
            <input
              style={{ ...inputStyle, width: 70 }}
              type="number" min={1}
              value={h.slots}
              onChange={(e) => { const n = [...hosts]; n[i] = { ...h, slots: Math.max(1, Number(e.target.value)) }; setHosts(n); }}
              placeholder="slots"
            />
            <button
              style={{ ...btnSecondary, padding: "4px 8px", fontSize: 14, lineHeight: 1 }}
              onClick={() => setHosts(hosts.filter((_, j) => j !== i))}
              disabled={hosts.length <= 1}
            >x</button>
          </div>
        ))}
        <div style={{ display: "flex", gap: 8, marginTop: 8 }}>
          <button style={btnSecondary} onClick={() => setHosts([...hosts, { hostname: `node-${hosts.length}`, slots: 8 }])}>+ Add Host</button>
          <button style={btnPrimary} onClick={() => {
            setHostfileOutput(hosts.map((h) => `${h.hostname} slots=${h.slots}`).join("\n"));
          }}>Generate Hostfile</button>
        </div>
        {hostfileOutput && <pre style={{ ...codeBlockStyle, marginTop: 10 }}>{hostfileOutput}</pre>}
      </div>

      {/* Memory estimator */}
      <div style={{ marginBottom: 20, padding: 12, border: "1px solid var(--border-color)", borderRadius: 6, background: "var(--bg-primary)" }}>
        <div style={{ fontSize: 13, fontWeight: 600, marginBottom: 10, color: "var(--text-primary)" }}>Memory Estimator</div>
        <div style={fieldRow}>
          <div>
            <div style={labelStyle}>Model Parameters (B)</div>
            <input style={inputStyle} type="number" step="0.1" min={0.1} value={estimatorParams} onChange={(e) => setEstimatorParams(Number(e.target.value))} />
          </div>
          <div>
            <div style={labelStyle}>Precision</div>
            <select style={selectStyle} value={estimatorPrecision} onChange={(e) => setEstimatorPrecision(e.target.value as MixedPrecision)}>
              {(["FP32", "FP16", "BF16", "FP8"] as MixedPrecision[]).map((p) => (
                <option key={p} value={p}>{p}</option>
              ))}
            </select>
          </div>
        </div>
        {(() => {
          const est = estimateVram(estimatorParams, estimatorPrecision);
          return (
            <div style={{ fontSize: 12, fontFamily: "monospace", color: "var(--text-primary)" }}>
              <table style={{ borderCollapse: "collapse", width: "100%" }}>
                <tbody>
                  {([
                    ["Model Weights", est.model],
                    ["Optimizer States", est.optimizer],
                    ["Gradients", est.gradients],
                    ["Activations (est.)", est.activations],
                    ["Total", est.total],
                  ] as [string, number][]).map(([label, val]) => (
                    <tr key={label} style={{ borderBottom: label === "Total" ? "none" : "1px solid var(--border-color)" }}>
                      <td style={{ padding: "4px 8px", fontWeight: label === "Total" ? 700 : 400 }}>{label}</td>
                      <td style={{ padding: "4px 8px", textAlign: "right", fontWeight: label === "Total" ? 700 : 400 }}>{val.toFixed(1)} GB</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          );
        })()}
      </div>

      {/* Parallelism suggestion */}
      <div style={{ padding: 12, border: "1px solid var(--border-color)", borderRadius: 6, background: "var(--bg-primary)" }}>
        <div style={{ fontSize: 13, fontWeight: 600, marginBottom: 10, color: "var(--text-primary)" }}>Parallelism Suggestion</div>
        <div style={fieldRow}>
          <div>
            <div style={labelStyle}>Model Size (B params)</div>
            <input style={inputStyle} type="number" step="0.1" min={0.1} value={estimatorParams} onChange={(e) => setEstimatorParams(Number(e.target.value))} />
          </div>
          <div>
            <div style={labelStyle}>GPU Count</div>
            <input style={inputStyle} type="number" min={1} value={estimatorGpuCount} onChange={(e) => setEstimatorGpuCount(Math.max(1, Number(e.target.value)))} />
          </div>
        </div>
        <div style={{ ...singleField, maxWidth: "calc(50% - 6px)" }}>
          <div style={labelStyle}>VRAM per GPU (GB)</div>
          <input style={inputStyle} type="number" min={1} value={estimatorVram} onChange={(e) => setEstimatorVram(Math.max(1, Number(e.target.value)))} />
        </div>
        {(() => {
          const suggestion = suggestParallelism(estimatorParams, estimatorGpuCount, estimatorVram);
          return (
            <div style={{ marginTop: 8, padding: 10, background: "var(--bg-secondary)", borderRadius: 4, fontSize: 12, color: "var(--text-primary)" }}>
              <div style={{ display: "flex", gap: 16, marginBottom: 6, fontFamily: "monospace" }}>
                <span>DP={suggestion.dp}</span>
                <span>TP={suggestion.tp}</span>
                <span>PP={suggestion.pp}</span>
              </div>
              <div style={{ opacity: 0.8, lineHeight: 1.5 }}>{suggestion.note}</div>
            </div>
          );
        })()}
      </div>
    </div>
  );

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", background: "var(--bg-primary)", color: "var(--text-primary)" }}>
      {/* Tab bar */}
      <div style={{ display: "flex", borderBottom: "1px solid var(--border-color)", background: "var(--bg-secondary)" }}>
        {([["config", "Config"], ["lora", "LoRA"], ["cluster", "Cluster"]] as [TabId, string][]).map(([id, label]) => (
          <button
            key={id}
            onClick={() => setTab(id)}
            style={{
              flex: 1, padding: "8px 0", border: "none", cursor: "pointer", fontSize: 12, fontWeight: 600,
              background: tab === id ? "var(--bg-primary)" : "transparent",
              color: tab === id ? "var(--accent-color)" : "var(--text-secondary)",
              borderBottom: tab === id ? "2px solid var(--accent-color)" : "2px solid transparent",
            }}
          >
            {label}
          </button>
        ))}
      </div>
      {tab === "config" && renderConfig()}
      {tab === "lora" && renderLora()}
      {tab === "cluster" && renderCluster()}
    </div>
  );
}
