# RL-Trained Next-Edit Prediction

Reinforcement learning model that predicts your next edit based on patterns and history.

## Triggers
- "edit prediction", "next edit", "predict edit", "edit suggestion"
- "RL prediction", "edit pattern", "learn edits"

## Usage
```
/predict enable                   # Enable edit prediction
/predict disable                  # Disable predictions
/predict stats                    # Show acceptance rate and model stats
/predict patterns                 # List detected patterns
/predict decay 0.95               # Decay exploration rate
```

## How It Works
1. **Record** — Every edit action is recorded (insert, delete, replace, cursor move, save, undo/redo, commands)
2. **Learn** — Q-learning model updates Q-values based on your accept/reject/modify feedback
3. **Detect** — Common action sequences are detected as patterns (e.g., insert -> save, delete -> undo)
4. **Predict** — Next edit is predicted from Q-table lookup or pattern matching
5. **Adapt** — Exploration rate decays over time as the model becomes more confident

## Features
- Q-learning with configurable learning rate, discount factor, exploration rate
- State hashing: file type + last 3 actions + context length
- 8 edit action types tracked
- 4 prediction outcomes: Accepted (+1.0), Modified (+0.5), Ignored (0.0), Rejected (-0.3)
- Automatic pattern detection from edit history
- Pattern confidence scoring (frequency + reward weighted)
- Sigmoid-based confidence conversion
- Configurable history window (default 1000 events)
- Exploration decay with floor (min 0.01)
