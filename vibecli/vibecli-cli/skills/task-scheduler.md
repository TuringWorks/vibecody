# Task Scheduler
Cron/interval/once task scheduler — compute next-run times, detect due tasks, and manage a task registry.

## When to Use
- Scheduling recurring background jobs (log rotation, health checks, model refresh)
- One-shot deferred tasks triggered at a specific Unix timestamp
- Simplified daily-cron scheduling by hour and minute

## Commands
- `Schedule::Interval { secs }` — repeat every N seconds
- `Schedule::Once { at_secs }` — fire exactly once at timestamp
- `Schedule::Cron { hour, minute }` — fire daily at HH:MM
- `Schedule::next_after(from)` — compute next trigger time
- `Scheduler::add(task)` — register a task
- `Scheduler::due_tasks(now)` — list overdue tasks
- `Scheduler::tick(now)` — advance all due tasks, return their IDs
- `Scheduler::remove(id)` — cancel a task by ID

## Examples
```rust
use vibecli_cli::task_scheduler::{CronTask, Schedule, Scheduler};

let mut sched = Scheduler::new();
sched.add(CronTask::new("cleanup", "Nightly cleanup", "rm -rf /tmp/cache", Schedule::Cron { hour: 2, minute: 0 }, 0));
sched.add(CronTask::new("ping", "Health ping", "curl localhost/health", Schedule::Interval { secs: 30 }, 0));

// Advance time and collect fired task IDs
let fired = sched.tick(30);
assert_eq!(fired, vec!["ping"]);
```
