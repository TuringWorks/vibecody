Feature: Pod manager — vLLM GPU pod deployment
  The pod manager validates VRAM, generates launch commands, assigns GPUs,
  and extracts models paths before deploying vLLM on remote GPU pods.

  Scenario: Preflight passes when VRAM is sufficient
    Given a pod spec with gpu_tier "a10-24gb" gpu_count 1 and model "mistralai/Mistral-7B-Instruct-v0.3"
    And the build variant is "release"
    When I run preflight on the spec
    Then the preflight result should pass
    And the vram_available_gb should be 24

  Scenario: Preflight fails when VRAM is insufficient
    Given a pod spec with gpu_tier "t4-16gb" gpu_count 1 and model "Qwen/Qwen2.5-72B-Instruct"
    And the build variant is "release"
    When I run preflight on the spec
    Then the preflight result should fail
    And the preflight errors should mention "Insufficient VRAM"

  Scenario: Launch command includes docker image and model flags
    Given a pod spec with gpu_tier "a100-80gb" gpu_count 1 and model "meta-llama/Meta-Llama-3-8B-Instruct"
    And the build variant is "nightly"
    And the port is 8080
    When I build the launch command
    Then the command should contain "vllm/vllm-openai:nightly"
    And the command should contain "--model"
    And the command should contain "meta-llama/Meta-Llama-3-8B-Instruct"

  Scenario: Multi-GPU assignment has no overlap
    Given two models "mistralai/Mistral-7B-Instruct-v0.3" and "meta-llama/Meta-Llama-3-8B-Instruct"
    And total_gpus is 4 with vram_per_gpu 24
    When I assign the models to GPUs
    Then the assignments should have no overlapping GPU indices
    And the assignment for "mistralai/Mistral-7B-Instruct-v0.3" should start at index 0

  Scenario: Extract models path from a volume mount string
    Given a volume mount string "/data/hf-models:/workspace/models"
    When I extract the models path
    Then the extracted path should be "/data/hf-models"
