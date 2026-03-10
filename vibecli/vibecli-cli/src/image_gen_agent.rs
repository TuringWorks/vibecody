#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// --- Enums ---

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ImageStyle {
    Photorealistic,
    Cartoon,
    Sketch,
    Watercolor,
    OilPainting,
    PixelArt,
    Anime,
    Abstract,
    Minimalist,
    Technical,
}

impl ImageStyle {
    fn prompt_modifier(&self) -> &str {
        match self {
            Self::Photorealistic => "photorealistic, ultra-detailed, 8k resolution, professional photography",
            Self::Cartoon => "cartoon style, bold outlines, vibrant colors, cel-shaded",
            Self::Sketch => "pencil sketch, hand-drawn, fine lines, crosshatching",
            Self::Watercolor => "watercolor painting, soft edges, color blending, artistic",
            Self::OilPainting => "oil painting, thick brushstrokes, rich textures, classical art",
            Self::PixelArt => "pixel art, retro style, limited palette, crisp edges",
            Self::Anime => "anime style, large expressive eyes, vibrant, Japanese animation",
            Self::Abstract => "abstract art, non-representational, bold shapes, experimental",
            Self::Minimalist => "minimalist, clean lines, simple composition, negative space",
            Self::Technical => "technical illustration, precise, labeled, blueprint style",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ImageFormat {
    Png,
    Jpeg,
    Svg,
    Webp,
}

impl ImageFormat {
    fn extension(&self) -> &str {
        match self {
            Self::Png => "png",
            Self::Jpeg => "jpg",
            Self::Svg => "svg",
            Self::Webp => "webp",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum GenerationModel {
    DallE3,
    StableDiffusion,
    Midjourney,
    Flux,
    LocalModel,
}

impl GenerationModel {
    fn cost_per_image(&self, width: u32, height: u32) -> f64 {
        let pixels = (width as f64) * (height as f64);
        let base_pixels = 1024.0 * 1024.0;
        let size_factor = pixels / base_pixels;

        match self {
            Self::DallE3 => 0.04 * size_factor.max(1.0),
            Self::StableDiffusion => 0.002 * size_factor.max(1.0),
            Self::Midjourney => 0.05 * size_factor.max(1.0),
            Self::Flux => 0.03 * size_factor.max(1.0),
            Self::LocalModel => 0.0,
        }
    }

    fn name(&self) -> &str {
        match self {
            Self::DallE3 => "dall-e-3",
            Self::StableDiffusion => "stable-diffusion-xl",
            Self::Midjourney => "midjourney-v6",
            Self::Flux => "flux-1.1-pro",
            Self::LocalModel => "local-model",
        }
    }
}

// --- Data structs ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageRequest {
    pub prompt: String,
    pub style: ImageStyle,
    pub width: u32,
    pub height: u32,
    pub format: ImageFormat,
    pub negative_prompt: Option<String>,
    pub seed: Option<u64>,
}

impl ImageRequest {
    pub fn new(prompt: impl Into<String>, style: ImageStyle) -> Self {
        Self {
            prompt: prompt.into(),
            style,
            width: 1024,
            height: 1024,
            format: ImageFormat::Png,
            negative_prompt: None,
            seed: None,
        }
    }

    pub fn with_dimensions(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    pub fn with_format(mut self, format: ImageFormat) -> Self {
        self.format = format;
        self
    }

    pub fn with_negative_prompt(mut self, negative: impl Into<String>) -> Self {
        self.negative_prompt = Some(negative.into());
        self
    }

    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = Some(seed);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageResult {
    pub id: String,
    pub prompt: String,
    pub image_path: String,
    pub width: u32,
    pub height: u32,
    pub format: ImageFormat,
    pub generation_time_ms: u64,
    pub model_used: String,
}

// --- PromptBuilder ---

#[derive(Debug, Clone)]
pub struct PromptBuilder {
    description: String,
    style: Option<ImageStyle>,
    lighting: Option<String>,
    composition: Option<String>,
    negatives: Vec<String>,
    extras: Vec<String>,
}

impl PromptBuilder {
    pub fn new(description: impl Into<String>) -> Self {
        Self {
            description: description.into(),
            style: None,
            lighting: None,
            composition: None,
            negatives: Vec::new(),
            extras: Vec::new(),
        }
    }

    pub fn add_style(mut self, style: ImageStyle) -> Self {
        self.style = Some(style);
        self
    }

    pub fn add_lighting(mut self, lighting: impl Into<String>) -> Self {
        self.lighting = Some(lighting.into());
        self
    }

    pub fn add_composition(mut self, composition: impl Into<String>) -> Self {
        self.composition = Some(composition.into());
        self
    }

    pub fn add_negative(mut self, negative: impl Into<String>) -> Self {
        self.negatives.push(negative.into());
        self
    }

    pub fn add_extra(mut self, extra: impl Into<String>) -> Self {
        self.extras.push(extra.into());
        self
    }

    pub fn build(&self) -> String {
        let mut parts = vec![self.description.clone()];

        if let Some(ref style) = self.style {
            parts.push(style.prompt_modifier().to_string());
        }
        if let Some(ref lighting) = self.lighting {
            parts.push(format!("lighting: {lighting}"));
        }
        if let Some(ref composition) = self.composition {
            parts.push(format!("composition: {composition}"));
        }
        for extra in &self.extras {
            parts.push(extra.clone());
        }

        let mut prompt = parts.join(", ");

        if !self.negatives.is_empty() {
            prompt.push_str(" | negative: ");
            prompt.push_str(&self.negatives.join(", "));
        }

        prompt
    }
}

// --- BatchGenerator ---

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BatchStatus {
    Queued,
    InProgress,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchEntry {
    pub request: ImageRequest,
    pub status: BatchStatus,
    pub result: Option<ImageResult>,
}

#[derive(Debug)]
pub struct BatchGenerator {
    entries: Vec<BatchEntry>,
    model: GenerationModel,
}

impl BatchGenerator {
    pub fn new(model: GenerationModel) -> Self {
        Self {
            entries: Vec::new(),
            model,
        }
    }

    pub fn add(&mut self, request: ImageRequest) {
        self.entries.push(BatchEntry {
            request,
            status: BatchStatus::Queued,
            result: None,
        });
    }

    pub fn queued_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|e| e.status == BatchStatus::Queued)
            .count()
    }

    pub fn completed_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|e| e.status == BatchStatus::Completed)
            .count()
    }

    pub fn total_count(&self) -> usize {
        self.entries.len()
    }

    pub fn estimated_total_cost(&self) -> f64 {
        self.entries
            .iter()
            .map(|e| {
                self.model
                    .cost_per_image(e.request.width, e.request.height)
            })
            .sum()
    }

    /// Simulate running all queued entries through the given agent.
    pub fn run_all(&mut self, agent: &mut ImageGenAgent) -> Vec<ImageResult> {
        let mut results = Vec::new();
        for entry in &mut self.entries {
            if entry.status != BatchStatus::Queued {
                continue;
            }
            entry.status = BatchStatus::InProgress;
            let result = agent.generate(&entry.request);
            entry.result = Some(result.clone());
            entry.status = BatchStatus::Completed;
            results.push(result);
        }
        results
    }

    pub fn entries(&self) -> &[BatchEntry] {
        &self.entries
    }
}

// --- ImageGenAgent ---

pub struct ImageGenAgent {
    model: GenerationModel,
    generated: Vec<ImageResult>,
    next_id: u64,
    default_width: u32,
    default_height: u32,
    output_dir: String,
    config: HashMap<String, String>,
}

impl ImageGenAgent {
    pub fn new() -> Self {
        let mut config = HashMap::new();
        config.insert("quality".to_string(), "high".to_string());
        config.insert("guidance_scale".to_string(), "7.5".to_string());
        config.insert("steps".to_string(), "50".to_string());

        Self {
            model: GenerationModel::StableDiffusion,
            generated: Vec::new(),
            next_id: 1,
            default_width: 1024,
            default_height: 1024,
            output_dir: "generated_images".to_string(),
            config,
        }
    }

    pub fn with_model(mut self, model: GenerationModel) -> Self {
        self.model = model;
        self
    }

    pub fn with_output_dir(mut self, dir: impl Into<String>) -> Self {
        self.output_dir = dir.into();
        self
    }

    /// Generate an image from a request, returning placeholder metadata.
    pub fn generate(&mut self, request: &ImageRequest) -> ImageResult {
        let id = format!("img_{:06}", self.next_id);
        self.next_id += 1;

        let full_prompt = self.build_prompt(&request.prompt, &request.style);

        let filename = format!("{}.{}", id, request.format.extension());
        let image_path = format!("{}/{}", self.output_dir, filename);

        // Simulate generation time based on resolution
        let pixels = (request.width as u64) * (request.height as u64);
        let base_time_ms = match self.model {
            GenerationModel::DallE3 => 8000,
            GenerationModel::StableDiffusion => 3000,
            GenerationModel::Midjourney => 12000,
            GenerationModel::Flux => 5000,
            GenerationModel::LocalModel => 15000,
        };
        let scale = (pixels as f64 / (1024.0 * 1024.0)).max(1.0);
        let generation_time_ms = (base_time_ms as f64 * scale) as u64;

        let result = ImageResult {
            id: id.clone(),
            prompt: full_prompt,
            image_path,
            width: request.width,
            height: request.height,
            format: request.format.clone(),
            generation_time_ms,
            model_used: self.model.name().to_string(),
        };

        self.generated.push(result.clone());
        result
    }

    /// Generate variations of a previously generated image.
    pub fn generate_variations(
        &mut self,
        image_id: &str,
        count: usize,
    ) -> Vec<ImageResult> {
        let source = self.generated.iter().find(|r| r.id == image_id).cloned();
        let Some(source) = source else {
            return Vec::new();
        };

        (0..count)
            .map(|i| {
                let request = ImageRequest {
                    prompt: format!("{} (variation {})", source.prompt, i + 1),
                    style: ImageStyle::Photorealistic,
                    width: source.width,
                    height: source.height,
                    format: source.format.clone(),
                    negative_prompt: None,
                    seed: Some((i as u64) * 42 + 7),
                };
                self.generate(&request)
            })
            .collect()
    }

    /// Upscale a previously generated image by a scale factor.
    pub fn upscale(&mut self, image_id: &str, scale_factor: f32) -> Option<ImageResult> {
        let source = self.generated.iter().find(|r| r.id == image_id).cloned();
        let source = source?;

        let new_width = (source.width as f32 * scale_factor) as u32;
        let new_height = (source.height as f32 * scale_factor) as u32;

        let request = ImageRequest {
            prompt: format!("{} (upscaled {}x)", source.prompt, scale_factor),
            style: ImageStyle::Photorealistic,
            width: new_width,
            height: new_height,
            format: source.format.clone(),
            negative_prompt: None,
            seed: None,
        };

        Some(self.generate(&request))
    }

    /// List all previously generated images.
    pub fn list_generated(&self) -> Vec<ImageResult> {
        self.generated.clone()
    }

    /// Build a detailed prompt from a description and style.
    pub fn build_prompt(&self, description: &str, style: &ImageStyle) -> String {
        let quality = self.config.get("quality").map(|s| s.as_str()).unwrap_or("high");
        let quality_suffix = match quality {
            "ultra" => ", masterpiece, best quality, extremely detailed",
            "high" => ", high quality, detailed",
            "medium" => ", good quality",
            _ => "",
        };

        format!(
            "{}, {}{}",
            description,
            style.prompt_modifier(),
            quality_suffix
        )
    }

    /// Estimate cost for a given request based on the current model.
    pub fn estimate_cost(&self, request: &ImageRequest) -> f64 {
        self.model.cost_per_image(request.width, request.height)
    }

    pub fn model(&self) -> &GenerationModel {
        &self.model
    }

    pub fn generated_count(&self) -> usize {
        self.generated.len()
    }
}

impl Default for ImageGenAgent {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- ImageRequest tests ---

    #[test]
    fn test_image_request_new_defaults() {
        let req = ImageRequest::new("a cat", ImageStyle::Cartoon);
        assert_eq!(req.prompt, "a cat");
        assert_eq!(req.style, ImageStyle::Cartoon);
        assert_eq!(req.width, 1024);
        assert_eq!(req.height, 1024);
        assert_eq!(req.format, ImageFormat::Png);
        assert!(req.negative_prompt.is_none());
        assert!(req.seed.is_none());
    }

    #[test]
    fn test_image_request_builder_chain() {
        let req = ImageRequest::new("a dog", ImageStyle::Sketch)
            .with_dimensions(512, 768)
            .with_format(ImageFormat::Jpeg)
            .with_negative_prompt("blurry")
            .with_seed(42);

        assert_eq!(req.width, 512);
        assert_eq!(req.height, 768);
        assert_eq!(req.format, ImageFormat::Jpeg);
        assert_eq!(req.negative_prompt.as_deref(), Some("blurry"));
        assert_eq!(req.seed, Some(42));
    }

    // --- ImageGenAgent basic tests ---

    #[test]
    fn test_agent_new_defaults() {
        let agent = ImageGenAgent::new();
        assert_eq!(*agent.model(), GenerationModel::StableDiffusion);
        assert_eq!(agent.generated_count(), 0);
    }

    #[test]
    fn test_agent_with_model() {
        let agent = ImageGenAgent::new().with_model(GenerationModel::DallE3);
        assert_eq!(*agent.model(), GenerationModel::DallE3);
    }

    #[test]
    fn test_agent_generate_returns_result() {
        let mut agent = ImageGenAgent::new();
        let req = ImageRequest::new("sunset", ImageStyle::Photorealistic);
        let result = agent.generate(&req);

        assert_eq!(result.id, "img_000001");
        assert!(result.prompt.contains("sunset"));
        assert!(result.image_path.ends_with(".png"));
        assert_eq!(result.width, 1024);
        assert_eq!(result.height, 1024);
        assert_eq!(result.model_used, "stable-diffusion-xl");
    }

    #[test]
    fn test_agent_generate_increments_id() {
        let mut agent = ImageGenAgent::new();
        let req = ImageRequest::new("test", ImageStyle::Minimalist);
        let r1 = agent.generate(&req);
        let r2 = agent.generate(&req);
        assert_eq!(r1.id, "img_000001");
        assert_eq!(r2.id, "img_000002");
    }

    #[test]
    fn test_agent_generate_stores_result() {
        let mut agent = ImageGenAgent::new();
        let req = ImageRequest::new("tree", ImageStyle::Watercolor);
        agent.generate(&req);
        assert_eq!(agent.generated_count(), 1);
        assert_eq!(agent.list_generated().len(), 1);
    }

    #[test]
    fn test_agent_generate_custom_dimensions() {
        let mut agent = ImageGenAgent::new();
        let req = ImageRequest::new("landscape", ImageStyle::OilPainting)
            .with_dimensions(2048, 1024);
        let result = agent.generate(&req);
        assert_eq!(result.width, 2048);
        assert_eq!(result.height, 1024);
    }

    #[test]
    fn test_agent_generate_jpeg_format() {
        let mut agent = ImageGenAgent::new();
        let req = ImageRequest::new("portrait", ImageStyle::Anime)
            .with_format(ImageFormat::Jpeg);
        let result = agent.generate(&req);
        assert!(result.image_path.ends_with(".jpg"));
        assert_eq!(result.format, ImageFormat::Jpeg);
    }

    // --- Variations and upscale ---

    #[test]
    fn test_generate_variations() {
        let mut agent = ImageGenAgent::new();
        let req = ImageRequest::new("mountain", ImageStyle::Abstract);
        let original = agent.generate(&req);

        let variations = agent.generate_variations(&original.id, 3);
        assert_eq!(variations.len(), 3);
        assert_eq!(agent.generated_count(), 4); // original + 3 variations
    }

    #[test]
    fn test_generate_variations_unknown_id() {
        let mut agent = ImageGenAgent::new();
        let variations = agent.generate_variations("nonexistent", 2);
        assert!(variations.is_empty());
    }

    #[test]
    fn test_upscale() {
        let mut agent = ImageGenAgent::new();
        let req = ImageRequest::new("flower", ImageStyle::PixelArt);
        let original = agent.generate(&req);

        let upscaled = agent.upscale(&original.id, 2.0).expect("upscale should succeed");
        assert_eq!(upscaled.width, 2048);
        assert_eq!(upscaled.height, 2048);
    }

    #[test]
    fn test_upscale_unknown_id() {
        let mut agent = ImageGenAgent::new();
        let result = agent.upscale("nonexistent", 2.0);
        assert!(result.is_none());
    }

    // --- build_prompt ---

    #[test]
    fn test_build_prompt_includes_style() {
        let agent = ImageGenAgent::new();
        let prompt = agent.build_prompt("a castle", &ImageStyle::Sketch);
        assert!(prompt.contains("a castle"));
        assert!(prompt.contains("pencil sketch"));
    }

    #[test]
    fn test_build_prompt_includes_quality() {
        let agent = ImageGenAgent::new();
        let prompt = agent.build_prompt("robot", &ImageStyle::Technical);
        assert!(prompt.contains("high quality"));
    }

    // --- estimate_cost ---

    #[test]
    fn test_estimate_cost_stable_diffusion() {
        let agent = ImageGenAgent::new(); // default StableDiffusion
        let req = ImageRequest::new("test", ImageStyle::Photorealistic);
        let cost = agent.estimate_cost(&req);
        assert!(cost > 0.0);
        assert!(cost < 0.01); // SD is cheap
    }

    #[test]
    fn test_estimate_cost_dalle3() {
        let agent = ImageGenAgent::new().with_model(GenerationModel::DallE3);
        let req = ImageRequest::new("test", ImageStyle::Photorealistic);
        let cost = agent.estimate_cost(&req);
        assert!(cost >= 0.04);
    }

    #[test]
    fn test_estimate_cost_local_model_free() {
        let agent = ImageGenAgent::new().with_model(GenerationModel::LocalModel);
        let req = ImageRequest::new("test", ImageStyle::Minimalist);
        let cost = agent.estimate_cost(&req);
        assert!((cost - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_estimate_cost_larger_image_costs_more() {
        let agent = ImageGenAgent::new().with_model(GenerationModel::DallE3);
        let small = ImageRequest::new("t", ImageStyle::Cartoon);
        let large = ImageRequest::new("t", ImageStyle::Cartoon)
            .with_dimensions(2048, 2048);
        assert!(agent.estimate_cost(&large) > agent.estimate_cost(&small));
    }

    // --- PromptBuilder ---

    #[test]
    fn test_prompt_builder_basic() {
        let prompt = PromptBuilder::new("a red car").build();
        assert_eq!(prompt, "a red car");
    }

    #[test]
    fn test_prompt_builder_with_style() {
        let prompt = PromptBuilder::new("a red car")
            .add_style(ImageStyle::Watercolor)
            .build();
        assert!(prompt.contains("a red car"));
        assert!(prompt.contains("watercolor"));
    }

    #[test]
    fn test_prompt_builder_full() {
        let prompt = PromptBuilder::new("a knight")
            .add_style(ImageStyle::OilPainting)
            .add_lighting("dramatic side lighting")
            .add_composition("rule of thirds")
            .add_negative("blurry")
            .add_negative("low quality")
            .build();

        assert!(prompt.contains("a knight"));
        assert!(prompt.contains("oil painting"));
        assert!(prompt.contains("lighting: dramatic side lighting"));
        assert!(prompt.contains("composition: rule of thirds"));
        assert!(prompt.contains("negative: blurry, low quality"));
    }

    // --- BatchGenerator ---

    #[test]
    fn test_batch_generator_add_and_count() {
        let mut batch = BatchGenerator::new(GenerationModel::Flux);
        batch.add(ImageRequest::new("img1", ImageStyle::Cartoon));
        batch.add(ImageRequest::new("img2", ImageStyle::Sketch));

        assert_eq!(batch.total_count(), 2);
        assert_eq!(batch.queued_count(), 2);
        assert_eq!(batch.completed_count(), 0);
    }

    #[test]
    fn test_batch_generator_run_all() {
        let mut agent = ImageGenAgent::new();
        let mut batch = BatchGenerator::new(GenerationModel::StableDiffusion);
        batch.add(ImageRequest::new("cat", ImageStyle::Anime));
        batch.add(ImageRequest::new("dog", ImageStyle::PixelArt));

        let results = batch.run_all(&mut agent);
        assert_eq!(results.len(), 2);
        assert_eq!(batch.completed_count(), 2);
        assert_eq!(batch.queued_count(), 0);
        assert_eq!(agent.generated_count(), 2);
    }

    #[test]
    fn test_batch_generator_estimated_cost() {
        let mut batch = BatchGenerator::new(GenerationModel::DallE3);
        batch.add(ImageRequest::new("a", ImageStyle::Cartoon));
        batch.add(ImageRequest::new("b", ImageStyle::Cartoon));

        let cost = batch.estimated_total_cost();
        assert!(cost > 0.0);
    }

    #[test]
    fn test_batch_entries_have_results_after_run() {
        let mut agent = ImageGenAgent::new();
        let mut batch = BatchGenerator::new(GenerationModel::StableDiffusion);
        batch.add(ImageRequest::new("tree", ImageStyle::Minimalist));

        batch.run_all(&mut agent);

        let entries = batch.entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].status, BatchStatus::Completed);
        assert!(entries[0].result.is_some());
    }

    // --- Format extension ---

    #[test]
    fn test_format_extensions() {
        assert_eq!(ImageFormat::Png.extension(), "png");
        assert_eq!(ImageFormat::Jpeg.extension(), "jpg");
        assert_eq!(ImageFormat::Svg.extension(), "svg");
        assert_eq!(ImageFormat::Webp.extension(), "webp");
    }

    // --- Style coverage ---

    #[test]
    fn test_all_styles_have_prompt_modifiers() {
        let styles = [
            ImageStyle::Photorealistic,
            ImageStyle::Cartoon,
            ImageStyle::Sketch,
            ImageStyle::Watercolor,
            ImageStyle::OilPainting,
            ImageStyle::PixelArt,
            ImageStyle::Anime,
            ImageStyle::Abstract,
            ImageStyle::Minimalist,
            ImageStyle::Technical,
        ];
        for style in &styles {
            let modifier = style.prompt_modifier();
            assert!(!modifier.is_empty(), "style {:?} has empty modifier", style);
        }
    }
}
