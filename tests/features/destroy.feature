Feature: am destroy — remove a session and its worktree

  Background:
    Given a git repository

  Scenario: destroy with --force removes the worktree and session record
    Given a session "my-feature" has been started
    When I run "am destroy --force my-feature"
    Then the command succeeds
    And the output contains "Destroyed session 'my-feature'"
    And the worktree ".am/worktrees/my-feature" does not exist
    And the session file does not contain "my-feature"

  Scenario: destroy fails when the session does not exist
    When I run "am destroy --force no-such-session"
    Then the command fails
    And the output contains "not found"

  Scenario: destroy without --force aborts when user says no
    Given a session "my-feature" has been started
    When I run "am destroy my-feature" with input "n"
    Then the command succeeds
    And the output contains "Aborted"
    And a worktree exists at ".am/worktrees/my-feature"
    And the session file contains "my-feature"

  Scenario: destroy without --force proceeds when user confirms
    Given a session "my-feature" has been started
    When I run "am destroy my-feature" with input "y"
    Then the command succeeds
    And the worktree ".am/worktrees/my-feature" does not exist
    And the session file does not contain "my-feature"
