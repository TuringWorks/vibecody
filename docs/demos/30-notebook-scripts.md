---
layout: page
title: "Demo 30: Notebook & Scripts"
permalink: /demos/notebook-scripts/
nav_order: 30
parent: Demos
---


## Overview

This demo covers VibeCody's Notebook panel for interactive code notebooks and the Script Runner for executing scripts with AI assistance. Notebooks let you mix code cells with markdown documentation, while the script runner manages script creation, execution, templating, and history.

**Time to complete:** ~12 minutes

## Prerequisites

- VibeCody installed and configured with an AI provider
- A language runtime for notebook execution (Python 3, Node.js, or Rust)
- For VibeUI: the desktop app running (`npm run tauri dev`)

## Step-by-Step Walkthrough

### Notebook: Interactive Code Notebooks

#### Step 1: Create a new notebook

```bash
vibecli
> /notebook create data-analysis.vnb
```

```
Created notebook: data-analysis.vnb
Type: VibeCody Notebook
Cells: 0
```

VibeCody notebooks use the `.vnb` format, which stores cells as JSON with inline outputs.

#### Step 2: Add a markdown cell

```bash
> /notebook cell add --type markdown --content "# Data Analysis\nExploring sales data with Python."
```

```
Cell [1] (markdown) added:
  # Data Analysis
  Exploring sales data with Python.
```

#### Step 3: Add and execute a code cell

```bash
> /notebook cell add --type code --lang python --content "
import json

data = [
    {'product': 'Widget A', 'sales': 150, 'revenue': 4500},
    {'product': 'Widget B', 'sales': 230, 'revenue': 6900},
    {'product': 'Widget C', 'sales': 85, 'revenue': 2550},
]

total_revenue = sum(item['revenue'] for item in data)
print(f'Total revenue: \${total_revenue:,}')
print(f'Products: {len(data)}')
print(f'Top seller: {max(data, key=lambda x: x[\"sales\"])[\"product\"]}')
"
```

```
Cell [2] (python) added and executed:
  Total revenue: $13,950
  Products: 3
  Top seller: Widget B
  [exit code: 0, 0.12s]
```

#### Step 4: Run a cell with AI assistance

Ask the AI to generate a cell based on a description:

```bash
> /notebook cell generate "Create a bar chart of the sales data using matplotlib"
```

```
Cell [3] (python) generated and executed:
  import matplotlib.pyplot as plt

  products = ['Widget A', 'Widget B', 'Widget C']
  sales = [150, 230, 85]

  plt.figure(figsize=(8, 5))
  plt.bar(products, sales, color=['#4CAF50', '#2196F3', '#FF9800'])
  plt.xlabel('Product')
  plt.ylabel('Units Sold')
  plt.title('Sales by Product')
  plt.savefig('sales_chart.png', dpi=100, bbox_inches='tight')
  plt.show()
  print('Chart saved to sales_chart.png')

  Output:
  Chart saved to sales_chart.png
  [exit code: 0, 0.85s]
  [image: sales_chart.png embedded]
```

#### Step 5: List and reorder cells

```bash
> /notebook cells
```

```
data-analysis.vnb (3 cells):
  [1] markdown  # Data Analysis
  [2] python    import json... (output: Total revenue: $13,950)
  [3] python    import matplotlib... (output: Chart saved)
```

Move a cell:

```bash
> /notebook cell move 3 --before 2
```

#### Step 6: Export the notebook

```bash
> /notebook export data-analysis.vnb --format markdown --output analysis.md
> /notebook export data-analysis.vnb --format html --output analysis.html
> /notebook export data-analysis.vnb --format ipynb --output analysis.ipynb
```

```
Exported: analysis.md (Markdown with code blocks)
Exported: analysis.html (Standalone HTML with outputs)
Exported: analysis.ipynb (Jupyter-compatible notebook)
```

#### Step 7: Use the Notebook panel in VibeUI

Open VibeUI and navigate to the **Notebook** panel. The interface provides:

- **Cell toolbar** at the top with buttons for adding code or markdown cells, running all cells, and clearing outputs.
- **Code cells** with syntax highlighting, line numbers, and a "Run" button. Output appears directly below each cell.
- **Markdown cells** render as formatted text when not being edited. Click to edit the raw markdown.
- **Cell actions** on hover: move up/down, delete, duplicate, change language.
- **AI Generate** button opens a prompt where you describe what the cell should do, and VibeCody writes and executes it.
- **Export menu** with format options (Markdown, HTML, Jupyter, PDF).


### Script Runner: Execute Scripts with AI

#### Step 8: List available script templates

```bash
> /script list templates
```

```
Script Templates:
  backup-db        Backup a database with timestamp
  deploy-staging   Deploy to staging environment
  clean-build      Remove build artifacts and rebuild
  health-check     Check service health endpoints
  data-seed        Seed database with test data
  log-rotate       Rotate and compress log files
```

#### Step 9: Create a script from a template

```bash
> /script create --template health-check --name check-services
```

```
Created: scripts/check-services.sh
Template: health-check
Variables:
  SERVICES: (list of URLs to check)
  TIMEOUT: 5 (seconds)

Edit variables? [Y/n]: y
  SERVICES: http://localhost:3000,http://localhost:8080,http://localhost:5432
  TIMEOUT: 10
```

#### Step 10: Generate a script with AI

Describe what you need, and VibeCody writes the script:

```bash
> /script create --ai "Write a bash script that finds all TODO comments in the current project, counts them by file, and outputs a sorted summary"
```

```
Generated: scripts/todo-summary.sh

  #!/bin/bash
  # Find and summarize TODO comments across the project
  set -euo pipefail

  echo "TODO Summary"
  echo "============"
  echo ""

  grep -rn "TODO\|FIXME\|HACK\|XXX" \
    --include="*.rs" --include="*.ts" --include="*.tsx" \
    --include="*.py" --include="*.js" \
    . 2>/dev/null \
    | awk -F: '{print $1}' \
    | sort \
    | uniq -c \
    | sort -rn \
    | while read count file; do
        printf "%4d  %s\n" "$count" "$file"
      done

  total=$(grep -rn "TODO\|FIXME\|HACK\|XXX" \
    --include="*.rs" --include="*.ts" --include="*.tsx" \
    --include="*.py" --include="*.js" \
    . 2>/dev/null | wc -l)
  echo ""
  echo "Total: $total items"

Review and run? [Y/n]:
```

#### Step 11: Run a script

```bash
> /script run check-services
```

```
Running: scripts/check-services.sh
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  Checking http://localhost:3000... ✓ 200 OK (42ms)
  Checking http://localhost:8080... ✓ 200 OK (38ms)
  Checking http://localhost:5432... ✗ Connection refused

  Results: 2/3 healthy
  [exit code: 1, 0.95s]
```

Run directly from the shell:

```bash
vibecli --script run todo-summary
```

#### Step 12: View script history

```bash
> /script history
```

```
Script Execution History:
  #  Script            Status  Duration  Date
  1  check-services    FAIL    0.95s     2026-03-13 10:30
  2  todo-summary      OK      1.23s     2026-03-13 10:32
  3  check-services    OK      0.87s     2026-03-13 11:00
```

View output from a previous run:

```bash
> /script history show 2
```

#### Step 13: Use the Script panel in VibeUI

Open VibeUI and navigate to the **Scripts** panel. The interface provides:

- **Script list** on the left showing all scripts in the project's `scripts/` directory.
- **Editor** in the center with syntax highlighting for bash, Python, and other scripting languages.
- **Run button** with output displayed in an integrated terminal below the editor.
- **AI Generate** button opens a prompt to describe a script. VibeCody generates it with explanation comments.
- **Templates** tab shows available templates with a "Use Template" button that pre-fills the editor.
- **History** tab lists previous runs with status, duration, and output logs.

## Demo Recording

```json
{
  "meta": {
    "title": "Notebook & Scripts",
    "description": "Create interactive notebooks and run AI-generated scripts.",
    "duration_seconds": 300,
    "version": "1.0.0"
  },
  "steps": [
    {
      "id": 1,
      "action": "repl",
      "commands": [
        { "input": "/notebook create demo.vnb", "delay_ms": 1500 }
      ],
      "description": "Create a new VibeCody notebook"
    },
    {
      "id": 2,
      "action": "repl",
      "commands": [
        { "input": "/notebook cell add --type markdown --content \"# Demo Notebook\\nExploring VibeCody notebooks.\"", "delay_ms": 1500 }
      ],
      "description": "Add a markdown cell for documentation"
    },
    {
      "id": 3,
      "action": "repl",
      "commands": [
        { "input": "/notebook cell add --type code --lang python --content \"print('Hello from VibeCody!')\\nprint(2 + 2)\"", "delay_ms": 3000 }
      ],
      "description": "Add and execute a Python code cell"
    },
    {
      "id": 4,
      "action": "repl",
      "commands": [
        { "input": "/notebook cell generate \"Calculate fibonacci numbers up to 100 and print them\"", "delay_ms": 4000 }
      ],
      "description": "Generate a code cell using AI"
    },
    {
      "id": 5,
      "action": "repl",
      "commands": [
        { "input": "/notebook export demo.vnb --format markdown --output demo.md", "delay_ms": 1500 }
      ],
      "description": "Export notebook as Markdown"
    },
    {
      "id": 6,
      "action": "repl",
      "commands": [
        { "input": "/script list templates", "delay_ms": 1500 }
      ],
      "description": "List available script templates"
    },
    {
      "id": 7,
      "action": "repl",
      "commands": [
        { "input": "/script create --ai \"Write a bash script that counts lines of code by language in the current directory\"", "delay_ms": 5000 }
      ],
      "description": "Generate a script with AI assistance"
    },
    {
      "id": 8,
      "action": "repl",
      "commands": [
        { "input": "/script run todo-summary", "delay_ms": 3000 }
      ],
      "description": "Run a script and view output"
    },
    {
      "id": 9,
      "action": "repl",
      "commands": [
        { "input": "/script history", "delay_ms": 1500 }
      ],
      "description": "View script execution history"
    },
    {
      "id": 10,
      "action": "vibeui",
      "panels": ["Notebook", "Scripts"],
      "actions": ["create_notebook", "add_cells", "run_cells", "generate_script", "run_script"],
      "description": "Tour the Notebook and Scripts panels in VibeUI",
      "delay_ms": 5000
    }
  ]
}
```

## What's Next

- [Demo 25: SWE-bench Benchmarking](../swe-bench/) -- Benchmark your AI provider with SWE-bench
- [Demo 26: QA Validation Pipeline](../qa-validation/) -- Validate code with 8 QA agents
- [Demo 27: HTTP Playground](../http-playground/) -- Build and test API requests interactively
