Feature: Subprocess dispatch (JobManager → Noise_NNpsk0 → vibecli worker)
  M7 of the async-agents subsystem. The parent daemon spawns a child
  `vibecli worker` process over a socketpair, handshakes with a per-job
  PSK, and streams encrypted JSON frames. The stub agent loop echoes the
  task back so this harness can validate the end-to-end dispatch contract
  before the real AI agent is moved into the subprocess (M7.2b).

  Scenario: JobManager dispatches a queued job to a real worker subprocess
    Given a JobManager backed by a fresh encrypted db
    And a queued job with task "hello from bdd"
    When I dispatch the job to a child worker using the test binary
    Then the job should eventually reach status "complete"
    And the job summary should contain "hello from bdd"
    And the broadcast stream should have delivered at least one chunk event
