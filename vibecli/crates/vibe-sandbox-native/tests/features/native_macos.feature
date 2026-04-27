Feature: macOS native sandbox via sandbox-exec
  The MacosSandbox builds a structured .sb (Sandbox Profile Language)
  profile and runs commands under sandbox-exec. Reads/writes outside
  bound paths are denied; reads/writes inside bound paths succeed.

  Scenario: bind_rw lets the sandboxed shell write inside the bound dir
    Given a fresh macOS sandbox
    And a temporary directory bound rw at "/work"
    When I spawn "/bin/sh -c 'echo hi > $WORKDIR/out.txt'"
    Then the spawn exit code is 0
    And the host file "out.txt" inside the bound dir contains "hi"

  Scenario: deny default — read outside any bound path is blocked
    Given a fresh macOS sandbox
    And a temporary directory bound rw at "/work"
    When I spawn "/bin/sh -c 'cat /etc/master.passwd >/dev/null 2>&1; echo $?'"
    Then the spawn stdout matches "1"

  Scenario: bind_ro disallows writes inside the read-only bind
    Given a fresh macOS sandbox
    And a temporary directory bound ro at "/ro"
    When I spawn "/bin/sh -c 'echo nope > $RODIR/out.txt 2>/dev/null; echo $?'"
    Then the spawn stdout matches "1"

  Scenario: profile renders deny-default and explicit allows
    Given a fresh macOS sandbox profile
    When I render the profile
    Then the rendered profile starts with "(version 1)"
    And the rendered profile contains "(deny default)"

  Scenario: profile rejects path traversal in subpath spec
    Given a fresh macOS sandbox profile
    When I add a subpath that contains ".." in the middle
    Then the profile build returns an error
