---
layout: page
title: "Tutorial: Model Wizard — Fine-Tune to Deployment"
permalink: /tutorials/model-wizard/
---

Build, fine-tune, quantize, and deploy a custom AI model in 7 steps using VibeCody's Model Wizard.

**Prerequisites:** VibeCody installed, a training dataset or codebase, and either a free Colab account or a local GPU.

---

## Quick Start (5 minutes)

The fastest path from zero to a running custom model:

```bash
# 1. Extract training data from your codebase
vibecli
> /train dataset from-codebase --format chatml --output data.jsonl

# 2. Fine-tune with Unsloth on free Colab GPU
# (Copy the generated script from /wizard into a Colab notebook)
> /wizard

# 3. Quantize the result
> /inference quantize --method gguf-q4km --model ./output

# 4. Deploy locally
> /inference deploy --backend ollama --model ./model-q4.gguf

# 5. Use your model in VibeCody
> /model ollama my-model
> /chat Hello from my custom model!
```

---

## The 7 Steps in Detail

### Step 1: Choose a Base Model

Pick a foundation model to fine-tune. Smaller models train faster and cheaper:

| Model | Size | Free Colab? | Best For |
|-------|------|-------------|----------|
| Llama 3.2 3B | 3B | Yes (T4) | IoT, edge, fast inference |
| Llama 3.1 8B | 8B | Yes (T4 with QLoRA) | General purpose, coding |
| Mistral 7B | 7B | Yes (T4) | Instruction following |
| Phi 3.5 Mini | 3.8B | Yes (T4) | Small, fast, surprisingly capable |
| Qwen 2.5 Coder 7B | 7B | Yes (T4) | Code generation |
| DeepSeek R1 Distill 7B | 7B | Yes (T4) | Reasoning |
| Mixtral 8x7B (MoE) | 47B | No (needs A100) | Mixture of Experts, high quality |

**Recommendation:** Start with **Llama 3.1 8B** — best quality-to-cost ratio, runs on free Colab.

### Step 2: Prepare Your Dataset

Four ways to create training data:

**From your codebase** (extracts function docs + implementations):

```bash
> /train dataset from-codebase --format chatml --output data.jsonl
```

**From git history** (commit messages + diffs):

```bash
> /train dataset from-git --max-commits 5000 --format chatml --output data.jsonl
```

**From documents** (PDF, Markdown, HTML — uses MinerU for complex PDFs):

```bash
# For simple docs:
> /ingest ./docs --output data.jsonl

# For scientific PDFs with formulas/tables:
pip install magic-pdf[full]
magic-pdf -p ./papers/ -o ./parsed/ -m auto
> /ingest ./parsed/ --output data.jsonl
```

**Existing dataset** (use a pre-prepared JSONL file):

```bash
> /train dataset validate --file my-data.jsonl
```

### Step 3: Configure Fine-Tuning

Six open-source libraries, each with different strengths:

| Library | Best For | GPU Needed |
|---------|----------|-----------|
| **Unsloth** | Single GPU, Colab, fastest setup | 1x T4 (free) |
| **Axolotl** | Reproducible configs, team workflows | 1x A10G+ |
| **LLaMA Factory** | RLHF/DPO alignment, 100+ models | 1+ GPUs |
| **HF TRL** | DPO/PPO preference optimization | 1+ GPUs |
| **PEFT** | LoRA adapters for any HF model | 1x T4 (free) |
| **DeepSpeed** | Multi-GPU distributed training | 2-8 GPUs |

**QLoRA with Unsloth** is the fastest path — trains an 8B model on a free T4 in about 30 minutes.

Alignment methods:

- **SFT** (Supervised Fine-Tuning) — learn from examples
- **DPO** (Direct Preference Optimization) — learn from preference pairs
- **PPO** (Proximal Policy Optimization) — learn from reward signals
- **KTO** (Kahneman-Tversky Optimization) — binary good/bad feedback

### Step 4: Select Training Environment

| Platform | GPU | Cost | Setup |
|----------|-----|------|-------|
| **Google Colab** | T4 16GB | Free | Paste script, run cells |
| **Kaggle** | P100 16GB | Free (30hr/week) | Upload notebook |
| **Local** | Your GPU(s) | Free | Run script directly |
| **RunPod** | A100 80GB | $1-4/hr | SSH, run script |
| **SageMaker** | A10G-A100 | $1-30/hr | Studio notebook |

### Step 5: Quantize

Compress your model for efficient deployment:

| Method | Size (8B model) | Quality Loss | CPU? | GPU? |
|--------|-----------------|-------------|------|------|
| **GGUF Q4_K_M** | ~4.5 GB | Minimal | Yes | Yes |
| **GGUF Q5_K_M** | ~5.5 GB | Very small | Yes | Yes |
| **GPTQ 4-bit** | ~4 GB | Small | No | Yes |
| **AWQ 4-bit** | ~4 GB | Small | No | Yes |
| **FP16** | ~16 GB | None | No | Yes |

**Recommendation:** GGUF Q4_K_M for maximum compatibility (runs on CPU and GPU, works with Ollama and llama.cpp).

### Step 6: Deploy Inference

| Backend | Best For | API Compatible |
|---------|----------|---------------|
| **Ollama** | Easiest setup, local dev | OpenAI-compatible |
| **vLLM** | Fastest GPU serving, production | OpenAI-compatible |
| **llama.cpp** | CPU+GPU, edge, IoT | OpenAI-compatible |
| **TGI** | Docker-ready, HuggingFace models | Custom API |

Deploy targets: local process, Docker container, Kubernetes, or edge device (Jetson, Raspberry Pi).

### Step 7: Review and Launch

The wizard generates a complete bash script. Copy it and run in your terminal or notebook. The script covers:

1. Environment setup (`pip install`)
2. Data preparation commands
3. Fine-tuning code (library-specific)
4. Quantization commands
5. Inference server launch
6. Docker packaging (if selected)
7. Kubernetes YAML (if selected)
8. VibeCody connection command

---

## Example: Fine-Tune a Code Assistant on Colab (Free)

Complete walkthrough using the free Colab T4 GPU:

```bash
# In VibeCody CLI — prepare data
vibecli
> /train dataset from-codebase --format chatml --output train.jsonl
> /train dataset validate --file train.jsonl
> /wizard   # Copy the generated script
```

Paste this into a Google Colab notebook:

```python
# Cell 1: Setup
!pip install unsloth transformers datasets accelerate bitsandbytes
import torch
print(f'GPU: {torch.cuda.get_device_name(0)}')

# Cell 2: Load and prepare model
from unsloth import FastLanguageModel
model, tokenizer = FastLanguageModel.from_pretrained(
    "meta-llama/Llama-3.1-8B-Instruct",
    max_seq_length=2048,
    load_in_4bit=True
)
model = FastLanguageModel.get_peft_model(model, r=16, lora_alpha=16)

# Cell 3: Train
from trl import SFTTrainer
from transformers import TrainingArguments
from datasets import load_dataset

# Upload train.jsonl to Colab first
dataset = load_dataset('json', data_files='train.jsonl')
trainer = SFTTrainer(
    model=model, tokenizer=tokenizer,
    train_dataset=dataset['train'],
    args=TrainingArguments(
        output_dir='./output',
        per_device_train_batch_size=4,
        num_train_epochs=3,
        learning_rate=2e-4,
    )
)
trainer.train()

# Cell 4: Save
model.save_pretrained('./output')
tokenizer.save_pretrained('./output')

# Cell 5: Quantize to GGUF (download llama.cpp converter)
!pip install llama-cpp-python
# Export and download the GGUF file
```

Then deploy locally:

```bash
# Create Ollama model from GGUF
ollama create my-code-assistant -f Modelfile

# Use in VibeCody
vibecli --provider ollama --model my-code-assistant
> /chat Explain this codebase to me
```

---

## VibeUI Model Wizard

For the full interactive experience, open the **Model Wizard** tab in VibeUI's AI panel. It provides:

- Visual step-by-step form with option cards
- Auto-calculated VRAM requirements
- Library-specific code generation
- One-click script copy
- Config summary at each step

The **AI/ML Workflow** tab provides the big-picture view of the full pipeline with 5 end-to-end example workflows.
