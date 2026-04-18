Feature: Session FTS5 search for long-running agent self-recall
  The session store indexes every message with SQLite FTS5 so an agent that has been
  paused — or that is reasoning about work it did days ago — can retrieve its own
  past conversations with bm25 ranking and project-scoped filtering.

  Scenario: Agent searches across all projects and finds messages by content
    Given a session "s1" in project "/p/alpha" with a user message "please refactor the authentication flow"
    And a session "s2" in project "/p/beta" with an assistant message "deployed the billing service"
    When the agent searches for "refactor" with no scope
    Then exactly 1 hit is returned
    And the top hit belongs to session "s1"

  Scenario: Project scope filters out matches from other projects
    Given a session "s1" in project "/p/alpha" with a user message "deploy the service"
    And a session "s2" in project "/p/beta" with a user message "deploy the service"
    When the agent searches for "deploy" scoped to project "/p/alpha"
    Then exactly 1 hit is returned
    And the top hit belongs to session "s1"

  Scenario: All-scope search returns matches regardless of project
    Given a session "s1" in project "/p/alpha" with a user message "deploy the service"
    And a session "s2" in project "/p/beta" with a user message "deploy the service"
    When the agent searches for "deploy" with no scope
    Then exactly 2 hits are returned

  Scenario: Deleting a session removes its messages from the search index
    Given a session "s1" in project "/p/x" with a user message "one of a kind phrase marmalade"
    When the session "s1" is deleted
    And the agent searches for "marmalade" with no scope
    Then exactly 0 hits are returned

  Scenario: Snippets include highlight markers around matched terms
    Given a session "s1" in project "/p/x" with a user message "the quick brown fox jumped over the lazy dog many times"
    When the agent searches for "fox" with no scope
    Then the top hit snippet contains "<mark>"
