# Remote Dispatch Queue
Queue agent jobs from mobile or web clients, track status, and dequeue for execution in priority order.

## When to Use
- Accepting jobs from remote/mobile frontends and processing them asynchronously
- Implementing a priority work queue for background agent tasks
- Tracking job lifecycle from Queued → Running → Completed/Failed

## Commands
- `DispatchQueue::new()` — create an empty queue
- `queue.enqueue(prompt, now_secs)` — add a job at default priority (128); returns job id
- `queue.enqueue_with_priority(prompt, priority, now_secs)` — add with explicit priority (0–255)
- `queue.poll(job_id)` — get current `PollResult` (status + optional output)
- `queue.dequeue_next()` — pop the highest-priority queued job (FIFO within equal priority)
- `queue.mark_running(job_id)` — transition job to Running
- `queue.mark_completed(job_id, output)` — transition job to Completed
- `queue.mark_failed(job_id, reason)` — transition job to Failed
- `queue.pending_count()` — number of jobs in Queued state
- `queue.job_count()` — total jobs in queue

## Examples
```rust
let mut q = DispatchQueue::new();
let id = q.enqueue_with_priority("run integration tests", 200, unix_now());

// Worker loop
while let Some(job) = q.dequeue_next() {
    q.mark_running(&job.id);
    let result = execute(&job.prompt);
    q.mark_completed(&job.id, result);
}

let r = q.poll(&id).unwrap();
println!("status: {:?}", r.status);
```
