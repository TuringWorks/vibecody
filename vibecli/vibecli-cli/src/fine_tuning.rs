//! Model fine-tuning infrastructure for coding models.
//!
//! Provides dataset preparation (from codebases, git history, conversations),
//! fine-tuning job management (OpenAI, TogetherAI, Fireworks, local LoRA),
//! SWE-bench evaluation harness, and LoRA adapter management.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// ── Provider & Format ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FineTuneProvider {
    OpenAI,
    TogetherAI,
    Fireworks,
    Local,
}

impl FineTuneProvider {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::OpenAI => "openai",
            Self::TogetherAI => "together_ai",
            Self::Fireworks => "fireworks",
            Self::Local => "local",
        }
    }

    /// Cost per 1k training tokens (USD).
    pub fn cost_per_1k_tokens(&self) -> f64 {
        match self {
            Self::OpenAI => 0.008,
            Self::TogetherAI => 0.002,
            Self::Fireworks => 0.003,
            Self::Local => 0.0,
        }
    }
}

// ── Fine-Tuning Libraries ────────────────────────────────────────────────

/// Open-source fine-tuning frameworks with command generation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FineTuneLibrary {
    /// Unsloth — fast, memory-efficient LoRA/QLoRA on single GPU or Colab.
    Unsloth,
    /// Axolotl — YAML-config-driven fine-tuning with HuggingFace integration.
    Axolotl,
    /// LLaMA Factory — 100+ LLMs, DPO/PPO/SFT/RLHF alignment, CLI-driven.
    LlamaFactory,
    /// DeepSpeed — distributed multi-GPU/multi-node training with ZeRO.
    DeepSpeed,
    /// HuggingFace TRL — Transformer Reinforcement Learning (SFT, DPO, PPO).
    HuggingFaceTRL,
    /// PEFT — Parameter-Efficient Fine-Tuning (LoRA, AdaLoRA, prefix tuning).
    Peft,
}

impl FineTuneLibrary {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Unsloth => "unsloth",
            Self::Axolotl => "axolotl",
            Self::LlamaFactory => "llama-factory",
            Self::DeepSpeed => "deepspeed",
            Self::HuggingFaceTRL => "trl",
            Self::Peft => "peft",
        }
    }

    pub fn github_url(&self) -> &'static str {
        match self {
            Self::Unsloth => "https://github.com/unslothai/unsloth",
            Self::Axolotl => "https://github.com/axolotl-ai-cloud/axolotl",
            Self::LlamaFactory => "https://github.com/hiyouga/LLaMA-Factory",
            Self::DeepSpeed => "https://github.com/deepspeedai/DeepSpeed",
            Self::HuggingFaceTRL => "https://github.com/huggingface/trl",
            Self::Peft => "https://github.com/huggingface/peft",
        }
    }

    pub fn pip_install(&self) -> &'static str {
        match self {
            Self::Unsloth => "pip install unsloth",
            Self::Axolotl => "pip install axolotl",
            Self::LlamaFactory => "pip install llamafactory",
            Self::DeepSpeed => "pip install deepspeed",
            Self::HuggingFaceTRL => "pip install trl",
            Self::Peft => "pip install peft",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::Unsloth => "Fast single-GPU fine-tuning with 2x speed, 60% less memory. Supports LoRA/QLoRA for Llama, Mistral, Gemma, Phi.",
            Self::Axolotl => "YAML-config-driven fine-tuning. Minimal code, reproducible setups. LoRA, QLoRA, full fine-tune with HuggingFace models.",
            Self::LlamaFactory => "100+ LLMs/VLMs. Full alignment: SFT, DPO, PPO, RLHF, KTO. CLI + Web UI. Distributed training support.",
            Self::DeepSpeed => "Multi-GPU/multi-node distributed training. ZeRO stages 0-3 + Infinity. Gradient checkpointing, mixed precision.",
            Self::HuggingFaceTRL => "Transformer RL: SFTTrainer, DPOTrainer, PPOTrainer. Direct preference optimization, reward modeling.",
            Self::Peft => "Parameter-efficient methods: LoRA, AdaLoRA, prefix tuning, prompt tuning, IA3. Works with any HuggingFace model.",
        }
    }

    /// Generate a training command/script for this library.
    pub fn generate_command(&self, model: &str, dataset: &str, output: &str) -> String {
        match self {
            Self::Unsloth => format!(
                "python -c \"\nfrom unsloth import FastLanguageModel\nmodel, tokenizer = FastLanguageModel.from_pretrained('{model}', max_seq_length=2048, load_in_4bit=True)\nmodel = FastLanguageModel.get_peft_model(model, r=16, lora_alpha=16)\nfrom trl import SFTTrainer\nfrom transformers import TrainingArguments\nfrom datasets import load_dataset\ndataset = load_dataset('json', data_files='{dataset}')\ntrainer = SFTTrainer(model=model, tokenizer=tokenizer, train_dataset=dataset['train'],\n    args=TrainingArguments(output_dir='{output}', per_device_train_batch_size=2, num_train_epochs=3))\ntrainer.train()\nmodel.save_pretrained('{output}')\n\""
            ),
            Self::Axolotl => format!(
                "# axolotl.yaml\nbase_model: {model}\ndatasets:\n  - path: {dataset}\n    type: alpaca\noutput_dir: {output}\nlora_r: 16\nlora_alpha: 32\nmicro_batch_size: 2\nnum_epochs: 3\nlearning_rate: 2e-4\n\n# Run:\naxolotl train axolotl.yaml"
            ),
            Self::LlamaFactory => format!(
                "llamafactory-cli train \\\n  --model_name_or_path {model} \\\n  --dataset {dataset} \\\n  --output_dir {output} \\\n  --finetuning_type lora \\\n  --lora_rank 16 \\\n  --num_train_epochs 3 \\\n  --per_device_train_batch_size 2 \\\n  --learning_rate 2e-4 \\\n  --template default"
            ),
            Self::DeepSpeed => format!(
                "deepspeed --num_gpus=4 train.py \\\n  --model_name_or_path {model} \\\n  --train_file {dataset} \\\n  --output_dir {output} \\\n  --deepspeed ds_config.json \\\n  --per_device_train_batch_size 2 \\\n  --num_train_epochs 3"
            ),
            Self::HuggingFaceTRL => format!(
                "python -c \"\nfrom trl import SFTTrainer, SFTConfig\nfrom transformers import AutoModelForCausalLM, AutoTokenizer\nfrom datasets import load_dataset\nmodel = AutoModelForCausalLM.from_pretrained('{model}')\ntokenizer = AutoTokenizer.from_pretrained('{model}')\ndataset = load_dataset('json', data_files='{dataset}')\ntrainer = SFTTrainer(model=model, tokenizer=tokenizer, train_dataset=dataset['train'],\n    args=SFTConfig(output_dir='{output}', num_train_epochs=3))\ntrainer.train()\n\""
            ),
            Self::Peft => format!(
                "python -c \"\nfrom peft import LoraConfig, get_peft_model\nfrom transformers import AutoModelForCausalLM, AutoTokenizer, TrainingArguments, Trainer\nfrom datasets import load_dataset\nmodel = AutoModelForCausalLM.from_pretrained('{model}')\nlora_config = LoraConfig(r=16, lora_alpha=32, target_modules=['q_proj', 'v_proj'])\nmodel = get_peft_model(model, lora_config)\ndataset = load_dataset('json', data_files='{dataset}')\ntrainer = Trainer(model=model, train_dataset=dataset['train'],\n    args=TrainingArguments(output_dir='{output}', num_train_epochs=3))\ntrainer.train()\nmodel.save_pretrained('{output}')\n\""
            ),
        }
    }

    pub fn all() -> Vec<Self> {
        vec![Self::Unsloth, Self::Axolotl, Self::LlamaFactory, Self::DeepSpeed, Self::HuggingFaceTRL, Self::Peft]
    }
}

// ── Notebook Environments ────────────────────────────────────────────────

/// Notebook/environment platforms for training runs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NotebookEnvironment {
    GoogleColab,
    KaggleNotebook,
    SageMakerStudio,
    LightningStudio,
    GradioSpaces,
    LocalJupyter,
}

impl NotebookEnvironment {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::GoogleColab => "google_colab",
            Self::KaggleNotebook => "kaggle",
            Self::SageMakerStudio => "sagemaker",
            Self::LightningStudio => "lightning",
            Self::GradioSpaces => "gradio_spaces",
            Self::LocalJupyter => "local_jupyter",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::GoogleColab => "Google Colab (Free T4 GPU)",
            Self::KaggleNotebook => "Kaggle Notebook (Free P100 GPU)",
            Self::SageMakerStudio => "AWS SageMaker Studio",
            Self::LightningStudio => "Lightning AI Studio",
            Self::GradioSpaces => "Hugging Face Spaces",
            Self::LocalJupyter => "Local Jupyter Lab",
        }
    }

    pub fn gpu_info(&self) -> &'static str {
        match self {
            Self::GoogleColab => "T4 16GB (free) / A100 40GB (Pro) / L4 (Pro+)",
            Self::KaggleNotebook => "P100 16GB (free, 30hr/week) / T4x2",
            Self::SageMakerStudio => "ml.g5.xlarge (A10G) to ml.p4d.24xlarge (A100x8)",
            Self::LightningStudio => "T4, A10G, A100 — pay-as-you-go",
            Self::GradioSpaces => "T4 (free with ZeroGPU) / A10G (paid)",
            Self::LocalJupyter => "Your local GPU(s)",
        }
    }

    /// Generate a notebook cell that sets up the fine-tuning environment.
    pub fn generate_setup_cell(&self, library: &FineTuneLibrary) -> String {
        let install = library.pip_install();
        match self {
            Self::GoogleColab => format!(
                "# Google Colab setup — run this cell first\n!{install}\n!pip install transformers datasets accelerate bitsandbytes\n\nimport torch\nprint(f'GPU: {{torch.cuda.get_device_name(0)}}')\nprint(f'VRAM: {{torch.cuda.get_device_properties(0).total_mem / 1e9:.1f}} GB')"
            ),
            Self::KaggleNotebook => format!(
                "# Kaggle Notebook setup\n!{install}\n!pip install transformers datasets accelerate bitsandbytes\n\nimport torch\nprint(f'GPU: {{torch.cuda.get_device_name(0)}}')"
            ),
            Self::SageMakerStudio => format!(
                "# SageMaker Studio setup\n!{install}\n!pip install sagemaker transformers datasets\n\nimport sagemaker\nsession = sagemaker.Session()\nprint(f'Region: {{session.boto_region_name}}')"
            ),
            Self::LightningStudio | Self::GradioSpaces | Self::LocalJupyter => format!(
                "# Environment setup\n!{install}\n!pip install transformers datasets accelerate bitsandbytes\n\nimport torch\nif torch.cuda.is_available():\n    print(f'GPU: {{torch.cuda.get_device_name(0)}}')\nelse:\n    print('No GPU — will use CPU (slow)')"
            ),
        }
    }

    pub fn all() -> Vec<Self> {
        vec![Self::GoogleColab, Self::KaggleNotebook, Self::SageMakerStudio, Self::LightningStudio, Self::GradioSpaces, Self::LocalJupyter]
    }
}

// ── RL Gym Environments ──────────────────────────────────────────────────

/// Reinforcement learning environment frameworks for agent training.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RlEnvironment {
    /// NeMo Gym (TuringWorks/Gym) — RL training environments for LLMs.
    NeMoGym,
    /// OpenAI Gymnasium — standard RL environments (CartPole, Atari, MuJoCo).
    Gymnasium,
    /// ReasoningGym — reasoning task environments for LLMs.
    ReasoningGym,
    /// SWE-Bench — software engineering task environment.
    SweBench,
    /// LMSYS Arena — human preference data for RLHF.
    LmsysArena,
    /// TRL + PPO — Transformer RL with proximal policy optimization.
    TrlPpo,
    /// Aviary — tool-use environments (search, calculators, APIs).
    Aviary,
}

impl RlEnvironment {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::NeMoGym => "nemo_gym",
            Self::Gymnasium => "gymnasium",
            Self::ReasoningGym => "reasoning_gym",
            Self::SweBench => "swe_bench",
            Self::LmsysArena => "lmsys_arena",
            Self::TrlPpo => "trl_ppo",
            Self::Aviary => "aviary",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::NeMoGym => "NeMo Gym (LLM RL environments)",
            Self::Gymnasium => "OpenAI Gymnasium (classic RL)",
            Self::ReasoningGym => "Reasoning Gym (logic/math tasks)",
            Self::SweBench => "SWE-Bench (code editing tasks)",
            Self::LmsysArena => "LMSYS Chatbot Arena (RLHF preference data)",
            Self::TrlPpo => "TRL PPO Trainer (direct RLHF)",
            Self::Aviary => "Aviary (tool-use RL environments)",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::NeMoGym => "60+ resource servers: GPQA, MMLU, ARC, coding, math proofs (Lean4), tool-use (Aviary), safety. Client-server architecture with Ray rollout collection.",
            Self::Gymnasium => "Standard RL API: CartPole, Atari, MuJoCo, robotics. Used for classic RL algorithm development. Wrapper-based extensibility.",
            Self::ReasoningGym => "Logic puzzles, mathematical reasoning, pattern recognition. Designed for evaluating and training LLM reasoning capabilities.",
            Self::SweBench => "Real-world GitHub issues. Agent must read codebase, understand bug, write fix. Gold standard for code agent evaluation.",
            Self::LmsysArena => "Human preference data from Chatbot Arena. ELO-ranked model comparisons. Used for DPO/RLHF training data.",
            Self::TrlPpo => "Proximal Policy Optimization with reward models. Train LLMs to optimize for human preferences via RL.",
            Self::Aviary => "Tool-use environments: web search, calculators, calendar scheduling, API calls. Tests agent ability to use external tools.",
        }
    }

    pub fn github_url(&self) -> &'static str {
        match self {
            Self::NeMoGym => "https://github.com/TuringWorks/Gym",
            Self::Gymnasium => "https://github.com/Farama-Foundation/Gymnasium",
            Self::ReasoningGym => "https://github.com/reasoning-gym/reasoning-gym",
            Self::SweBench => "https://github.com/princeton-nlp/SWE-bench",
            Self::LmsysArena => "https://github.com/lm-sys/FastChat",
            Self::TrlPpo => "https://github.com/huggingface/trl",
            Self::Aviary => "https://github.com/TuringWorks/Gym",
        }
    }

    pub fn pip_install(&self) -> &'static str {
        match self {
            Self::NeMoGym => "pip install nemo-rl-gym",
            Self::Gymnasium => "pip install gymnasium",
            Self::ReasoningGym => "pip install reasoning-gym",
            Self::SweBench => "pip install swebench",
            Self::LmsysArena => "pip install fschat",
            Self::TrlPpo => "pip install trl",
            Self::Aviary => "pip install nemo-rl-gym",
        }
    }

    /// Generate a sample training loop for this RL environment.
    pub fn generate_sample(&self, model: &str) -> String {
        match self {
            Self::NeMoGym => format!(
                "# NeMo Gym — multi-step RL environment for LLMs\nfrom nemo_rl import Environment, Agent\n\nenv = Environment.from_config('gpqa_diamond')\nagent = Agent.from_pretrained('{model}')\n\nfor episode in range(100):\n    obs = env.reset()\n    done = False\n    while not done:\n        action = agent.act(obs)\n        obs, reward, done, info = env.step(action)\n        agent.learn(reward)\n    print(f'Episode {{episode}}: reward={{reward:.2f}}')"
            ),
            Self::Gymnasium =>
                "# OpenAI Gymnasium — classic RL\nimport gymnasium as gym\n\nenv = gym.make('CartPole-v1')\nobs, info = env.reset()\n\nfor _ in range(1000):\n    action = env.action_space.sample()  # Replace with your policy\n    obs, reward, terminated, truncated, info = env.step(action)\n    if terminated or truncated:\n        obs, info = env.reset()\nenv.close()".to_string(),
            Self::TrlPpo => format!(
                "# TRL PPO — RLHF training\nfrom trl import PPOTrainer, PPOConfig, AutoModelForCausalLMWithValueHead\nfrom transformers import AutoTokenizer\n\nmodel = AutoModelForCausalLMWithValueHead.from_pretrained('{model}')\ntokenizer = AutoTokenizer.from_pretrained('{model}')\n\nppo_config = PPOConfig(batch_size=4, learning_rate=1e-5)\ntrainer = PPOTrainer(config=ppo_config, model=model, tokenizer=tokenizer)\n\n# Training loop with reward model\nfor batch in dataloader:\n    queries = tokenizer(batch['query'], return_tensors='pt')\n    responses = model.generate(**queries)\n    rewards = reward_model(queries, responses)\n    trainer.step(queries, responses, rewards)"
            ),
            Self::SweBench => format!(
                "# SWE-Bench — evaluate code agent on real GitHub issues\nfrom swebench import get_model_report\n\nresults = get_model_report(\n    model='{model}',\n    dataset='swebench_lite',\n    split='test'\n)\nprint(f'Pass@1: {{results[\"pass@1\"]:.1%}}')"
            ),
            _ => format!("# See {} for setup instructions\n# pip install: {}", self.github_url(), self.pip_install()),
        }
    }

    pub fn all() -> Vec<Self> {
        vec![Self::NeMoGym, Self::Gymnasium, Self::ReasoningGym, Self::SweBench, Self::LmsysArena, Self::TrlPpo, Self::Aviary]
    }
}

// ── Document Processing (MinerU integration) ─────────────────────────────

/// Document processing tools for RAG pipeline data preparation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DocumentProcessor {
    /// MinerU — PDF to structured Markdown/JSON for LLMs.
    MinerU,
    /// Docling (IBM) — document understanding for RAG.
    Docling,
    /// Unstructured — open-source document ETL.
    Unstructured,
    /// LlamaParse — LlamaIndex document parsing.
    LlamaParse,
    /// VibeCody built-in — document_ingest.rs.
    BuiltIn,
}

impl DocumentProcessor {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::MinerU => "mineru",
            Self::Docling => "docling",
            Self::Unstructured => "unstructured",
            Self::LlamaParse => "llamaparse",
            Self::BuiltIn => "builtin",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::MinerU => "MinerU (PDF to Markdown/JSON)",
            Self::Docling => "Docling (IBM document understanding)",
            Self::Unstructured => "Unstructured (document ETL)",
            Self::LlamaParse => "LlamaParse (LlamaIndex)",
            Self::BuiltIn => "VibeCody Built-in (9 formats)",
        }
    }

    pub fn install_command(&self) -> &'static str {
        match self {
            Self::MinerU => "pip install magic-pdf[full]",
            Self::Docling => "pip install docling",
            Self::Unstructured => "pip install unstructured[all-docs]",
            Self::LlamaParse => "pip install llama-parse",
            Self::BuiltIn => "# Built into VibeCody — use /ingest",
        }
    }

    pub fn process_command(&self, input: &str, output: &str) -> String {
        match self {
            Self::MinerU => format!("magic-pdf -p {} -o {} -m auto", input, output),
            Self::Docling => format!("docling {} --output {}", input, output),
            Self::Unstructured => format!("unstructured-ingest local --input-path {} --output-dir {}", input, output),
            Self::LlamaParse => format!("# Requires LLAMA_CLOUD_API_KEY\npython -c \"from llama_parse import LlamaParse; parser = LlamaParse(); result = parser.load_data('{}')\"", input),
            Self::BuiltIn => format!("/ingest {}", input),
        }
    }

    pub fn all() -> Vec<Self> {
        vec![Self::MinerU, Self::Docling, Self::Unstructured, Self::LlamaParse, Self::BuiltIn]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DatasetFormat {
    ChatML,
    Alpaca,
    ShareGPT,
    Completion,
    SWEBench,
}

impl DatasetFormat {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ChatML => "chatml",
            Self::Alpaca => "alpaca",
            Self::ShareGPT => "sharegpt",
            Self::Completion => "completion",
            Self::SWEBench => "swe_bench",
        }
    }
}

// ── Training Example ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingExample {
    pub system: Option<String>,
    pub messages: Vec<(String, String)>, // (role, content)
    pub metadata: HashMap<String, String>,
}

impl TrainingExample {
    pub fn new(messages: Vec<(String, String)>) -> Self {
        Self {
            system: None,
            messages,
            metadata: HashMap::new(),
        }
    }

    pub fn with_system(mut self, system: &str) -> Self {
        self.system = Some(system.to_string());
        self
    }

    /// Rough token count (words * 1.3).
    pub fn estimated_tokens(&self) -> usize {
        let mut chars = 0;
        if let Some(s) = &self.system { chars += s.len(); }
        for (_, content) in &self.messages { chars += content.len(); }
        (chars as f64 / 4.0) as usize // ~4 chars per token
    }

    pub fn is_valid(&self) -> bool {
        !self.messages.is_empty()
            && self.messages.iter().all(|(role, content)| !role.is_empty() && !content.is_empty())
    }
}

// ── Dataset Stats ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DatasetStats {
    pub example_count: usize,
    pub total_tokens: usize,
    pub avg_tokens_per_example: f64,
    pub max_tokens: usize,
    pub min_tokens: usize,
    pub languages: HashMap<String, usize>,
    pub invalid_count: usize,
}

// ── Dataset ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dataset {
    pub name: String,
    pub examples: Vec<TrainingExample>,
    pub format: DatasetFormat,
}

impl Dataset {
    pub fn new(name: &str, format: DatasetFormat) -> Self {
        Self {
            name: name.to_string(),
            examples: Vec::new(),
            format,
        }
    }

    /// Extract training data from a codebase — function docstrings + implementations.
    pub fn from_codebase(path: &Path, language_filter: Option<&str>) -> Result<Self> {
        let mut examples = Vec::new();

        for entry in walkdir::WalkDir::new(path)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let p = entry.path();
            if p.is_dir() { continue; }

            let path_str = p.to_string_lossy();
            if path_str.contains("/.") || path_str.contains("/node_modules/")
                || path_str.contains("/target/") {
                continue;
            }

            let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");
            let lang = match ext {
                "rs" => "rust",
                "ts" | "tsx" => "typescript",
                "js" | "jsx" => "javascript",
                "py" | "pyi" => "python",
                "go" => "go",
                _ => continue,
            };

            if let Some(filter) = language_filter {
                if lang != filter { continue; }
            }

            let content = match std::fs::read_to_string(p) {
                Ok(c) => c,
                Err(_) => continue,
            };

            // Extract function definitions with their doc comments
            let funcs = extract_documented_functions(&content, lang);
            for (doc, signature, body) in funcs {
                let mut metadata = HashMap::new();
                metadata.insert("language".to_string(), lang.to_string());
                metadata.insert("file".to_string(), p.display().to_string());

                examples.push(TrainingExample {
                    system: Some("You are a coding assistant. Implement the described function.".to_string()),
                    messages: vec![
                        ("user".to_string(), format!("{}\n\nImplement: {}", doc, signature)),
                        ("assistant".to_string(), body),
                    ],
                    metadata,
                });
            }
        }

        Ok(Dataset {
            name: format!("codebase-{}", path.file_name().unwrap_or_default().to_string_lossy()),
            examples,
            format: DatasetFormat::ChatML,
        })
    }

    /// Extract commit message + diff pairs from git history.
    pub fn from_git_history(repo: &Path, max_commits: usize) -> Result<Self> {
        let output = std::process::Command::new("git")
            .args(["log", "--oneline", "-n", &max_commits.to_string(), "--format=%H %s"])
            .current_dir(repo)
            .output()
            .context("failed to run git log")?;

        let log_output = String::from_utf8_lossy(&output.stdout);
        let mut examples = Vec::new();

        for line in log_output.lines().take(max_commits) {
            let parts: Vec<&str> = line.splitn(2, ' ').collect();
            if parts.len() < 2 { continue; }
            let (hash, message) = (parts[0], parts[1]);

            let diff_output = std::process::Command::new("git")
                .args(["show", "--stat", "--format=", hash])
                .current_dir(repo)
                .output();

            let diff = match diff_output {
                Ok(o) => String::from_utf8_lossy(&o.stdout).to_string(),
                Err(_) => continue,
            };

            if diff.trim().is_empty() { continue; }

            let mut metadata = HashMap::new();
            metadata.insert("commit".to_string(), hash.to_string());

            examples.push(TrainingExample {
                system: Some("You are a coding assistant. Given a description of code changes, produce the diff.".to_string()),
                messages: vec![
                    ("user".to_string(), format!("Make the following change: {}", message)),
                    ("assistant".to_string(), diff),
                ],
                metadata,
            });
        }

        Ok(Dataset {
            name: format!("git-history-{}", repo.file_name().unwrap_or_default().to_string_lossy()),
            examples,
            format: DatasetFormat::ChatML,
        })
    }

    /// Load from saved agent conversations (JSONL format).
    pub fn from_conversations(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .context("failed to read conversations file")?;

        let mut examples = Vec::new();
        for line in content.lines() {
            if line.trim().is_empty() { continue; }
            match serde_json::from_str::<TrainingExample>(line) {
                Ok(ex) => examples.push(ex),
                Err(_) => continue,
            }
        }

        Ok(Dataset {
            name: format!("conversations-{}", path.file_name().unwrap_or_default().to_string_lossy()),
            examples,
            format: DatasetFormat::ChatML,
        })
    }

    /// Export as OpenAI-compatible JSONL.
    pub fn to_jsonl(&self, path: &Path) -> Result<()> {
        use std::io::Write;
        let mut file = std::fs::File::create(path)
            .context("failed to create JSONL file")?;

        for example in &self.examples {
            let mut messages = Vec::new();
            if let Some(sys) = &example.system {
                messages.push(serde_json::json!({"role": "system", "content": sys}));
            }
            for (role, content) in &example.messages {
                messages.push(serde_json::json!({"role": role, "content": content}));
            }
            let obj = serde_json::json!({"messages": messages});
            writeln!(file, "{}", serde_json::to_string(&obj).unwrap_or_default())?;
        }

        Ok(())
    }

    /// Split into train/validation sets.
    pub fn split(&self, train_ratio: f32) -> (Dataset, Dataset) {
        let split_idx = (self.examples.len() as f32 * train_ratio) as usize;
        let (train, val) = self.examples.split_at(split_idx.min(self.examples.len()));

        (
            Dataset {
                name: format!("{}-train", self.name),
                examples: train.to_vec(),
                format: self.format.clone(),
            },
            Dataset {
                name: format!("{}-val", self.name),
                examples: val.to_vec(),
                format: self.format.clone(),
            },
        )
    }

    /// Validate the dataset and return issues found.
    pub fn validate(&self) -> Vec<String> {
        let mut issues = Vec::new();

        if self.examples.is_empty() {
            issues.push("Dataset is empty".to_string());
        }

        for (i, ex) in self.examples.iter().enumerate() {
            if ex.messages.is_empty() {
                issues.push(format!("Example {} has no messages", i));
            }
            for (j, (role, content)) in ex.messages.iter().enumerate() {
                if role.is_empty() {
                    issues.push(format!("Example {} message {} has empty role", i, j));
                }
                if content.is_empty() {
                    issues.push(format!("Example {} message {} has empty content", i, j));
                }
            }
            if ex.estimated_tokens() > 8192 {
                issues.push(format!("Example {} exceeds 8192 tokens (~{})", i, ex.estimated_tokens()));
            }
        }

        issues
    }

    /// Compute dataset statistics.
    pub fn stats(&self) -> DatasetStats {
        if self.examples.is_empty() {
            return DatasetStats::default();
        }

        let token_counts: Vec<usize> = self.examples.iter()
            .map(|e| e.estimated_tokens())
            .collect();

        let total: usize = token_counts.iter().sum();
        let mut languages: HashMap<String, usize> = HashMap::new();
        let mut invalid = 0;

        for ex in &self.examples {
            if !ex.is_valid() { invalid += 1; }
            if let Some(lang) = ex.metadata.get("language") {
                *languages.entry(lang.clone()).or_default() += 1;
            }
        }

        DatasetStats {
            example_count: self.examples.len(),
            total_tokens: total,
            avg_tokens_per_example: total as f64 / self.examples.len() as f64,
            max_tokens: token_counts.iter().copied().max().unwrap_or(0),
            min_tokens: token_counts.iter().copied().min().unwrap_or(0),
            languages,
            invalid_count: invalid,
        }
    }

    /// Filter examples by language metadata.
    pub fn filter_by_language(&self, lang: &str) -> Dataset {
        Dataset {
            name: format!("{}-{}", self.name, lang),
            examples: self.examples.iter()
                .filter(|e| e.metadata.get("language").map(|l| l == lang).unwrap_or(false))
                .cloned()
                .collect(),
            format: self.format.clone(),
        }
    }

    /// Remove near-duplicate examples (>90% content overlap by length ratio).
    pub fn deduplicate(&self) -> Dataset {
        let mut deduped: Vec<TrainingExample> = Vec::new();
        let mut content_hashes: std::collections::HashSet<u64> = std::collections::HashSet::new();

        for ex in &self.examples {
            let hash = simple_hash(&ex.messages);
            if content_hashes.insert(hash) {
                deduped.push(ex.clone());
            }
        }

        Dataset {
            name: format!("{}-deduped", self.name),
            examples: deduped,
            format: self.format.clone(),
        }
    }

    pub fn len(&self) -> usize { self.examples.len() }
    pub fn is_empty(&self) -> bool { self.examples.is_empty() }
}

fn simple_hash(messages: &[(String, String)]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for (role, content) in messages {
        role.hash(&mut hasher);
        content.hash(&mut hasher);
    }
    hasher.finish()
}

/// Extract documented functions from source code.
fn extract_documented_functions(content: &str, lang: &str) -> Vec<(String, String, String)> {
    let lines: Vec<&str> = content.lines().collect();
    let mut results = Vec::new();

    let (doc_prefix, fn_pattern) = match lang {
        "rust" => ("///", r"(?:pub\s+)?fn\s+\w+"),
        "python" => ("\"\"\"", r"def\s+\w+"),
        "typescript" | "javascript" => ("/**", r"(?:export\s+)?function\s+\w+"),
        "go" => ("//", r"func\s+\w+"),
        _ => return results,
    };

    let fn_re = match regex::Regex::new(fn_pattern) {
        Ok(r) => r,
        Err(_) => return results,
    };

    let mut i = 0;
    while i < lines.len() {
        // Look for doc comment followed by function
        if lines[i].trim().starts_with(doc_prefix) {
            let doc_start = i;
            while i < lines.len() && lines[i].trim().starts_with(doc_prefix) {
                i += 1;
            }
            // Check if next non-empty line is a function
            if i < lines.len() && fn_re.is_match(lines[i]) {
                let doc: String = lines[doc_start..i].iter()
                    .map(|l| l.trim().trim_start_matches(doc_prefix).trim())
                    .collect::<Vec<_>>()
                    .join("\n");
                let signature = lines[i].trim().to_string();
                // Collect body (simplified: take next 20 lines or until blank line)
                let body_start = i;
                let body_end = (i + 20).min(lines.len());
                let body = lines[body_start..body_end].join("\n");
                results.push((doc, signature, body));
            }
        }
        i += 1;
    }

    results
}

// ── Fine-Tune Config ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FineTuneConfig {
    pub provider: FineTuneProvider,
    pub base_model: String,
    pub suffix: String,
    pub n_epochs: u32,
    pub batch_size: u32,
    pub learning_rate: f64,
    pub lora_rank: Option<u32>,
    pub lora_alpha: Option<f32>,
    pub warmup_steps: u32,
    pub max_seq_length: usize,
}

impl Default for FineTuneConfig {
    fn default() -> Self {
        Self {
            provider: FineTuneProvider::OpenAI,
            base_model: "gpt-4o-mini-2024-07-18".to_string(),
            suffix: "vibecody".to_string(),
            n_epochs: 1,
            batch_size: 4,
            learning_rate: 2e-5,
            lora_rank: None,
            lora_alpha: None,
            warmup_steps: 100,
            max_seq_length: 8192,
        }
    }
}

// ── Job Status & Metrics ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum JobStatus {
    Pending,
    Validating,
    Running { epoch: u32, loss: f64 },
    Completed,
    Failed(String),
    Cancelled,
}

impl JobStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed(_) | Self::Cancelled)
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Validating => "validating",
            Self::Running { .. } => "running",
            Self::Completed => "completed",
            Self::Failed(_) => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TrainingMetrics {
    pub train_loss: f64,
    pub val_loss: f64,
    pub train_accuracy: f64,
    pub epochs_completed: u32,
    pub tokens_processed: u64,
}

// ── Fine-Tune Job ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FineTuneJob {
    pub id: String,
    pub config: FineTuneConfig,
    pub status: JobStatus,
    pub created_at: String,
    pub dataset_name: String,
    pub dataset_size: usize,
    pub metrics: Option<TrainingMetrics>,
    pub result_model: Option<String>,
}

// ── Cost Estimate ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostEstimate {
    pub estimated_cost_usd: f64,
    pub estimated_duration_mins: f64,
    pub tokens_total: u64,
    pub provider: String,
}

// ── Fine-Tune Manager ────────────────────────────────────────────────────────

pub struct FineTuneManager {
    jobs: Vec<FineTuneJob>,
    next_job_num: u32,
}

impl FineTuneManager {
    pub fn new() -> Self {
        Self { jobs: Vec::new(), next_job_num: 1 }
    }

    pub fn create_job(&mut self, config: FineTuneConfig, dataset: &Dataset) -> Result<&FineTuneJob> {
        let issues = dataset.validate();
        if issues.iter().any(|i| i.contains("empty")) {
            anyhow::bail!("Dataset has critical validation issues: {:?}", issues);
        }

        let id = format!("ft-{:04}", self.next_job_num);
        self.next_job_num += 1;

        let job = FineTuneJob {
            id,
            config,
            status: JobStatus::Pending,
            created_at: {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default();
                format!("{}", now.as_secs())
            },
            dataset_name: dataset.name.clone(),
            dataset_size: dataset.len(),
            metrics: None,
            result_model: None,
        };

        self.jobs.push(job);
        Ok(self.jobs.last().expect("just pushed"))
    }

    pub fn list_jobs(&self) -> &[FineTuneJob] {
        &self.jobs
    }

    pub fn get_job(&self, id: &str) -> Option<&FineTuneJob> {
        self.jobs.iter().find(|j| j.id == id)
    }

    pub fn get_job_mut(&mut self, id: &str) -> Option<&mut FineTuneJob> {
        self.jobs.iter_mut().find(|j| j.id == id)
    }

    pub fn cancel_job(&mut self, id: &str) -> Result<()> {
        let job = self.jobs.iter_mut()
            .find(|j| j.id == id)
            .context("job not found")?;

        if job.status.is_terminal() {
            anyhow::bail!("Cannot cancel job in terminal state: {}", job.status.as_str());
        }

        job.status = JobStatus::Cancelled;
        Ok(())
    }

    pub fn estimate_cost(&self, config: &FineTuneConfig, dataset: &Dataset) -> CostEstimate {
        let total_tokens: u64 = dataset.examples.iter()
            .map(|e| e.estimated_tokens() as u64)
            .sum::<u64>() * config.n_epochs as u64;

        let cost = (total_tokens as f64 / 1000.0) * config.provider.cost_per_1k_tokens();
        let duration = total_tokens as f64 / 50_000.0; // ~50k tokens/min

        CostEstimate {
            estimated_cost_usd: cost,
            estimated_duration_mins: duration,
            tokens_total: total_tokens,
            provider: config.provider.as_str().to_string(),
        }
    }

    pub fn job_count(&self) -> usize { self.jobs.len() }
}

impl Default for FineTuneManager {
    fn default() -> Self { Self::new() }
}

// ── SWE-Bench Evaluation ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SWEBenchTask {
    pub instance_id: String,
    pub repo: String,
    pub problem_statement: String,
    pub patch: String,
    pub test_patch: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalResults {
    pub model_name: String,
    pub tasks_attempted: usize,
    pub tasks_resolved: usize,
    pub resolution_rate: f64,
    pub avg_time_s: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonReport {
    pub models: Vec<String>,
    pub results: Vec<EvalResults>,
    pub winner: String,
}

pub struct SWEBenchEval {
    pub tasks: Vec<SWEBenchTask>,
}

impl SWEBenchEval {
    pub fn new() -> Self {
        Self { tasks: Vec::new() }
    }

    /// Load SWE-bench tasks from a JSONL file.
    pub fn load_tasks(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .context("failed to read SWE-bench tasks file")?;

        let mut tasks = Vec::new();
        for line in content.lines() {
            if line.trim().is_empty() { continue; }
            match serde_json::from_str::<SWEBenchTask>(line) {
                Ok(task) => tasks.push(task),
                Err(e) => eprintln!("Skipping invalid task: {}", e),
            }
        }

        Ok(Self { tasks })
    }

    /// Evaluate a model on loaded tasks (stub — returns synthetic results).
    pub fn evaluate_model(&self, model: &str, _provider: &FineTuneProvider) -> EvalResults {
        EvalResults {
            model_name: model.to_string(),
            tasks_attempted: self.tasks.len(),
            tasks_resolved: 0,
            resolution_rate: 0.0,
            avg_time_s: 0.0,
        }
    }

    /// Compare multiple evaluation results.
    pub fn compare_models(results: &[EvalResults]) -> ComparisonReport {
        let winner = results.iter()
            .max_by(|a, b| a.resolution_rate.partial_cmp(&b.resolution_rate).unwrap_or(std::cmp::Ordering::Equal))
            .map(|r| r.model_name.clone())
            .unwrap_or_default();

        ComparisonReport {
            models: results.iter().map(|r| r.model_name.clone()).collect(),
            results: results.to_vec(),
            winner,
        }
    }

    pub fn task_count(&self) -> usize { self.tasks.len() }
}

impl Default for SWEBenchEval {
    fn default() -> Self { Self::new() }
}

// ── LoRA Adapter ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoraAdapter {
    pub name: String,
    pub base_model: String,
    pub rank: u32,
    pub path: PathBuf,
    pub size_bytes: u64,
}

impl LoraAdapter {
    /// List LoRA adapters in a directory.
    pub fn list_adapters(dir: &Path) -> Result<Vec<LoraAdapter>> {
        let mut adapters = Vec::new();

        if !dir.exists() {
            return Ok(adapters);
        }

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                // Look for adapter_config.json
                let config_path = path.join("adapter_config.json");
                if config_path.exists() {
                    let config_str = std::fs::read_to_string(&config_path)?;
                    if let Ok(config) = serde_json::from_str::<serde_json::Value>(&config_str) {
                        adapters.push(LoraAdapter {
                            name: path.file_name().unwrap_or_default().to_string_lossy().to_string(),
                            base_model: config.get("base_model_name_or_path")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown")
                                .to_string(),
                            rank: config.get("r").and_then(|v| v.as_u64()).unwrap_or(8) as u32,
                            path: path.clone(),
                            size_bytes: dir_size(&path),
                        });
                    }
                }
            }
        }

        Ok(adapters)
    }

    /// Stub for merging LoRA weights into base model.
    pub fn merge_adapter(&self, _output: &Path) -> Result<()> {
        anyhow::bail!(
            "LoRA merge requires llama.cpp or PEFT. Adapter: {}, base: {}",
            self.name,
            self.base_model
        )
    }
}

fn dir_size(path: &Path) -> u64 {
    walkdir::WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| e.metadata().ok())
        .map(|m| m.len())
        .sum()
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_example() -> TrainingExample {
        TrainingExample::new(vec![
            ("user".to_string(), "Write a hello world function".to_string()),
            ("assistant".to_string(), "fn hello() { println!(\"Hello!\"); }".to_string()),
        ])
    }

    fn sample_dataset() -> Dataset {
        let mut ds = Dataset::new("test", DatasetFormat::ChatML);
        ds.examples.push(sample_example());
        ds.examples.push(TrainingExample::new(vec![
            ("user".to_string(), "Write a sort function".to_string()),
            ("assistant".to_string(), "fn sort(v: &mut Vec<i32>) { v.sort(); }".to_string()),
        ]));
        ds
    }

    // ── Provider tests ───────────────────────────────────────────────────

    #[test]
    fn test_provider_as_str() {
        assert_eq!(FineTuneProvider::OpenAI.as_str(), "openai");
        assert_eq!(FineTuneProvider::Local.as_str(), "local");
    }

    #[test]
    fn test_provider_cost() {
        assert!(FineTuneProvider::OpenAI.cost_per_1k_tokens() > 0.0);
        assert_eq!(FineTuneProvider::Local.cost_per_1k_tokens(), 0.0);
    }

    #[test]
    fn test_format_as_str() {
        assert_eq!(DatasetFormat::ChatML.as_str(), "chatml");
        assert_eq!(DatasetFormat::SWEBench.as_str(), "swe_bench");
    }

    // ── TrainingExample tests ────────────────────────────────────────────

    #[test]
    fn test_example_new() {
        let ex = sample_example();
        assert_eq!(ex.messages.len(), 2);
        assert!(ex.system.is_none());
    }

    #[test]
    fn test_example_with_system() {
        let ex = sample_example().with_system("You are helpful.");
        assert_eq!(ex.system.as_deref(), Some("You are helpful."));
    }

    #[test]
    fn test_example_estimated_tokens() {
        let ex = sample_example();
        assert!(ex.estimated_tokens() > 0);
    }

    #[test]
    fn test_example_is_valid() {
        assert!(sample_example().is_valid());

        let invalid = TrainingExample::new(vec![]);
        assert!(!invalid.is_valid());

        let empty_content = TrainingExample::new(vec![("user".to_string(), "".to_string())]);
        assert!(!empty_content.is_valid());
    }

    // ── Dataset tests ────────────────────────────────────────────────────

    #[test]
    fn test_dataset_new() {
        let ds = Dataset::new("test", DatasetFormat::ChatML);
        assert!(ds.is_empty());
        assert_eq!(ds.len(), 0);
    }

    #[test]
    fn test_dataset_len() {
        let ds = sample_dataset();
        assert_eq!(ds.len(), 2);
        assert!(!ds.is_empty());
    }

    #[test]
    fn test_dataset_validate_valid() {
        let ds = sample_dataset();
        let issues = ds.validate();
        assert!(issues.is_empty(), "Unexpected issues: {:?}", issues);
    }

    #[test]
    fn test_dataset_validate_empty() {
        let ds = Dataset::new("empty", DatasetFormat::ChatML);
        let issues = ds.validate();
        assert!(issues.iter().any(|i| i.contains("empty")));
    }

    #[test]
    fn test_dataset_validate_empty_message() {
        let mut ds = Dataset::new("test", DatasetFormat::ChatML);
        ds.examples.push(TrainingExample::new(vec![
            ("user".to_string(), "".to_string()),
        ]));
        let issues = ds.validate();
        assert!(!issues.is_empty());
    }

    #[test]
    fn test_dataset_split() {
        let ds = sample_dataset();
        let (train, val) = ds.split(0.5);
        assert_eq!(train.len(), 1);
        assert_eq!(val.len(), 1);
        assert!(train.name.contains("train"));
        assert!(val.name.contains("val"));
    }

    #[test]
    fn test_dataset_split_all_train() {
        let ds = sample_dataset();
        let (train, val) = ds.split(1.0);
        assert_eq!(train.len(), 2);
        assert_eq!(val.len(), 0);
    }

    #[test]
    fn test_dataset_stats() {
        let ds = sample_dataset();
        let stats = ds.stats();
        assert_eq!(stats.example_count, 2);
        assert!(stats.total_tokens > 0);
        assert!(stats.avg_tokens_per_example > 0.0);
        assert!(stats.max_tokens >= stats.min_tokens);
        assert_eq!(stats.invalid_count, 0);
    }

    #[test]
    fn test_dataset_stats_empty() {
        let ds = Dataset::new("empty", DatasetFormat::ChatML);
        let stats = ds.stats();
        assert_eq!(stats.example_count, 0);
    }

    #[test]
    fn test_dataset_filter_by_language() {
        let mut ds = sample_dataset();
        ds.examples[0].metadata.insert("language".to_string(), "rust".to_string());
        ds.examples[1].metadata.insert("language".to_string(), "python".to_string());

        let rust_only = ds.filter_by_language("rust");
        assert_eq!(rust_only.len(), 1);
        assert!(rust_only.name.contains("rust"));
    }

    #[test]
    fn test_dataset_deduplicate() {
        let mut ds = sample_dataset();
        ds.examples.push(ds.examples[0].clone()); // duplicate
        assert_eq!(ds.len(), 3);

        let deduped = ds.deduplicate();
        assert_eq!(deduped.len(), 2);
    }

    #[test]
    fn test_dataset_to_jsonl() {
        let ds = sample_dataset();
        let tmp = std::env::temp_dir().join("vibecody_ft_test.jsonl");
        ds.to_jsonl(&tmp).expect("export failed");

        let content = std::fs::read_to_string(&tmp).expect("read failed");
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 2);

        // Verify each line is valid JSON
        for line in &lines {
            serde_json::from_str::<serde_json::Value>(line).expect("invalid JSON");
        }
        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn test_dataset_from_conversations_nonexistent() {
        let result = Dataset::from_conversations(Path::new("/nonexistent/file.jsonl"));
        assert!(result.is_err());
    }

    // ── Config tests ─────────────────────────────────────────────────────

    #[test]
    fn test_config_default() {
        let config = FineTuneConfig::default();
        assert_eq!(config.provider, FineTuneProvider::OpenAI);
        assert_eq!(config.n_epochs, 1);
        assert_eq!(config.batch_size, 4);
        assert_eq!(config.max_seq_length, 8192);
        assert!(config.lora_rank.is_none());
    }

    // ── Job Status tests ─────────────────────────────────────────────────

    #[test]
    fn test_job_status_terminal() {
        assert!(!JobStatus::Pending.is_terminal());
        assert!(!JobStatus::Validating.is_terminal());
        assert!(!JobStatus::Running { epoch: 1, loss: 0.5 }.is_terminal());
        assert!(JobStatus::Completed.is_terminal());
        assert!(JobStatus::Failed("error".to_string()).is_terminal());
        assert!(JobStatus::Cancelled.is_terminal());
    }

    #[test]
    fn test_job_status_as_str() {
        assert_eq!(JobStatus::Pending.as_str(), "pending");
        assert_eq!(JobStatus::Running { epoch: 1, loss: 0.5 }.as_str(), "running");
    }

    // ── Manager tests ────────────────────────────────────────────────────

    #[test]
    fn test_manager_new() {
        let mgr = FineTuneManager::new();
        assert_eq!(mgr.job_count(), 0);
    }

    #[test]
    fn test_manager_create_job() {
        let mut mgr = FineTuneManager::new();
        let ds = sample_dataset();
        let config = FineTuneConfig::default();
        let job = mgr.create_job(config, &ds).expect("create failed");
        assert!(job.id.starts_with("ft-"));
        assert_eq!(job.status, JobStatus::Pending);
        assert_eq!(mgr.job_count(), 1);
    }

    #[test]
    fn test_manager_create_job_empty_dataset() {
        let mut mgr = FineTuneManager::new();
        let ds = Dataset::new("empty", DatasetFormat::ChatML);
        let result = mgr.create_job(FineTuneConfig::default(), &ds);
        assert!(result.is_err());
    }

    #[test]
    fn test_manager_get_job() {
        let mut mgr = FineTuneManager::new();
        let ds = sample_dataset();
        mgr.create_job(FineTuneConfig::default(), &ds).unwrap();
        assert!(mgr.get_job("ft-0001").is_some());
        assert!(mgr.get_job("nonexistent").is_none());
    }

    #[test]
    fn test_manager_cancel_job() {
        let mut mgr = FineTuneManager::new();
        let ds = sample_dataset();
        mgr.create_job(FineTuneConfig::default(), &ds).unwrap();
        mgr.cancel_job("ft-0001").expect("cancel failed");
        assert_eq!(mgr.get_job("ft-0001").unwrap().status, JobStatus::Cancelled);
    }

    #[test]
    fn test_manager_cancel_completed_job() {
        let mut mgr = FineTuneManager::new();
        let ds = sample_dataset();
        mgr.create_job(FineTuneConfig::default(), &ds).unwrap();
        mgr.get_job_mut("ft-0001").unwrap().status = JobStatus::Completed;
        let result = mgr.cancel_job("ft-0001");
        assert!(result.is_err());
    }

    #[test]
    fn test_manager_cancel_nonexistent() {
        let mut mgr = FineTuneManager::new();
        assert!(mgr.cancel_job("nonexistent").is_err());
    }

    #[test]
    fn test_manager_list_jobs() {
        let mut mgr = FineTuneManager::new();
        let ds = sample_dataset();
        mgr.create_job(FineTuneConfig::default(), &ds).unwrap();
        mgr.create_job(FineTuneConfig::default(), &ds).unwrap();
        assert_eq!(mgr.list_jobs().len(), 2);
    }

    #[test]
    fn test_cost_estimate() {
        let mgr = FineTuneManager::new();
        let ds = sample_dataset();
        let config = FineTuneConfig { n_epochs: 3, ..Default::default() };
        let estimate = mgr.estimate_cost(&config, &ds);
        assert!(estimate.estimated_cost_usd >= 0.0);
        assert!(estimate.tokens_total > 0);
        assert_eq!(estimate.provider, "openai");
    }

    #[test]
    fn test_cost_estimate_local() {
        let mgr = FineTuneManager::new();
        let ds = sample_dataset();
        let config = FineTuneConfig {
            provider: FineTuneProvider::Local,
            ..Default::default()
        };
        let estimate = mgr.estimate_cost(&config, &ds);
        assert_eq!(estimate.estimated_cost_usd, 0.0);
    }

    // ── SWE-Bench tests ──────────────────────────────────────────────────

    #[test]
    fn test_swe_bench_new() {
        let eval = SWEBenchEval::new();
        assert_eq!(eval.task_count(), 0);
    }

    #[test]
    fn test_swe_bench_evaluate_empty() {
        let eval = SWEBenchEval::new();
        let results = eval.evaluate_model("test-model", &FineTuneProvider::OpenAI);
        assert_eq!(results.tasks_attempted, 0);
        assert_eq!(results.resolution_rate, 0.0);
    }

    #[test]
    fn test_swe_bench_compare_models() {
        let results = vec![
            EvalResults {
                model_name: "model-a".to_string(),
                tasks_attempted: 10,
                tasks_resolved: 3,
                resolution_rate: 0.3,
                avg_time_s: 60.0,
            },
            EvalResults {
                model_name: "model-b".to_string(),
                tasks_attempted: 10,
                tasks_resolved: 5,
                resolution_rate: 0.5,
                avg_time_s: 45.0,
            },
        ];
        let report = SWEBenchEval::compare_models(&results);
        assert_eq!(report.winner, "model-b");
        assert_eq!(report.models.len(), 2);
    }

    #[test]
    fn test_swe_bench_load_nonexistent() {
        let result = SWEBenchEval::load_tasks(Path::new("/nonexistent/tasks.jsonl"));
        assert!(result.is_err());
    }

    // ── LoRA Adapter tests ───────────────────────────────────────────────

    #[test]
    fn test_lora_list_empty_dir() {
        let tmp = std::env::temp_dir().join("vibecody_lora_test_empty");
        std::fs::create_dir_all(&tmp).ok();
        let adapters = LoraAdapter::list_adapters(&tmp).expect("list failed");
        assert!(adapters.is_empty());
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn test_lora_list_nonexistent_dir() {
        let adapters = LoraAdapter::list_adapters(Path::new("/nonexistent/dir"));
        assert!(adapters.unwrap().is_empty());
    }

    #[test]
    fn test_lora_merge_stub() {
        let adapter = LoraAdapter {
            name: "test".to_string(),
            base_model: "llama-7b".to_string(),
            rank: 8,
            path: PathBuf::from("/tmp/test"),
            size_bytes: 1024,
        };
        assert!(adapter.merge_adapter(Path::new("/tmp/output")).is_err());
    }

    // ── Extract helpers ──────────────────────────────────────────────────

    #[test]
    fn test_extract_documented_functions_rust() {
        let content = "/// Says hello.\npub fn hello() {\n    println!(\"hi\");\n}\n";
        let funcs = extract_documented_functions(content, "rust");
        assert_eq!(funcs.len(), 1);
        assert!(funcs[0].0.contains("Says hello"));
    }

    #[test]
    fn test_extract_documented_functions_no_docs() {
        let content = "pub fn hello() {}\n";
        let funcs = extract_documented_functions(content, "rust");
        assert!(funcs.is_empty());
    }

    #[test]
    fn test_extract_documented_functions_unknown_lang() {
        let funcs = extract_documented_functions("anything", "cobol");
        assert!(funcs.is_empty());
    }
}
