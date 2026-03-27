Feature: am start — create an isolated agent session

  Background:
    Given a git repository

  Scenario: start a session creates a worktree and records state
    When I run "am start my-feature"
    Then the command succeeds
    And a worktree exists at ".am/worktrees/my-feature"
    And the session file contains "my-feature"

  Scenario: starting a duplicate session fails
    Given a session "my-feature" has been started
    When I run "am start my-feature"
    Then the command fails
    And the output contains "already exists"
