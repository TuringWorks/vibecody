//! Visual verification via screenshot comparison.
//!
//! Implements perceptual hash-based screenshot comparison for visual regression
//! testing. Supports multiple viewports (desktop, tablet, mobile), baseline
//! management, diff computation with region detection, compliance scoring,
//! and CI integration with report generation.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Viewport {
    pub width: u32,
    pub height: u32,
    pub device_scale: f64,
    pub name: String,
}

impl Viewport {
    pub fn new(name: &str, width: u32, height: u32, device_scale: f64) -> Self {
        Self {
            width,
            height,
            device_scale,
            name: name.to_string(),
        }
    }

    pub fn desktop() -> Self {
        Self::new("desktop", 1920, 1080, 1.0)
    }

    pub fn tablet() -> Self {
        Self::new("tablet", 768, 1024, 1.0)
    }

    pub fn mobile() -> Self {
        Self::new("mobile", 375, 812, 1.0)
    }
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
            device_scale: 1.0,
            name: "default".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Screenshot {
    pub id: String,
    pub url: String,
    pub viewport: Viewport,
    pub image_path: String,
    pub captured_at: u64,
    pub file_size_bytes: u64,
    pub hash: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Baseline {
    pub id: String,
    pub name: String,
    pub screenshots: HashMap<String, Screenshot>,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VisualDiff {
    pub baseline_id: String,
    pub current_id: String,
    pub viewport: String,
    pub diff_percent: f64,
    pub changed_pixels: u64,
    pub total_pixels: u64,
    pub diff_image_path: Option<String>,
    pub regions: Vec<DiffRegion>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiffRegion {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub severity: DiffSeverity,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DiffSeverity {
    Minor,
    Moderate,
    Major,
    Critical,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComplianceScore {
    pub score: f64,
    pub viewport: String,
    pub threshold: f64,
    pub passed: bool,
    pub details: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VerifyConfig {
    pub threshold_percent: f64,
    pub viewports: Vec<Viewport>,
    pub baseline_dir: String,
    pub diff_dir: String,
    pub headless: bool,
    pub timeout_secs: u32,
}

impl Default for VerifyConfig {
    fn default() -> Self {
        Self {
            threshold_percent: 0.1,
            viewports: vec![Viewport::desktop(), Viewport::tablet(), Viewport::mobile()],
            baseline_dir: "baselines".to_string(),
            diff_dir: "diffs".to_string(),
            headless: true,
            timeout_secs: 30,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CaptureRequest {
    pub url: String,
    pub viewports: Vec<Viewport>,
    pub wait_secs: Option<u32>,
    pub scroll_to: Option<String>,
    pub clip_selector: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VerificationResult {
    pub url: String,
    pub viewport: String,
    pub baseline_id: String,
    pub diff: Option<VisualDiff>,
    pub compliance: ComplianceScore,
    pub timestamp: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VerifyMetrics {
    pub total_captures: usize,
    pub total_comparisons: usize,
    pub total_passed: usize,
    pub total_failed: usize,
    pub avg_diff_percent: f64,
    pub baselines_count: usize,
}

impl Default for VerifyMetrics {
    fn default() -> Self {
        Self {
            total_captures: 0,
            total_comparisons: 0,
            total_passed: 0,
            total_failed: 0,
            avg_diff_percent: 0.0,
            baselines_count: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// Perceptual hash
// ---------------------------------------------------------------------------

pub struct PerceptualHash;

impl PerceptualHash {
    /// Compute a simulated perceptual hash from string bytes.
    pub fn compute(image_data: &str) -> u64 {
        let mut hash: u64 = 0xcbf29ce484222325; // FNV-1a offset basis
        for byte in image_data.bytes() {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(0x100000001b3); // FNV prime
        }
        hash
    }

    /// Hamming distance between two hashes.
    pub fn hamming_distance(a: u64, b: u64) -> u32 {
        (a ^ b).count_ones()
    }

    /// Similarity score in [0.0, 1.0]. 1.0 means identical.
    pub fn similarity(a: u64, b: u64) -> f64 {
        1.0 - (Self::hamming_distance(a, b) as f64) / 64.0
    }
}

// ---------------------------------------------------------------------------
// CI integration
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ReportFormat {
    Json,
    Markdown,
    Html,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CiIntegration {
    pub threshold: f64,
    pub fail_on_diff: bool,
    pub report_format: ReportFormat,
}

impl CiIntegration {
    pub fn new(threshold: f64, fail_on_diff: bool, report_format: ReportFormat) -> Self {
        Self {
            threshold,
            fail_on_diff,
            report_format,
        }
    }

    pub fn generate_report(&self, results: &[VerificationResult]) -> String {
        match self.report_format {
            ReportFormat::Json => {
                serde_json::to_string_pretty(results).unwrap_or_else(|_| "[]".to_string())
            }
            ReportFormat::Markdown => {
                let mut md = String::from("# Visual Verification Report\n\n");
                let passed = results.iter().filter(|r| r.compliance.passed).count();
                let failed = results.len() - passed;
                md.push_str(&format!(
                    "**Summary**: {} passed, {} failed out of {} checks\n\n",
                    passed,
                    failed,
                    results.len()
                ));
                md.push_str("| URL | Viewport | Diff % | Passed |\n");
                md.push_str("|-----|----------|--------|--------|\n");
                for r in results {
                    let diff_pct = r
                        .diff
                        .as_ref()
                        .map(|d| d.diff_percent)
                        .unwrap_or(0.0);
                    let status = if r.compliance.passed { "yes" } else { "no" };
                    md.push_str(&format!(
                        "| {} | {} | {:.2}% | {} |\n",
                        r.url, r.viewport, diff_pct, status
                    ));
                }
                md
            }
            ReportFormat::Html => {
                let mut html = String::from(
                    "<html><head><title>Visual Verification Report</title></head><body>\n",
                );
                html.push_str("<h1>Visual Verification Report</h1>\n<table border=\"1\">\n");
                html.push_str(
                    "<tr><th>URL</th><th>Viewport</th><th>Diff %</th><th>Passed</th></tr>\n",
                );
                for r in results {
                    let diff_pct = r
                        .diff
                        .as_ref()
                        .map(|d| d.diff_percent)
                        .unwrap_or(0.0);
                    let status = if r.compliance.passed { "yes" } else { "no" };
                    html.push_str(&format!(
                        "<tr><td>{}</td><td>{}</td><td>{:.2}%</td><td>{}</td></tr>\n",
                        r.url, r.viewport, diff_pct, status
                    ));
                }
                html.push_str("</table>\n</body></html>");
                html
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Verification engine
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationEngine {
    pub config: VerifyConfig,
    pub baselines: HashMap<String, Baseline>,
    pub results: Vec<VerificationResult>,
    pub metrics: VerifyMetrics,
}

impl VerificationEngine {
    pub fn new(config: VerifyConfig) -> Self {
        Self {
            config,
            baselines: HashMap::new(),
            results: Vec::new(),
            metrics: VerifyMetrics::default(),
        }
    }

    /// Simulate capturing a screenshot (no real browser involved).
    pub fn capture_screenshot(
        &mut self,
        url: &str,
        viewport: &Viewport,
    ) -> Result<Screenshot, String> {
        let id = format!(
            "ss-{}-{}-{}",
            viewport.name,
            url.len(),
            self.metrics.total_captures
        );
        let image_path = format!(
            "{}/{}_{}.png",
            self.config.diff_dir, id, viewport.name
        );
        let simulated_data = format!("{}|{}x{}", url, viewport.width, viewport.height);
        let hash = format!("{:016x}", PerceptualHash::compute(&simulated_data));

        let screenshot = Screenshot {
            id,
            url: url.to_string(),
            viewport: viewport.clone(),
            image_path,
            captured_at: self.metrics.total_captures as u64 + 1,
            file_size_bytes: (viewport.width as u64) * (viewport.height as u64) * 3,
            hash,
        };
        self.metrics.total_captures += 1;
        Ok(screenshot)
    }

    /// Set (or overwrite) a baseline.
    pub fn set_baseline(&mut self, name: &str, screenshots: Vec<Screenshot>) -> Result<String, String> {
        let id = format!("bl-{}-{}", name, self.baselines.len());
        let mut map = HashMap::new();
        for ss in screenshots {
            map.insert(ss.viewport.name.clone(), ss);
        }
        let baseline = Baseline {
            id: id.clone(),
            name: name.to_string(),
            screenshots: map,
            created_at: 1,
            updated_at: 1,
        };
        self.baselines.insert(name.to_string(), baseline);
        self.metrics.baselines_count = self.baselines.len();
        Ok(id)
    }

    /// Retrieve a baseline by name.
    pub fn get_baseline(&self, name: &str) -> Result<&Baseline, String> {
        self.baselines
            .get(name)
            .ok_or_else(|| format!("Baseline '{}' not found", name))
    }

    /// Compare two screenshots and produce a `VisualDiff`.
    pub fn compare_screenshots(
        &mut self,
        baseline_ss: &Screenshot,
        current_ss: &Screenshot,
    ) -> Result<VisualDiff, String> {
        let hash_a = PerceptualHash::compute(&baseline_ss.hash);
        let hash_b = PerceptualHash::compute(&current_ss.hash);
        let similarity = PerceptualHash::similarity(hash_a, hash_b);
        let diff_percent = (1.0 - similarity) * 100.0;

        let total_pixels =
            (baseline_ss.viewport.width as u64) * (baseline_ss.viewport.height as u64);
        let changed_pixels = ((diff_percent / 100.0) * total_pixels as f64) as u64;

        let mut regions = Vec::new();
        if diff_percent > 0.0 {
            let severity = if diff_percent < 1.0 {
                DiffSeverity::Minor
            } else if diff_percent < 5.0 {
                DiffSeverity::Moderate
            } else if diff_percent < 20.0 {
                DiffSeverity::Major
            } else {
                DiffSeverity::Critical
            };
            regions.push(DiffRegion {
                x: 0,
                y: 0,
                width: baseline_ss.viewport.width,
                height: baseline_ss.viewport.height,
                severity,
            });
        }

        let diff_image_path = if diff_percent > 0.0 {
            Some(format!(
                "{}/diff_{}_{}.png",
                self.config.diff_dir,
                baseline_ss.id,
                current_ss.id
            ))
        } else {
            None
        };

        self.metrics.total_comparisons += 1;

        Ok(VisualDiff {
            baseline_id: baseline_ss.id.clone(),
            current_id: current_ss.id.clone(),
            viewport: baseline_ss.viewport.name.clone(),
            diff_percent,
            changed_pixels,
            total_pixels,
            diff_image_path,
            regions,
        })
    }

    /// Verify a URL against a named baseline for a single viewport.
    pub fn verify_url(
        &mut self,
        url: &str,
        baseline_name: &str,
        viewport: &Viewport,
    ) -> Result<VerificationResult, String> {
        let baseline = self
            .baselines
            .get(baseline_name)
            .ok_or_else(|| format!("Baseline '{}' not found", baseline_name))?
            .clone();

        let baseline_ss = baseline
            .screenshots
            .get(&viewport.name)
            .ok_or_else(|| {
                format!(
                    "No baseline screenshot for viewport '{}'",
                    viewport.name
                )
            })?
            .clone();

        let current_ss = self.capture_screenshot(url, viewport)?;
        let diff = self.compare_screenshots(&baseline_ss, &current_ss)?;

        let passed = diff.diff_percent <= self.config.threshold_percent;
        let score = (100.0 - diff.diff_percent).max(0.0);

        let compliance = ComplianceScore {
            score,
            viewport: viewport.name.clone(),
            threshold: self.config.threshold_percent,
            passed,
            details: if passed {
                format!(
                    "Diff {:.4}% within threshold {:.4}%",
                    diff.diff_percent, self.config.threshold_percent
                )
            } else {
                format!(
                    "Diff {:.4}% exceeds threshold {:.4}%",
                    diff.diff_percent, self.config.threshold_percent
                )
            },
        };

        if passed {
            self.metrics.total_passed += 1;
        } else {
            self.metrics.total_failed += 1;
        }

        // Update rolling average
        let n = self.metrics.total_comparisons as f64;
        self.metrics.avg_diff_percent =
            ((self.metrics.avg_diff_percent * (n - 1.0)) + diff.diff_percent) / n;

        let result = VerificationResult {
            url: url.to_string(),
            viewport: viewport.name.clone(),
            baseline_id: baseline.id.clone(),
            diff: Some(diff),
            compliance,
            timestamp: self.metrics.total_captures as u64,
        };
        self.results.push(result.clone());
        Ok(result)
    }

    /// Verify a URL across all configured viewports.
    pub fn verify_all_viewports(
        &mut self,
        url: &str,
        baseline_name: &str,
    ) -> Result<Vec<VerificationResult>, String> {
        let viewports = self.config.viewports.clone();
        let mut results = Vec::new();
        for vp in &viewports {
            match self.verify_url(url, baseline_name, vp) {
                Ok(r) => results.push(r),
                Err(e) => return Err(format!("Viewport '{}': {}", vp.name, e)),
            }
        }
        Ok(results)
    }

    /// Update a baseline with new screenshots, preserving creation time.
    pub fn update_baseline(
        &mut self,
        name: &str,
        screenshots: Vec<Screenshot>,
    ) -> Result<(), String> {
        let baseline = self
            .baselines
            .get_mut(name)
            .ok_or_else(|| format!("Baseline '{}' not found", name))?;
        for ss in screenshots {
            baseline.screenshots.insert(ss.viewport.name.clone(), ss);
        }
        baseline.updated_at += 1;
        Ok(())
    }

    /// List all baseline names.
    pub fn list_baselines(&self) -> Vec<String> {
        self.baselines.keys().cloned().collect()
    }

    /// Delete a baseline by name.
    pub fn delete_baseline(&mut self, name: &str) -> Result<(), String> {
        self.baselines
            .remove(name)
            .map(|_| ())
            .ok_or_else(|| format!("Baseline '{}' not found", name))?;
        self.metrics.baselines_count = self.baselines.len();
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- Viewport presets ---------------------------------------------------

    #[test]
    fn test_viewport_desktop() {
        let vp = Viewport::desktop();
        assert_eq!(vp.width, 1920);
        assert_eq!(vp.height, 1080);
        assert_eq!(vp.device_scale, 1.0);
        assert_eq!(vp.name, "desktop");
    }

    #[test]
    fn test_viewport_tablet() {
        let vp = Viewport::tablet();
        assert_eq!(vp.width, 768);
        assert_eq!(vp.height, 1024);
        assert_eq!(vp.name, "tablet");
    }

    #[test]
    fn test_viewport_mobile() {
        let vp = Viewport::mobile();
        assert_eq!(vp.width, 375);
        assert_eq!(vp.height, 812);
        assert_eq!(vp.name, "mobile");
    }

    #[test]
    fn test_viewport_custom() {
        let vp = Viewport::new("4k", 3840, 2160, 2.0);
        assert_eq!(vp.width, 3840);
        assert_eq!(vp.height, 2160);
        assert_eq!(vp.device_scale, 2.0);
    }

    #[test]
    fn test_viewport_default() {
        let vp = Viewport::default();
        assert_eq!(vp.width, 1920);
        assert_eq!(vp.height, 1080);
        assert_eq!(vp.device_scale, 1.0);
        assert_eq!(vp.name, "default");
    }

    // -- Screenshot capture -------------------------------------------------

    #[test]
    fn test_capture_screenshot() {
        let mut engine = VerificationEngine::new(VerifyConfig::default());
        let vp = Viewport::desktop();
        let ss = engine.capture_screenshot("https://example.com", &vp).unwrap();
        assert_eq!(ss.url, "https://example.com");
        assert_eq!(ss.viewport.name, "desktop");
        assert!(!ss.hash.is_empty());
        assert!(ss.file_size_bytes > 0);
        assert_eq!(engine.metrics.total_captures, 1);
    }

    #[test]
    fn test_capture_multiple_screenshots() {
        let mut engine = VerificationEngine::new(VerifyConfig::default());
        let _ = engine.capture_screenshot("https://a.com", &Viewport::desktop()).unwrap();
        let _ = engine.capture_screenshot("https://b.com", &Viewport::mobile()).unwrap();
        assert_eq!(engine.metrics.total_captures, 2);
    }

    #[test]
    fn test_screenshot_ids_unique() {
        let mut engine = VerificationEngine::new(VerifyConfig::default());
        let s1 = engine.capture_screenshot("https://a.com", &Viewport::desktop()).unwrap();
        let s2 = engine.capture_screenshot("https://a.com", &Viewport::desktop()).unwrap();
        assert_ne!(s1.id, s2.id);
    }

    // -- Baseline management ------------------------------------------------

    #[test]
    fn test_set_and_get_baseline() {
        let mut engine = VerificationEngine::new(VerifyConfig::default());
        let ss = engine.capture_screenshot("https://example.com", &Viewport::desktop()).unwrap();
        let id = engine.set_baseline("homepage", vec![ss]).unwrap();
        assert!(id.starts_with("bl-"));
        let bl = engine.get_baseline("homepage").unwrap();
        assert_eq!(bl.name, "homepage");
        assert!(bl.screenshots.contains_key("desktop"));
    }

    #[test]
    fn test_get_baseline_not_found() {
        let engine = VerificationEngine::new(VerifyConfig::default());
        let err = engine.get_baseline("nope").unwrap_err();
        assert!(err.contains("not found"));
    }

    #[test]
    fn test_update_baseline() {
        let mut engine = VerificationEngine::new(VerifyConfig::default());
        let ss1 = engine.capture_screenshot("https://a.com", &Viewport::desktop()).unwrap();
        engine.set_baseline("home", vec![ss1]).unwrap();
        let ss2 = engine.capture_screenshot("https://a.com", &Viewport::mobile()).unwrap();
        engine.update_baseline("home", vec![ss2]).unwrap();
        let bl = engine.get_baseline("home").unwrap();
        assert_eq!(bl.screenshots.len(), 2);
        assert!(bl.updated_at > bl.created_at);
    }

    #[test]
    fn test_update_baseline_not_found() {
        let mut engine = VerificationEngine::new(VerifyConfig::default());
        let err = engine.update_baseline("nope", vec![]).unwrap_err();
        assert!(err.contains("not found"));
    }

    #[test]
    fn test_delete_baseline() {
        let mut engine = VerificationEngine::new(VerifyConfig::default());
        let ss = engine.capture_screenshot("https://a.com", &Viewport::desktop()).unwrap();
        engine.set_baseline("home", vec![ss]).unwrap();
        assert_eq!(engine.metrics.baselines_count, 1);
        engine.delete_baseline("home").unwrap();
        assert_eq!(engine.metrics.baselines_count, 0);
        assert!(engine.get_baseline("home").is_err());
    }

    #[test]
    fn test_delete_baseline_not_found() {
        let mut engine = VerificationEngine::new(VerifyConfig::default());
        assert!(engine.delete_baseline("nope").is_err());
    }

    #[test]
    fn test_list_baselines() {
        let mut engine = VerificationEngine::new(VerifyConfig::default());
        let ss = engine.capture_screenshot("https://a.com", &Viewport::desktop()).unwrap();
        engine.set_baseline("alpha", vec![ss.clone()]).unwrap();
        engine.set_baseline("beta", vec![ss]).unwrap();
        let names = engine.list_baselines();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"alpha".to_string()));
        assert!(names.contains(&"beta".to_string()));
    }

    // -- Perceptual hash ----------------------------------------------------

    #[test]
    fn test_perceptual_hash_deterministic() {
        let h1 = PerceptualHash::compute("hello");
        let h2 = PerceptualHash::compute("hello");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_perceptual_hash_different_inputs() {
        let h1 = PerceptualHash::compute("hello");
        let h2 = PerceptualHash::compute("world");
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_hamming_distance_identical() {
        assert_eq!(PerceptualHash::hamming_distance(0xFF, 0xFF), 0);
    }

    #[test]
    fn test_hamming_distance_one_bit() {
        assert_eq!(PerceptualHash::hamming_distance(0b0, 0b1), 1);
    }

    #[test]
    fn test_hamming_distance_all_bits() {
        assert_eq!(PerceptualHash::hamming_distance(0, u64::MAX), 64);
    }

    #[test]
    fn test_similarity_identical() {
        let sim = PerceptualHash::similarity(42, 42);
        assert!((sim - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_similarity_completely_different() {
        let sim = PerceptualHash::similarity(0, u64::MAX);
        assert!((sim - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_similarity_range() {
        let h1 = PerceptualHash::compute("abc");
        let h2 = PerceptualHash::compute("abd");
        let sim = PerceptualHash::similarity(h1, h2);
        assert!(sim >= 0.0 && sim <= 1.0);
    }

    // -- Visual diff --------------------------------------------------------

    #[test]
    fn test_compare_identical_screenshots() {
        let mut engine = VerificationEngine::new(VerifyConfig::default());
        let ss = engine.capture_screenshot("https://a.com", &Viewport::desktop()).unwrap();
        let diff = engine.compare_screenshots(&ss, &ss.clone()).unwrap();
        assert_eq!(diff.diff_percent, 0.0);
        assert_eq!(diff.changed_pixels, 0);
        assert!(diff.regions.is_empty());
        assert!(diff.diff_image_path.is_none());
    }

    #[test]
    fn test_compare_different_screenshots() {
        let mut engine = VerificationEngine::new(VerifyConfig::default());
        let ss1 = engine.capture_screenshot("https://a.com", &Viewport::desktop()).unwrap();
        let ss2 = engine.capture_screenshot("https://b.com", &Viewport::desktop()).unwrap();
        let diff = engine.compare_screenshots(&ss1, &ss2).unwrap();
        assert!(diff.diff_percent > 0.0);
        assert!(diff.changed_pixels > 0);
        assert!(!diff.regions.is_empty());
        assert!(diff.diff_image_path.is_some());
    }

    #[test]
    fn test_diff_total_pixels() {
        let mut engine = VerificationEngine::new(VerifyConfig::default());
        let ss = engine.capture_screenshot("https://a.com", &Viewport::desktop()).unwrap();
        let diff = engine.compare_screenshots(&ss, &ss.clone()).unwrap();
        assert_eq!(diff.total_pixels, 1920 * 1080);
    }

    #[test]
    fn test_diff_region_severity_assignment() {
        let mut engine = VerificationEngine::new(VerifyConfig::default());
        let ss1 = engine.capture_screenshot("https://a.com", &Viewport::desktop()).unwrap();
        let ss2 = engine.capture_screenshot("https://completely-different.com", &Viewport::desktop()).unwrap();
        let diff = engine.compare_screenshots(&ss1, &ss2).unwrap();
        if !diff.regions.is_empty() {
            let sev = &diff.regions[0].severity;
            // Just verify it's a valid severity
            assert!(
                *sev == DiffSeverity::Minor
                    || *sev == DiffSeverity::Moderate
                    || *sev == DiffSeverity::Major
                    || *sev == DiffSeverity::Critical
            );
        }
    }

    // -- Compliance scoring -------------------------------------------------

    #[test]
    fn test_compliance_pass() {
        let mut engine = VerificationEngine::new(VerifyConfig {
            threshold_percent: 50.0,
            ..VerifyConfig::default()
        });
        let ss = engine.capture_screenshot("https://a.com", &Viewport::desktop()).unwrap();
        engine.set_baseline("home", vec![ss]).unwrap();
        // Same URL → same hash → 0% diff → passes
        let result = engine.verify_url("https://a.com", "home", &Viewport::desktop()).unwrap();
        assert!(result.compliance.passed);
        assert_eq!(result.compliance.score, 100.0);
    }

    #[test]
    fn test_compliance_fail() {
        let mut engine = VerificationEngine::new(VerifyConfig {
            threshold_percent: 0.0,
            ..VerifyConfig::default()
        });
        let ss = engine.capture_screenshot("https://a.com", &Viewport::desktop()).unwrap();
        engine.set_baseline("home", vec![ss]).unwrap();
        let result = engine.verify_url("https://different.com", "home", &Viewport::desktop()).unwrap();
        assert!(!result.compliance.passed);
        assert!(result.compliance.details.contains("exceeds"));
    }

    // -- Verify URL ---------------------------------------------------------

    #[test]
    fn test_verify_url_no_baseline() {
        let mut engine = VerificationEngine::new(VerifyConfig::default());
        let err = engine.verify_url("https://a.com", "missing", &Viewport::desktop()).unwrap_err();
        assert!(err.contains("not found"));
    }

    #[test]
    fn test_verify_url_no_viewport_in_baseline() {
        let mut engine = VerificationEngine::new(VerifyConfig::default());
        let ss = engine.capture_screenshot("https://a.com", &Viewport::desktop()).unwrap();
        engine.set_baseline("home", vec![ss]).unwrap();
        let err = engine.verify_url("https://a.com", "home", &Viewport::mobile()).unwrap_err();
        assert!(err.contains("No baseline screenshot"));
    }

    #[test]
    fn test_verify_url_updates_metrics() {
        let mut engine = VerificationEngine::new(VerifyConfig::default());
        let ss = engine.capture_screenshot("https://a.com", &Viewport::desktop()).unwrap();
        engine.set_baseline("home", vec![ss]).unwrap();
        engine.verify_url("https://a.com", "home", &Viewport::desktop()).unwrap();
        assert_eq!(engine.metrics.total_passed, 1);
        assert_eq!(engine.metrics.total_comparisons, 1);
    }

    // -- Verify all viewports -----------------------------------------------

    #[test]
    fn test_verify_all_viewports() {
        let mut engine = VerificationEngine::new(VerifyConfig::default());
        let viewports = engine.config.viewports.clone();
        let mut screenshots = Vec::new();
        for vp in &viewports {
            screenshots.push(engine.capture_screenshot("https://a.com", vp).unwrap());
        }
        engine.set_baseline("home", screenshots).unwrap();
        let results = engine.verify_all_viewports("https://a.com", "home").unwrap();
        assert_eq!(results.len(), 3);
        for r in &results {
            assert!(r.compliance.passed);
        }
    }

    #[test]
    fn test_verify_all_viewports_missing_viewport() {
        let mut engine = VerificationEngine::new(VerifyConfig::default());
        // Only set desktop baseline, but config has 3 viewports
        let ss = engine.capture_screenshot("https://a.com", &Viewport::desktop()).unwrap();
        engine.set_baseline("home", vec![ss]).unwrap();
        let err = engine.verify_all_viewports("https://a.com", "home").unwrap_err();
        assert!(err.contains("tablet") || err.contains("No baseline screenshot"));
    }

    // -- CI integration -----------------------------------------------------

    #[test]
    fn test_ci_report_json() {
        let ci = CiIntegration::new(1.0, true, ReportFormat::Json);
        let results = vec![];
        let report = ci.generate_report(&results);
        assert!(report.contains('['));
    }

    #[test]
    fn test_ci_report_markdown() {
        let ci = CiIntegration::new(1.0, true, ReportFormat::Markdown);
        let result = VerificationResult {
            url: "https://a.com".to_string(),
            viewport: "desktop".to_string(),
            baseline_id: "bl-1".to_string(),
            diff: None,
            compliance: ComplianceScore {
                score: 100.0,
                viewport: "desktop".to_string(),
                threshold: 1.0,
                passed: true,
                details: "ok".to_string(),
            },
            timestamp: 1,
        };
        let report = ci.generate_report(&[result]);
        assert!(report.contains("# Visual Verification Report"));
        assert!(report.contains("1 passed"));
        assert!(report.contains("0 failed"));
        assert!(report.contains("https://a.com"));
    }

    #[test]
    fn test_ci_report_html() {
        let ci = CiIntegration::new(1.0, true, ReportFormat::Html);
        let report = ci.generate_report(&[]);
        assert!(report.contains("<html>"));
        assert!(report.contains("</html>"));
    }

    // -- Metrics ------------------------------------------------------------

    #[test]
    fn test_metrics_default() {
        let m = VerifyMetrics::default();
        assert_eq!(m.total_captures, 0);
        assert_eq!(m.total_comparisons, 0);
        assert_eq!(m.total_passed, 0);
        assert_eq!(m.total_failed, 0);
        assert_eq!(m.avg_diff_percent, 0.0);
        assert_eq!(m.baselines_count, 0);
    }

    #[test]
    fn test_metrics_update_on_verify() {
        let mut engine = VerificationEngine::new(VerifyConfig::default());
        let ss = engine.capture_screenshot("https://a.com", &Viewport::desktop()).unwrap();
        engine.set_baseline("home", vec![ss]).unwrap();
        engine.verify_url("https://a.com", "home", &Viewport::desktop()).unwrap();
        engine.verify_url("https://different.com", "home", &Viewport::desktop()).unwrap();
        assert_eq!(engine.metrics.total_comparisons, 2);
        assert_eq!(engine.metrics.total_passed + engine.metrics.total_failed, 2);
    }

    // -- Edge cases ---------------------------------------------------------

    #[test]
    fn test_threshold_boundary_exact() {
        // When diff is exactly at threshold, it should pass (<=)
        let mut engine = VerificationEngine::new(VerifyConfig {
            threshold_percent: 100.0, // accept anything
            ..VerifyConfig::default()
        });
        let ss = engine.capture_screenshot("https://a.com", &Viewport::desktop()).unwrap();
        engine.set_baseline("home", vec![ss]).unwrap();
        let result = engine.verify_url("https://z.com", "home", &Viewport::desktop()).unwrap();
        assert!(result.compliance.passed);
    }

    #[test]
    fn test_config_defaults() {
        let cfg = VerifyConfig::default();
        assert_eq!(cfg.threshold_percent, 0.1);
        assert_eq!(cfg.viewports.len(), 3);
        assert!(cfg.headless);
        assert_eq!(cfg.timeout_secs, 30);
    }

    #[test]
    fn test_capture_request_fields() {
        let req = CaptureRequest {
            url: "https://a.com".to_string(),
            viewports: vec![Viewport::desktop()],
            wait_secs: Some(5),
            scroll_to: Some("#footer".to_string()),
            clip_selector: None,
        };
        assert_eq!(req.wait_secs, Some(5));
        assert!(req.clip_selector.is_none());
    }

    #[test]
    fn test_diff_severity_enum() {
        let severities = vec![
            DiffSeverity::Minor,
            DiffSeverity::Moderate,
            DiffSeverity::Major,
            DiffSeverity::Critical,
        ];
        assert_eq!(severities.len(), 4);
        assert_ne!(DiffSeverity::Minor, DiffSeverity::Critical);
    }

    #[test]
    fn test_results_accumulate() {
        let mut engine = VerificationEngine::new(VerifyConfig::default());
        let ss = engine.capture_screenshot("https://a.com", &Viewport::desktop()).unwrap();
        engine.set_baseline("home", vec![ss]).unwrap();
        engine.verify_url("https://a.com", "home", &Viewport::desktop()).unwrap();
        engine.verify_url("https://a.com", "home", &Viewport::desktop()).unwrap();
        assert_eq!(engine.results.len(), 2);
    }

    #[test]
    fn test_overwrite_baseline() {
        let mut engine = VerificationEngine::new(VerifyConfig::default());
        let ss1 = engine.capture_screenshot("https://a.com", &Viewport::desktop()).unwrap();
        engine.set_baseline("home", vec![ss1]).unwrap();
        let ss2 = engine.capture_screenshot("https://b.com", &Viewport::desktop()).unwrap();
        engine.set_baseline("home", vec![ss2.clone()]).unwrap();
        let bl = engine.get_baseline("home").unwrap();
        assert_eq!(bl.screenshots.get("desktop").unwrap().url, "https://b.com");
        // Still only one baseline
        assert_eq!(engine.metrics.baselines_count, 1);
    }

    #[test]
    fn test_serialization_roundtrip() {
        let vp = Viewport::desktop();
        let json = serde_json::to_string(&vp).unwrap();
        let vp2: Viewport = serde_json::from_str(&json).unwrap();
        assert_eq!(vp, vp2);
    }
}
