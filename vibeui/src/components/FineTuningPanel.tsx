/**
 * FineTuningPanel — Model fine-tuning dashboard.
 *
 * Dataset builder, training config editor, job launcher with cost estimation,
 * active jobs list, SWE-bench evaluation runner, and LoRA adapter manager.
 */
import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

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

const STATUS_COLORS: Record<string, string> = {
 completed: "var(--vp-c-success)",
 running: "var(--vp-c-brand)",
 pending: "var(--vp-c-warning)",
 failed: "var(--vp-c-danger)",
 cancelled: "var(--vp-c-border)",
};

const EMPTY_STATS: DatasetStats = {
 example_count: 0,
 total_tokens: 0,
 avg_tokens_per_example: 0,
 max_tokens: 0,
 languages: {},
 invalid_count: 0,
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

 const [stats, setStats] = useState<DatasetStats>(EMPTY_STATS);
 const [jobs, setJobs] = useState<FineTuneJob[]>([]);
 const [evals, setEvals] = useState<EvalResult[]>([]);
 const [adapters, setAdapters] = useState<LoraAdapter[]>([]);
 const [loading, setLoading] = useState(false);
 const [error, setError] = useState<string | null>(null);

 const loadStats = useCallback(async () => {
   try {
     setLoading(true);
     const result = await invoke<DatasetStats>("get_fine_tuning_stats", { workspace: "." });
     setStats(result);
   } catch (e) {
     setError(String(e));
   } finally {
     setLoading(false);
   }
 }, []);

 const loadJobs = useCallback(async () => {
   try {
     const result = await invoke<FineTuneJob[]>("list_fine_tuning_jobs");
     setJobs(result);
   } catch (e) {
     setError(String(e));
   }
 }, []);

 const loadEvals = useCallback(async () => {
   try {
     const result = await invoke<EvalResult[]>("list_fine_tuning_evals");
     setEvals(result);
   } catch (e) {
     setError(String(e));
   }
 }, []);

 const loadAdapters = useCallback(async () => {
   try {
     const result = await invoke<LoraAdapter[]>("list_fine_tuning_adapters");
     setAdapters(result);
   } catch (e) {
     setError(String(e));
   }
 }, []);

 useEffect(() => {
   if (tab === "dataset") loadStats();
   else if (tab === "jobs") loadJobs();
   else if (tab === "eval") loadEvals();
   else if (tab === "lora") loadAdapters();
   else if (tab === "train") loadStats(); // need stats for cost estimate
 }, [tab, loadStats, loadJobs, loadEvals, loadAdapters]);

 const handleCreateJob = async () => {
   try {
     setLoading(true);
     setError(null);
     await invoke<FineTuneJob>("create_fine_tuning_job", {
       baseModel,
       dataset: dataSource === "codebase" ? "codebase-workspace" : dataSource === "git" ? "git-history-main" : "conversations-agent",
       epochs,
       provider,
       learningRate: lr,
       batchSize,
       loraRank: provider === "Local (LoRA)" ? loraRank : null,
     });
     setTab("jobs");
     await loadJobs();
   } catch (e) {
     setError(String(e));
   } finally {
     setLoading(false);
   }
 };

 const handleCreateAdapter = async () => {
   try {
     setError(null);
     const name = `adapter-r${loraRank}-${Date.now()}`;
     await invoke<LoraAdapter>("create_fine_tuning_adapter", {
       name,
       baseModel,
       rank: loraRank,
       sizeMb: Math.round(loraRank * 3.2),
     });
     await loadAdapters();
   } catch (e) {
     setError(String(e));
   }
 };

 return (
   <div style={{ padding: 16, color: "var(--vp-c-text)", background: "var(--vp-c-bg)", minHeight: "100%" }}>
     <h2 style={{ margin: "0 0 12px", fontSize: 18 }}>Fine-Tuning</h2>

     {error && (
       <div style={{ padding: 8, marginBottom: 8, background: "var(--error-bg)", color: "var(--text-danger)", borderRadius: 4, fontSize: 12 }}>
         {error}
         <button onClick={() => setError(null)} style={{ marginLeft: 8, background: "transparent", color: "var(--text-danger)", border: "none", cursor: "pointer" }}>dismiss</button>
       </div>
     )}

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
         {loading ? (
           <div style={{ padding: 20, textAlign: "center", color: "var(--vp-c-text-2)" }}>Scanning workspace...</div>
         ) : (
           <>
             <div style={{ display: "grid", gridTemplateColumns: "repeat(4, 1fr)", gap: 8, marginBottom: 12 }}>
               <div style={{ padding: 10, border: "1px solid var(--vp-c-border)", borderRadius: 6, textAlign: "center" }}>
                 <div style={{ fontSize: 22, fontWeight: 700, color: "var(--vp-c-brand)" }}>{stats.example_count.toLocaleString()}</div>
                 <div style={{ fontSize: 11 }}>Examples</div>
               </div>
               <div style={{ padding: 10, border: "1px solid var(--vp-c-border)", borderRadius: 6, textAlign: "center" }}>
                 <div style={{ fontSize: 22, fontWeight: 700, color: "var(--vp-c-success)" }}>{(stats.total_tokens / 1000).toFixed(0)}K</div>
                 <div style={{ fontSize: 11 }}>Tokens</div>
               </div>
               <div style={{ padding: 10, border: "1px solid var(--vp-c-border)", borderRadius: 6, textAlign: "center" }}>
                 <div style={{ fontSize: 22, fontWeight: 700 }}>{stats.avg_tokens_per_example.toFixed(0)}</div>
                 <div style={{ fontSize: 11 }}>Avg Tokens/Ex</div>
               </div>
               <div style={{ padding: 10, border: "1px solid var(--vp-c-border)", borderRadius: 6, textAlign: "center" }}>
                 <div style={{ fontSize: 22, fontWeight: 700, color: stats.invalid_count > 0 ? "var(--vp-c-danger)" : "var(--vp-c-success)" }}>
                   {stats.invalid_count}
                 </div>
                 <div style={{ fontSize: 11 }}>Invalid</div>
               </div>
             </div>

             {/* Language distribution */}
             <strong style={{ fontSize: 12 }}>Language Distribution</strong>
             <div style={{ marginTop: 4 }}>
               {Object.entries(stats.languages).map(([lang, count]) => (
                 <div key={lang} style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 4 }}>
                   <span style={{ width: 80 }}>{lang}</span>
                   <div style={{ flex: 1, height: 16, background: "var(--vp-c-border)", borderRadius: 4, overflow: "hidden" }}>
                     <div style={{ height: "100%", width: `${stats.example_count > 0 ? (count / stats.example_count) * 100 : 0}%`, background: "var(--vp-c-brand)", borderRadius: 4 }} />
                   </div>
                   <span style={{ width: 50, textAlign: "right", fontSize: 12 }}>{count}</span>
                 </div>
               ))}
             </div>
           </>
         )}

         <div style={{ display: "flex", gap: 6, marginTop: 12 }}>
           <button onClick={loadStats} style={{ padding: "6px 14px", background: "var(--vp-c-brand)", color: "var(--text-primary)", border: "none", borderRadius: 4, cursor: "pointer" }}>
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
             Tokens: {(stats.total_tokens * epochs / 1000).toFixed(0)}K |
             Est. Cost: <span style={{ color: "var(--vp-c-warning)" }}>${(stats.total_tokens * epochs * 0.000008).toFixed(2)}</span> |
             Est. Time: ~{Math.ceil(stats.total_tokens * epochs / 50000)} min
           </div>
         </div>

         <button onClick={handleCreateJob} disabled={loading} style={{ marginTop: 12, padding: "8px 20px", background: "var(--vp-c-brand)", color: "var(--text-primary)", border: "none", borderRadius: 4, cursor: "pointer", fontWeight: 600, opacity: loading ? 0.6 : 1 }}>
           {loading ? "Creating..." : "Launch Training Job"}
         </button>
       </div>
     )}

     {tab === "jobs" && (
       <>
         {jobs.length === 0 ? (
           <div style={{ padding: 20, textAlign: "center", color: "var(--vp-c-text-2)" }}>
             No fine-tuning jobs yet. Go to the Train tab to create one.
           </div>
         ) : (
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
               {jobs.map(job => (
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
       </>
     )}

     {tab === "eval" && (
       <>
         <strong>SWE-Bench Evaluation Results</strong>
         {evals.length === 0 ? (
           <div style={{ padding: 20, textAlign: "center", color: "var(--vp-c-text-2)" }}>
             No evaluation results yet. Run an evaluation to see results.
           </div>
         ) : (
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
               {evals.map(ev => (
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
         )}
         <button style={{ marginTop: 12, padding: "6px 14px", background: "var(--vp-c-brand)", color: "var(--text-primary)", border: "none", borderRadius: 4, cursor: "pointer" }}>
           Run Evaluation
         </button>
       </>
     )}

     {tab === "lora" && (
       <>
         <strong>LoRA Adapters</strong>
         {adapters.length === 0 ? (
           <div style={{ padding: 20, textAlign: "center", color: "var(--vp-c-text-2)" }}>
             No LoRA adapters configured yet.
           </div>
         ) : (
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
               {adapters.map(a => (
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
         )}
         <button onClick={handleCreateAdapter} style={{ marginTop: 12, padding: "6px 14px", background: "var(--vp-c-brand)", color: "var(--text-primary)", border: "none", borderRadius: 4, cursor: "pointer" }}>
           Create Adapter
         </button>
       </>
     )}
   </div>
 );
}
