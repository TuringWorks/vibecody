// Inference server orchestration module for VibeCody CLI.
// Provides CLI command generation, Docker Compose configs, K8s manifests,
// and benchmarking utilities for vLLM, TGI, Triton, llama.cpp, and more.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InferenceBackend {
    Vllm,
    Tgi,
    Triton,
    OllamaServe,
    TorchServe,
    Onnxruntime,
    LlamaCpp,
    TrtLlm,
    /// Mistral.rs — in-process Rust runtime (`vibe-infer::mistral`). No
    /// sidecar container; runs inside the VibeCLI daemon with PagedAttention
    /// and in-situ quantization.
    MistralRs,
}

impl InferenceBackend {
    /// True when this backend runs **inside** the VibeCLI daemon (no
    /// separate container / process). Today only Mistral.rs qualifies;
    /// callers use this to hide Docker / K8s surfaces that don't apply.
    pub fn is_in_process(&self) -> bool {
        matches!(self, InferenceBackend::MistralRs)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelFormat {
    SafeTensors,
    Pytorch,
    Onnx,
    TensorRt,
    Gguf,
    Ggml,
    Awq,
    Gptq,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuantizationMethod {
    None,
    Fp16,
    Bf16,
    Int8,
    Int4,
    Gptq,
    Awq,
    SqueezeLlm,
    GgufQ4Km,
    GgufQ5Km,
    GgufQ80,
    /// TurboQuant: PolarQuant + QJL two-stage KV-cache compression (~3 bits/dim).
    /// Requires backend support (vLLM ≥ 0.7, llama.cpp with --kv-quant turbo).
    TurboQuant,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EndpointStatus {
    Starting,
    Ready,
    Degraded,
    Error(String),
    Stopped,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoadBalanceStrategy {
    RoundRobin,
    LeastConnections,
    WeightedRandom,
    ConsistentHash,
}

// ---------------------------------------------------------------------------
// Structs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServingConfig {
    pub backend: InferenceBackend,
    pub model_path: String,
    pub model_name: String,
    pub port: u16,
    pub host: String,
    pub gpu_count: u32,
    pub tensor_parallel: u32,
    pub max_model_len: Option<u32>,
    pub max_batch_size: u32,
    pub quantization: QuantizationMethod,
    pub dtype: String,
    pub trust_remote_code: bool,
    pub gpu_memory_utilization: f64,
    pub extra_args: HashMap<String, String>,
}

impl Default for ServingConfig {
    fn default() -> Self {
        Self {
            backend: InferenceBackend::Vllm,
            model_path: String::new(),
            model_name: String::new(),
            port: 8000,
            host: "0.0.0.0".to_string(),
            gpu_count: 1,
            tensor_parallel: 1,
            max_model_len: None,
            max_batch_size: 32,
            quantization: QuantizationMethod::None,
            dtype: "auto".to_string(),
            trust_remote_code: false,
            gpu_memory_utilization: 0.9,
            extra_args: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceEndpoint {
    pub id: String,
    pub config: ServingConfig,
    pub status: EndpointStatus,
    pub url: Option<String>,
    pub started_at: Option<String>,
    pub requests_served: u64,
    pub avg_latency_ms: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadBalancerConfig {
    pub endpoints: Vec<String>,
    pub strategy: LoadBalanceStrategy,
    pub health_check_interval_secs: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoScaleConfig {
    pub min_replicas: u32,
    pub max_replicas: u32,
    pub target_gpu_utilization: f64,
    pub target_latency_ms: u64,
    pub scale_up_cooldown_secs: u32,
    pub scale_down_cooldown_secs: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    pub backend: InferenceBackend,
    pub model_name: String,
    pub prompt_tokens: u32,
    pub output_tokens: u32,
    pub time_to_first_token_ms: f64,
    pub tokens_per_second: f64,
    pub total_latency_ms: f64,
    pub gpu_memory_used_mb: u64,
}

// ---------------------------------------------------------------------------
// Command builders
// ---------------------------------------------------------------------------

pub fn build_vllm_command(config: &ServingConfig) -> Vec<String> {
    let mut cmd = vec![
        "python".to_string(), "-m".to_string(),
        "vllm.entrypoints.openai.api_server".to_string(),
        "--model".to_string(), config.model_path.clone(),
        "--host".to_string(), config.host.clone(),
        "--port".to_string(), config.port.to_string(),
        "--tensor-parallel-size".to_string(), config.tensor_parallel.to_string(),
        "--max-num-batched-tokens".to_string(), (config.max_batch_size * 512).to_string(),
        "--dtype".to_string(), config.dtype.clone(),
        "--gpu-memory-utilization".to_string(), config.gpu_memory_utilization.to_string(),
    ];
    if let Some(max_len) = config.max_model_len {
        cmd.push("--max-model-len".to_string());
        cmd.push(max_len.to_string());
    }
    match config.quantization {
        QuantizationMethod::Gptq => { cmd.push("--quantization".to_string()); cmd.push("gptq".to_string()); }
        QuantizationMethod::Awq => { cmd.push("--quantization".to_string()); cmd.push("awq".to_string()); }
        QuantizationMethod::SqueezeLlm => { cmd.push("--quantization".to_string()); cmd.push("squeezellm".to_string()); }
        _ => {}
    }
    if config.trust_remote_code {
        cmd.push("--trust-remote-code".to_string());
    }
    if !config.model_name.is_empty() {
        cmd.push("--served-model-name".to_string());
        cmd.push(config.model_name.clone());
    }
    for (k, v) in &config.extra_args {
        cmd.push(format!("--{k}"));
        if !v.is_empty() { cmd.push(v.clone()); }
    }
    cmd
}

pub fn build_tgi_command(config: &ServingConfig) -> Vec<String> {
    let mut cmd = vec![
        "text-generation-launcher".to_string(),
        "--model-id".to_string(), config.model_path.clone(),
        "--hostname".to_string(), config.host.clone(),
        "--port".to_string(), config.port.to_string(),
        "--num-shard".to_string(), config.tensor_parallel.to_string(),
        "--max-batch-total-tokens".to_string(), (config.max_batch_size * 1024).to_string(),
        "--dtype".to_string(), config.dtype.clone(),
    ];
    if let Some(max_len) = config.max_model_len {
        cmd.push("--max-total-tokens".to_string());
        cmd.push(max_len.to_string());
    }
    match config.quantization {
        QuantizationMethod::Gptq => { cmd.push("--quantize".to_string()); cmd.push("gptq".to_string()); }
        QuantizationMethod::Awq => { cmd.push("--quantize".to_string()); cmd.push("awq".to_string()); }
        QuantizationMethod::Int8 => { cmd.push("--quantize".to_string()); cmd.push("bitsandbytes".to_string()); }
        _ => {}
    }
    if config.trust_remote_code {
        cmd.push("--trust-remote-code".to_string());
    }
    cmd
}

pub fn build_triton_command(config: &ServingConfig) -> Vec<String> {
    vec![
        "tritonserver".to_string(),
        "--model-repository".to_string(), config.model_path.clone(),
        "--http-port".to_string(), config.port.to_string(),
        "--grpc-port".to_string(), (config.port + 1).to_string(),
        "--metrics-port".to_string(), (config.port + 2).to_string(),
    ]
}

pub fn build_llamacpp_command(config: &ServingConfig) -> Vec<String> {
    let mut cmd = vec![
        "llama-server".to_string(),
        "-m".to_string(), config.model_path.clone(),
        "--host".to_string(), config.host.clone(),
        "--port".to_string(), config.port.to_string(),
        "-ngl".to_string(), "999".to_string(), // offload all layers to GPU
        "-b".to_string(), config.max_batch_size.to_string(),
    ];
    if config.gpu_count > 1 {
        cmd.push("--tensor-split".to_string());
        cmd.push(vec!["1.0"; config.gpu_count as usize].join(","));
    }
    if let Some(max_len) = config.max_model_len {
        cmd.push("-c".to_string());
        cmd.push(max_len.to_string());
    }
    cmd
}

pub fn build_ollama_command(_config: &ServingConfig) -> Vec<String> {
    vec![
        "ollama".to_string(),
        "serve".to_string(),
    ]
}

/// Build the shell command that reproduces the deployment with Mistral.rs's
/// in-process backend via `vibe-infer`. Emits the `cargo run ... --example
/// generate` form we document in `vibeui/crates/vibe-infer/examples/generate.rs`.
///
/// Mistral.rs does not ship as a Docker sidecar — the emitted command runs
/// inside whatever VibeCLI is embedded in. Pick the feature flag closest to
/// the target host: `mistralrs-metal` on Apple Silicon, `mistralrs-cuda` on
/// NVIDIA, plain `mistralrs` for CPU.
pub fn build_mistralrs_command(config: &ServingConfig) -> Vec<String> {
    let feature = if config.gpu_count == 0 {
        "mistralrs"
    } else {
        // Can't tell CUDA vs Metal from ServingConfig alone — default to
        // `mistralrs` and let the operator flip the flag. We surface this
        // in the UI where the distinction is available.
        "mistralrs"
    };
    let model = if config.model_path.is_empty() {
        "Qwen/Qwen2.5-0.5B-Instruct".to_string()
    } else {
        config.model_path.clone()
    };
    vec![
        format!("VIBE_INFER_MODEL={model}"),
        "cargo".to_string(),
        "run".to_string(),
        "--release".to_string(),
        "-p".to_string(),
        "vibe-infer".to_string(),
        "--features".to_string(),
        feature.to_string(),
        "--example".to_string(),
        "generate".to_string(),
        "--".to_string(),
        "\"Say hi in one word.\"".to_string(),
    ]
}

// ---------------------------------------------------------------------------
// Config generators
// ---------------------------------------------------------------------------

pub fn generate_vllm_docker_compose(config: &ServingConfig) -> String {
    format!(
        r#"version: '3.8'
services:
  vllm:
    image: vllm/vllm-openai:latest
    ports:
      - '{port}:{port}'
    volumes:
      - ~/.cache/huggingface:/root/.cache/huggingface
    environment:
      - HUGGING_FACE_HUB_TOKEN=${{HF_TOKEN}}
    command: >
      --model {model}
      --host 0.0.0.0
      --port {port}
      --tensor-parallel-size {tp}
      --gpu-memory-utilization {gpu_mem}
      --dtype {dtype}
    deploy:
      resources:
        reservations:
          devices:
            - driver: nvidia
              count: {gpus}
              capabilities: [gpu]
"#,
        port = config.port,
        model = config.model_path,
        tp = config.tensor_parallel,
        gpu_mem = config.gpu_memory_utilization,
        dtype = config.dtype,
        gpus = config.gpu_count,
    )
}

pub fn generate_tgi_docker_compose(config: &ServingConfig) -> String {
    format!(
        r#"version: '3.8'
services:
  tgi:
    image: ghcr.io/huggingface/text-generation-inference:latest
    ports:
      - '{port}:{port}'
    volumes:
      - ~/.cache/huggingface:/data
    environment:
      - HUGGING_FACE_HUB_TOKEN=${{HF_TOKEN}}
    command: >
      --model-id {model}
      --port {port}
      --num-shard {tp}
      --dtype {dtype}
    deploy:
      resources:
        reservations:
          devices:
            - driver: nvidia
              count: {gpus}
              capabilities: [gpu]
    shm_size: '1g'
"#,
        port = config.port,
        model = config.model_path,
        tp = config.tensor_parallel,
        dtype = config.dtype,
        gpus = config.gpu_count,
    )
}

pub fn generate_triton_model_config(config: &ServingConfig) -> String {
    format!(
        r#"name: "{name}"
platform: "python"
max_batch_size: {batch}
input [
  {{
    name: "text_input"
    data_type: TYPE_STRING
    dims: [ 1 ]
  }}
]
output [
  {{
    name: "text_output"
    data_type: TYPE_STRING
    dims: [ 1 ]
  }}
]
instance_group [
  {{
    count: {gpus}
    kind: KIND_GPU
  }}
]
"#,
        name = config.model_name,
        batch = config.max_batch_size,
        gpus = config.gpu_count,
    )
}

pub fn generate_k8s_inference_deployment(config: &ServingConfig, replicas: u32) -> String {
    let image = match config.backend {
        InferenceBackend::Vllm => "vllm/vllm-openai:latest",
        InferenceBackend::Tgi => "ghcr.io/huggingface/text-generation-inference:latest",
        InferenceBackend::LlamaCpp => "ghcr.io/ggerganov/llama.cpp:server",
        // Mistral.rs is in-process — there is no standalone image, so we
        // fall through to the VibeCLI daemon container that already
        // embeds vibe-infer. Callers who deploy Mistral.rs to K8s use the
        // daemon image and enable the `mistralrs` feature at build time.
        InferenceBackend::MistralRs => "vibecody/vibecli-daemon:latest",
        _ => "custom-inference:latest",
    };
    format!(
        r#"apiVersion: apps/v1
kind: Deployment
metadata:
  name: {name}-inference
  labels:
    app: {name}
spec:
  replicas: {replicas}
  selector:
    matchLabels:
      app: {name}
  template:
    metadata:
      labels:
        app: {name}
    spec:
      containers:
      - name: inference
        image: {image}
        ports:
        - containerPort: {port}
        resources:
          limits:
            nvidia.com/gpu: "{gpus}"
          requests:
            nvidia.com/gpu: "{gpus}"
            memory: "16Gi"
        env:
        - name: HUGGING_FACE_HUB_TOKEN
          valueFrom:
            secretKeyRef:
              name: hf-secret
              key: token
        readinessProbe:
          httpGet:
            path: /health
            port: {port}
          initialDelaySeconds: 30
          periodSeconds: 10
---
apiVersion: v1
kind: Service
metadata:
  name: {name}-service
spec:
  selector:
    app: {name}
  ports:
  - port: 80
    targetPort: {port}
  type: ClusterIP
"#,
        name = config.model_name.replace('/', "-"),
        replicas = replicas,
        image = image,
        port = config.port,
        gpus = config.gpu_count,
    )
}

// ---------------------------------------------------------------------------
// Utility functions
// ---------------------------------------------------------------------------

pub fn validate_serving_config(config: &ServingConfig) -> Vec<String> {
    let mut errors = vec![];
    if config.model_path.is_empty() {
        errors.push("Model path is required".to_string());
    }
    if config.port == 0 {
        errors.push("Port must be > 0".to_string());
    }
    if config.gpu_memory_utilization <= 0.0 || config.gpu_memory_utilization > 1.0 {
        errors.push("GPU memory utilization must be between 0.0 and 1.0".to_string());
    }
    if config.tensor_parallel > config.gpu_count {
        errors.push("Tensor parallel size cannot exceed GPU count".to_string());
    }
    if config.max_batch_size == 0 {
        errors.push("Max batch size must be > 0".to_string());
    }
    errors
}

pub fn estimate_gpu_memory(model_params_b: f64, quantization: &QuantizationMethod) -> u64 {
    // Rough estimation: params * bytes_per_param + overhead
    let bytes_per_param = match quantization {
        QuantizationMethod::None | QuantizationMethod::Fp16 | QuantizationMethod::Bf16 => 2.0,
        QuantizationMethod::Int8 | QuantizationMethod::GgufQ80 => 1.0,
        QuantizationMethod::Int4 | QuantizationMethod::Gptq | QuantizationMethod::Awq
        | QuantizationMethod::GgufQ4Km | QuantizationMethod::GgufQ5Km
        | QuantizationMethod::SqueezeLlm => 0.5,
        // TurboQuant: ~3 bits/param for KV cache, ~0.375 bytes/param
        QuantizationMethod::TurboQuant => 0.375,
    };
    let model_mb = (model_params_b * 1_000.0 * bytes_per_param) as u64;
    // Add ~20% overhead for KV cache, activations
    model_mb + (model_mb / 5)
}

pub fn suggest_serving_config(model_name: &str, gpu_vram_mb: u64) -> String {
    let name_lower = model_name.to_lowercase();
    // Try to detect model size from name
    let params_b = if name_lower.contains("70b") || name_lower.contains("72b") { 70.0 }
        else if name_lower.contains("34b") || name_lower.contains("33b") { 34.0 }
        else if name_lower.contains("13b") || name_lower.contains("14b") { 13.0 }
        else if name_lower.contains("7b") || name_lower.contains("8b") { 7.0 }
        else if name_lower.contains("3b") { 3.0 }
        else if name_lower.contains("1b") || name_lower.contains("1.5b") { 1.5 }
        else { 7.0 }; // default guess

    let fp16_mem = estimate_gpu_memory(params_b, &QuantizationMethod::Fp16);
    let int4_mem = estimate_gpu_memory(params_b, &QuantizationMethod::Int4);

    if fp16_mem <= gpu_vram_mb {
        format!("Use vLLM with FP16 — {model_name} fits in {gpu_vram_mb}MB (needs ~{fp16_mem}MB). \
                 Recommended: --dtype float16 --gpu-memory-utilization 0.9")
    } else if int4_mem <= gpu_vram_mb {
        format!("Use vLLM with GPTQ/AWQ quantization — {model_name} needs quantization to fit {gpu_vram_mb}MB (FP16: ~{fp16_mem}MB, INT4: ~{int4_mem}MB). \
                 Recommended: --quantization awq --dtype float16")
    } else {
        let gpus_needed = (fp16_mem as f64 / gpu_vram_mb as f64).ceil() as u32;
        format!("Use vLLM with tensor parallelism — {model_name} needs ~{gpus_needed} GPUs at FP16 ({fp16_mem}MB) or INT4 quantization ({int4_mem}MB). \
                 Recommended: --tensor-parallel-size {gpus_needed} --dtype float16")
    }
}

pub fn compare_backends(results: &[BenchmarkResult]) -> String {
    if results.is_empty() { return "No benchmark results to compare.".to_string(); }
    let mut output = format!("{:<12} {:<20} {:>8} {:>8} {:>10} {:>10} {:>8}\n",
        "Backend", "Model", "Prompt", "Output", "TTFT(ms)", "Tok/s", "VRAM(MB)");
    output.push_str(&"-".repeat(78));
    output.push('\n');
    for r in results {
        output.push_str(&format!("{:<12} {:<20} {:>8} {:>8} {:>10.1} {:>10.1} {:>8}\n",
            format!("{:?}", r.backend),
            if r.model_name.len() > 20 { &r.model_name[..20] } else { &r.model_name },
            r.prompt_tokens, r.output_tokens,
            r.time_to_first_token_ms, r.tokens_per_second, r.gpu_memory_used_mb));
    }
    output
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn default_config() -> ServingConfig {
        ServingConfig {
            model_path: "meta-llama/Llama-3-8B-Instruct".to_string(),
            model_name: "llama3-8b".to_string(),
            ..ServingConfig::default()
        }
    }

    #[test]
    fn test_backend_serialization() {
        let b = InferenceBackend::Vllm;
        let json = serde_json::to_string(&b).unwrap();
        assert_eq!(json, "\"vllm\"");
        let back: InferenceBackend = serde_json::from_str(&json).unwrap();
        assert_eq!(back, InferenceBackend::Vllm);
    }

    #[test]
    fn test_quantization_serialization() {
        let q = QuantizationMethod::GgufQ4Km;
        let json = serde_json::to_string(&q).unwrap();
        assert!(json.contains("gguf"));
    }

    #[test]
    fn test_build_vllm_command() {
        let cmd = build_vllm_command(&default_config());
        assert!(cmd.contains(&"vllm.entrypoints.openai.api_server".to_string()));
        assert!(cmd.contains(&"meta-llama/Llama-3-8B-Instruct".to_string()));
        assert!(cmd.contains(&"--tensor-parallel-size".to_string()));
    }

    #[test]
    fn test_build_vllm_command_with_quantization() {
        let mut config = default_config();
        config.quantization = QuantizationMethod::Awq;
        config.trust_remote_code = true;
        let cmd = build_vllm_command(&config);
        assert!(cmd.contains(&"awq".to_string()));
        assert!(cmd.contains(&"--trust-remote-code".to_string()));
    }

    #[test]
    fn test_build_tgi_command() {
        let cmd = build_tgi_command(&default_config());
        assert!(cmd.contains(&"text-generation-launcher".to_string()));
        assert!(cmd.contains(&"--model-id".to_string()));
    }

    #[test]
    fn test_build_triton_command() {
        let cmd = build_triton_command(&default_config());
        assert!(cmd.contains(&"tritonserver".to_string()));
        assert!(cmd.contains(&"--model-repository".to_string()));
    }

    #[test]
    fn test_build_llamacpp_command() {
        let cmd = build_llamacpp_command(&default_config());
        assert!(cmd.contains(&"llama-server".to_string()));
        assert!(cmd.contains(&"-m".to_string()));
        assert!(cmd.contains(&"-ngl".to_string()));
    }

    #[test]
    fn test_build_ollama_command() {
        let cmd = build_ollama_command(&default_config());
        assert!(cmd.contains(&"ollama".to_string()));
        assert!(cmd.contains(&"serve".to_string()));
    }

    #[test]
    fn test_build_mistralrs_command() {
        let mut config = default_config();
        config.model_path = "Qwen/Qwen2.5-3B-Instruct".to_string();
        let cmd = build_mistralrs_command(&config);
        assert!(cmd.iter().any(|s| s == "cargo"));
        assert!(cmd.iter().any(|s| s == "vibe-infer"));
        assert!(cmd.iter().any(|s| s == "generate"));
        assert!(cmd.iter().any(|s| s.starts_with("VIBE_INFER_MODEL=")));
        assert!(cmd.iter().any(|s| s.contains("Qwen2.5-3B-Instruct")));
    }

    #[test]
    fn test_build_mistralrs_command_defaults_small_model() {
        let mut config = default_config();
        config.model_path = String::new();
        let cmd = build_mistralrs_command(&config);
        assert!(
            cmd.iter().any(|s| s.contains("Qwen2.5-0.5B-Instruct")),
            "expected small default model when model_path is empty"
        );
    }

    #[test]
    fn test_mistralrs_is_in_process() {
        assert!(InferenceBackend::MistralRs.is_in_process());
        assert!(!InferenceBackend::Vllm.is_in_process());
        assert!(!InferenceBackend::Tgi.is_in_process());
    }

    #[test]
    fn test_mistralrs_backend_serialization() {
        let b = InferenceBackend::MistralRs;
        let json = serde_json::to_string(&b).unwrap();
        assert_eq!(json, "\"mistral_rs\"");
        let back: InferenceBackend = serde_json::from_str(&json).unwrap();
        assert_eq!(back, InferenceBackend::MistralRs);
    }

    #[test]
    fn test_k8s_deployment_uses_daemon_image_for_mistralrs() {
        let mut config = default_config();
        config.backend = InferenceBackend::MistralRs;
        let yaml = generate_k8s_inference_deployment(&config, 1);
        assert!(yaml.contains("vibecody/vibecli-daemon"));
    }

    #[test]
    fn test_generate_vllm_docker_compose() {
        let yaml = generate_vllm_docker_compose(&default_config());
        assert!(yaml.contains("vllm/vllm-openai"));
        assert!(yaml.contains("--model"));
        assert!(yaml.contains("nvidia"));
    }

    #[test]
    fn test_generate_tgi_docker_compose() {
        let yaml = generate_tgi_docker_compose(&default_config());
        assert!(yaml.contains("text-generation-inference"));
        assert!(yaml.contains("shm_size"));
    }

    #[test]
    fn test_generate_triton_model_config() {
        let pbtxt = generate_triton_model_config(&default_config());
        assert!(pbtxt.contains("llama3-8b"));
        assert!(pbtxt.contains("KIND_GPU"));
        assert!(pbtxt.contains("text_input"));
    }

    #[test]
    fn test_generate_k8s_inference_deployment() {
        let yaml = generate_k8s_inference_deployment(&default_config(), 3);
        assert!(yaml.contains("replicas: 3"));
        assert!(yaml.contains("nvidia.com/gpu"));
        assert!(yaml.contains("readinessProbe"));
        assert!(yaml.contains("vllm/vllm-openai"));
    }

    #[test]
    fn test_validate_serving_config_valid() {
        let errors = validate_serving_config(&default_config());
        assert!(errors.is_empty());
    }

    #[test]
    fn test_validate_serving_config_bad_port() {
        let mut config = default_config();
        config.port = 0;
        let errors = validate_serving_config(&config);
        assert!(errors.iter().any(|e| e.contains("Port")));
    }

    #[test]
    fn test_estimate_gpu_memory_fp16() {
        let mem = estimate_gpu_memory(7.0, &QuantizationMethod::Fp16);
        assert!(mem > 14000 && mem < 20000); // ~7B * 2 bytes = 14GB + overhead
    }

    #[test]
    fn test_estimate_gpu_memory_int4() {
        let mem = estimate_gpu_memory(7.0, &QuantizationMethod::Int4);
        assert!(mem > 3000 && mem < 6000); // ~7B * 0.5 bytes = 3.5GB + overhead
    }

    #[test]
    fn test_suggest_serving_config() {
        let suggestion = suggest_serving_config("llama-3-8b", 24000);
        assert!(suggestion.contains("FP16") || suggestion.contains("vLLM"));
    }

    #[test]
    fn test_compare_backends() {
        let results = vec![
            BenchmarkResult {
                backend: InferenceBackend::Vllm,
                model_name: "llama-3-8b".to_string(),
                prompt_tokens: 128, output_tokens: 256,
                time_to_first_token_ms: 45.0, tokens_per_second: 120.0,
                total_latency_ms: 2180.0, gpu_memory_used_mb: 16000,
            },
        ];
        let table = compare_backends(&results);
        assert!(table.contains("Vllm"));
        assert!(table.contains("120.0"));
    }
}
