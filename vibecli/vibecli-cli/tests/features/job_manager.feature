Feature: Durable async-job queue (JobManager) — M1 of the async-agents subsystem
  The JobManager owns an encrypted SQLite store of queued / running / terminal
  jobs plus an in-memory event stream registry for SSE fan-out. It is the
  foundation for fire-and-forget agents that outlive a single HTTP request.

  Scenario: Creating a job inserts a row in queued state
    Given a fresh JobManager
    When I create a job with task "ship the feature"
    Then the job should exist
    And the job status should be "queued"
    And the job task should be "ship the feature"
    And the job priority should be 5
    And the job queued_at should be non-zero
    And the job started_at should be zero

  Scenario: Marking a job running then terminal updates timestamps
    Given a fresh JobManager
    And a created job with task "do the thing"
    When I mark the job running
    Then the job status should be "running"
    And the job started_at should be non-zero
    When I mark the job complete with summary "all done"
    Then the job status should be "complete"
    And the job summary should be "all done"
    And the job finished_at should be non-zero

  Scenario: Cancelling a running job records the reason
    Given a fresh JobManager
    And a created job with task "cancel me"
    When I mark the job running
    And I cancel the job with reason "user requested"
    Then the job status should be "cancelled"
    And the job cancellation_reason should be "user requested"

  Scenario: recover_interrupted promotes queued and running jobs to failed
    Given a fresh JobManager
    And a created job with task "orphaned"
    When I mark the job running
    And I call recover_interrupted
    Then the recovery count should be 1
    And the job status should be "failed"
    And the job cancellation_reason should be "daemon restart"

  Scenario: list returns jobs sorted newest first by effective timestamp
    Given a fresh JobManager
    When I create a job with task "alpha"
    And I create a job with task "beta"
    And I mark the job "alpha" running
    And I wait 5 ms
    And I mark the job "beta" running
    Then the job list should have 2 entries
    And the first listed job task should be "beta"
    And the second listed job task should be "alpha"

  Scenario: Pre-M1 JSON jobs migrate once and running rows become failed
    Given a fresh JobManager
    And a legacy jobs directory with a running JSON record "legacy-1"
    When I call migrate_json_jobs
    Then the migration imported count should be 1
    And the job "legacy-1" status should be "failed"
    When I call migrate_json_jobs again
    Then the migration imported count should be 0

  Scenario: Event streams broadcast to active subscribers only
    Given a fresh JobManager
    And a created job with task "stream me"
    When I open a stream for the job
    And I subscribe to the job's stream
    And I publish a chunk event with content "hello"
    Then the subscriber should receive a chunk event with content "hello"
    When I close the stream
    Then subscribing to the job's stream should return nothing
