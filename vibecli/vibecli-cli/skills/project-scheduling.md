---
triggers: ["project scheduling", "Gantt chart", "CPM", "PERT", "Microsoft Project", "Primavera", "resource planning", "WBS"]
tools_allowed: ["read_file", "write_file", "bash"]
category: management
---

# Project Scheduling

When working with project scheduling:

1. Develop a Work Breakdown Structure (WBS) using a deliverable-oriented decomposition. Follow the 100% rule (each level captures all work), apply the 8/80 rule for work package duration, assign unique WBS codes, and create a WBS dictionary defining scope, assumptions, and acceptance criteria for each element.

2. Apply the Critical Path Method (CPM) by identifying all activities, sequencing them with dependencies (FS, FF, SS, SF), estimating durations, performing forward and backward passes, and calculating total and free float. Focus management attention on zero-float critical path activities.

3. Use PERT three-point estimation (optimistic, most likely, pessimistic) with the weighted formula (O + 4M + P) / 6 for duration estimates. Calculate standard deviation for each activity and use the sum of variances along the critical path to determine project completion probability.

4. Build Gantt charts that clearly show activity bars, milestones (zero-duration diamonds), summary tasks, dependencies, critical path highlighting, baseline vs. actual progress, and resource assignments. Keep the chart readable by rolling up detail for executive audiences.

5. Perform resource leveling to resolve over-allocations by adjusting activity timing within float, splitting tasks, or extending the schedule. Distinguish between resource leveling (may extend schedule) and resource smoothing (within existing float only). Track resource utilization rates.

6. Define milestones at meaningful project checkpoints: phase gates, deliverable completions, external dependencies, and decision points. Milestones should be binary (achieved or not), measurable, and tied to specific acceptance criteria.

7. Model dependencies accurately using all four types: Finish-to-Start (most common), Start-to-Start, Finish-to-Finish, and Start-to-Finish. Apply leads (negative lag) and lags judiciously and document the rationale for each non-standard dependency.

8. Implement Earned Value Management (EVM) using planned value (PV), earned value (EV), and actual cost (AC) to calculate SPI, CPI, EAC, and ETC. Set thresholds for variance that trigger corrective action and report trends over time.

9. Use Microsoft Project effectively: set the project calendar, define task modes (auto vs. manual), link tasks with dependencies, assign resources with effort-driven scheduling, set baselines before execution, and use tracking Gantt views for progress monitoring.

10. Leverage Primavera P6 for large-scale projects: define OBS/EPS/WBS hierarchy, use activity codes for filtering, apply resource calendars, run schedule analysis (longest path, multiple float paths), and generate time-scaled logic diagrams and resource histograms.

11. Apply schedule compression techniques when deadlines are at risk: crashing (add resources to critical path activities with lowest cost slope) and fast-tracking (overlap sequential activities accepting increased risk). Document cost and risk trade-offs for each option.

12. Produce clear progress reports with percent complete by activity, schedule variance narrative, look-ahead schedules (2-4 weeks), critical path status, milestone tracker, risk-adjusted forecasts, and change log. Tailor detail level to the audience (team, management, client).
