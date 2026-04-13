Feature: Remote agent dispatch queue
  Jobs are enqueued from remote clients, polled for status, and dequeued
  by workers in priority order (FIFO within equal priority).

  Scenario: Enqueue a job and poll its status
    Given an empty dispatch queue
    When I enqueue a job with prompt "run tests" at time 0
    Then polling the job should return status Queued

  Scenario: Higher priority job is dequeued first
    Given an empty dispatch queue
    When I enqueue a job with prompt "low" at priority 10 at time 0
    And I enqueue a job with prompt "high" at priority 200 at time 1
    Then dequeuing the next job should return prompt "high"

  Scenario: Mark a job running then completed
    Given an empty dispatch queue
    When I enqueue a job with prompt "compile" at time 0
    And I mark the job as running
    And I mark the job as completed with output "success"
    Then polling the job should return status Completed

  Scenario: Pending count tracks only queued jobs
    Given an empty dispatch queue
    When I enqueue a job with prompt "job-a" at time 0
    And I enqueue a job with prompt "job-b" at time 1
    Then the pending count should be 2
    When I mark job "job-a" as running
    Then the pending count should be 1
