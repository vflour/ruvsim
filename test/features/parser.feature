Feature: Parser classification
  As a developer I want the parser to classify different kinds of lines so I can react to prompts, logs and output

  Scenario: Classify TCL comment line
    Given a parser with input "# this is a tcl comment\n"
    When I parse lines
    Then buffer has 1 lines
    And last line content is "# this is a tcl comment"
    And last line type is Log

  Scenario: Detect VSIM prompt
    Given a parser with input "VSIM 42>"
    When I parse lines
    Then buffer has 1 lines
    And last line content is "VSIM 42>"
    And last line type is Prompt

  Scenario: Parse multiple mixed lines
    Given a parser with input "Hello world\n# comment\nVSIM 5>"
    When I parse lines
    Then buffer has 3 lines
    And last line type is Prompt
