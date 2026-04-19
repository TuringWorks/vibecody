Feature: Proactive scanner does real filesystem I/O (US-006)
  ProactiveScanner walks the project tree, categorizes files by
  extension, runs the SuggestionGenerator for each category, and
  exposes a notify-backed watcher that fires when files change on
  disk. None of this relies on simulated input lists any more.

  Scenario: discover_files walks a real directory tree
    Given a temp project with files:
      | src/main.rs    |
      | src/lib.rs     |
      | web/app.tsx    |
      | docs/README.md |
    When the scanner discovers files under the project root
    Then the discovered set contains path "src/main.rs"
    And the discovered set contains path "web/app.tsx"
    And the discovered count is 4

  Scenario: discover_files skips common ignored directories
    Given a temp project with files:
      | src/main.rs           |
      | target/debug/x.rs     |
      | node_modules/lib.js   |
      | .git/config           |
    When the scanner discovers files under the project root
    Then the discovered count is 1
    And the discovered set contains path "src/main.rs"

  Scenario: scan_project yields suggestions for supported source files
    Given a temp project with files:
      | src/main.rs |
      | web/app.tsx |
    When the scanner scans the project for categories "Performance,Security"
    Then the scan produces at least 2 suggestions

  Scenario: notify watcher fires when a new file is written
    Given a temp project with files:
      | src/seed.rs |
    When a watcher is started on the project root
    And a new file "src/touched.rs" is written
    Then the watcher reports an event for "touched.rs" within 5 seconds
