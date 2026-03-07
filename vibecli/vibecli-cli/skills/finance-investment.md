---
triggers: ["investment", "portfolio", "asset allocation", "portfolio optimization", "Markowitz", "Sharpe ratio", "alpha", "beta", "risk-adjusted return", "backtesting", "rebalancing", "robo-advisor"]
tools_allowed: ["read_file", "write_file", "bash"]
category: finance
---

# Finance - Investment & Portfolio Management

When working with investment and portfolio management systems:

1. Implement mean-variance optimization (Markowitz) by constructing the covariance matrix from historical returns and solving the quadratic programming problem to find the efficient frontier. Use shrinkage estimators (Ledoit-Wolf) on the covariance matrix to reduce estimation error, and constrain weights to prevent unrealistic short positions or excessive concentration.

2. Calculate risk metrics comprehensively: Value at Risk (VaR) using parametric, historical simulation, and Monte Carlo methods; Conditional VaR (Expected Shortfall) for tail risk; Sharpe ratio for risk-adjusted return; Sortino ratio using only downside deviation; maximum drawdown for peak-to-trough loss; and tracking error against the benchmark. Present all metrics with confidence intervals, not just point estimates.

3. Build backtesting frameworks with strict separation between the strategy logic and the simulation engine. Prevent look-ahead bias by ensuring the strategy only accesses data available at each point in time. Account for survivorship bias by including delisted securities, and model realistic transaction costs, slippage, and market impact rather than assuming frictionless execution.

4. Implement factor models (Fama-French three-factor, Carhart four-factor, or custom multi-factor) by regressing portfolio returns against factor returns. Decompose alpha into factor exposures to determine whether outperformance comes from genuine skill or from unintended factor tilts like size, value, or momentum.

5. Design rebalancing strategies with configurable triggers: calendar-based (monthly, quarterly), threshold-based (rebalance when any asset drifts beyond a tolerance band), or hybrid. Compare the cost of rebalancing (transaction fees, tax impact, market impact) against the tracking error cost of not rebalancing to find the optimal frequency.

6. Implement tax-loss harvesting by scanning the portfolio for positions with unrealized losses, selling them to realize the loss for tax offset, and simultaneously purchasing a correlated but not substantially identical replacement to maintain market exposure. Track wash sale windows (30 days before and after) and adjust cost basis accordingly.

7. Build benchmark tracking by computing daily return attribution: allocation effect (over/underweight in sectors vs benchmark), selection effect (stock picking within sectors), and interaction effect. Store benchmark constituent weights and returns as time series so historical tracking error and information ratio can be computed over rolling windows.

8. Integrate ESG scoring by normalizing scores from multiple providers (MSCI, Sustainalytics, Bloomberg) onto a common scale, handling missing data with sector-median imputation. Support ESG-constrained optimization where minimum portfolio ESG scores or maximum carbon intensity limits are added as constraints to the optimizer alongside return and risk objectives.

9. Implement portfolio attribution at multiple levels: asset class attribution, sector attribution, and security-level attribution. Use Brinson-Hood-Beebower for equity portfolios and duration/credit/curve attribution for fixed income. Store attribution results daily so cumulative effects can be analyzed over any reporting period.

10. Run Monte Carlo simulations for portfolio projection by sampling from the joint return distribution (using copulas for non-normal dependencies). Generate thousands of paths to produce probability cones for wealth outcomes, estimate the probability of meeting return targets, and stress-test against historical crisis scenarios (2008, 2020, rate shocks).

11. Design the order management system (OMS) integration layer to translate portfolio target weights into trade orders. Calculate the trade list as the difference between target and current holdings, apply trade minimums and round-lot constraints, route orders to the appropriate execution venue, and reconcile fills back to update portfolio positions in real time.

12. Implement robo-advisor logic with a risk questionnaire that maps to a model portfolio on the efficient frontier. Automate the full lifecycle: initial investment allocation, ongoing rebalancing, dividend reinvestment, tax-loss harvesting, and glide-path adjustment as the client ages or risk tolerance changes. Log every automated decision for regulatory transparency.

13. Handle multi-asset portfolio construction spanning equities, fixed income, alternatives, and cash. Use Black-Litterman to blend market-implied equilibrium returns with investor views, producing more stable and intuitive allocations than pure mean-variance. Constrain illiquid allocations to respect redemption schedules and commitment pacing.

14. Build a comprehensive position management layer that tracks cost basis using specific lot identification (FIFO, LIFO, highest-cost, tax-optimal), handles corporate actions (splits, mergers, spin-offs, dividends), and maintains accurate realized and unrealized gain/loss calculations across multiple tax lots per security.

15. Implement risk budgeting by allocating the portfolio's total risk (measured as volatility or VaR) across asset classes or strategies. Use risk parity to equalize each asset's marginal contribution to risk, and monitor risk consumption in real time so that any strategy exceeding its risk budget triggers an alert or automatic position reduction.
