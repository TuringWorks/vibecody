---
triggers: ["scientific research", "research paper", "experiment", "hypothesis", "statistical analysis", "reproducibility", "peer review", "citation", "LaTeX", "scientific computing", "research workflow"]
tools_allowed: ["read_file", "write_file", "bash"]
category: scientific
---

# Scientific Research Workflow

When supporting scientific research, computational experiments, and paper writing:

1. Structure projects for reproducibility: `data/raw/`, `data/processed/`, `src/`, `notebooks/`, `results/`, `figures/`, `paper/` — use a Makefile or Snakemake to automate the full pipeline from raw data to final figures.
2. Use version control for everything except large data: track code, configs, notebooks, and paper source in Git; use `git-lfs` or DVC (`dvc add data/raw/`) for datasets too large for Git.
3. Pin all dependencies: `pip freeze > requirements.txt` or `conda env export > environment.yml` — include Python version, OS, and hardware notes in a `REPRODUCIBILITY.md` for exact replication.
4. Write experiments as deterministic scripts: set `random.seed(42)`, `np.random.seed(42)`, `torch.manual_seed(42)` — document any non-deterministic components (GPU operations, parallel I/O).
5. Use `pandas` + `scipy.stats` for statistical analysis — report effect sizes alongside p-values; use `statsmodels` for regression, ANOVA, and mixed-effects models with proper multiple comparison correction (Bonferroni, FDR).
6. Visualize results with `matplotlib` for publication-quality figures: `plt.rcParams.update({'font.size': 12, 'figure.dpi': 300, 'savefig.bbox': 'tight'})` — save as PDF/SVG for vector graphics in papers.
7. Use Jupyter notebooks for exploration but extract finalized analysis into `.py` scripts — notebooks are for storytelling and presentation, scripts are for the reproducible pipeline.
8. Write papers in LaTeX with BibTeX: use `\cite{author2024}` for citations, `\label{fig:results}` + `\ref{fig:results}` for cross-references — keep `.bib` files organized with consistent citation keys.
9. Track experiment results with `MLflow`, `Weights & Biases`, or a simple CSV/JSON log — record hyperparameters, metrics, timestamps, and git commit hashes for every run.
10. Use `pytest` for testing research code: test data loaders, preprocessing functions, and metric calculations — a test that verifies known analytical solutions catches subtle bugs before they corrupt results.
11. Apply the scientific method in code: state the hypothesis, design the experiment (controlled variables), run the analysis, and interpret results before writing the paper section.
12. For large-scale experiments: use `SLURM` or `PBS` job schedulers — write job arrays for parameter sweeps: `sbatch --array=0-99 experiment.sh` with parameters read from a config file indexed by `$SLURM_ARRAY_TASK_ID`.
13. Use `Snakemake` or `Nextflow` for complex multi-step pipelines — define rules with inputs/outputs so only changed steps re-run: `rule align: input: "data/{sample}.fastq", output: "results/{sample}.bam"`.
14. Document methods thoroughly: describe preprocessing steps, hyperparameters, evaluation metrics, dataset splits, and hardware used — another researcher should reproduce your results from the methods section alone.
