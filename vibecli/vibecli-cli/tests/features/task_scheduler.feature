Feature: Cron/interval task scheduler
  The Scheduler manages ScheduledTask entries. Tasks fire when is_due()
  returns true. tick() marks due tasks as run and advances their next_run_secs.
  Once tasks complete after a single run.

  Scenario: Interval task is due after its period has elapsed
    Given a scheduler with an interval task "heartbeat" every 60 seconds starting at time 0
    When I tick the scheduler at time 60
    Then the ticked ids should include "heartbeat"

  Scenario: Task is not due before its period
    Given a scheduler with an interval task "heartbeat" every 60 seconds starting at time 0
    When I check due tasks at time 30
    Then no tasks should be due

  Scenario: Once task fires exactly at its scheduled time
    Given a scheduler with a one-time task "deploy" at time 1000 created at time 0
    When I check due tasks at time 1000
    Then the due task should be "deploy"

  Scenario: Task is removed by id
    Given a scheduler with an interval task "cleanup" every 300 seconds starting at time 0
    When I remove task "cleanup"
    Then the scheduler should have 0 tasks
