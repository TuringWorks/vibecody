---
triggers: ["GPU cluster", "GPU provisioning", "CUDA", "GPU server", "GPU node", "multi-GPU", "GPU scheduling"]
tools_allowed: ["read_file", "write_file", "bash"]
category: infrastructure
---

# GPU Cluster Provisioning

When provisioning and managing GPU clusters:

1. **CUDA/ROCm setup** — Install the driver first (nvidia-driver-XXX or amdgpu-dkms), then the toolkit (CUDA Toolkit or ROCm). Pin driver and toolkit versions together. Use nvidia-container-toolkit for Docker GPU passthrough. Verify with `nvidia-smi` (NVIDIA) or `rocm-smi` (AMD). Set `CUDA_VISIBLE_DEVICES` to control GPU visibility per process.

2. **nvidia-smi monitoring** — Use `nvidia-smi dmon` for continuous monitoring of utilization, memory, temperature, and power. Export metrics to Prometheus via dcgm-exporter or nvidia-gpu-exporter. Set up alerts for GPU memory > 90%, temperature > 80C, and ECC errors. Use `nvidia-smi topo -m` to inspect NVLink/PCIe topology.

3. **SLURM configuration** — Install slurm-wlm with gres.conf defining GPU resources per node (e.g., `NodeName=gpu01 Gres=gpu:a100:8`). Configure `GresTypes=gpu` in slurm.conf. Users request GPUs with `--gres=gpu:N`. Set up accounting to track GPU-hours per user/project. Use `srun --gres=gpu:4` for interactive GPU jobs.

4. **Kubernetes GPU operator** — Deploy the NVIDIA GPU Operator (includes driver, toolkit, device plugin, dcgm-exporter, MIG manager) via Helm. It automates the entire GPU software stack on K8s nodes. Pods request GPUs with `nvidia.com/gpu: 1` in resource limits. Use `kubectl describe node` to verify allocatable GPU count.

5. **Node affinity and taints** — Taint GPU nodes (`kubectl taint nodes gpu-node nvidia.com/gpu=present:NoSchedule`) to prevent non-GPU workloads from scheduling there. Add tolerations to GPU workload specs. Use node affinity to target specific GPU types (A100 vs H100) via labels like `nvidia.com/gpu.product=NVIDIA-A100-SXM4-80GB`.

6. **MIG (Multi-Instance GPU)** — Enable MIG on A100/H100 to partition a single GPU into up to 7 isolated instances. Configure profiles (e.g., 3g.40gb, 1g.10gb) via `nvidia-smi mig -cgi`. Each instance gets dedicated memory and compute. Useful for inference workloads or multi-tenant clusters. Integrate with K8s via the MIG strategy in the device plugin (single, mixed, or none).

7. **NVLink/NVSwitch topology** — For multi-GPU training, ensure GPUs communicate over NVLink (600 GB/s on H100) rather than PCIe. Use `nvidia-smi topo -m` to verify connectivity. DGX systems use NVSwitch for all-to-all GPU communication. Set `NCCL_P2P_LEVEL` and `NCCL_NET_GDR_LEVEL` for optimal NCCL performance across the topology.

8. **GPU memory management** — Monitor GPU memory with `nvidia-smi` or programmatically via `torch.cuda.memory_allocated()`. Use gradient checkpointing and mixed precision to reduce memory footprint. Set `PYTORCH_CUDA_ALLOC_CONF=expandable_segments:True` to reduce fragmentation. For inference, use quantization (int8/int4) to fit larger models in GPU memory.

9. **Cloud GPU instance types** — A100 (80GB, training/inference), H100 (80GB, fastest training), L4 (24GB, cost-effective inference), T4 (16GB, budget inference), A10G (24GB, balanced). Compare on-demand vs reserved vs spot pricing. Use spot instances for fault-tolerant training with checkpointing. Consider cloud GPU services (Lambda, CoreWeave, RunPod) for burst capacity.

10. **Spot/preemptible strategies** — Save 60-90% on GPU costs with spot instances. Implement checkpointing every N steps to resume after preemption. Use multiple availability zones and instance types for higher spot availability. Set up automatic requeue (SLURM `--requeue` or K8s Job `restartPolicy`). Store checkpoints on networked storage (S3, GCS, NFS) not local disk.
