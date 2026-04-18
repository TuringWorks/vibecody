Feature: Context Assembler — single entry point for system context
  The context assembler replaces three ad-hoc injection sites in main.rs with
  one policy-driven function that pulls from project memory, orchestration
  rules, project profile, task-relevant files, and OpenMemory under an
  explicit budget. Phase 4 will tune the per-policy budget; Phase 2 just
  centralizes the contract.

  Scenario: Chat policy always emits the orchestration section
    Given a fresh workspace
    When the assembler runs with policy "chat"
    Then the assembled context includes a section named "orchestration"

  Scenario: Chat policy surfaces project memory when VIBECLI.md exists
    Given a fresh workspace
    And the workspace contains a "VIBECLI.md" file with body "ALPHA-RULES"
    When the assembler runs with policy "chat"
    Then the assembled context includes a section named "project_memory"
    And the section "project_memory" contains "ALPHA-RULES"

  Scenario: Budget exhaustion drops low-priority sections
    Given a fresh workspace
    And the workspace contains a "VIBECLI.md" file of size 5000
    When the assembler runs with policy "chat" and total budget 500
    Then the assembled context omits a section named "orchestration"
    And the assembled total chars are at most 600

  Scenario: Agent policy without OpenMemory toggles omits open_memory
    Given a fresh workspace
    When the assembler runs with policy "agent" and task "rewrite the auth flow" and OpenMemory disabled
    Then the assembled context omits a section named "open_memory"

  Scenario: Combined output joins sections with a separator
    Given a fresh workspace
    And the workspace contains a "VIBECLI.md" file with body "BETA-MARKER"
    When the assembler runs with policy "chat"
    Then the combined context contains "BETA-MARKER"
    And the combined context contains "---"

  Scenario: Agent scratchpad surfaces durable working state for a job
    Given a fresh workspace
    And job "job-42" has scratchpad entry "plan" with value "PLAN-LINE-ONE"
    When the assembler runs with policy "agent" for task "resume work" and job "job-42"
    Then the assembled context includes a section named "agent_scratchpad"
    And the section "agent_scratchpad" contains "PLAN-LINE-ONE"

  Scenario: Agent scratchpad has the highest priority under a tight budget
    Given a fresh workspace
    And job "job-42" has scratchpad entry "cursor" with value "CURSOR-MARKER"
    When the assembler runs with policy "agent" for task "resume work" and job "job-42" under total budget 300
    Then the assembled context includes a section named "agent_scratchpad"
    And the section "agent_scratchpad" contains "CURSOR-MARKER"

  Scenario: CodingAgent budget allocates more to task_files than to open_memory
    Then the budget for kind "coding" caps "task_files" higher than "open_memory"

  Scenario: ResearchAgent budget allocates more to open_memory than to task_files
    Then the budget for kind "research" caps "open_memory" higher than "task_files"

  Scenario: BackgroundJob budget is scratchpad-dominant
    Then the budget for kind "background" caps "agent_scratchpad" higher than "task_files"

  Scenario: Chat budget is compact — not the giant default
    Then the budget for kind "chat" has total at most 64000
