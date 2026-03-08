---
triggers: ["statistics", "statistical analysis", "SPSS", "SAS", "Stata", "research methods", "hypothesis testing", "regression analysis"]
tools_allowed: ["read_file", "write_file", "bash"]
category: science
---

# Statistics and Research Methods

When working with statistics and research methods:

1. Begin with descriptive statistics (mean, median, mode, standard deviation, skewness, kurtosis) to understand data distributions before applying inferential methods. Visualize distributions with histograms, box plots, and Q-Q plots to check assumptions.

2. Formulate clear null and alternative hypotheses before conducting tests. Select appropriate tests based on data type, distribution, sample size, and independence: t-tests for means, chi-square for categorical data, ANOVA for multiple groups, and non-parametric alternatives when assumptions are violated.

3. Build regression models systematically: check linearity, independence, homoscedasticity, and normality of residuals. For multiple regression, assess multicollinearity via VIF, use stepwise or criterion-based selection, and report R-squared, adjusted R-squared, and coefficient confidence intervals.

4. Design experiments with proper controls, randomization, and blinding. Choose between completely randomized, randomized block, factorial, or crossover designs based on research questions. Account for confounders and interaction effects.

5. Apply appropriate sampling strategies (simple random, stratified, cluster, systematic, convenience) based on population characteristics and research constraints. Calculate required sample sizes and document sampling frame limitations.

6. Use statistical software effectively: SPSS for survey analysis and ANOVA, SAS for clinical trials and large datasets, Stata for panel data and econometrics, R for custom analyses and visualization, Python (scipy/statsmodels) for programmatic workflows. Document software versions and analysis scripts for reproducibility.

7. Conduct power analysis before data collection to determine minimum sample sizes for desired effect sizes, significance levels, and statistical power (typically 0.80). Report post-hoc power only with appropriate caveats.

8. Apply non-parametric tests (Mann-Whitney U, Wilcoxon signed-rank, Kruskal-Wallis, Spearman correlation, Friedman test) when data violate parametric assumptions or involve ordinal scales. Understand their trade-offs in statistical power.

9. Handle time series data with stationarity checks (ADF test), appropriate differencing, ACF/PACF analysis for ARIMA model identification, seasonal decomposition, and forecast validation using holdout sets or cross-validation.

10. Apply Bayesian methods when prior information is available or frequentist assumptions are problematic. Specify priors transparently, use MCMC for posterior estimation, report credible intervals, and conduct sensitivity analysis on prior choices.

11. Clean data systematically: identify missing data mechanisms (MCAR, MAR, MNAR), apply appropriate imputation methods (mean, regression, multiple imputation), detect outliers using statistical criteria (IQR, Mahalanobis distance), and document all transformations.

12. Create effective statistical visualizations: use scatter plots for correlations, forest plots for meta-analyses, Kaplan-Meier curves for survival data, and heatmaps for correlation matrices. Follow principles of clarity, accuracy, and minimal chart junk. Always include confidence intervals or error bars.
