/**
 * RLTrainingDashboard — Setup, launch, and monitor RL training runs.
 *
 * Two modes:
 *   1. Setup Wizard — multi-step form to configure a new training run
 *      (algorithm, environment, policy network, hyperparameters, distributed, curriculum)
 *   2. Monitor — real-time reward curves, loss plots, GPU utilization, episode stats
 */
import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

// ── Types ────────────────────────────────────────────────────────────────

interface TrainingRun {
  id: string;
  name: string;
  status: string;
  algorithm: string;
  environment: string;
  startedAt: number;
  episodes: number;
  currentReward: number;
}

interface TrainingMetrics {
  runId: string;
  rewards: number[];
  losses: number[];
  gpuUtil: number[];
  episodeStats: EpisodeStat[];
}

interface EpisodeStat {
  episode: number;
  reward: number;
  length: number;
  loss: number;
}

// ── Training Run Configuration ───────────────────────────────────────────

interface TrainRunConfig {
  // Step 1: Algorithm
  algorithmFamily: string;
  algorithmId: string;
  // Step 2: Environment
  environmentName: string;
  environmentVersion: string;
  // Step 3: Policy Network
  networkType: string;
  hiddenLayers: string;
  activation: string;
  // Step 4: Hyperparameters
  learningRate: string;
  gamma: string;
  batchSize: number;
  totalTimesteps: number;
  nEnvs: number;
  clipRange: string;
  entropyCoef: string;
  vfCoef: string;
  gaeLambda: string;
  // Step 5: Distributed
  distributed: boolean;
  numWorkers: number;
  gpusPerWorker: number;
  strategy: string;
  faultTolerant: boolean;
  checkpointFrequency: number;
  // Step 6: Curriculum
  useCurriculum: boolean;
  curriculumStages: CurriculumStageConfig[];
  // Step 7: Name & Review
  runName: string;
}

interface CurriculumStageConfig {
  name: string;
  envOverride: string;
  duration: number;
  promotionMetric: string;
  promotionThreshold: number;
}

const DEFAULT_CONFIG: TrainRunConfig = {
  algorithmFamily: "On-Policy",
  algorithmId: "PPO",
  environmentName: "",
  environmentVersion: "latest",
  networkType: "MLP",
  hiddenLayers: "256, 256",
  activation: "ReLU",
  learningRate: "3e-4",
  gamma: "0.99",
  batchSize: 256,
  totalTimesteps: 1_000_000,
  nEnvs: 8,
  clipRange: "0.2",
  entropyCoef: "0.01",
  vfCoef: "0.5",
  gaeLambda: "0.95",
  distributed: false,
  numWorkers: 1,
  gpusPerWorker: 1,
  strategy: "data_parallel",
  faultTolerant: true,
  checkpointFrequency: 100_000,
  useCurriculum: false,
  curriculumStages: [],
  runName: "",
};

const ALGORITHM_FAMILIES: Record<string, string[]> = {
  "On-Policy": ["PPO", "A2C", "TRPO", "PPG"],
  "Off-Policy": ["SAC", "TD3", "DQN", "DDPG", "C51", "QR-DQN", "IQN"],
  "Offline RL": ["CQL", "IQL", "BCQ", "BEAR", "CRR", "TD3+BC", "Decision Transformer", "COMBO"],
  "Model-Based": ["DreamerV3", "World Models", "MuZero-style"],
  "Multi-Agent": ["MAPPO", "QMIX", "VDN", "MADDPG", "COMA"],
  "Imitation": ["BC", "GAIL", "DAgger"],
};

const NETWORK_TYPES = ["MLP", "CNN", "LSTM", "Transformer", "Custom"];
const ACTIVATIONS = ["ReLU", "Tanh", "GELU", "SiLU", "ELU"];
const STRATEGIES = ["data_parallel", "model_parallel", "pipeline"];

const STEPS = [
  { id: "algorithm", label: "Algorithm" },
  { id: "environment", label: "Environment" },
  { id: "network", label: "Policy Network" },
  { id: "hyperparams", label: "Hyperparameters" },
  { id: "distributed", label: "Distributed" },
  { id: "curriculum", label: "Curriculum" },
  { id: "review", label: "Review & Launch" },
];

// ── Shared inline styles for elements not covered by design system classes ──

const tableStyle: React.CSSProperties = { width: "100%", borderCollapse: "collapse", fontSize: 12 };
const thStyle: React.CSSProperties = { textAlign: "left", padding: "6px 8px", borderBottom: "1px solid var(--border-color)", color: "var(--text-secondary)", fontWeight: 600 };
const tdStyle: React.CSSProperties = { padding: "6px 8px", borderBottom: "1px solid var(--border-color)" };
const fieldRow: React.CSSProperties = { display: "grid", gridTemplateColumns: "1fr 1fr", gap: 10, marginBottom: 10 };
const fieldCol: React.CSSProperties = { display: "flex", flexDirection: "column", gap: 4 };
const fieldLabel: React.CSSProperties = { fontSize: 11, fontWeight: 600, color: "var(--text-secondary)" };
const checkRow: React.CSSProperties = { display: "flex", alignItems: "center", gap: 8, marginBottom: 8 };
const statusColor = (s: string) => s === "running" ? "var(--success-color)" : s === "paused" ? "var(--warning-color)" : "var(--text-secondary)";
const stepBarItem = (active: boolean, done: boolean): React.CSSProperties => ({
  padding: "4px 10px", borderRadius: 12, fontSize: 11, fontWeight: active ? 700 : 400,
  background: active ? "var(--accent-color)" : done ? "var(--bg-tertiary)" : "transparent",
  color: active ? "#fff" : done ? "var(--text-primary)" : "var(--text-secondary)",
  cursor: done ? "pointer" : "default", whiteSpace: "nowrap",
});

// ── Component ────────────────────────────────────────────────────────────

export function RLTrainingDashboard(_props: { workspacePath?: string | null; provider?: string }) {
  const [mode, setMode] = useState<"list" | "setup" | "monitor">("list");
  const [runs, setRuns] = useState<TrainingRun[]>([]);
  const [_selectedRun, setSelectedRun] = useState<string | null>(null);
  const [metrics, setMetrics] = useState<TrainingMetrics | null>(null);
  const [loading, setLoading] = useState(false);
  const [config, setConfig] = useState<TrainRunConfig>({ ...DEFAULT_CONFIG });
  const [step, setStep] = useState(0);
  const [launching, setLaunching] = useState(false);
  const [envList, setEnvList] = useState<string[]>([]);

  const upd = (partial: Partial<TrainRunConfig>) => setConfig(c => ({ ...c, ...partial }));

  const fetchRuns = useCallback(async () => {
    try { setRuns(await invoke<TrainingRun[]>("rl_list_training_runs")); } catch { /* backend not wired yet */ }
  }, []);

  const fetchEnvs = useCallback(async () => {
    try {
      const res = await invoke<{ name: string }[]>("rl_list_environments");
      setEnvList(res.map(e => e.name));
    } catch { /* fallback */ }
  }, []);

  useEffect(() => { fetchRuns(); fetchEnvs(); }, [fetchRuns, fetchEnvs]);

  const fetchMetrics = useCallback(async (runId: string) => {
    setLoading(true);
    try { setMetrics(await invoke<TrainingMetrics>("rl_get_training_metrics", { runId })); setSelectedRun(runId); } catch (e) { console.error(e); }
    setLoading(false);
  }, []);

  const handleAction = useCallback(async (action: "start" | "stop", runId: string) => {
    try {
      if (action === "start") await invoke("rl_start_training", { runId });
      else await invoke("rl_stop_training", { runId });
      fetchRuns();
    } catch (e) { console.error(e); }
  }, [fetchRuns]);

  const handleLaunch = useCallback(async () => {
    setLaunching(true);
    try {
      await invoke("rl_create_training_run", { config: JSON.stringify(config) });
      setMode("list");
      setConfig({ ...DEFAULT_CONFIG });
      setStep(0);
      fetchRuns();
    } catch (e) { console.error(e); }
    setLaunching(false);
  }, [config, fetchRuns]);

  // ── Setup Wizard ─────────────────────────────────────────────────

  const renderStepBar = () => (
    <div style={{ display: "flex", gap: 4, marginBottom: 14, flexWrap: "wrap" }}>
      {STEPS.map((s, i) => (
        <div key={s.id} style={stepBarItem(i === step, i < step)} onClick={() => { if (i < step) setStep(i); }}>
          {i < step ? "\u2713 " : ""}{s.label}
        </div>
      ))}
    </div>
  );

  const renderAlgorithmStep = () => (
    <div className="panel-card">
      <div style={fieldRow}>
        <div style={fieldCol}>
          <span style={fieldLabel}>Algorithm Family</span>
          <select className="panel-select" value={config.algorithmFamily} onChange={e => {
            const fam = e.target.value;
            const algos = ALGORITHM_FAMILIES[fam] || [];
            upd({ algorithmFamily: fam, algorithmId: algos[0] || "" });
          }}>
            {Object.keys(ALGORITHM_FAMILIES).map(f => <option key={f}>{f}</option>)}
          </select>
        </div>
        <div style={fieldCol}>
          <span style={fieldLabel}>Algorithm</span>
          <select className="panel-select" value={config.algorithmId} onChange={e => upd({ algorithmId: e.target.value })}>
            {(ALGORITHM_FAMILIES[config.algorithmFamily] || []).map(a => <option key={a}>{a}</option>)}
          </select>
        </div>
      </div>
      <div className="panel-label" style={{ marginTop: 8, lineHeight: 1.5 }}>
        {config.algorithmFamily === "On-Policy" && "On-policy methods learn from fresh experience collected under the current policy. PPO is recommended for most tasks."}
        {config.algorithmFamily === "Off-Policy" && "Off-policy methods reuse past experience via replay buffers. SAC is recommended for continuous control."}
        {config.algorithmFamily === "Offline RL" && "Offline RL learns from a fixed dataset without environment interaction. CQL/IQL are strongest general-purpose choices."}
        {config.algorithmFamily === "Model-Based" && "Model-based methods learn a world model then plan through it. DreamerV3 is state-of-the-art."}
        {config.algorithmFamily === "Multi-Agent" && "Multi-agent algorithms train multiple cooperating or competing agents. MAPPO is recommended for cooperative tasks."}
        {config.algorithmFamily === "Imitation" && "Imitation learning trains from expert demonstrations. BC for simple tasks, DAgger for interactive learning."}
      </div>
    </div>
  );

  const renderEnvironmentStep = () => (
    <div className="panel-card">
      <div style={fieldRow}>
        <div style={fieldCol}>
          <span style={fieldLabel}>Environment</span>
          {envList.length > 0 ? (
            <select className="panel-select" value={config.environmentName} onChange={e => upd({ environmentName: e.target.value })}>
              <option value="">-- Select environment --</option>
              {envList.map(e => <option key={e}>{e}</option>)}
            </select>
          ) : (
            <input className="panel-input panel-input-full" placeholder="e.g. trading-env, franka-reach, CartPole-v1" value={config.environmentName} onChange={e => upd({ environmentName: e.target.value })} />
          )}
        </div>
        <div style={fieldCol}>
          <span style={fieldLabel}>Version</span>
          <input className="panel-input panel-input-full" placeholder="latest" value={config.environmentVersion} onChange={e => upd({ environmentVersion: e.target.value })} />
        </div>
      </div>
      <div style={fieldRow}>
        <div style={fieldCol}>
          <span style={fieldLabel}>Parallel Environments</span>
          <input type="number" className="panel-input panel-input-full" min={1} max={4096} value={config.nEnvs} onChange={e => upd({ nEnvs: parseInt(e.target.value) || 1 })} />
        </div>
        <div style={fieldCol}>
          <span style={fieldLabel}>Total Timesteps</span>
          <select className="panel-select" value={config.totalTimesteps} onChange={e => upd({ totalTimesteps: parseInt(e.target.value) })}>
            <option value={100000}>100K (quick test)</option>
            <option value={500000}>500K (short run)</option>
            <option value={1000000}>1M (standard)</option>
            <option value={5000000}>5M (long run)</option>
            <option value={10000000}>10M (full training)</option>
            <option value={50000000}>50M (large scale)</option>
          </select>
        </div>
      </div>
    </div>
  );

  const renderNetworkStep = () => (
    <div className="panel-card">
      <div style={fieldRow}>
        <div style={fieldCol}>
          <span style={fieldLabel}>Network Architecture</span>
          <select className="panel-select" value={config.networkType} onChange={e => upd({ networkType: e.target.value })}>
            {NETWORK_TYPES.map(n => <option key={n}>{n}</option>)}
          </select>
        </div>
        <div style={fieldCol}>
          <span style={fieldLabel}>Activation Function</span>
          <select className="panel-select" value={config.activation} onChange={e => upd({ activation: e.target.value })}>
            {ACTIVATIONS.map(a => <option key={a}>{a}</option>)}
          </select>
        </div>
      </div>
      <div style={fieldCol}>
        <span style={fieldLabel}>Hidden Layers (comma-separated dimensions)</span>
        <input className="panel-input panel-input-full" placeholder="256, 256" value={config.hiddenLayers} onChange={e => upd({ hiddenLayers: e.target.value })} />
        <span className="panel-label">
          {config.networkType === "MLP" && "Fully connected layers. 2-3 layers of 64-512 units typical."}
          {config.networkType === "CNN" && "Convolutional network for image observations (Atari, visual control)."}
          {config.networkType === "LSTM" && "Recurrent network for partially observable environments."}
          {config.networkType === "Transformer" && "Attention-based network for sequence decision-making."}
          {config.networkType === "Custom" && "Define a custom architecture via config file."}
        </span>
      </div>
    </div>
  );

  const renderHyperparamsStep = () => (
    <div className="panel-card">
      <div style={fieldRow}>
        <div style={fieldCol}>
          <span style={fieldLabel}>Learning Rate</span>
          <select className="panel-select" value={config.learningRate} onChange={e => upd({ learningRate: e.target.value })}>
            {["1e-2", "3e-3", "1e-3", "3e-4", "1e-4", "3e-5", "1e-5"].map(lr => <option key={lr}>{lr}</option>)}
          </select>
        </div>
        <div style={fieldCol}>
          <span style={fieldLabel}>Discount Factor (gamma)</span>
          <select className="panel-select" value={config.gamma} onChange={e => upd({ gamma: e.target.value })}>
            {["0.9", "0.95", "0.99", "0.995", "0.999", "1.0"].map(g => <option key={g}>{g}</option>)}
          </select>
        </div>
      </div>
      <div style={fieldRow}>
        <div style={fieldCol}>
          <span style={fieldLabel}>Batch Size</span>
          <select className="panel-select" value={config.batchSize} onChange={e => upd({ batchSize: parseInt(e.target.value) })}>
            {[32, 64, 128, 256, 512, 1024, 2048].map(b => <option key={b} value={b}>{b}</option>)}
          </select>
        </div>
        <div style={fieldCol}>
          <span style={fieldLabel}>GAE Lambda</span>
          <select className="panel-select" value={config.gaeLambda} onChange={e => upd({ gaeLambda: e.target.value })}>
            {["0.9", "0.92", "0.95", "0.97", "0.99", "1.0"].map(l => <option key={l}>{l}</option>)}
          </select>
        </div>
      </div>
      {(config.algorithmId === "PPO" || config.algorithmId === "TRPO" || config.algorithmId === "PPG") && (
        <div style={fieldRow}>
          <div style={fieldCol}>
            <span style={fieldLabel}>Clip Range (PPO)</span>
            <select className="panel-select" value={config.clipRange} onChange={e => upd({ clipRange: e.target.value })}>
              {["0.1", "0.15", "0.2", "0.25", "0.3", "0.4"].map(c => <option key={c}>{c}</option>)}
            </select>
          </div>
          <div style={fieldCol}>
            <span style={fieldLabel}>Entropy Coefficient</span>
            <select className="panel-select" value={config.entropyCoef} onChange={e => upd({ entropyCoef: e.target.value })}>
              {["0.0", "0.001", "0.005", "0.01", "0.02", "0.05"].map(e2 => <option key={e2}>{e2}</option>)}
            </select>
          </div>
        </div>
      )}
    </div>
  );

  const renderDistributedStep = () => (
    <div className="panel-card">
      <div style={checkRow}>
        <input type="checkbox" checked={config.distributed} onChange={e => upd({ distributed: e.target.checked })} />
        <span style={fieldLabel}>Enable Distributed Training</span>
      </div>
      {config.distributed && (
        <>
          <div style={fieldRow}>
            <div style={fieldCol}>
              <span style={fieldLabel}>Number of Workers</span>
              <input type="number" className="panel-input panel-input-full" min={1} max={64} value={config.numWorkers} onChange={e => upd({ numWorkers: parseInt(e.target.value) || 1 })} />
            </div>
            <div style={fieldCol}>
              <span style={fieldLabel}>GPUs per Worker</span>
              <input type="number" className="panel-input panel-input-full" min={0} max={8} value={config.gpusPerWorker} onChange={e => upd({ gpusPerWorker: parseInt(e.target.value) || 0 })} />
            </div>
          </div>
          <div style={fieldRow}>
            <div style={fieldCol}>
              <span style={fieldLabel}>Strategy</span>
              <select className="panel-select" value={config.strategy} onChange={e => upd({ strategy: e.target.value })}>
                {STRATEGIES.map(s => <option key={s}>{s}</option>)}
              </select>
            </div>
            <div style={fieldCol}>
              <span style={fieldLabel}>Checkpoint Every N Steps</span>
              <select className="panel-select" value={config.checkpointFrequency} onChange={e => upd({ checkpointFrequency: parseInt(e.target.value) })}>
                {[10000, 50000, 100000, 250000, 500000].map(f => <option key={f} value={f}>{(f / 1000) + "K"}</option>)}
              </select>
            </div>
          </div>
          <div style={checkRow}>
            <input type="checkbox" checked={config.faultTolerant} onChange={e => upd({ faultTolerant: e.target.checked })} />
            <span style={fieldLabel}>Fault-tolerant (auto-resume on preemption)</span>
          </div>
        </>
      )}
    </div>
  );

  const renderCurriculumStep = () => (
    <div className="panel-card">
      <div style={checkRow}>
        <input type="checkbox" checked={config.useCurriculum} onChange={e => upd({ useCurriculum: e.target.checked })} />
        <span style={fieldLabel}>Enable Curriculum Learning</span>
      </div>
      {config.useCurriculum && (
        <>
          {config.curriculumStages.map((stage, idx) => (
            <div key={idx} className="panel-card" style={{ background: "var(--bg-tertiary)" }}>
              <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 6 }}>
                <span style={{ ...fieldLabel, fontWeight: 700 }}>Stage {idx + 1}</span>
                <button className="panel-btn panel-btn-secondary" style={{ padding: "2px 8px", fontSize: 11 }} onClick={() => {
                  const next = [...config.curriculumStages];
                  next.splice(idx, 1);
                  upd({ curriculumStages: next });
                }}>Remove</button>
              </div>
              <div style={fieldRow}>
                <div style={fieldCol}>
                  <span style={fieldLabel}>Stage Name</span>
                  <input className="panel-input panel-input-full" value={stage.name} onChange={e => {
                    const next = [...config.curriculumStages];
                    next[idx] = { ...stage, name: e.target.value };
                    upd({ curriculumStages: next });
                  }} />
                </div>
                <div style={fieldCol}>
                  <span style={fieldLabel}>Duration (timesteps)</span>
                  <input type="number" className="panel-input panel-input-full" value={stage.duration} onChange={e => {
                    const next = [...config.curriculumStages];
                    next[idx] = { ...stage, duration: parseInt(e.target.value) || 0 };
                    upd({ curriculumStages: next });
                  }} />
                </div>
              </div>
              <div style={fieldRow}>
                <div style={fieldCol}>
                  <span style={fieldLabel}>Promotion Metric</span>
                  <select className="panel-select" value={stage.promotionMetric} onChange={e => {
                    const next = [...config.curriculumStages];
                    next[idx] = { ...stage, promotionMetric: e.target.value };
                    upd({ curriculumStages: next });
                  }}>
                    {["mean_reward", "success_rate", "sharpe_ratio", "episode_length"].map(m => <option key={m}>{m}</option>)}
                  </select>
                </div>
                <div style={fieldCol}>
                  <span style={fieldLabel}>Threshold</span>
                  <input type="number" className="panel-input panel-input-full" step="0.1" value={stage.promotionThreshold} onChange={e => {
                    const next = [...config.curriculumStages];
                    next[idx] = { ...stage, promotionThreshold: parseFloat(e.target.value) || 0 };
                    upd({ curriculumStages: next });
                  }} />
                </div>
              </div>
              <div style={fieldCol}>
                <span style={fieldLabel}>Environment Override (optional YAML)</span>
                <input className="panel-input panel-input-full" placeholder='e.g. { difficulty: "hard" }' value={stage.envOverride} onChange={e => {
                  const next = [...config.curriculumStages];
                  next[idx] = { ...stage, envOverride: e.target.value };
                  upd({ curriculumStages: next });
                }} />
              </div>
            </div>
          ))}
          <button className="panel-btn panel-btn-secondary" onClick={() => upd({
            curriculumStages: [...config.curriculumStages, { name: `stage_${config.curriculumStages.length + 1}`, envOverride: "", duration: 500000, promotionMetric: "mean_reward", promotionThreshold: 10.0 }],
          })}>+ Add Stage</button>
        </>
      )}
    </div>
  );

  const renderReviewStep = () => (
    <div className="panel-card">
      <div style={fieldCol}>
        <span style={fieldLabel}>Training Run Name</span>
        <input className="panel-input panel-input-full" placeholder={`${config.algorithmId.toLowerCase()}-${config.environmentName || "env"}-run`} value={config.runName} onChange={e => upd({ runName: e.target.value })} />
      </div>
      <div style={{ marginTop: 12 }}>
        <div style={{ ...fieldLabel, marginBottom: 8 }}>Configuration Summary</div>
        <table style={tableStyle}>
          <tbody>
            <tr><td style={{ ...tdStyle, fontWeight: 600, width: 160 }}>Algorithm</td><td style={tdStyle}>{config.algorithmId} ({config.algorithmFamily})</td></tr>
            <tr><td style={{ ...tdStyle, fontWeight: 600 }}>Environment</td><td style={tdStyle}>{config.environmentName || "(not set)"}@{config.environmentVersion}</td></tr>
            <tr><td style={{ ...tdStyle, fontWeight: 600 }}>Network</td><td style={tdStyle}>{config.networkType} [{config.hiddenLayers}] ({config.activation})</td></tr>
            <tr><td style={{ ...tdStyle, fontWeight: 600 }}>Learning Rate</td><td style={tdStyle}>{config.learningRate}</td></tr>
            <tr><td style={{ ...tdStyle, fontWeight: 600 }}>Gamma / GAE Lambda</td><td style={tdStyle}>{config.gamma} / {config.gaeLambda}</td></tr>
            <tr><td style={{ ...tdStyle, fontWeight: 600 }}>Batch Size</td><td style={tdStyle}>{config.batchSize}</td></tr>
            <tr><td style={{ ...tdStyle, fontWeight: 600 }}>Total Timesteps</td><td style={tdStyle}>{(config.totalTimesteps / 1_000_000).toFixed(1)}M</td></tr>
            <tr><td style={{ ...tdStyle, fontWeight: 600 }}>Parallel Envs</td><td style={tdStyle}>{config.nEnvs}</td></tr>
            {config.distributed && <tr><td style={{ ...tdStyle, fontWeight: 600 }}>Distributed</td><td style={tdStyle}>{config.numWorkers} workers x {config.gpusPerWorker} GPUs ({config.strategy})</td></tr>}
            {config.useCurriculum && <tr><td style={{ ...tdStyle, fontWeight: 600 }}>Curriculum</td><td style={tdStyle}>{config.curriculumStages.length} stages</td></tr>}
          </tbody>
        </table>
      </div>
    </div>
  );

  const renderSetupWizard = () => (
    <>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 10 }}>
        <h2 style={{ margin: 0, fontSize: 15, fontWeight: 600, color: "var(--text-primary)" }}>New Training Run</h2>
        <button className="panel-btn panel-btn-secondary" onClick={() => { setMode("list"); setStep(0); }}>Cancel</button>
      </div>
      {renderStepBar()}

      {step === 0 && renderAlgorithmStep()}
      {step === 1 && renderEnvironmentStep()}
      {step === 2 && renderNetworkStep()}
      {step === 3 && renderHyperparamsStep()}
      {step === 4 && renderDistributedStep()}
      {step === 5 && renderCurriculumStep()}
      {step === 6 && renderReviewStep()}

      <div style={{ display: "flex", justifyContent: "space-between", marginTop: 12 }}>
        <button className="panel-btn panel-btn-secondary" disabled={step === 0} onClick={() => setStep(s => s - 1)}>Back</button>
        {step < STEPS.length - 1 ? (
          <button className="panel-btn panel-btn-primary" onClick={() => setStep(s => s + 1)}>Next: {STEPS[step + 1].label}</button>
        ) : (
          <button className="panel-btn panel-btn-primary" disabled={launching || !config.environmentName} onClick={handleLaunch}>
            {launching ? "Launching..." : "Launch Training Run"}
          </button>
        )}
      </div>
    </>
  );

  // ── Monitor View ──────────────────────────────────────────────────

  const renderMonitor = () => (
    <>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 10 }}>
        <h2 style={{ margin: 0, fontSize: 15, fontWeight: 600, color: "var(--text-primary)" }}>Training Monitor</h2>
        <button className="panel-btn panel-btn-secondary" onClick={() => { setMode("list"); setSelectedRun(null); setMetrics(null); }}>Back to Runs</button>
      </div>
      {loading && <div className="panel-loading">Loading metrics...</div>}
      {metrics && !loading && (
        <>
          <div className="panel-card">
            <div className="panel-label">Reward Curve ({metrics.rewards.length} points)</div>
            <div style={{ display: "flex", alignItems: "flex-end", gap: 1, height: 80 }}>
              {metrics.rewards.slice(-100).map((v, i, a) => {
                const max = Math.max(...a, 1);
                const min = Math.min(...a, 0);
                const range = max - min || 1;
                return <div key={i} style={{ flex: 1, background: v >= 0 ? "var(--accent-blue)" : "#dc3545", height: `${((v - min) / range) * 100}%`, borderRadius: "2px 2px 0 0", minHeight: 1 }} />;
              })}
            </div>
          </div>
          <div className="panel-card">
            <div className="panel-label">Loss Curve</div>
            <div style={{ display: "flex", alignItems: "flex-end", gap: 1, height: 60 }}>
              {metrics.losses.slice(-80).map((v, i, a) => {
                const max = Math.max(...a, 0.01);
                return <div key={i} style={{ flex: 1, background: "var(--warning-color)", height: `${(v / max) * 100}%`, borderRadius: "2px 2px 0 0", minHeight: 1 }} />;
              })}
            </div>
          </div>
          <div className="panel-card">
            <div className="panel-label">GPU Utilization</div>
            <div style={{ display: "flex", gap: 4 }}>
              {metrics.gpuUtil.map((g, i) => (
                <div key={i} style={{ flex: 1, textAlign: "center" }}>
                  <div style={{ height: 40, background: "var(--bg-tertiary)", borderRadius: 4, position: "relative", overflow: "hidden" }}>
                    <div style={{ position: "absolute", bottom: 0, width: "100%", height: `${g}%`, background: g > 80 ? "var(--success-color)" : "var(--warning-color)", borderRadius: 4 }} />
                  </div>
                  <div className="panel-label" style={{ marginTop: 2 }}>GPU{i}: {g}%</div>
                </div>
              ))}
            </div>
          </div>
          <div className="panel-card">
            <div className="panel-label">Recent Episodes</div>
            <table style={tableStyle}>
              <thead><tr><th style={thStyle}>Episode</th><th style={thStyle}>Reward</th><th style={thStyle}>Length</th><th style={thStyle}>Loss</th></tr></thead>
              <tbody>
                {metrics.episodeStats.slice(-10).map(s => (
                  <tr key={s.episode}><td style={tdStyle}>{s.episode}</td><td style={tdStyle}>{s.reward.toFixed(2)}</td><td style={tdStyle}>{s.length}</td><td style={tdStyle}>{s.loss.toFixed(4)}</td></tr>
                ))}
              </tbody>
            </table>
          </div>
        </>
      )}
    </>
  );

  // ── Runs List View ────────────────────────────────────────────────

  const renderRunsList = () => (
    <>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 10 }}>
        <h2 style={{ margin: 0, fontSize: 15, fontWeight: 600, color: "var(--text-primary)" }}>RL Training Dashboard</h2>
        <div>
          <button className="panel-btn panel-btn-primary" onClick={() => { setMode("setup"); setStep(0); setConfig({ ...DEFAULT_CONFIG }); }}>+ New Training Run</button>
          <button className="panel-btn panel-btn-secondary" onClick={fetchRuns}>Refresh</button>
        </div>
      </div>

      <div className="panel-card">
        <div className="panel-label">Training Runs</div>
        <table style={tableStyle}>
          <thead><tr><th style={thStyle}>Name</th><th style={thStyle}>Algorithm</th><th style={thStyle}>Environment</th><th style={thStyle}>Status</th><th style={thStyle}>Episodes</th><th style={thStyle}>Reward</th><th style={thStyle}>Actions</th></tr></thead>
          <tbody>
            {runs.map(r => (
              <tr key={r.id} style={{ cursor: "pointer" }} onClick={() => { fetchMetrics(r.id); setMode("monitor"); }}>
                <td style={tdStyle}>{r.name}</td>
                <td style={tdStyle}><span style={{ background: "var(--bg-tertiary)", padding: "2px 6px", borderRadius: 3, fontSize: 11 }}>{r.algorithm}</span></td>
                <td style={tdStyle}>{r.environment}</td>
                <td style={tdStyle}><span style={{ color: statusColor(r.status), fontWeight: 600 }}>{r.status}</span></td>
                <td style={tdStyle}>{r.episodes.toLocaleString()}</td>
                <td style={tdStyle}>{r.currentReward.toFixed(2)}</td>
                <td style={tdStyle}>
                  {r.status === "running"
                    ? <button className="panel-btn panel-btn-danger" onClick={e => { e.stopPropagation(); handleAction("stop", r.id); }}>Stop</button>
                    : <button className="panel-btn panel-btn-primary" onClick={e => { e.stopPropagation(); handleAction("start", r.id); }}>Start</button>}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
        {runs.length === 0 && (
          <div style={{ textAlign: "center", padding: "24px 0" }}>
            <div className="panel-empty" style={{ marginBottom: 8 }}>No training runs yet</div>
            <button className="panel-btn panel-btn-primary" onClick={() => { setMode("setup"); setStep(0); setConfig({ ...DEFAULT_CONFIG }); }}>Create Your First Training Run</button>
          </div>
        )}
      </div>
    </>
  );

  // ── Main Render ───────────────────────────────────────────────────

  return (
    <div className="panel-container">
      {mode === "list" && renderRunsList()}
      {mode === "setup" && renderSetupWizard()}
      {mode === "monitor" && renderMonitor()}
    </div>
  );
}
