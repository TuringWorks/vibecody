# perf-regression

Automated performance regression detection using statistical baselines.

## Usage

```
/perf baseline <benchmark> <value> [--unit ms]   # set or update baseline
/perf record <benchmark> <value>                  # record observation + check
/perf analyze                                     # analyze all recorded samples
/perf report                                      # show regression summary
```

## Features

- Baseline statistics: mean + standard deviation from historical samples
- Z-score detection (default threshold: 2.0σ) — configurable
- Severity tiers: Minor (5–10%), Major (10–25%), Critical (>25%)
- Batch analysis across all benchmarks
- Automatic baseline computation from recorded history
- History ring buffer (default: 100 samples per benchmark)

## Severity Levels

| Severity | % Degradation |
|----------|--------------|
| None | < 5% |
| Minor | 5–10% |
| Major | 10–25% |
| Critical | > 25% |

## Example

```
> /perf record api_latency_ms 142
  ✓ OK — baseline: 100ms ± 8ms, observed: 142ms (+42%)
  🔴 CRITICAL regression detected in api_latency_ms

> /perf report
1 regression(s) detected:
  [critical] api_latency_ms — 42.0% slower (baseline: 100.00, observed: 142.00)
```

## Module

`vibecli/vibecli-cli/src/perf_regression.rs`
