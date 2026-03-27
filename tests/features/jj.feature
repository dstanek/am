Feature: jj workspace support — sessions in jj repos

  Background:
    Given a jj repository

  Scenario: start creates a jj workspace and records the session
    When I run "am start my-feature"
    Then the command succeeds
    And a worktree exists at ".am/worktrees/my-feature"
    And the session file contains "my-feature"

  Scenario: clean removes the jj workspace and session record
    Given a session "my-feature" has been started
    When I run "am clean --force my-feature"
    Then the command succeeds
    And the worktree ".am/worktrees/my-feature" does not exist
    And the session file does not contain "my-feature"
