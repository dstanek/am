Feature: am start and am attach with tmux

  Background:
    Given a git repository
    And I am inside a tmux session

  Scenario: start creates a named window and splits it
    When I run "am start my-feature"
    Then the command succeeds
    And the output contains "Started session 'my-feature'"
    And the mock tmux log contains "new-window"
    And the mock tmux log contains "split-window"

  Scenario: attach switches to the session window
    Given a session "my-feature" has been started
    When I run "am attach my-feature"
    Then the command succeeds
    And the output contains "my-feature"
    And the mock tmux log contains "select-window"

  Scenario: run sends an agent command to the session's agent pane
    Given a session "my-feature" has been started
    When I run "am run my-feature claude"
    Then the command succeeds
    And the output contains "Launched 'claude'"
    And the mock tmux log contains "send-keys"
    And the mock tmux log contains "claude"
