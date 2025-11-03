Feature: VSIM integration via Runner
  As a developer I want to drive vsim through the Runner so I can validate end-to-end behavior

  Scenario: Start fake vsim and see startup log
    Given a vsim runner
    When I wait for vsim prompt
    Then a log line contains "Fake VSIM started"
    And I stop the runner

  Scenario: run -all completes
    Given a vsim runner
    When I wait for vsim prompt
    And I send command "run -all"
    Then a log line contains "run finished"
    And I stop the runner

  Scenario: find nets lists top-level nets
    Given a vsim runner
    When I wait for vsim prompt
    And I send command "find nets -in /*"
    Then a log line contains "/Dummy/clk"
    And a log line contains "/Dummy/a"
    And a log line contains "/Dummy/rst"
    And I stop the runner

  Scenario: describe returns a range
    Given a vsim runner
    When I wait for vsim prompt
    And I send command "describe /Dummy/a"
    Then a log line contains "Register [7:0]"
    And I stop the runner

  Scenario: report returns a numeric value
    Given a vsim runner
    When I wait for vsim prompt
    And I send command "report"
    Then a log line contains "123"
    And I stop the runner
