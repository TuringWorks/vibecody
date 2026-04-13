Feature: Alternative exploration tournament
  The tournament scores agent candidates by test pass rate, diff size,
  and compile success, then selects the highest-scoring winner.

  Scenario: Perfect candidate scores 1.0
    Given a candidate with pass_rate 1.0 diff_lines 0 and compile true
    When I score the candidate
    Then the score should be 1.0

  Scenario: Non-compiling candidate scores at most 0.8
    Given a candidate with pass_rate 1.0 diff_lines 0 and compile false
    When I score the candidate
    Then the score should be less than or equal to 0.8

  Scenario: Higher-scoring candidate is ranked first
    Given two candidates where "strong" has higher score than "weak"
    When I rank the candidates
    Then the first candidate should be "strong"

  Scenario: Disqualification removes non-compiling candidates when required
    Given two candidates where "good" compiles and "bad" does not
    And min_compile_required is true
    When I disqualify non-compiling candidates
    Then only "good" should remain
