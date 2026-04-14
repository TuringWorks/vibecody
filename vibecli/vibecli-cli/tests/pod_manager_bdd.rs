/*!
 * BDD tests for pod_manager using Cucumber.
 * Run with: cargo test --test pod_manager_bdd
 *
 * The module is included directly via #[path] so that it compiles as part of
 * this test binary without requiring a lib.rs declaration.
 */
use cucumber::{World, given, then, when};

#[path = "../src/pod_manager.rs"]
mod pod_manager;

use pod_manager::{
    GpuAssignment, GpuTier, ModelConfig, PodManager, PodSpec, PreflightResult, VllmBuild,
    assign_gpus,
};

// ---------------------------------------------------------------------------
// World
// ---------------------------------------------------------------------------

#[derive(Debug, Default, World)]
pub struct PmWorld {
    /// Current pod spec under test.
    spec: Option<PodSpec>,
    /// Result from the last preflight call.
    preflight: Option<PreflightResult>,
    /// Result from the last build_launch_command call.
    launch_cmd: Vec<String>,
    /// Models queued for GPU assignment.
    models_for_assign: Vec<ModelConfig>,
    /// Total GPUs available for assignment.
    total_gpus: u32,
    /// VRAM per GPU for assignment.
    vram_per_gpu: u32,
    /// GPU assignment results.
    assignments: Vec<GpuAssignment>,
    /// Raw volume mount string for extraction.
    mount_string: String,
    /// Extracted path result.
    extracted_path: Option<String>,
}

// ---------------------------------------------------------------------------
// Given
// ---------------------------------------------------------------------------

#[given(expr = "a pod spec with gpu_tier {string} gpu_count {int} and model {string}")]
fn set_spec(world: &mut PmWorld, gpu_tier_str: String, gpu_count: u32, model_name: String) {
    let gpu_tier = GpuTier::from_str(&gpu_tier_str)
        .unwrap_or_else(|| panic!("Unknown GPU tier: {}", gpu_tier_str));
    let model = ModelConfig::for_model(&model_name);
    world.spec = Some(PodSpec {
        gpu_tier,
        gpu_count,
        model,
        build: VllmBuild::Release,
        port: 8000,
        api_key: None,
        models_path: None,
        extra_flags: vec![],
    });
}

#[given(expr = "the build variant is {string}")]
fn set_build(world: &mut PmWorld, build_str: String) {
    let build = match build_str.as_str() {
        "release" => VllmBuild::Release,
        "nightly" => VllmBuild::Nightly,
        "gpt-oss" => VllmBuild::GptOss,
        other => panic!("Unknown build variant: {}", other),
    };
    if let Some(spec) = world.spec.as_mut() {
        spec.build = build;
    }
}

#[given(expr = "the port is {int}")]
fn set_port(world: &mut PmWorld, port: u16) {
    if let Some(spec) = world.spec.as_mut() {
        spec.port = port;
    }
}

#[given(expr = "two models {string} and {string}")]
fn set_two_models(world: &mut PmWorld, model1: String, model2: String) {
    world.models_for_assign = vec![
        ModelConfig::for_model(&model1),
        ModelConfig::for_model(&model2),
    ];
}

#[given(expr = "total_gpus is {int} with vram_per_gpu {int}")]
fn set_gpu_pool(world: &mut PmWorld, total_gpus: u32, vram_per_gpu: u32) {
    world.total_gpus = total_gpus;
    world.vram_per_gpu = vram_per_gpu;
}

#[given(expr = "a volume mount string {string}")]
fn set_mount_string(world: &mut PmWorld, mount: String) {
    world.mount_string = mount;
}

// ---------------------------------------------------------------------------
// When
// ---------------------------------------------------------------------------

#[when("I run preflight on the spec")]
fn do_preflight(world: &mut PmWorld) {
    let pm = PodManager::new();
    let spec = world.spec.as_ref().expect("spec not set");
    world.preflight = Some(pm.preflight(spec));
}

#[when("I build the launch command")]
fn do_build_launch(world: &mut PmWorld) {
    let pm = PodManager::new();
    let spec = world.spec.as_ref().expect("spec not set");
    world.launch_cmd = pm.build_launch_command(spec);
}

#[when("I assign the models to GPUs")]
fn do_assign(world: &mut PmWorld) {
    let result = assign_gpus(&world.models_for_assign, world.total_gpus, world.vram_per_gpu)
        .expect("GPU assignment failed");
    world.assignments = result;
}

#[when("I extract the models path")]
fn do_extract(world: &mut PmWorld) {
    world.extracted_path = PodManager::extract_models_path(&world.mount_string);
}

// ---------------------------------------------------------------------------
// Then
// ---------------------------------------------------------------------------

#[then("the preflight result should pass")]
fn check_preflight_pass(world: &mut PmWorld) {
    let pf = world.preflight.as_ref().expect("preflight not run");
    assert!(
        pf.is_ok(),
        "expected preflight to pass but got errors: {:?}",
        pf.errors
    );
}

#[then("the preflight result should fail")]
fn check_preflight_fail(world: &mut PmWorld) {
    let pf = world.preflight.as_ref().expect("preflight not run");
    assert!(
        !pf.is_ok(),
        "expected preflight to fail but it passed"
    );
}

#[then(expr = "the vram_available_gb should be {int}")]
fn check_vram_available(world: &mut PmWorld, expected: u32) {
    let pf = world.preflight.as_ref().expect("preflight not run");
    assert_eq!(
        pf.vram_available_gb, expected,
        "vram_available_gb mismatch"
    );
}

#[then(expr = "the preflight errors should mention {string}")]
fn check_preflight_error_message(world: &mut PmWorld, needle: String) {
    let pf = world.preflight.as_ref().expect("preflight not run");
    let all_errors = pf.errors.join(" ");
    assert!(
        all_errors.contains(&needle),
        "expected error containing '{}' but got: {}",
        needle,
        all_errors
    );
}

#[then(expr = "the command should contain {string}")]
fn check_command_contains(world: &mut PmWorld, token: String) {
    let joined = world.launch_cmd.join(" ");
    assert!(
        joined.contains(&token),
        "expected command to contain '{}' but got:\n{}",
        token,
        joined
    );
}

#[then("the assignments should have no overlapping GPU indices")]
fn check_no_overlap(world: &mut PmWorld) {
    let all: Vec<u32> = world
        .assignments
        .iter()
        .flat_map(|a| a.gpu_indices.clone())
        .collect();
    let unique: std::collections::HashSet<u32> = all.iter().cloned().collect();
    assert_eq!(
        all.len(),
        unique.len(),
        "GPU indices overlap: {:?}",
        all
    );
}

#[then(expr = "the assignment for {string} should start at index {int}")]
fn check_assignment_start(world: &mut PmWorld, model_name: String, expected_start: u32) {
    let asgn = world
        .assignments
        .iter()
        .find(|a| a.model_name == model_name)
        .unwrap_or_else(|| panic!("No assignment found for model '{}'", model_name));
    assert_eq!(
        asgn.gpu_indices[0],
        expected_start,
        "assignment for '{}' should start at index {}",
        model_name,
        expected_start
    );
}

#[then(expr = "the extracted path should be {string}")]
fn check_extracted_path(world: &mut PmWorld, expected: String) {
    let path = world
        .extracted_path
        .as_ref()
        .expect("extract_models_path returned None");
    assert_eq!(path, &expected, "extracted path mismatch");
}

// ---------------------------------------------------------------------------
// Runner
// ---------------------------------------------------------------------------

fn main() {
    futures::executor::block_on(PmWorld::run("tests/features/pod_manager.feature"));
}
