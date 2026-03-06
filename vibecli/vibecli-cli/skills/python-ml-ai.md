---
triggers: ["scikit-learn", "pytorch", "tensorflow", "transformers", "model training", "machine learning python", "neural network", "huggingface"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["python3"]
category: python
---

# Python ML & AI

When building machine learning and AI systems:

1. Start with scikit-learn for classical ML — use `Pipeline` to chain preprocessing + model
2. Always split data: `train_test_split(X, y, test_size=0.2, random_state=42, stratify=y)`
3. Use cross-validation: `cross_val_score(model, X, y, cv=5, scoring='accuracy')`
4. Feature engineering: `StandardScaler` for numeric, `OneHotEncoder` for categorical, `ColumnTransformer` for mixed
5. PyTorch: define `nn.Module`, implement `forward()`, use `DataLoader` for batching
6. Training loop: zero gradients → forward pass → compute loss → backward pass → optimizer step
7. Use `transformers` library for NLP — `AutoTokenizer` + `AutoModelForSequenceClassification`
8. Fine-tune with `Trainer` API: define `TrainingArguments`, pass train/eval datasets
9. Track experiments with `wandb` or `mlflow` — log hyperparams, metrics, artifacts
10. Use `torch.no_grad()` context for inference — saves memory, speeds up computation
11. Save models: `torch.save(model.state_dict(), 'model.pt')` — load with `model.load_state_dict()`
12. Use GPU when available: `device = torch.device('cuda' if torch.cuda.is_available() else 'cpu')`
