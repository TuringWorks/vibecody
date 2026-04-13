Feature: Priority-queue task scheduler
  The TaskScheduler enqueues tasks with priorities and optional run-after
  timestamps, then dequeues them in priority order when they become eligible.

  Scenario: High-priority task is dequeued before low-priority
    Given a scheduler with a Low task "low-task" and a High task "high-task"
    When I pop a ready task at time 0
    Then the task id should be "high-task"

  Scenario: Future-scheduled task is not returned before its time
    Given a scheduler with a Normal task "future" scheduled at time 1000
    When I pop a ready task at time 0
    Then no task should be ready

  Scenario: Future task becomes ready after its scheduled time
    Given a scheduler with a Normal task "future" scheduled at time 1000
    When I pop a ready task at time 1000
    Then the task id should be "future"

  Scenario: Scheduler is empty after all tasks are dequeued
    Given a scheduler with a Normal task "only-task" and no delay
    When I pop a ready task at time 0
    Then the task id should be "only-task"
    And the scheduler should be empty
