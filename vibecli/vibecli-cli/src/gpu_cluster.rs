// GPU cluster provisioning and management module.
//
// Provides abstractions for discovering GPUs, provisioning multi-node GPU clusters,
// submitting training and inference jobs, and generating orchestrator configs
// (SLURM sbatch scripts, Kubernetes Job manifests).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GpuVendor {
    Nvidia,
    Amd,
    Intel,
    Apple,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClusterProvider {
    Local,
    Slurm,
    KubernetesGpu,
    AwsEc2,
    GcpCompute,
    AzureVm,
    Lambda,
    RunPod,
    CoreWeave,
    Vast,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GpuNodeStatus {
    Available,
    Busy,
    Offline,
    Provisioning,
    Draining,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrainingFramework {
    PyTorch,
    TensorFlow,
    Jax,
    DeepSpeed,
    Megatron,
    HuggingFace,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InferenceEngine {
    Vllm,
    Tgi,
    Triton,
    OllamaServe,
    TorchServe,
    Onnxruntime,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Quantization {
    Fp16,
    Bf16,
    Int8,
    Int4,
    Gptq,
    Awq,
    Gguf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

// ---------------------------------------------------------------------------
// Structs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuInfo {
    pub vendor: GpuVendor,
    pub model_name: String,
    pub vram_mb: u64,
    pub cuda_cores: Option<u32>,
    pub compute_capability: Option<String>,
    pub driver_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuNode {
    pub id: String,
    pub hostname: String,
    pub gpus: Vec<GpuInfo>,
    pub status: GpuNodeStatus,
    pub labels: HashMap<String, String>,
    pub provider: ClusterProvider,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuCluster {
    pub name: String,
    pub nodes: Vec<GpuNode>,
    pub provider: ClusterProvider,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingJobConfig {
    pub name: String,
    pub model_path: String,
    pub dataset_path: String,
    pub output_dir: String,
    pub batch_size: u32,
    pub learning_rate: f64,
    pub epochs: u32,
    pub distributed: bool,
    pub num_gpus: u32,
    pub framework: TrainingFramework,
    pub mixed_precision: bool,
    pub gradient_checkpointing: bool,
    pub extra_args: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceConfig {
    pub model_path: String,
    pub engine: InferenceEngine,
    pub port: u16,
    pub max_batch_size: u32,
    pub quantization: Option<Quantization>,
    pub tensor_parallel: u32,
    pub gpu_memory_fraction: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingJob {
    pub id: String,
    pub config: TrainingJobConfig,
    pub status: JobStatus,
    pub cluster_name: String,
    pub started_at: Option<String>,
    pub metrics: HashMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceEndpoint {
    pub id: String,
    pub config: InferenceConfig,
    pub status: JobStatus,
    pub url: Option<String>,
    pub cluster_name: String,
}

// ---------------------------------------------------------------------------
// Functions
// ---------------------------------------------------------------------------

/// Detect GPUs available on the local machine.
///
/// A real implementation would shell out to `nvidia-smi --query-gpu=...` for
/// Nvidia cards, `rocm-smi` for AMD, or read sysfs/IOKit for Intel/Apple.
/// This stub returns an empty vector.
pub fn detect_local_gpus() -> Vec<GpuInfo> {
    // TODO: parse `nvidia-smi --query-gpu=name,memory.total,driver_version --format=csv,noheader`
    // TODO: parse `rocm-smi --showproductname --showmeminfo vram` for AMD
    // TODO: read IOKit registry for Apple Silicon GPU cores
    // TODO: read /sys/class/drm/card*/device for Intel Arc
    Vec::new()
}

/// Rough hourly cost estimate (USD) for `gpu_count` GPUs over `hours` hours.
pub fn estimate_gpu_cost(provider: &ClusterProvider, gpu_count: u32, hours: f64) -> f64 {
    let per_gpu_per_hour = match provider {
        ClusterProvider::Local => 0.0,
        ClusterProvider::Slurm => 0.0, // on-prem, no cloud cost
        ClusterProvider::KubernetesGpu => 2.50,
        ClusterProvider::AwsEc2 => 3.06,        // p4d.24xlarge / 8 GPUs
        ClusterProvider::GcpCompute => 2.95,     // a2-highgpu per GPU
        ClusterProvider::AzureVm => 3.10,        // ND-series per GPU
        ClusterProvider::Lambda => 1.10,         // A10 on-demand
        ClusterProvider::RunPod => 0.74,         // A100 community
        ClusterProvider::CoreWeave => 2.06,      // A100 80 GB
        ClusterProvider::Vast => 0.50,           // marketplace average
    };
    per_gpu_per_hour * gpu_count as f64 * hours
}

/// Suggest a GPU configuration given an estimated model size and task type.
pub fn suggest_gpu_config(model_params_billions: f64, task: &str) -> String {
    let task_lower = task.to_lowercase();
    let is_training = task_lower.contains("train") || task_lower.contains("fine");
    let is_inference = task_lower.contains("infer") || task_lower.contains("serv");

    if model_params_billions <= 1.0 {
        if is_training {
            "1x GPU with 16 GB VRAM (e.g. T4 or RTX 4080). Mixed precision recommended.".to_string()
        } else {
            "1x GPU with 8 GB VRAM is sufficient for inference at this scale.".to_string()
        }
    } else if model_params_billions <= 7.0 {
        if is_training {
            "1-2x A100 40 GB GPUs. Use DeepSpeed ZeRO-2 or FSDP for multi-GPU.".to_string()
        } else if is_inference {
            "1x A10G or L4 with int8 quantization for cost-effective serving.".to_string()
        } else {
            "1x A100 40 GB GPU recommended for general workloads at 7B scale.".to_string()
        }
    } else if model_params_billions <= 70.0 {
        if is_training {
            "8x A100 80 GB (1 node) with DeepSpeed ZeRO-3 or Megatron-LM tensor parallelism.".to_string()
        } else {
            "2-4x A100 80 GB with tensor parallelism, or use GPTQ/AWQ int4 quantization on fewer GPUs.".to_string()
        }
    } else {
        format!(
            "Multi-node cluster required for {:.0}B params. \
             Recommend 4+ nodes of 8x H100 80 GB with NVLink + InfiniBand. \
             Use Megatron-LM with 3D parallelism (tensor + pipeline + data).",
            model_params_billions
        )
    }
}

/// Validate a training job configuration. Returns a list of error messages (empty = valid).
pub fn validate_training_config(config: &TrainingJobConfig) -> Vec<String> {
    let mut errors = Vec::new();

    if config.name.trim().is_empty() {
        errors.push("Job name must not be empty.".to_string());
    }
    if config.model_path.trim().is_empty() {
        errors.push("Model path must not be empty.".to_string());
    }
    if config.dataset_path.trim().is_empty() {
        errors.push("Dataset path must not be empty.".to_string());
    }
    if config.output_dir.trim().is_empty() {
        errors.push("Output directory must not be empty.".to_string());
    }
    if config.batch_size == 0 {
        errors.push("Batch size must be greater than zero.".to_string());
    }
    if config.learning_rate <= 0.0 || config.learning_rate > 1.0 {
        errors.push("Learning rate must be in the range (0.0, 1.0].".to_string());
    }
    if config.epochs == 0 {
        errors.push("Epochs must be at least 1.".to_string());
    }
    if config.distributed && config.num_gpus < 2 {
        errors.push("Distributed training requires at least 2 GPUs.".to_string());
    }
    if !config.distributed && config.num_gpus == 0 {
        errors.push("At least 1 GPU is required.".to_string());
    }

    errors
}

/// Validate an inference configuration. Returns a list of error messages (empty = valid).
pub fn validate_inference_config(config: &InferenceConfig) -> Vec<String> {
    let mut errors = Vec::new();

    if config.model_path.trim().is_empty() {
        errors.push("Model path must not be empty.".to_string());
    }
    if config.port == 0 {
        errors.push("Port must be greater than zero.".to_string());
    }
    if config.port < 1024 {
        errors.push("Port should be >= 1024 to avoid requiring root privileges.".to_string());
    }
    if config.max_batch_size == 0 {
        errors.push("Max batch size must be at least 1.".to_string());
    }
    if config.tensor_parallel == 0 {
        errors.push("Tensor parallel degree must be at least 1.".to_string());
    }
    if config.gpu_memory_fraction <= 0.0 || config.gpu_memory_fraction > 1.0 {
        errors.push("GPU memory fraction must be in the range (0.0, 1.0].".to_string());
    }

    errors
}

/// Generate a SLURM sbatch script for a training job.
pub fn generate_slurm_script(
    job: &TrainingJobConfig,
    gpus_per_node: u32,
    num_nodes: u32,
) -> String {
    let framework_cmd = match job.framework {
        TrainingFramework::PyTorch | TrainingFramework::DeepSpeed | TrainingFramework::HuggingFace => {
            format!(
                "torchrun --nproc_per_node={gpus_per_node} --nnodes={num_nodes} \\\n  \
                 --master_addr=$MASTER_ADDR --master_port=$MASTER_PORT \\\n  \
                 {model_path} \\\n  \
                 --dataset {dataset} \\\n  \
                 --output_dir {output} \\\n  \
                 --batch_size {bs} \\\n  \
                 --learning_rate {lr} \\\n  \
                 --epochs {epochs}",
                gpus_per_node = gpus_per_node,
                num_nodes = num_nodes,
                model_path = job.model_path,
                dataset = job.dataset_path,
                output = job.output_dir,
                bs = job.batch_size,
                lr = job.learning_rate,
                epochs = job.epochs,
            )
        }
        TrainingFramework::TensorFlow => {
            format!(
                "python {model_path} \\\n  \
                 --dataset {dataset} \\\n  \
                 --output_dir {output} \\\n  \
                 --batch_size {bs} \\\n  \
                 --learning_rate {lr} \\\n  \
                 --epochs {epochs}",
                model_path = job.model_path,
                dataset = job.dataset_path,
                output = job.output_dir,
                bs = job.batch_size,
                lr = job.learning_rate,
                epochs = job.epochs,
            )
        }
        TrainingFramework::Jax => {
            format!(
                "python {model_path} \\\n  \
                 --dataset {dataset} \\\n  \
                 --output_dir {output} \\\n  \
                 --batch_size {bs} \\\n  \
                 --learning_rate {lr} \\\n  \
                 --epochs {epochs}",
                model_path = job.model_path,
                dataset = job.dataset_path,
                output = job.output_dir,
                bs = job.batch_size,
                lr = job.learning_rate,
                epochs = job.epochs,
            )
        }
        TrainingFramework::Megatron => {
            format!(
                "python -m megatron.training \\\n  \
                 --model-path {model_path} \\\n  \
                 --data-path {dataset} \\\n  \
                 --save {output} \\\n  \
                 --micro-batch-size {bs} \\\n  \
                 --lr {lr} \\\n  \
                 --train-iters {epochs}",
                model_path = job.model_path,
                dataset = job.dataset_path,
                output = job.output_dir,
                bs = job.batch_size,
                lr = job.learning_rate,
                epochs = job.epochs,
            )
        }
    };

    let mixed_flag = if job.mixed_precision { "\n#SBATCH --comment=mixed-precision" } else { "" };
    let total_gpus = gpus_per_node * num_nodes;

    format!(
        r#"#!/bin/bash
#SBATCH --job-name={name}
#SBATCH --nodes={num_nodes}
#SBATCH --ntasks-per-node=1
#SBATCH --gres=gpu:{gpus_per_node}
#SBATCH --time=72:00:00
#SBATCH --output={name}_%j.out
#SBATCH --error={name}_%j.err{mixed_flag}

# Total GPUs: {total_gpus}

export MASTER_ADDR=$(scontrol show hostname $SLURM_NODELIST | head -n1)
export MASTER_PORT=29500

echo "Starting training job: {name}"
echo "Nodes: {num_nodes}, GPUs per node: {gpus_per_node}, Total GPUs: {total_gpus}"

srun {framework_cmd}
"#,
        name = job.name,
        num_nodes = num_nodes,
        gpus_per_node = gpus_per_node,
        mixed_flag = mixed_flag,
        total_gpus = total_gpus,
        framework_cmd = framework_cmd,
    )
}

/// Generate a Kubernetes Job YAML manifest requesting GPU resources.
pub fn generate_k8s_gpu_manifest(
    job: &TrainingJobConfig,
    gpu_type: &str,
    num_gpus: u32,
) -> String {
    let mixed_env = if job.mixed_precision {
        "\n        - name: MIXED_PRECISION\n          value: \"true\""
    } else {
        ""
    };

    format!(
        r#"apiVersion: batch/v1
kind: Job
metadata:
  name: {name}
  labels:
    app: vibecody-training
    framework: {framework}
spec:
  backoffLimit: 2
  template:
    metadata:
      labels:
        app: vibecody-training
        job: {name}
    spec:
      restartPolicy: OnFailure
      containers:
      - name: training
        image: nvcr.io/nvidia/pytorch:24.01-py3
        command: ["python", "{model_path}"]
        args:
        - "--dataset={dataset}"
        - "--output_dir={output}"
        - "--batch_size={bs}"
        - "--learning_rate={lr}"
        - "--epochs={epochs}"
        resources:
          limits:
            nvidia.com/gpu: {num_gpus}
          requests:
            nvidia.com/gpu: {num_gpus}
        env:
        - name: NVIDIA_VISIBLE_DEVICES
          value: "all"{mixed_env}
      nodeSelector:
        gpu-type: {gpu_type}
      tolerations:
      - key: nvidia.com/gpu
        operator: Exists
        effect: NoSchedule
"#,
        name = job.name,
        framework = format!("{:?}", job.framework).to_lowercase(),
        model_path = job.model_path,
        dataset = job.dataset_path,
        output = job.output_dir,
        bs = job.batch_size,
        lr = job.learning_rate,
        epochs = job.epochs,
        num_gpus = num_gpus,
        gpu_type = gpu_type,
        mixed_env = mixed_env,
    )
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_training_config() -> TrainingJobConfig {
        TrainingJobConfig {
            name: "llama-finetune".to_string(),
            model_path: "/models/llama-7b/train.py".to_string(),
            dataset_path: "/data/alpaca".to_string(),
            output_dir: "/output/llama-ft".to_string(),
            batch_size: 4,
            learning_rate: 2e-5,
            epochs: 3,
            distributed: false,
            num_gpus: 1,
            framework: TrainingFramework::PyTorch,
            mixed_precision: true,
            gradient_checkpointing: true,
            extra_args: HashMap::new(),
        }
    }

    fn sample_inference_config() -> InferenceConfig {
        InferenceConfig {
            model_path: "/models/llama-7b".to_string(),
            engine: InferenceEngine::Vllm,
            port: 8080,
            max_batch_size: 32,
            quantization: Some(Quantization::Awq),
            tensor_parallel: 2,
            gpu_memory_fraction: 0.9,
        }
    }

    #[test]
    fn test_gpu_vendor_serialization() {
        let vendor = GpuVendor::Nvidia;
        let json = serde_json::to_string(&vendor).expect("serialize GpuVendor");
        assert_eq!(json, "\"Nvidia\"");
        let deserialized: GpuVendor =
            serde_json::from_str(&json).expect("deserialize GpuVendor");
        assert_eq!(deserialized, GpuVendor::Nvidia);
    }

    #[test]
    fn test_detect_local_gpus_returns_empty() {
        let gpus = detect_local_gpus();
        assert!(gpus.is_empty(), "stub should return an empty vec");
    }

    #[test]
    fn test_estimate_cost_aws() {
        let cost = estimate_gpu_cost(&ClusterProvider::AwsEc2, 8, 10.0);
        assert!(cost > 200.0, "8 GPUs for 10h on AWS should exceed $200");
        assert!((cost - 244.8).abs() < 0.01);
    }

    #[test]
    fn test_estimate_cost_local() {
        let cost = estimate_gpu_cost(&ClusterProvider::Local, 4, 100.0);
        assert_eq!(cost, 0.0, "local GPUs should have zero cost");
    }

    #[test]
    fn test_suggest_config_small_model() {
        let suggestion = suggest_gpu_config(0.5, "inference");
        assert!(
            suggestion.contains("8 GB"),
            "small model inference should mention 8 GB"
        );
    }

    #[test]
    fn test_suggest_config_large_model() {
        let suggestion = suggest_gpu_config(70.0, "training");
        assert!(
            suggestion.contains("A100 80 GB"),
            "70B training should recommend A100 80 GB"
        );
        assert!(
            suggestion.contains("DeepSpeed") || suggestion.contains("Megatron"),
            "should recommend distributed framework"
        );
    }

    #[test]
    fn test_validate_training_config_valid() {
        let config = sample_training_config();
        let errors = validate_training_config(&config);
        assert!(errors.is_empty(), "valid config should have no errors: {:?}", errors);
    }

    #[test]
    fn test_validate_training_config_empty_name() {
        let mut config = sample_training_config();
        config.name = "".to_string();
        let errors = validate_training_config(&config);
        assert!(errors.iter().any(|e| e.contains("name")));
    }

    #[test]
    fn test_validate_training_config_bad_lr() {
        let mut config = sample_training_config();
        config.learning_rate = -0.01;
        let errors = validate_training_config(&config);
        assert!(errors.iter().any(|e| e.contains("Learning rate")));
    }

    #[test]
    fn test_validate_inference_config_valid() {
        let config = sample_inference_config();
        let errors = validate_inference_config(&config);
        assert!(errors.is_empty(), "valid config should produce no errors: {:?}", errors);
    }

    #[test]
    fn test_validate_inference_config_bad_port() {
        let mut config = sample_inference_config();
        config.port = 0;
        let errors = validate_inference_config(&config);
        assert!(errors.iter().any(|e| e.contains("Port")));
    }

    #[test]
    fn test_generate_slurm_script_contains_gpu() {
        let config = sample_training_config();
        let script = generate_slurm_script(&config, 4, 1);
        assert!(script.contains("#SBATCH --gres=gpu:4"));
        assert!(script.contains("--job-name=llama-finetune"));
        assert!(script.contains("torchrun"));
    }

    #[test]
    fn test_generate_slurm_script_multi_node() {
        let mut config = sample_training_config();
        config.distributed = true;
        config.num_gpus = 16;
        let script = generate_slurm_script(&config, 8, 2);
        assert!(script.contains("#SBATCH --nodes=2"));
        assert!(script.contains("#SBATCH --gres=gpu:8"));
        assert!(script.contains("Total GPUs: 16"));
        assert!(script.contains("--nnodes=2"));
    }

    #[test]
    fn test_generate_k8s_manifest_contains_gpu_limit() {
        let config = sample_training_config();
        let manifest = generate_k8s_gpu_manifest(&config, "a100", 4);
        assert!(manifest.contains("nvidia.com/gpu: 4"));
        assert!(manifest.contains("gpu-type: a100"));
        assert!(manifest.contains("kind: Job"));
        assert!(manifest.contains("MIXED_PRECISION"));
    }

    #[test]
    fn test_quantization_serialization() {
        let quant = Quantization::Gptq;
        let json = serde_json::to_string(&quant).expect("serialize Quantization");
        assert_eq!(json, "\"Gptq\"");
        let deserialized: Quantization =
            serde_json::from_str(&json).expect("deserialize Quantization");
        assert_eq!(deserialized, Quantization::Gptq);
    }

    #[test]
    fn test_node_status_serialization() {
        let status = GpuNodeStatus::Provisioning;
        let json = serde_json::to_string(&status).expect("serialize GpuNodeStatus");
        assert_eq!(json, "\"Provisioning\"");
        let deserialized: GpuNodeStatus =
            serde_json::from_str(&json).expect("deserialize GpuNodeStatus");
        assert_eq!(deserialized, GpuNodeStatus::Provisioning);
    }
}
