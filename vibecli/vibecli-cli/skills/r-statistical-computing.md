---
triggers: ["R language", "R statistics", "ggplot2", "tidyverse", "dplyr", "R markdown", "Shiny", "CRAN", "Bioconductor", "R programming", "statistical computing R"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["Rscript"]
category: r-lang
---

# R Statistical Computing

When writing R code for statistical analysis, visualization, and data science:

1. Use the `tidyverse` ecosystem: `library(tidyverse)` loads `dplyr`, `ggplot2`, `tidyr`, `readr`, `purrr`, `stringr`, `forcats`, `tibble` — these form a consistent, pipe-friendly data analysis toolkit.
2. Use the pipe operator for readable chains: `df %>% filter(age > 30) %>% mutate(decade = age %/% 10) %>% group_by(decade) %>% summarise(mean_salary = mean(salary))` — native pipe `|>` available in R 4.1+.
3. Read data with `readr`: `read_csv("data.csv")` is faster and more consistent than base `read.csv()` — it guesses types well, handles encoding, and returns a tibble with sensible defaults.
4. Visualize with `ggplot2`: `ggplot(df, aes(x=time, y=value, color=group)) + geom_line() + theme_minimal() + labs(title="Results", x="Time (s)", y="Value")` — layer geoms, facets, and themes.
5. For statistical tests: `t.test(x, y, paired=TRUE)` for paired t-test; `wilcox.test()` for non-parametric; `aov()` for ANOVA; `lm(y ~ x1 + x2, data=df)` for linear regression — always check assumptions.
6. Use `broom` to tidy model outputs: `tidy(model)` gives coefficient table as a tibble; `glance(model)` gives summary statistics; `augment(model)` adds fitted values and residuals to the data.
7. Handle factors explicitly: `mutate(group = factor(group, levels=c("control", "treatment")))` — R's default alphabetical factor ordering can silently change plot ordering and contrast coding.
8. Reshape data with `tidyr`: `pivot_longer(df, cols=c(pre, post), names_to="time", values_to="score")` for wide-to-long; `pivot_wider()` for long-to-wide — tidy data has one observation per row.
9. Use `purrr::map()` for functional iteration: `map_dbl(models, ~summary(.)$r.squared)` — replaces `lapply`/`sapply` with type-safe variants (`map_chr`, `map_lgl`, `map_dfr`).
10. Write reproducible reports with R Markdown: `rmarkdown::render("analysis.Rmd")` generates HTML/PDF/Word with embedded code, figures, and narrative — use `knitr::kable()` for formatted tables.
11. Build interactive apps with Shiny: `shinyApp(ui = fluidPage(plotOutput("plot")), server = function(input, output) { output$plot <- renderPlot({ ... }) })` — reactive programming for data dashboards.
12. Install Bioconductor packages for genomics: `BiocManager::install("DESeq2")` — Bioconductor provides 2000+ packages for genomics, proteomics, and high-throughput assay analysis.
13. Use `testthat` for testing: `test_that("mean works", { expect_equal(mean(1:10), 5.5) })` — organize tests in `tests/testthat/` and run with `devtools::test()`.
14. Profile with `profvis::profvis({ slow_function() })` for interactive flame graphs — identify bottlenecks; use `Rcpp` for performance-critical inner loops: `cppFunction('double fast_sum(NumericVector x) { ... }')`.
15. Manage environments with `renv`: `renv::init()` creates a project-local library; `renv::snapshot()` captures package versions in `renv.lock` for reproducible environments across machines.
