Feature: full session lifecycle

  Scenario: start then list then clean
    Given a git repository
    When I run "am start my-feature"
    Then the command succeeds
    When I run "am list"
    Then the command succeeds
    And the output contains "my-feature"
    When I run "am clean --force my-feature"
    Then the command succeeds
    When I run "am list"
    Then the command succeeds
    And the output contains "No active sessions"

  Scenario: multiple sessions can coexist
    Given a git repository
    When I run "am start feature-a"
    Then the command succeeds
    When I run "am start feature-b"
    Then the command succeeds
    When I run "am list"
    Then the command succeeds
    And the output contains "feature-a"
    And the output contains "feature-b"
