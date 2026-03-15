/**
 * FineTuningPanel — Model fine-tuning dashboard.
 *
 * Dataset builder, training config editor, job launcher with cost estimation,
 * active jobs list, SWE-bench evaluation runner, and LoRA adapter manager.
 */
import { useState } from "react";

interface DatasetStats {
 example_count: number;
 total_tokens: number;
 avg_tokens_per_example: number;
 max_tokens: number;
 languages: Record<string, number>;
 invalid_count: number;
}

interface FineTuneJob {
 id: string;
 status: string;
 base_model: string;
 dataset: string;
 epochs: number;
 loss: number;
 progress: number;
 created: string;
 cost_usd: number;
}

interface EvalResult {
 model: string;
 tasks: number;
 resolved: number;
 rate: number;
 avg_time: number;
}

interface LoraAdapter {
 name: string;
 base_model: string;
 rank: number;
 size_mb: number;
}

const PROVIDERS = ["OpenAI", "TogetherAI", "Fireworks", "Local (LoRA)"];

const SAMPLE_STATS: DatasetStats = {
 example_count: 2847,
 total_tokens: 1_245_600,
 avg_tokens_per_example: 437,
 max_tokens: 4200,
 languages: { rust: 1200, typescript: 890, python: 540, go: 217 },
 invalid_count: 3,
};

const SAMPLE_JOBS: FineTuneJob[] = [
 { id: "ft-0001", status: "completed", base_model: "gpt-4o-mini", dataset: "codebase-vibecody", epochs: 3, loss: 0.42, progress: 100, created: "2026-03-06", cost_usd: 12.50 },
 { id: "ft-0002", status: "running", base_model: "codellama-13b", dataset: "git-history-main", epochs: 2, loss: 0.68, progress: 65, created: "2026-03-07", cost_usd: 4.20 },
 { id: "ft-0003", status: "pending", base_model: "mistral-7b", dataset: "conversations-agent", epochs: 1, loss: 0, progress: 0, created: "2026-03-08", cost_usd: 0 },
];

const SAMPLE_EVALS: EvalResult[] = [
 { model: "ft-0001 (gpt-4o-mini)", tasks: 300, resolved: 78, rate: 26.0, avg_time: 45.2 },
 { model: "gpt-4o-mini (base)", tasks: 300, resolved: 61, rate: 20.3, avg_time: 42.1 },
 { model: "codellama-13b (base)", tasks: 300, resolved: 42, rate: 14.0, avg_time: 38.5 },
];

const SAMPLE_ADAPTERS: LoraAdapter[] = [
 { name: "vibecody-rust-r16", base_model: "codellama-13b", rank: 16, size_mb: 42 },
 { name: "vibecody-ts-r8", base_model: "mistral-7b", rank: 8, size_mb: 24 },
];

const STATUS_COLORS: Record<string, string> = {
 completed: "var(--vp-c-success)",
 running: "var(--vp-c-brand)",
 pending: "var(--vp-c-warning)",
 failed: "var(--vp-c-danger)",
 cancelled: "var(--vp-c-border)",
};

export default function FineTuningPanel() {
 const [tab, setTab] = useState<"dataset" | "train" | "jobs" | "eval" | "lora">("jobs");
 const [provider, setProvider] = useState("OpenAI");
 const [baseModel, setBaseModel] = useState("gpt-4o-mini-2024-07-18");
 const [epochs, setEpochs] = useState(1);
 const [batchSize, setBatchSize] = useState(4);
 const [lr, setLr] = useState("2e-5");
 const [loraRank, setLoraRank] = useState(8);
 const [dataSource, setDataSource] = useState<"codebase" | "git" | "conversations">("codebase");

 return (
   <div style={{ padding: 16, color: "var(--vp-c-text)", background: "var(--vp-c-bg)", minHeight: "100%" }}>
     <h2 style={{ margin: "0 0 12px", fontSize: 18 }}>Fine-Tuning</h2>

     {/* Tabs */}
     <div style={{ display: "flex", gap: 4, marginBottom: 12 }}>
       {(["dataset", "train", "jobs", "eval", "lora"] as const).map(t => (
         <button key={t} onClick={() => setTab(t)} style={{
           padding: "4px 12px", border: "1px solid var(--vp-c-border)", borderRadius: 4, cursor: "pointer",
           background: tab === t ? "var(--vp-c-brand)" : "transparent", color: tab === t ? "var(--text-primary)" : "var(--vp-c-text)",
         }}>{t === "eval" ? "SWE-Bench" : t === "lora" ? "LoRA" : t.charAt(0).toUpperCase() + t.slice(1)}</button>
       ))}
     </div>

     {tab === "dataset" && (
       <>
         {/* Source selector */}
         <div style={{ marginBottom: 12 }}>
           <strong style={{ fontSize: 12 }}>Data Source</strong>
           <div style={{ display: "flex", gap: 6, marginTop: 4 }}>
             {(["codebase", "git", "conversations"] as const).map(s => (
               <button key={s} onClick={() => setDataSource(s)} style={{
                 padding: "6px 14px", border: "1px solid var(--vp-c-border)", borderRadius: 4, cursor: "pointer",
                 background: dataSource === s ? "var(--vp-c-brand)" : "transparent",
                 color: dataSource === s ? "var(--text-primary)" : "var(--vp-c-text)",
               }}>{s === "git" ? "Git History" : s.charAt(0).toUpperCase() + s.slice(1)}</button>
             ))}
           </div>
         </div>

         {/* Stats */}
         <div style={{ display: "grid", gridTemplateColumns: "repeat(4, 1fr)", gap: 8, marginBottom: 12 }}>
           <div style={{ padding: 10, border: "1px solid var(--vp-c-border)", borderRadius: 6, textAlign: "center" }}>
             <div style={{ fontSize: 22, fontWeight: 700, color: "var(--vp-c-brand)" }}>{SAMPLE_STATS.example_count.toLocaleString()}</div>
             <div style={{ fontSize: 11 }}>Examples</div>
           </div>
           <div style={{ padding: 10, border: "1px solid var(--vp-c-border)", borderRadius: 6, textAlign: "center" }}>
             <div style={{ fontSize: 22, fontWeight: 700, color: "var(--vp-c-success)" }}>{(SAMPLE_STATS.total_tokens / 1000).toFixed(0)}K</div>
             <div style={{ fontSize: 11 }}>Tokens</div>
           </div>
           <div style={{ padding: 10, border: "1px solid var(--vp-c-border)", borderRadius: 6, textAlign: "center" }}>
             <div style={{ fontSize: 22, fontWeight: 700 }}>{SAMPLE_STATS.avg_tokens_per_example.toFixed(0)}</div>
             <div style={{ fontSize: 11 }}>Avg Tokens/Ex</div>
           </div>
           <div style={{ padding: 10, border: "1px solid var(--vp-c-border)", borderRadius: 6, textAlign: "center" }}>
             <div style={{ fontSize: 22, fontWeight: 700, color: SAMPLE_STATS.invalid_count > 0 ? "var(--vp-c-danger)" : "var(--vp-c-success)" }}>
               {SAMPLE_STATS.invalid_count}
             </div>
             <div style={{ fontSize: 11 }}>Invalid</div>
           </div>
         </div>

         {/* Language distribution */}
         <strong style={{ fontSize: 12 }}>Language Distribution</strong>
         <div style={{ marginTop: 4 }}>
           {Object.entries(SAMPLE_STATS.languages).map(([lang, count]) => (
             <div key={lang} style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 4 }}>
               <span style={{ width: 80 }}>{lang}</span>
               <div style={{ flex: 1, height: 16, background: "var(--vp-c-border)", borderRadius: 4, overflow: "hidden" }}>
                 <div style={{ height: "100%", width: `${(count / SAMPLE_STATS.example_count) * 100}%`, background: "var(--vp-c-brand)", borderRadius: 4 }} />
               </div>
               <span style={{ width: 50, textAlign: "right", fontSize: 12 }}>{count}</span>
             </div>
           ))}
         </div>

         <div style={{ display: "flex", gap: 6, marginTop: 12 }}>
           <button style={{ padding: "6px 14px", background: "var(--vp-c-brand)", color: "var(--text-primary)", border: "none", borderRadius: 4, cursor: "pointer" }}>
             Build Dataset
           </button>
           <button style={{ padding: "6px 14px", background: "transparent", color: "var(--vp-c-text)", border: "1px solid var(--vp-c-border)", borderRadius: 4, cursor: "pointer" }}>
             Export JSONL
           </button>
           <button style={{ padding: "6px 14px", background: "transparent", color: "var(--vp-c-text)", border: "1px solid var(--vp-c-border)", borderRadius: 4, cursor: "pointer" }}>
             Deduplicate
           </button>
         </div>
       </>
     )}

     {tab === "train" && (
       <div style={{ maxWidth: 450 }}>
         <div style={{ display: "grid", gap: 10 }}>
           <label style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
             <span>Provider</span>
             <select value={provider} onChange={e => setProvider(e.target.value)}
               style={{ width: 180, padding: 4, background: "var(--vp-c-bg)", color: "var(--vp-c-text)", border: "1px solid var(--vp-c-border)", borderRadius: 4 }}>
               {PROVIDERS.map(p => <option key={p}>{p}</option>)}
             </select>
           </label>
           <label style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
             <span>Base Model</span>
             <input value={baseModel} onChange={e => setBaseModel(e.target.value)}
               style={{ width: 180, padding: 4, background: "var(--vp-c-bg)", color: "var(--vp-c-text)", border: "1px solid var(--vp-c-border)", borderRadius: 4 }} />
           </label>
           <label style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
             <span>Epochs</span>
             <input type="number" value={epochs} onChange={e => setEpochs(+e.target.value)} min={1} max={10}
               style={{ width: 70, padding: 4, background: "var(--vp-c-bg)", color: "var(--vp-c-text)", border: "1px solid var(--vp-c-border)", borderRadius: 4 }} />
           </label>
           <label style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
             <span>Batch Size</span>
             <input type="number" value={batchSize} onChange={e => setBatchSize(+e.target.value)} min={1} max={64}
               style={{ width: 70, padding: 4, background: "var(--vp-c-bg)", color: "var(--vp-c-text)", border: "1px solid var(--vp-c-border)", borderRadius: 4 }} />
           </label>
           <label style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
             <span>Learning Rate</span>
             <input value={lr} onChange={e => setLr(e.target.value)}
               style={{ width: 100, padding: 4, background: "var(--vp-c-bg)", color: "var(--vp-c-text)", border: "1px solid var(--vp-c-border)", borderRadius: 4 }} />
           </label>
           {provider === "Local (LoRA)" && (
             <label style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
               <span>LoRA Rank</span>
               <input type="number" value={loraRank} onChange={e => setLoraRank(+e.target.value)} min={4} max={128}
                 style={{ width: 70, padding: 4, background: "var(--vp-c-bg)", color: "var(--vp-c-text)", border: "1px solid var(--vp-c-border)", borderRadius: 4 }} />
             </label>
           )}
         </div>

         {/* Cost estimate */}
         <div style={{ marginTop: 12, padding: 10, border: "1px solid var(--vp-c-border)", borderRadius: 6, fontSize: 12 }}>
           <strong>Cost Estimate</strong>
           <div style={{ marginTop: 4 }}>
             Tokens: {(SAMPLE_STATS.total_tokens * epochs / 1000).toFixed(0)}K |
             Est. Cost: <span style={{ color: "var(--vp-c-warning)" }}>${(SAMPLE_STATS.total_tokens * epochs * 0.000008).toFixed(2)}</span> |
             Est. Time: ~{Math.ceil(SAMPLE_STATS.total_tokens * epochs / 50000)} min
           </div>
         </div>

         <button style={{ marginTop: 12, padding: "8px 20px", background: "var(--vp-c-brand)", color: "var(--text-primary)", border: "none", borderRadius: 4, cursor: "pointer", fontWeight: 600 }}>
           Launch Training Job
         </button>
       </div>
     )}

     {tab === "jobs" && (
       <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 12 }}>
         <thead>
           <tr style={{ borderBottom: "1px solid var(--vp-c-border)" }}>
             <th style={{ textAlign: "left", padding: 6 }}>ID</th>
             <th style={{ textAlign: "left", padding: 6 }}>Status</th>
             <th style={{ textAlign: "left", padding: 6 }}>Model</th>
             <th style={{ textAlign: "left", padding: 6 }}>Dataset</th>
             <th style={{ textAlign: "right", padding: 6 }}>Loss</th>
             <th style={{ textAlign: "right", padding: 6 }}>Progress</th>
             <th style={{ textAlign: "right", padding: 6 }}>Cost</th>
           </tr>
         </thead>
         <tbody>
           {SAMPLE_JOBS.map(job => (
             <tr key={job.id} style={{ borderBottom: "1px solid var(--vp-c-border)" }}>
               <td style={{ padding: 6, fontFamily: "monospace" }}>{job.id}</td>
               <td style={{ padding: 6 }}>
                 <span style={{ color: STATUS_COLORS[job.status] || "var(--vp-c-text)" }}>
                   {job.status}
                 </span>
               </td>
               <td style={{ padding: 6 }}>{job.base_model}</td>
               <td style={{ padding: 6 }}>{job.dataset}</td>
               <td style={{ padding: 6, textAlign: "right" }}>{job.loss > 0 ? job.loss.toFixed(3) : "-"}</td>
               <td style={{ padding: 6, textAlign: "right" }}>
                 <div style={{ display: "flex", alignItems: "center", gap: 4, justifyContent: "flex-end" }}>
                   <div style={{ width: 60, height: 8, background: "var(--vp-c-border)", borderRadius: 4, overflow: "hidden" }}>
                     <div style={{ height: "100%", width: `${job.progress}%`, background: STATUS_COLORS[job.status], borderRadius: 4 }} />
                   </div>
                   {job.progress}%
                 </div>
               </td>
               <td style={{ padding: 6, textAlign: "right" }}>${job.cost_usd.toFixed(2)}</td>
             </tr>
           ))}
         </tbody>
       </table>
     )}

     {tab === "eval" && (
       <>
         <strong>SWE-Bench Evaluation Results</strong>
         <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 12, marginTop: 8 }}>
           <thead>
             <tr style={{ borderBottom: "1px solid var(--vp-c-border)" }}>
               <th style={{ textAlign: "left", padding: 6 }}>Model</th>
               <th style={{ textAlign: "right", padding: 6 }}>Tasks</th>
               <th style={{ textAlign: "right", padding: 6 }}>Resolved</th>
               <th style={{ textAlign: "right", padding: 6 }}>Rate</th>
               <th style={{ textAlign: "right", padding: 6 }}>Avg Time</th>
             </tr>
           </thead>
           <tbody>
             {SAMPLE_EVALS.map(ev => (
               <tr key={ev.model} style={{ borderBottom: "1px solid var(--vp-c-border)" }}>
                 <td style={{ padding: 6, color: ev.rate > 20 ? "var(--vp-c-success)" : "var(--vp-c-text)" }}>{ev.model}</td>
                 <td style={{ padding: 6, textAlign: "right" }}>{ev.tasks}</td>
                 <td style={{ padding: 6, textAlign: "right" }}>{ev.resolved}</td>
                 <td style={{ padding: 6, textAlign: "right", fontWeight: 700 }}>{ev.rate.toFixed(1)}%</td>
                 <td style={{ padding: 6, textAlign: "right" }}>{ev.avg_time.toFixed(1)}s</td>
               </tr>
             ))}
           </tbody>
         </table>
         <button style={{ marginTop: 12, padding: "6px 14px", background: "var(--vp-c-brand)", color: "var(--text-primary)", border: "none", borderRadius: 4, cursor: "pointer" }}>
           Run Evaluation
         </button>
       </>
     )}

     {tab === "lora" && (
       <>
         <strong>LoRA Adapters</strong>
         <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 12, marginTop: 8 }}>
           <thead>
             <tr style={{ borderBottom: "1px solid var(--vp-c-border)" }}>
               <th style={{ textAlign: "left", padding: 6 }}>Name</th>
               <th style={{ textAlign: "left", padding: 6 }}>Base Model</th>
               <th style={{ textAlign: "right", padding: 6 }}>Rank</th>
               <th style={{ textAlign: "right", padding: 6 }}>Size</th>
               <th style={{ textAlign: "right", padding: 6 }}>Actions</th>
             </tr>
           </thead>
           <tbody>
             {SAMPLE_ADAPTERS.map(a => (
               <tr key={a.name} style={{ borderBottom: "1px solid var(--vp-c-border)" }}>
                 <td style={{ padding: 6, fontFamily: "monospace", color: "var(--vp-c-brand)" }}>{a.name}</td>
                 <td style={{ padding: 6 }}>{a.base_model}</td>
                 <td style={{ padding: 6, textAlign: "right" }}>{a.rank}</td>
                 <td style={{ padding: 6, textAlign: "right" }}>{a.size_mb} MB</td>
                 <td style={{ padding: 6, textAlign: "right" }}>
                   <button style={{ padding: "2px 8px", background: "transparent", color: "var(--vp-c-brand)", border: "1px solid var(--vp-c-brand)", borderRadius: 4, cursor: "pointer", marginRight: 4, fontSize: 11 }}>Merge</button>
                   <button style={{ padding: "2px 8px", background: "transparent", color: "var(--vp-c-danger)", border: "1px solid var(--vp-c-danger)", borderRadius: 4, cursor: "pointer", fontSize: 11 }}>Delete</button>
                 </td>
               </tr>
             ))}
           </tbody>
         </table>
       </>
     )}
   </div>
 );
}
