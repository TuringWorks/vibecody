//! vLLM GPU pod deployment manager.
//! Pi-mono gap bridge: Phase D1.
//!
//! Deploys and manages vLLM on remote GPU pods (RunPod, Lambda Labs, Vast.ai).
//! Validates VRAM before starting, auto-selects the correct vLLM
//! `--tool-call-parser` per model family, supports multiple build variants
//! (release/nightly/gpt-oss), and auto-assigns models to GPU devices.

// ---------------------------------------------------------------------------
// GPU / VRAM
// ---------------------------------------------------------------------------

/// Known GPU tiers with their VRAM capacities.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum GpuTier {
    T4_16GB,
    A10_24GB,
    A100_40GB,
    A100_80GB,
    H100_80GB,
    H200_141GB,
    Custom { name: String, vram_gb: u32 },
}

impl GpuTier {
    /// VRAM available on this GPU in gigabytes.
    pub fn vram_gb(&self) -> u32 {
        match self {
            GpuTier::T4_16GB => 16,
            GpuTier::A10_24GB => 24,
            GpuTier::A100_40GB => 40,
            GpuTier::A100_80GB => 80,
            GpuTier::H100_80GB => 80,
            GpuTier::H200_141GB => 141,
            GpuTier::Custom { vram_gb, .. } => *vram_gb,
        }
    }

    /// Human-readable GPU name.
    pub fn name(&self) -> String {
        match self {
            GpuTier::T4_16GB => "NVIDIA T4 16 GB".into(),
            GpuTier::A10_24GB => "NVIDIA A10 24 GB".into(),
            GpuTier::A100_40GB => "NVIDIA A100 40 GB".into(),
            GpuTier::A100_80GB => "NVIDIA A100 80 GB".into(),
            GpuTier::H100_80GB => "NVIDIA H100 80 GB".into(),
            GpuTier::H200_141GB => "NVIDIA H200 141 GB".into(),
            GpuTier::Custom { name, .. } => name.clone(),
        }
    }

    /// Parse a GPU identifier string into a `GpuTier`.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "t4" | "t4-16gb" | "t4_16gb" => Some(GpuTier::T4_16GB),
            "a10" | "a10-24gb" | "a10_24gb" => Some(GpuTier::A10_24GB),
            "a100-40" | "a100-40gb" | "a100_40gb" => Some(GpuTier::A100_40GB),
            "a100" | "a100-80" | "a100-80gb" | "a100_80gb" => Some(GpuTier::A100_80GB),
            "h100" | "h100-80gb" | "h100_80gb" => Some(GpuTier::H100_80GB),
            "h200" | "h200-141gb" | "h200_141gb" => Some(GpuTier::H200_141GB),
            _ => None,
        }
    }

    /// Return true if this GPU has enough VRAM to run the model (single GPU).
    pub fn can_fit_model(&self, model: &ModelConfig) -> bool {
        self.vram_gb() >= model.min_vram_gb
    }
}

// ---------------------------------------------------------------------------
// Model family and tool-call parser
// ---------------------------------------------------------------------------

/// High-level model family used for heuristic flag selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelFamily {
    Qwen,
    Qwen3Coder,
    Glm4Moe,
    Mistral,
    Llama,
    GptOss,
    Other(String),
}

impl ModelFamily {
    /// Heuristic: derive the family from the model name string.
    pub fn from_model_name(name: &str) -> Self {
        let lower = name.to_lowercase();
        if lower.contains("qwen3") && lower.contains("coder") {
            ModelFamily::Qwen3Coder
        } else if lower.contains("glm-4-moe") || lower.contains("glm4moe") {
            ModelFamily::Glm4Moe
        } else if lower.contains("qwen") {
            ModelFamily::Qwen
        } else if lower.contains("mistral") {
            ModelFamily::Mistral
        } else if lower.contains("llama") {
            ModelFamily::Llama
        } else if lower.contains("gpt-oss") || lower.contains("gptoss") {
            ModelFamily::GptOss
        } else {
            ModelFamily::Other(name.to_string())
        }
    }
}

/// vLLM `--tool-call-parser` option.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolCallParser {
    /// Hermes — Qwen, Mistral, most open models.
    Hermes,
    /// Qwen3-Coder family.
    Qwen3Coder,
    /// GLM-4-MoE family.
    Glm4Moe,
    /// GPT-OSS models using /v1/responses endpoint.
    OpenAIResponses,
    /// Let vLLM auto-detect.
    Auto,
    /// No tool-call parser (model does not support tool calls).
    None,
}

impl ToolCallParser {
    /// Return the value to pass to `--tool-call-parser`, or `Option::None` if
    /// the flag should be omitted entirely.
    pub fn as_flag(&self) -> Option<&str> {
        match self {
            ToolCallParser::Hermes => Some("hermes"),
            ToolCallParser::Qwen3Coder => Some("qwen3-coder"),
            ToolCallParser::Glm4Moe => Some("glm4-moe"),
            ToolCallParser::OpenAIResponses => Some("openai-responses"),
            ToolCallParser::Auto => Some("auto"),
            ToolCallParser::None => Option::None,
        }
    }

    /// Select the correct parser for a given model family.
    pub fn from_model_family(family: &ModelFamily) -> Self {
        match family {
            ModelFamily::Qwen3Coder => ToolCallParser::Qwen3Coder,
            ModelFamily::Glm4Moe => ToolCallParser::Glm4Moe,
            ModelFamily::GptOss => ToolCallParser::OpenAIResponses,
            ModelFamily::Qwen | ModelFamily::Mistral | ModelFamily::Llama => ToolCallParser::Hermes,
            ModelFamily::Other(_) => ToolCallParser::Auto,
        }
    }
}

// ---------------------------------------------------------------------------
// ModelConfig
// ---------------------------------------------------------------------------

/// Full deployment configuration for one model.
#[derive(Debug, Clone)]
pub struct ModelConfig {
    /// HuggingFace model id, e.g. `"Qwen/Qwen3-Coder-32B-Instruct"`.
    pub name: String,
    pub family: ModelFamily,
    /// Minimum single-GPU VRAM required (model weights only, no KV cache).
    pub min_vram_gb: u32,
    pub tool_call_parser: ToolCallParser,
    /// Extra arbitrary vLLM CLI flags, e.g. `["--max-model-len", "32768"]`.
    pub extra_vllm_flags: Vec<String>,
    /// When `Some(n)`, force tensor-parallel size to `n`; otherwise auto-calculate.
    pub tensor_parallel_size: Option<u32>,
    /// True for GPT-OSS models that use `/v1/responses` instead of `/v1/chat`.
    pub uses_responses_api: bool,
}

impl ModelConfig {
    /// Look up a well-known model by name, or synthesise a sensible default.
    pub fn for_model(name: &str) -> Self {
        let known = known_models();
        if let Some(m) = known.into_iter().find(|m| m.name == name) {
            return m;
        }
        // Derive from name heuristics.
        let family = ModelFamily::from_model_name(name);
        let parser = ToolCallParser::from_model_family(&family);
        let uses_responses = family == ModelFamily::GptOss;
        ModelConfig {
            name: name.to_string(),
            family,
            min_vram_gb: 16,
            tool_call_parser: parser,
            extra_vllm_flags: vec![],
            tensor_parallel_size: None,
            uses_responses_api: uses_responses,
        }
    }

    /// Build the complete list of vLLM CLI flags for this model.
    ///
    /// The returned `Vec<String>` can be appended to `python -m vllm.entrypoints.openai.api_server`.
    pub fn vllm_flags(&self, gpu_count: u32) -> Vec<String> {
        let mut flags = vec!["--model".into(), self.name.clone()];

        if let Some(flag_value) = self.tool_call_parser.as_flag() {
            flags.push("--tool-call-parser".into());
            flags.push(flag_value.into());
            // Enable tool calling when a parser is set.
            flags.push("--enable-auto-tool-choice".into());
        }

        let tp = self
            .tensor_parallel_size
            .unwrap_or_else(|| gpu_count.max(1));
        if tp > 1 {
            flags.push("--tensor-parallel-size".into());
            flags.push(tp.to_string());
        }

        if self.uses_responses_api {
            flags.push("--enable-responses-api".into());
        }

        for f in &self.extra_vllm_flags {
            flags.push(f.clone());
        }

        flags
    }
}

// ---------------------------------------------------------------------------
// Known models catalogue
// ---------------------------------------------------------------------------

/// Return a catalogue of well-known model configurations.
pub fn known_models() -> Vec<ModelConfig> {
    vec![
        ModelConfig {
            name: "Qwen/Qwen3-Coder-32B-Instruct".into(),
            family: ModelFamily::Qwen3Coder,
            min_vram_gb: 64,
            tool_call_parser: ToolCallParser::Qwen3Coder,
            extra_vllm_flags: vec!["--max-model-len".into(), "32768".into()],
            tensor_parallel_size: None,
            uses_responses_api: false,
        },
        ModelConfig {
            name: "THUDM/GLM-4-MoE-9B".into(),
            family: ModelFamily::Glm4Moe,
            min_vram_gb: 20,
            tool_call_parser: ToolCallParser::Glm4Moe,
            extra_vllm_flags: vec![],
            tensor_parallel_size: None,
            uses_responses_api: false,
        },
        ModelConfig {
            name: "mistralai/Mistral-7B-Instruct-v0.3".into(),
            family: ModelFamily::Mistral,
            min_vram_gb: 14,
            tool_call_parser: ToolCallParser::Hermes,
            extra_vllm_flags: vec![],
            tensor_parallel_size: None,
            uses_responses_api: false,
        },
        ModelConfig {
            name: "meta-llama/Meta-Llama-3-8B-Instruct".into(),
            family: ModelFamily::Llama,
            min_vram_gb: 16,
            tool_call_parser: ToolCallParser::Hermes,
            extra_vllm_flags: vec![],
            tensor_parallel_size: None,
            uses_responses_api: false,
        },
        ModelConfig {
            name: "openai/gpt-oss-mini".into(),
            family: ModelFamily::GptOss,
            min_vram_gb: 24,
            tool_call_parser: ToolCallParser::OpenAIResponses,
            extra_vllm_flags: vec![],
            tensor_parallel_size: None,
            uses_responses_api: true,
        },
        ModelConfig {
            name: "Qwen/Qwen2.5-72B-Instruct".into(),
            family: ModelFamily::Qwen,
            min_vram_gb: 140,
            tool_call_parser: ToolCallParser::Hermes,
            extra_vllm_flags: vec!["--max-model-len".into(), "65536".into()],
            tensor_parallel_size: None,
            uses_responses_api: false,
        },
    ]
}

// ---------------------------------------------------------------------------
// Build variants
// ---------------------------------------------------------------------------

/// vLLM Docker image variant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VllmBuild {
    /// Stable PyPI release — `vllm/vllm-openai:latest`.
    Release,
    /// Latest nightly from the CI registry.
    Nightly,
    /// Special build supporting `/v1/responses` (GPT-OSS compatibility).
    GptOss,
}

impl VllmBuild {
    /// Docker image tag for this build.
    pub fn docker_image(&self) -> &str {
        match self {
            VllmBuild::Release => "vllm/vllm-openai:latest",
            VllmBuild::Nightly => "vllm/vllm-openai:nightly",
            VllmBuild::GptOss => "vllm/vllm-openai:gpt-oss",
        }
    }

    /// True when this build supports `/v1/responses`.
    pub fn supports_responses_api(&self) -> bool {
        matches!(self, VllmBuild::GptOss)
    }
}

// ---------------------------------------------------------------------------
// Pod spec and preflight
// ---------------------------------------------------------------------------

/// Full specification of a vLLM pod deployment.
#[derive(Debug, Clone)]
pub struct PodSpec {
    pub gpu_tier: GpuTier,
    /// Number of GPU devices to use.
    pub gpu_count: u32,
    pub model: ModelConfig,
    pub build: VllmBuild,
    /// Port vLLM will listen on inside the container.
    pub port: u16,
    pub api_key: Option<String>,
    /// Host path to mount as the models directory.
    pub models_path: Option<String>,
    pub extra_flags: Vec<String>,
}

/// Result of pre-deployment VRAM validation.
#[derive(Debug, Clone)]
pub struct PreflightResult {
    pub passed: bool,
    pub vram_required_gb: u32,
    pub vram_available_gb: u32,
    pub tensor_parallel_size: u32,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

impl PreflightResult {
    /// True when preflight passed with no errors.
    pub fn is_ok(&self) -> bool {
        self.passed && self.errors.is_empty()
    }

    /// One-line human-readable summary.
    pub fn summary(&self) -> String {
        if self.is_ok() {
            format!(
                "Preflight OK — {}/{} GB VRAM, TP={}{}",
                self.vram_required_gb,
                self.vram_available_gb,
                self.tensor_parallel_size,
                if self.warnings.is_empty() {
                    String::new()
                } else {
                    format!(", {} warning(s)", self.warnings.len())
                }
            )
        } else {
            format!(
                "Preflight FAILED — {}/{} GB VRAM: {}",
                self.vram_required_gb,
                self.vram_available_gb,
                self.errors.join("; ")
            )
        }
    }
}

// ---------------------------------------------------------------------------
// PodManager
// ---------------------------------------------------------------------------

/// Stateless helper for pod deployment operations.
pub struct PodManager;

impl PodManager {
    /// Create a new `PodManager`.
    pub fn new() -> Self {
        PodManager
    }

    /// Validate VRAM and GPU count before deployment.
    ///
    /// Rules:
    /// - Total VRAM across all GPUs must cover `model.min_vram_gb` + 20% overhead.
    /// - The build must support the responses API when the model requires it.
    pub fn preflight(&self, spec: &PodSpec) -> PreflightResult {
        let vram_per_gpu = spec.gpu_tier.vram_gb();
        let total_vram = vram_per_gpu * spec.gpu_count;
        // Add 20 % overhead for KV cache and CUDA context.
        let vram_required = (spec.model.min_vram_gb as f64 * 1.20).ceil() as u32;
        let tp = Self::auto_tensor_parallel(&spec.model, spec.gpu_count, vram_per_gpu);

        let mut warnings = Vec::new();
        let mut errors = Vec::new();

        if total_vram < vram_required {
            errors.push(format!(
                "Insufficient VRAM: {} GB available across {} GPU(s) but {} GB required (model {} GB + 20% overhead)",
                total_vram,
                spec.gpu_count,
                vram_required,
                spec.model.min_vram_gb,
            ));
        }

        if spec.model.uses_responses_api && !spec.build.supports_responses_api() {
            errors.push(format!(
                "Model '{}' requires the responses API but build variant '{}' does not support it; use VllmBuild::GptOss",
                spec.model.name,
                spec.build.docker_image(),
            ));
        }

        if total_vram > vram_required * 2 {
            warnings.push(format!(
                "GPU VRAM utilisation below 50% ({}/{} GB) — consider a smaller GPU tier to reduce cost",
                vram_required,
                total_vram,
            ));
        }

        if spec.gpu_count > 1 && spec.model.tensor_parallel_size.is_none() {
            warnings.push(format!(
                "tensor_parallel_size not set explicitly; auto-selected TP={}",
                tp
            ));
        }

        let passed = errors.is_empty();
        PreflightResult {
            passed,
            vram_required_gb: vram_required,
            vram_available_gb: total_vram,
            tensor_parallel_size: tp,
            warnings,
            errors,
        }
    }

    /// Generate the complete `docker run` launch command for the pod spec.
    pub fn build_launch_command(&self, spec: &PodSpec) -> Vec<String> {
        let mut cmd = vec!["docker".into(), "run".into(), "--gpus".into(), "all".into()];

        cmd.push("-p".into());
        cmd.push(format!("{}:{}", spec.port, spec.port));

        if let Some(ref key) = spec.api_key {
            cmd.push("-e".into());
            cmd.push(format!("VLLM_API_KEY={}", key));
        }

        if let Some(ref models_path) = spec.models_path {
            cmd.push("-v".into());
            cmd.push(format!("{}:/workspace/models", models_path));
        }

        cmd.push(spec.build.docker_image().into());

        // Append vLLM server flags.
        let tp = Self::auto_tensor_parallel(&spec.model, spec.gpu_count, spec.gpu_tier.vram_gb());
        let model_tp = ModelConfig {
            tensor_parallel_size: Some(tp),
            ..spec.model.clone()
        };
        cmd.extend(model_tp.vllm_flags(spec.gpu_count));

        cmd.push("--port".into());
        cmd.push(spec.port.to_string());

        for f in &spec.extra_flags {
            cmd.push(f.clone());
        }

        cmd
    }

    /// Auto-calculate tensor parallel size.
    ///
    /// Returns the smallest power-of-two number of GPUs that provides enough
    /// total VRAM (model VRAM + 20% overhead) across `gpu_count` GPUs.
    pub fn auto_tensor_parallel(model: &ModelConfig, gpu_count: u32, vram_per_gpu: u32) -> u32 {
        if gpu_count <= 1 {
            return 1;
        }
        let required = model.min_vram_gb;
        let mut tp = 1u32;
        while tp < gpu_count && tp * vram_per_gpu < required {
            tp *= 2;
        }
        tp.min(gpu_count)
    }

    /// Parse a volume-mount string `"host_path:/workspace/models"` and return
    /// the host path component.
    pub fn extract_models_path(mount_arg: &str) -> Option<String> {
        let parts: Vec<&str> = mount_arg.splitn(2, ':').collect();
        if !parts.is_empty() && !parts[0].is_empty() {
            Some(parts[0].to_string())
        } else {
            Option::None
        }
    }
}

impl Default for PodManager {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Multi-GPU assignment
// ---------------------------------------------------------------------------

/// Assignment of a single model to a contiguous range of CUDA device indices.
#[derive(Debug, Clone)]
pub struct GpuAssignment {
    pub model_name: String,
    pub gpu_indices: Vec<u32>,
}

/// Greedily assign multiple models to GPUs without overlap.
///
/// Each model is given a contiguous slice of CUDA device indices large enough
/// to hold its VRAM requirement across `vram_per_gpu` GB GPUs.  Returns an
/// error when the total GPU demand exceeds `total_gpus`.
pub fn assign_gpus(
    models: &[ModelConfig],
    total_gpus: u32,
    vram_per_gpu: u32,
) -> Result<Vec<GpuAssignment>, String> {
    let mut assignments = Vec::with_capacity(models.len());
    let mut next_index: u32 = 0;

    for model in models {
        // Minimum GPUs needed for this model (ceil division).
        let vram_with_overhead = (model.min_vram_gb as f64 * 1.20).ceil() as u32;
        let gpus_needed = vram_with_overhead.div_ceil(vram_per_gpu).max(1);

        if next_index + gpus_needed > total_gpus {
            return Err(format!(
                "Not enough GPUs to assign model '{}': need {} more GPU(s) but only {} of {} remain",
                model.name,
                gpus_needed,
                total_gpus.saturating_sub(next_index),
                total_gpus,
            ));
        }

        let gpu_indices: Vec<u32> = (next_index..next_index + gpus_needed).collect();
        next_index += gpus_needed;

        assignments.push(GpuAssignment {
            model_name: model.name.clone(),
            gpu_indices,
        });
    }

    Ok(assignments)
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    // --- GpuTier::can_fit_model ---

    #[test]
    fn can_fit_model_true_when_vram_sufficient() {
        let gpu = GpuTier::A100_80GB;
        let model = ModelConfig {
            name: "test".into(),
            family: ModelFamily::Llama,
            min_vram_gb: 16,
            tool_call_parser: ToolCallParser::Hermes,
            extra_vllm_flags: vec![],
            tensor_parallel_size: None,
            uses_responses_api: false,
        };
        assert!(gpu.can_fit_model(&model));
    }

    #[test]
    fn can_fit_model_false_when_vram_insufficient() {
        let gpu = GpuTier::T4_16GB;
        let model = ModelConfig {
            name: "big-model".into(),
            family: ModelFamily::Qwen,
            min_vram_gb: 64,
            tool_call_parser: ToolCallParser::Hermes,
            extra_vllm_flags: vec![],
            tensor_parallel_size: None,
            uses_responses_api: false,
        };
        assert!(!gpu.can_fit_model(&model));
    }

    #[test]
    fn can_fit_model_exact_boundary() {
        let gpu = GpuTier::A10_24GB;
        let model = ModelConfig {
            name: "exact".into(),
            family: ModelFamily::Mistral,
            min_vram_gb: 24,
            tool_call_parser: ToolCallParser::Hermes,
            extra_vllm_flags: vec![],
            tensor_parallel_size: None,
            uses_responses_api: false,
        };
        assert!(gpu.can_fit_model(&model));
    }

    // --- ToolCallParser::from_model_family ---

    #[test]
    fn parser_qwen3coder_family() {
        assert_eq!(
            ToolCallParser::from_model_family(&ModelFamily::Qwen3Coder),
            ToolCallParser::Qwen3Coder
        );
    }

    #[test]
    fn parser_glm4moe_family() {
        assert_eq!(
            ToolCallParser::from_model_family(&ModelFamily::Glm4Moe),
            ToolCallParser::Glm4Moe
        );
    }

    #[test]
    fn parser_gptoss_family() {
        assert_eq!(
            ToolCallParser::from_model_family(&ModelFamily::GptOss),
            ToolCallParser::OpenAIResponses
        );
    }

    #[test]
    fn parser_qwen_family() {
        assert_eq!(
            ToolCallParser::from_model_family(&ModelFamily::Qwen),
            ToolCallParser::Hermes
        );
    }

    #[test]
    fn parser_mistral_family() {
        assert_eq!(
            ToolCallParser::from_model_family(&ModelFamily::Mistral),
            ToolCallParser::Hermes
        );
    }

    #[test]
    fn parser_llama_family() {
        assert_eq!(
            ToolCallParser::from_model_family(&ModelFamily::Llama),
            ToolCallParser::Hermes
        );
    }

    #[test]
    fn parser_other_family() {
        assert_eq!(
            ToolCallParser::from_model_family(&ModelFamily::Other("falcon".into())),
            ToolCallParser::Auto
        );
    }

    // --- ModelFamily::from_model_name ---

    #[test]
    fn family_from_name_qwen3_coder() {
        assert_eq!(
            ModelFamily::from_model_name("Qwen/Qwen3-Coder-32B-Instruct"),
            ModelFamily::Qwen3Coder
        );
    }

    #[test]
    fn family_from_name_glm4moe() {
        assert_eq!(
            ModelFamily::from_model_name("THUDM/GLM-4-MoE-9B"),
            ModelFamily::Glm4Moe
        );
    }

    #[test]
    fn family_from_name_qwen() {
        assert_eq!(
            ModelFamily::from_model_name("Qwen/Qwen2.5-72B-Instruct"),
            ModelFamily::Qwen
        );
    }

    #[test]
    fn family_from_name_mistral() {
        assert_eq!(
            ModelFamily::from_model_name("mistralai/Mistral-7B-Instruct-v0.3"),
            ModelFamily::Mistral
        );
    }

    #[test]
    fn family_from_name_llama() {
        assert_eq!(
            ModelFamily::from_model_name("meta-llama/Meta-Llama-3-8B-Instruct"),
            ModelFamily::Llama
        );
    }

    #[test]
    fn family_from_name_gptoss() {
        assert_eq!(
            ModelFamily::from_model_name("openai/gpt-oss-mini"),
            ModelFamily::GptOss
        );
    }

    #[test]
    fn family_from_name_unknown() {
        assert_eq!(
            ModelFamily::from_model_name("microsoft/phi-3"),
            ModelFamily::Other("microsoft/phi-3".into())
        );
    }

    // --- ModelConfig::vllm_flags ---

    #[test]
    fn vllm_flags_includes_parser_flag() {
        let model = ModelConfig::for_model("Qwen/Qwen3-Coder-32B-Instruct");
        let flags = model.vllm_flags(1);
        let joined = flags.join(" ");
        assert!(
            joined.contains("--tool-call-parser"),
            "expected --tool-call-parser in: {}",
            joined
        );
        assert!(
            joined.contains("qwen3-coder"),
            "expected qwen3-coder parser in: {}",
            joined
        );
    }

    #[test]
    fn vllm_flags_no_parser_for_none() {
        let model = ModelConfig {
            name: "test".into(),
            family: ModelFamily::Other("test".into()),
            min_vram_gb: 8,
            tool_call_parser: ToolCallParser::None,
            extra_vllm_flags: vec![],
            tensor_parallel_size: None,
            uses_responses_api: false,
        };
        let flags = model.vllm_flags(1);
        assert!(!flags.contains(&"--tool-call-parser".into()));
    }

    #[test]
    fn vllm_flags_includes_tensor_parallel_for_multi_gpu() {
        let model = ModelConfig {
            name: "test".into(),
            family: ModelFamily::Llama,
            min_vram_gb: 16,
            tool_call_parser: ToolCallParser::Hermes,
            extra_vllm_flags: vec![],
            tensor_parallel_size: Some(4),
            uses_responses_api: false,
        };
        let flags = model.vllm_flags(4);
        let joined = flags.join(" ");
        assert!(
            joined.contains("--tensor-parallel-size 4"),
            "expected TP=4 in: {}",
            joined
        );
    }

    #[test]
    fn vllm_flags_responses_api_flag() {
        let model = ModelConfig::for_model("openai/gpt-oss-mini");
        let flags = model.vllm_flags(1);
        assert!(
            flags.contains(&"--enable-responses-api".into()),
            "expected --enable-responses-api"
        );
    }

    // --- PodManager::preflight ---

    #[test]
    fn preflight_passes_with_sufficient_vram() {
        let pm = PodManager::new();
        let model = ModelConfig::for_model("mistralai/Mistral-7B-Instruct-v0.3");
        // A10 24 GB > 14 GB * 1.2 = 16.8 GB
        let spec = PodSpec {
            gpu_tier: GpuTier::A10_24GB,
            gpu_count: 1,
            model,
            build: VllmBuild::Release,
            port: 8000,
            api_key: None,
            models_path: None,
            extra_flags: vec![],
        };
        let result = pm.preflight(&spec);
        assert!(result.is_ok(), "preflight should pass: {:?}", result.errors);
    }

    #[test]
    fn preflight_fails_with_insufficient_vram() {
        let pm = PodManager::new();
        let model = ModelConfig::for_model("Qwen/Qwen2.5-72B-Instruct"); // needs 140 GB
        // T4 16 GB total — nowhere near enough
        let spec = PodSpec {
            gpu_tier: GpuTier::T4_16GB,
            gpu_count: 1,
            model,
            build: VllmBuild::Release,
            port: 8000,
            api_key: None,
            models_path: None,
            extra_flags: vec![],
        };
        let result = pm.preflight(&spec);
        assert!(!result.is_ok(), "preflight should fail for 16 GB vs 140 GB model");
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn preflight_fails_when_responses_api_not_supported() {
        let pm = PodManager::new();
        let model = ModelConfig::for_model("openai/gpt-oss-mini");
        let spec = PodSpec {
            gpu_tier: GpuTier::A10_24GB,
            gpu_count: 1,
            model,
            build: VllmBuild::Release, // wrong build
            port: 8000,
            api_key: None,
            models_path: None,
            extra_flags: vec![],
        };
        let result = pm.preflight(&spec);
        assert!(
            !result.is_ok(),
            "should fail when responses API not supported by build"
        );
    }

    // --- PodManager::build_launch_command ---

    #[test]
    fn build_launch_command_includes_docker_image() {
        let pm = PodManager::new();
        let model = ModelConfig::for_model("mistralai/Mistral-7B-Instruct-v0.3");
        let spec = PodSpec {
            gpu_tier: GpuTier::A10_24GB,
            gpu_count: 1,
            model,
            build: VllmBuild::Nightly,
            port: 8080,
            api_key: None,
            models_path: None,
            extra_flags: vec![],
        };
        let cmd = pm.build_launch_command(&spec);
        assert!(
            cmd.contains(&"vllm/vllm-openai:nightly".into()),
            "command should include nightly image: {:?}",
            cmd
        );
    }

    #[test]
    fn build_launch_command_includes_port() {
        let pm = PodManager::new();
        let model = ModelConfig::for_model("mistralai/Mistral-7B-Instruct-v0.3");
        let spec = PodSpec {
            gpu_tier: GpuTier::A10_24GB,
            gpu_count: 1,
            model,
            build: VllmBuild::Release,
            port: 9000,
            api_key: None,
            models_path: None,
            extra_flags: vec![],
        };
        let cmd = pm.build_launch_command(&spec);
        let joined = cmd.join(" ");
        assert!(joined.contains("9000"), "port 9000 should appear in command");
    }

    #[test]
    fn build_launch_command_includes_api_key_env() {
        let pm = PodManager::new();
        let model = ModelConfig::for_model("meta-llama/Meta-Llama-3-8B-Instruct");
        let spec = PodSpec {
            gpu_tier: GpuTier::A100_80GB,
            gpu_count: 1,
            model,
            build: VllmBuild::Release,
            port: 8000,
            api_key: Some("sk-test-key".into()),
            models_path: None,
            extra_flags: vec![],
        };
        let cmd = pm.build_launch_command(&spec);
        let joined = cmd.join(" ");
        assert!(
            joined.contains("VLLM_API_KEY=sk-test-key"),
            "API key should appear in command"
        );
    }

    #[test]
    fn build_launch_command_includes_volume_mount() {
        let pm = PodManager::new();
        let model = ModelConfig::for_model("meta-llama/Meta-Llama-3-8B-Instruct");
        let spec = PodSpec {
            gpu_tier: GpuTier::A100_80GB,
            gpu_count: 1,
            model,
            build: VllmBuild::Release,
            port: 8000,
            api_key: None,
            models_path: Some("/data/models".into()),
            extra_flags: vec![],
        };
        let cmd = pm.build_launch_command(&spec);
        let joined = cmd.join(" ");
        assert!(
            joined.contains("/data/models:/workspace/models"),
            "volume mount should appear in command"
        );
    }

    // --- auto_tensor_parallel ---

    #[test]
    fn auto_tensor_parallel_single_gpu() {
        let model = ModelConfig::for_model("mistralai/Mistral-7B-Instruct-v0.3");
        assert_eq!(PodManager::auto_tensor_parallel(&model, 1, 24), 1);
    }

    #[test]
    fn auto_tensor_parallel_two_gpus_needed() {
        // model needs 140 GB, each GPU has 80 GB → 2 GPUs needed
        let model = ModelConfig::for_model("Qwen/Qwen2.5-72B-Instruct");
        let tp = PodManager::auto_tensor_parallel(&model, 4, 80);
        assert_eq!(tp, 2, "should need 2 GPUs for 140 GB model on 80 GB GPUs");
    }

    #[test]
    fn auto_tensor_parallel_capped_at_gpu_count() {
        let model = ModelConfig {
            name: "huge".into(),
            family: ModelFamily::Llama,
            min_vram_gb: 500,
            tool_call_parser: ToolCallParser::Hermes,
            extra_vllm_flags: vec![],
            tensor_parallel_size: None,
            uses_responses_api: false,
        };
        // Only 4 GPUs available even though model wants more
        let tp = PodManager::auto_tensor_parallel(&model, 4, 80);
        assert_eq!(tp, 4, "TP should be capped at gpu_count=4");
    }

    // --- assign_gpus ---

    #[test]
    fn assign_gpus_no_overlap() {
        let models = vec![
            ModelConfig::for_model("mistralai/Mistral-7B-Instruct-v0.3"), // 14 GB -> 1 A10
            ModelConfig::for_model("meta-llama/Meta-Llama-3-8B-Instruct"), // 16 GB -> 1 A10
        ];
        let result = assign_gpus(&models, 4, 24).unwrap();
        assert_eq!(result.len(), 2);
        // Indices must not overlap
        let all_indices: Vec<u32> = result.iter().flat_map(|a| a.gpu_indices.clone()).collect();
        let unique: std::collections::HashSet<u32> = all_indices.iter().cloned().collect();
        assert_eq!(
            all_indices.len(),
            unique.len(),
            "GPU indices must not overlap"
        );
    }

    #[test]
    fn assign_gpus_returns_error_when_insufficient() {
        let models = vec![
            ModelConfig::for_model("Qwen/Qwen2.5-72B-Instruct"), // 140 GB -> 2 A100s
            ModelConfig::for_model("Qwen/Qwen2.5-72B-Instruct"), // another 2 A100s
        ];
        // Only 2 GPUs total — not enough for both
        let result = assign_gpus(&models, 2, 80);
        assert!(result.is_err(), "should fail when not enough GPUs");
    }

    #[test]
    fn assign_gpus_correct_cuda_indices() {
        let models = vec![ModelConfig::for_model("mistralai/Mistral-7B-Instruct-v0.3")];
        let result = assign_gpus(&models, 2, 24).unwrap();
        assert_eq!(result[0].gpu_indices, vec![0]);
    }

    // --- extract_models_path ---

    #[test]
    fn extract_models_path_parses_colon_form() {
        let path = PodManager::extract_models_path("/data/models:/workspace/models");
        assert_eq!(path, Some("/data/models".into()));
    }

    #[test]
    fn extract_models_path_plain_path() {
        let path = PodManager::extract_models_path("/data/models");
        assert_eq!(path, Some("/data/models".into()));
    }

    #[test]
    fn extract_models_path_empty_returns_none() {
        let path = PodManager::extract_models_path("");
        assert_eq!(path, Option::None);
    }

    // --- GpuTier::from_str ---

    #[test]
    fn gpu_tier_from_str_h100() {
        assert_eq!(GpuTier::from_str("h100"), Some(GpuTier::H100_80GB));
    }

    #[test]
    fn gpu_tier_from_str_unknown() {
        assert_eq!(GpuTier::from_str("rtx4090"), Option::None);
    }

    // --- VllmBuild ---

    #[test]
    fn vllm_build_docker_images() {
        assert_eq!(VllmBuild::Release.docker_image(), "vllm/vllm-openai:latest");
        assert_eq!(VllmBuild::Nightly.docker_image(), "vllm/vllm-openai:nightly");
        assert_eq!(VllmBuild::GptOss.docker_image(), "vllm/vllm-openai:gpt-oss");
    }

    #[test]
    fn vllm_build_responses_api_support() {
        assert!(!VllmBuild::Release.supports_responses_api());
        assert!(!VllmBuild::Nightly.supports_responses_api());
        assert!(VllmBuild::GptOss.supports_responses_api());
    }

    // --- known_models ---

    #[test]
    fn known_models_contains_expected_entries() {
        let models = known_models();
        let names: Vec<&str> = models.iter().map(|m| m.name.as_str()).collect();
        assert!(names.contains(&"Qwen/Qwen3-Coder-32B-Instruct"));
        assert!(names.contains(&"THUDM/GLM-4-MoE-9B"));
        assert!(names.contains(&"mistralai/Mistral-7B-Instruct-v0.3"));
        assert!(names.contains(&"meta-llama/Meta-Llama-3-8B-Instruct"));
        assert!(names.contains(&"openai/gpt-oss-mini"));
    }

    // --- PreflightResult::summary ---

    #[test]
    fn preflight_summary_pass() {
        let r = PreflightResult {
            passed: true,
            vram_required_gb: 17,
            vram_available_gb: 24,
            tensor_parallel_size: 1,
            warnings: vec![],
            errors: vec![],
        };
        assert!(r.summary().contains("OK"));
    }

    #[test]
    fn preflight_summary_fail() {
        let r = PreflightResult {
            passed: false,
            vram_required_gb: 168,
            vram_available_gb: 16,
            tensor_parallel_size: 1,
            warnings: vec![],
            errors: vec!["Insufficient VRAM".into()],
        };
        assert!(r.summary().contains("FAILED"));
    }
}
