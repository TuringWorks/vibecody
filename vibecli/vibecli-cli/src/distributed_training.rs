// Distributed training orchestration module for VibeCody CLI.
// Generates DeepSpeed configs, torchrun/accelerate commands, SLURM scripts,
// hostfiles, and LoRA configs for multi-node GPU training.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DistributedFramework {
    DeepSpeed,
    Fsdp,
    Megatron,
    Horovod,
    RayTrain,
    ColossalAi,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParallelismStrategy {
    DataParallel,
    TensorParallel,
    PipelineParallel,
    ExpertParallel,
    SequenceParallel,
    Hybrid(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeepSpeedStage {
    Stage0,
    Stage1,
    Stage2,
    Stage3,
    Infinity,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MixedPrecision {
    Fp32,
    Fp16,
    Bf16,
    Fp8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DatasetFormat {
    Alpaca,
    ShareGpt,
    OpenAi,
    Completion,
    Custom(String),
}

// ---------------------------------------------------------------------------
// Structs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingConfig {
    pub framework: DistributedFramework,
    pub model_path: String,
    pub dataset_path: String,
    pub output_dir: String,
    pub num_nodes: u32,
    pub gpus_per_node: u32,
    pub batch_size_per_gpu: u32,
    pub gradient_accumulation_steps: u32,
    pub learning_rate: f64,
    pub weight_decay: f64,
    pub warmup_steps: u32,
    pub max_steps: u64,
    pub epochs: Option<u32>,
    pub mixed_precision: MixedPrecision,
    pub gradient_checkpointing: bool,
    pub flash_attention: bool,
    pub seed: u64,
    pub deepspeed_stage: Option<DeepSpeedStage>,
    pub tensor_parallel_size: Option<u32>,
    pub pipeline_parallel_size: Option<u32>,
    pub extra_args: HashMap<String, String>,
}

impl Default for TrainingConfig {
    fn default() -> Self {
        Self {
            framework: DistributedFramework::DeepSpeed,
            model_path: String::new(),
            dataset_path: String::new(),
            output_dir: "./output".to_string(),
            num_nodes: 1,
            gpus_per_node: 1,
            batch_size_per_gpu: 4,
            gradient_accumulation_steps: 4,
            learning_rate: 2e-5,
            weight_decay: 0.01,
            warmup_steps: 100,
            max_steps: 1000,
            epochs: None,
            mixed_precision: MixedPrecision::Bf16,
            gradient_checkpointing: true,
            flash_attention: true,
            seed: 42,
            deepspeed_stage: Some(DeepSpeedStage::Stage2),
            tensor_parallel_size: None,
            pipeline_parallel_size: None,
            extra_args: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoraConfig {
    pub r: u32,
    pub alpha: u32,
    pub dropout: f64,
    pub target_modules: Vec<String>,
    pub bias: String,
    pub task_type: String,
}

impl Default for LoraConfig {
    fn default() -> Self {
        Self {
            r: 16,
            alpha: 32,
            dropout: 0.05,
            target_modules: vec!["q_proj".into(), "k_proj".into(), "v_proj".into(), "o_proj".into()],
            bias: "none".to_string(),
            task_type: "CAUSAL_LM".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetConfig {
    pub train_path: String,
    pub eval_path: Option<String>,
    pub format: DatasetFormat,
    pub max_seq_length: u32,
    pub packing: bool,
    pub shuffle: bool,
    pub num_workers: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointConfig {
    pub save_steps: u64,
    pub save_total_limit: u32,
    pub resume_from: Option<String>,
    pub save_optimizer: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingMetrics {
    pub step: u64,
    pub loss: f64,
    pub learning_rate: f64,
    pub grad_norm: f64,
    pub tokens_per_second: f64,
    pub gpu_memory_used_mb: u64,
    pub epoch: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WandbConfig {
    pub project: String,
    pub entity: Option<String>,
    pub run_name: Option<String>,
    pub tags: Vec<String>,
}

// ---------------------------------------------------------------------------
// Config generators
// ---------------------------------------------------------------------------

pub fn generate_deepspeed_config(config: &TrainingConfig) -> String {
    let stage = match &config.deepspeed_stage {
        Some(DeepSpeedStage::Stage0) => 0,
        Some(DeepSpeedStage::Stage1) => 1,
        Some(DeepSpeedStage::Stage2) => 2,
        Some(DeepSpeedStage::Stage3) | Some(DeepSpeedStage::Infinity) => 3,
        None => 2,
    };

    let fp16_enabled = config.mixed_precision == MixedPrecision::Fp16;
    let bf16_enabled = config.mixed_precision == MixedPrecision::Bf16;

    let offload = if matches!(config.deepspeed_stage, Some(DeepSpeedStage::Stage3 | DeepSpeedStage::Infinity)) {
        r#",
    "offload_optimizer": {
      "device": "cpu",
      "pin_memory": true
    },
    "offload_param": {
      "device": "cpu",
      "pin_memory": true
    }"#
    } else {
        ""
    };

    format!(
        r#"{{
  "train_batch_size": "auto",
  "train_micro_batch_size_per_gpu": {batch},
  "gradient_accumulation_steps": {grad_accum},
  "gradient_clipping": 1.0,
  "zero_optimization": {{
    "stage": {stage},
    "contiguous_gradients": true,
    "overlap_comm": true,
    "reduce_scatter": true,
    "reduce_bucket_size": 5e8,
    "allgather_bucket_size": 5e8{offload}
  }},
  "fp16": {{
    "enabled": {fp16}
  }},
  "bf16": {{
    "enabled": {bf16}
  }},
  "zero_allow_untested_optimizer": true,
  "wall_clock_breakdown": false,
  "steps_per_print": 100
}}"#,
        batch = config.batch_size_per_gpu,
        grad_accum = config.gradient_accumulation_steps,
        stage = stage,
        offload = offload,
        fp16 = fp16_enabled,
        bf16 = bf16_enabled,
    )
}

pub fn generate_deepspeed_launch_command(config: &TrainingConfig, script: &str) -> Vec<String> {
    let total_gpus = config.num_nodes * config.gpus_per_node;
    let mut cmd = vec![
        "deepspeed".to_string(),
        "--num_gpus".to_string(), total_gpus.to_string(),
    ];
    if config.num_nodes > 1 {
        cmd.push("--num_nodes".to_string());
        cmd.push(config.num_nodes.to_string());
        cmd.push("--hostfile".to_string());
        cmd.push("hostfile".to_string());
    }
    cmd.push(script.to_string());
    cmd.push("--model_name_or_path".to_string());
    cmd.push(config.model_path.clone());
    cmd.push("--output_dir".to_string());
    cmd.push(config.output_dir.clone());
    cmd.push("--per_device_train_batch_size".to_string());
    cmd.push(config.batch_size_per_gpu.to_string());
    cmd.push("--gradient_accumulation_steps".to_string());
    cmd.push(config.gradient_accumulation_steps.to_string());
    cmd.push("--learning_rate".to_string());
    cmd.push(config.learning_rate.to_string());
    cmd.push("--max_steps".to_string());
    cmd.push(config.max_steps.to_string());
    cmd.push("--deepspeed".to_string());
    cmd.push("ds_config.json".to_string());
    if config.gradient_checkpointing {
        cmd.push("--gradient_checkpointing".to_string());
    }
    match config.mixed_precision {
        MixedPrecision::Fp16 => { cmd.push("--fp16".to_string()); }
        MixedPrecision::Bf16 => { cmd.push("--bf16".to_string()); }
        _ => {}
    }
    cmd
}

pub fn generate_torchrun_command(config: &TrainingConfig, script: &str) -> Vec<String> {
    let nproc = config.gpus_per_node;
    let mut cmd = vec![
        "torchrun".to_string(),
        "--nproc_per_node".to_string(), nproc.to_string(),
    ];
    if config.num_nodes > 1 {
        cmd.push("--nnodes".to_string());
        cmd.push(config.num_nodes.to_string());
        cmd.push("--rdzv_backend".to_string());
        cmd.push("c10d".to_string());
        cmd.push("--rdzv_endpoint".to_string());
        cmd.push("master:29500".to_string());
    }
    cmd.push(script.to_string());
    cmd.push("--model_name_or_path".to_string());
    cmd.push(config.model_path.clone());
    cmd.push("--output_dir".to_string());
    cmd.push(config.output_dir.clone());
    cmd.push("--per_device_train_batch_size".to_string());
    cmd.push(config.batch_size_per_gpu.to_string());
    cmd.push("--learning_rate".to_string());
    cmd.push(config.learning_rate.to_string());
    if config.gradient_checkpointing {
        cmd.push("--gradient_checkpointing".to_string());
    }
    if config.framework == DistributedFramework::Fsdp {
        cmd.push("--fsdp".to_string());
        cmd.push("full_shard auto_wrap".to_string());
        cmd.push("--fsdp_transformer_layer_cls_to_wrap".to_string());
        cmd.push("LlamaDecoderLayer".to_string());
    }
    cmd
}

pub fn generate_accelerate_config(config: &TrainingConfig) -> String {
    let compute_env = if config.num_nodes > 1 { "MULTI_GPU" } else { "LOCAL_MACHINE" };
    let distributed_type = match config.framework {
        DistributedFramework::DeepSpeed => "DEEPSPEED",
        DistributedFramework::Fsdp => "FSDP",
        _ => "MULTI_GPU",
    };
    let mixed = match config.mixed_precision {
        MixedPrecision::Fp16 => "fp16",
        MixedPrecision::Bf16 => "bf16",
        _ => "no",
    };
    format!(
        r#"compute_environment: {compute_env}
distributed_type: {distributed_type}
machine_rank: 0
main_process_ip: null
main_process_port: 29500
main_training_function: main
mixed_precision: {mixed}
num_machines: {nodes}
num_processes: {total}
use_cpu: false
gpu_ids: all
"#,
        compute_env = compute_env,
        distributed_type = distributed_type,
        mixed = mixed,
        nodes = config.num_nodes,
        total = config.num_nodes * config.gpus_per_node,
    )
}

pub fn generate_lora_config_json(lora: &LoraConfig) -> String {
    serde_json::to_string_pretty(lora).unwrap_or_default()
}

pub fn generate_hostfile(nodes: &[(&str, u32)]) -> String {
    let mut lines = Vec::new();
    for (hostname, slots) in nodes {
        lines.push(format!("{hostname} slots={slots}"));
    }
    lines.join("\n")
}

pub fn validate_training_config(config: &TrainingConfig) -> Vec<String> {
    let mut errors = vec![];
    if config.model_path.is_empty() {
        errors.push("Model path is required".to_string());
    }
    if config.dataset_path.is_empty() {
        errors.push("Dataset path is required".to_string());
    }
    if config.learning_rate <= 0.0 || config.learning_rate > 1.0 {
        errors.push("Learning rate must be between 0 and 1".to_string());
    }
    if config.gpus_per_node == 0 {
        errors.push("GPUs per node must be > 0".to_string());
    }
    if config.num_nodes == 0 {
        errors.push("Number of nodes must be > 0".to_string());
    }
    if config.batch_size_per_gpu == 0 {
        errors.push("Batch size must be > 0".to_string());
    }
    if config.output_dir.is_empty() {
        errors.push("Output directory is required".to_string());
    }
    errors
}

pub fn estimate_memory_per_gpu(model_params_b: f64, config: &TrainingConfig) -> String {
    let bytes_per_param = match config.mixed_precision {
        MixedPrecision::Fp32 => 4.0,
        MixedPrecision::Fp16 | MixedPrecision::Bf16 => 2.0,
        MixedPrecision::Fp8 => 1.0,
    };
    let total_gpus = (config.num_nodes * config.gpus_per_node) as f64;

    // Model params memory
    let model_mem = model_params_b * 1_000.0 * bytes_per_param;

    // Optimizer states (Adam: 2x for fp32 copies, momentum, variance)
    let optimizer_mem = model_params_b * 1_000.0 * 8.0; // 8 bytes per param for Adam states

    // Gradients
    let gradient_mem = model_params_b * 1_000.0 * bytes_per_param;

    // Activations (rough estimate)
    let activation_mem = (config.batch_size_per_gpu as f64) * model_params_b * 50.0; // ~50 bytes/param/sample
    let activation_mem = if config.gradient_checkpointing { activation_mem / 3.0 } else { activation_mem };

    let (per_gpu_model, per_gpu_opt, per_gpu_grad) = match &config.deepspeed_stage {
        Some(DeepSpeedStage::Stage0) | None => (model_mem, optimizer_mem, gradient_mem),
        Some(DeepSpeedStage::Stage1) => (model_mem, optimizer_mem / total_gpus, gradient_mem),
        Some(DeepSpeedStage::Stage2) => (model_mem, optimizer_mem / total_gpus, gradient_mem / total_gpus),
        Some(DeepSpeedStage::Stage3) | Some(DeepSpeedStage::Infinity) => {
            (model_mem / total_gpus, optimizer_mem / total_gpus, gradient_mem / total_gpus)
        }
    };

    let total_per_gpu = per_gpu_model + per_gpu_opt + per_gpu_grad + activation_mem;
    let total_mb = total_per_gpu as u64;

    format!(
        "Estimated VRAM per GPU: ~{total_mb} MB\n  \
         Model: {:.0} MB, Optimizer: {:.0} MB, Gradients: {:.0} MB, Activations: {:.0} MB\n  \
         ({} GPUs, {:?} precision{})",
        per_gpu_model, per_gpu_opt, per_gpu_grad, activation_mem,
        total_gpus as u32,
        config.mixed_precision,
        if config.gradient_checkpointing { ", gradient checkpointing ON" } else { "" },
    )
}

pub fn suggest_parallelism(model_params_b: f64, gpu_count: u32, gpu_vram_mb: u64) -> String {
    let fp16_model_mb = (model_params_b * 2_000.0) as u64;
    let training_overhead = fp16_model_mb * 4; // ~4x for optimizer states + gradients + activations

    if training_overhead / (gpu_count as u64) < gpu_vram_mb {
        format!("Data Parallel (DDP) — {model_params_b}B model fits with {gpu_count} GPUs at BF16. \
                 Use torchrun or DeepSpeed ZeRO-1.")
    } else if fp16_model_mb * 3 / (gpu_count as u64) < gpu_vram_mb {
        format!("DeepSpeed ZeRO Stage 2 — shard optimizer + gradients across {gpu_count} GPUs. \
                 Model: {model_params_b}B, ~{} MB/GPU after sharding.",
                fp16_model_mb + (training_overhead - fp16_model_mb) / (gpu_count as u64))
    } else if fp16_model_mb / (gpu_count as u64) < gpu_vram_mb {
        format!("DeepSpeed ZeRO Stage 3 — shard everything across {gpu_count} GPUs. \
                 Model: {model_params_b}B, ~{} MB/GPU after full sharding.",
                training_overhead / (gpu_count as u64))
    } else {
        let needed = (training_overhead / gpu_vram_mb) + 1;
        format!("Need more GPUs — {model_params_b}B model requires at least {needed} GPUs with {gpu_vram_mb}MB VRAM each. \
                 Consider ZeRO-3 + CPU offloading, or use LoRA/QLoRA to reduce memory.")
    }
}

pub fn generate_slurm_distributed_script(config: &TrainingConfig, script: &str, partition: &str) -> String {
    let total_gpus = config.num_nodes * config.gpus_per_node;
    let framework_launch = match config.framework {
        DistributedFramework::DeepSpeed => format!(
            "deepspeed --num_gpus {gpus_per_node} --num_nodes $SLURM_NNODES \\\n  \
             --hostfile $HOSTFILE \\\n  \
             {script} \\\n  \
             --deepspeed ds_config.json",
            gpus_per_node = config.gpus_per_node,
            script = script,
        ),
        _ => format!(
            "torchrun --nproc_per_node {gpus_per_node} \\\n  \
             --nnodes $SLURM_NNODES \\\n  \
             --rdzv_backend c10d \\\n  \
             --rdzv_endpoint $MASTER_ADDR:$MASTER_PORT \\\n  \
             {script}",
            gpus_per_node = config.gpus_per_node,
            script = script,
        ),
    };

    format!(
        r#"#!/bin/bash
#SBATCH --job-name=vibecody-train
#SBATCH --partition={partition}
#SBATCH --nodes={nodes}
#SBATCH --ntasks-per-node=1
#SBATCH --gres=gpu:{gpus_per_node}
#SBATCH --cpus-per-task={cpus}
#SBATCH --mem=0
#SBATCH --time=72:00:00
#SBATCH --output=train_%j.log
#SBATCH --error=train_%j.err

# Environment setup
module load cuda/12.1
source activate training

# Distributed setup
export MASTER_ADDR=$(scontrol show hostname $SLURM_NODELIST | head -n1)
export MASTER_PORT=29500
export WORLD_SIZE={total_gpus}
export NCCL_DEBUG=INFO
export NCCL_IB_DISABLE=0

# Generate hostfile for DeepSpeed
HOSTFILE=hostfile_$SLURM_JOB_ID
scontrol show hostname $SLURM_NODELIST | while read node; do
  echo "$node slots={gpus_per_node}" >> $HOSTFILE
done

# Launch training
srun {launch}

echo "Training complete. Output: {output_dir}"
"#,
        partition = partition,
        nodes = config.num_nodes,
        gpus_per_node = config.gpus_per_node,
        cpus = config.gpus_per_node * 4,
        total_gpus = total_gpus,
        launch = framework_launch,
        output_dir = config.output_dir,
    )
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn default_config() -> TrainingConfig {
        TrainingConfig {
            model_path: "meta-llama/Llama-3-8B".to_string(),
            dataset_path: "./data/train.jsonl".to_string(),
            ..TrainingConfig::default()
        }
    }

    #[test]
    fn test_framework_serialization() {
        let f = DistributedFramework::DeepSpeed;
        let json = serde_json::to_string(&f).unwrap();
        assert_eq!(json, "\"deep_speed\"");
        let back: DistributedFramework = serde_json::from_str(&json).unwrap();
        assert_eq!(back, DistributedFramework::DeepSpeed);
    }

    #[test]
    fn test_deepspeed_stage_serialization() {
        let s = DeepSpeedStage::Stage3;
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(json, "\"stage3\"");
    }

    #[test]
    fn test_generate_deepspeed_config_stage2() {
        let config = default_config();
        let ds = generate_deepspeed_config(&config);
        assert!(ds.contains("\"stage\": 2"));
        assert!(ds.contains("\"bf16\""));
        assert!(!ds.contains("offload_optimizer"));
    }

    #[test]
    fn test_generate_deepspeed_config_stage3() {
        let mut config = default_config();
        config.deepspeed_stage = Some(DeepSpeedStage::Stage3);
        let ds = generate_deepspeed_config(&config);
        assert!(ds.contains("\"stage\": 3"));
        assert!(ds.contains("offload_optimizer"));
        assert!(ds.contains("offload_param"));
    }

    #[test]
    fn test_generate_deepspeed_launch_command() {
        let config = default_config();
        let cmd = generate_deepspeed_launch_command(&config, "train.py");
        assert!(cmd.contains(&"deepspeed".to_string()));
        assert!(cmd.contains(&"train.py".to_string()));
        assert!(cmd.contains(&"--deepspeed".to_string()));
        assert!(cmd.contains(&"--bf16".to_string()));
    }

    #[test]
    fn test_generate_torchrun_command_single_node() {
        let config = default_config();
        let cmd = generate_torchrun_command(&config, "train.py");
        assert!(cmd.contains(&"torchrun".to_string()));
        assert!(!cmd.contains(&"--nnodes".to_string()));
    }

    #[test]
    fn test_generate_torchrun_command_multi_node() {
        let mut config = default_config();
        config.num_nodes = 4;
        config.gpus_per_node = 8;
        let cmd = generate_torchrun_command(&config, "train.py");
        assert!(cmd.contains(&"--nnodes".to_string()));
        assert!(cmd.contains(&"4".to_string()));
        assert!(cmd.contains(&"c10d".to_string()));
    }

    #[test]
    fn test_generate_accelerate_config() {
        let config = default_config();
        let yaml = generate_accelerate_config(&config);
        assert!(yaml.contains("DEEPSPEED"));
        assert!(yaml.contains("bf16"));
        assert!(yaml.contains("num_machines: 1"));
    }

    #[test]
    fn test_generate_lora_config_json() {
        let lora = LoraConfig::default();
        let json = generate_lora_config_json(&lora);
        assert!(json.contains("\"r\": 16"));
        assert!(json.contains("q_proj"));
        assert!(json.contains("CAUSAL_LM"));
    }

    #[test]
    fn test_generate_hostfile() {
        let nodes = vec![("node-0", 8), ("node-1", 8), ("node-2", 4)];
        let hf = generate_hostfile(&nodes);
        assert!(hf.contains("node-0 slots=8"));
        assert!(hf.contains("node-2 slots=4"));
        assert_eq!(hf.lines().count(), 3);
    }

    #[test]
    fn test_validate_config_valid() {
        let config = default_config();
        let errors = validate_training_config(&config);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_validate_config_bad_lr() {
        let mut config = default_config();
        config.learning_rate = -0.01;
        let errors = validate_training_config(&config);
        assert!(errors.iter().any(|e| e.contains("Learning rate")));
    }

    #[test]
    fn test_validate_config_no_gpus() {
        let mut config = default_config();
        config.gpus_per_node = 0;
        let errors = validate_training_config(&config);
        assert!(errors.iter().any(|e| e.contains("GPUs")));
    }

    #[test]
    fn test_estimate_memory_per_gpu() {
        let config = default_config();
        let estimate = estimate_memory_per_gpu(7.0, &config);
        assert!(estimate.contains("VRAM per GPU"));
        assert!(estimate.contains("Model:"));
        assert!(estimate.contains("gradient checkpointing ON"));
    }

    #[test]
    fn test_suggest_parallelism_small_model() {
        let suggestion = suggest_parallelism(7.0, 4, 80_000);
        assert!(suggestion.contains("Data Parallel") || suggestion.contains("DDP"));
    }

    #[test]
    fn test_suggest_parallelism_large_model() {
        let suggestion = suggest_parallelism(70.0, 2, 24_000);
        assert!(suggestion.contains("ZeRO") || suggestion.contains("Need more"));
    }

    #[test]
    fn test_generate_slurm_distributed_script() {
        let mut config = default_config();
        config.num_nodes = 4;
        config.gpus_per_node = 8;
        let script = generate_slurm_distributed_script(&config, "train.py", "gpu");
        assert!(script.contains("#SBATCH --nodes=4"));
        assert!(script.contains("#SBATCH --gres=gpu:8"));
        assert!(script.contains("MASTER_ADDR"));
        assert!(script.contains("deepspeed"));
        assert!(script.contains("NCCL_DEBUG"));
    }
}
