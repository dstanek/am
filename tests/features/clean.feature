Feature: am clean — remove a session and its worktree

  Background:
    Given a git repository

  Scenario: clean with --force removes the worktree and session record
    Given a session "my-feature" has been started
    When I run "am clean --force my-feature"
    Then the command succeeds
    And the output contains "Cleaned session 'my-feature'"
    And the worktree ".am/worktrees/my-feature" does not exist
    And the session file does not contain "my-feature"

  Scenario: clean fails when the session does not exist
    When I run "am clean --force no-such-session"
    Then the command fails
    And the output contains "not found"

  Scenario: clean without --force aborts when user says no
    Given a session "my-feature" has been started
    When I run "am clean my-feature" with input "n"
    Then the command succeeds
    And the output contains "Aborted"
    And a worktree exists at ".am/worktrees/my-feature"
    And the session file contains "my-feature"

  Scenario: clean without --force proceeds when user confirms
    Given a session "my-feature" has been started
    When I run "am clean my-feature" with input "y"
    Then the command succeeds
    And the worktree ".am/worktrees/my-feature" does not exist
    And the session file does not contain "my-feature"
