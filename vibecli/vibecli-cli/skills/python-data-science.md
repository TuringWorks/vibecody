---
triggers: ["pandas", "numpy", "matplotlib", "jupyter", "dataframe", "data analysis python", "seaborn", "plotly"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["python3"]
category: python
---

# Python Data Science

When doing data analysis and visualization:

1. Use `pandas` for tabular data — read with `pd.read_csv()`, `read_parquet()`, `read_sql()`
2. Avoid iterating DataFrames with `for` loops — use vectorized operations and `.apply()` as last resort
3. Use `df.groupby('col').agg({'val': ['mean', 'sum']})` for aggregations
4. Chain operations: `df.query('age > 30').assign(decade=lambda x: x.age // 10).sort_values('salary')`
5. Use `numpy` for numerical computation — broadcasting, linear algebra, random sampling
6. Visualize with `matplotlib` for publication-ready plots; `seaborn` for statistical visualizations
7. Use `%matplotlib inline` in Jupyter; `plt.figure(figsize=(10, 6))` for readable sizes
8. Handle missing data: `df.isna().sum()`, `df.fillna()`, `df.dropna(subset=['critical_col'])`
9. Use `df.dtypes` and `df.describe()` for initial data exploration
10. Use `pd.to_datetime()` for date parsing; `.dt` accessor for date components
11. Save results: `df.to_parquet('output.parquet')` (faster, smaller) over CSV
12. Use `tqdm` for progress bars on long operations: `df.progress_apply()` with `tqdm.pandas()`
