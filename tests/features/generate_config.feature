Feature: am generate-config — print global config template

  Scenario: prints a TOML config template with all sections
    When I run "am generate-config"
    Then the command succeeds
    And the output contains "[container]"
    And the output contains "[tmux]"
    And the output contains "AM_CONTAINER_USER"
    And the output contains "username inside the container"
